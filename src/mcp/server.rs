use std::sync::Arc;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

use crate::client::AnnaClient;
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::tools::ToolManager;

pub struct McpServer {
    tool_manager: ToolManager,
}

impl McpServer {
    pub fn new(client: Arc<AnnaClient>) -> Self {
        let tool_manager = ToolManager::new(client);
        Self { tool_manager }
    }

    pub async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Starting Anna's Archive MCP Server over stdio...");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin).lines();

        while let Some(line) = reader.next_line().await? {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }

            debug!("Received JSON-RPC request: {}", line_trimmed);

            let request: JsonRpcRequest = match serde_json::from_str(line_trimmed) {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to parse JSON-RPC line: {e}");
                    let err_resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {e}"));
                    let json_line = serde_json::to_string(&err_resp)? + "\n";
                    stdout.write_all(json_line.as_bytes()).await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            // Notification check (no response if id is None and it's a notification)
            let is_notification = request.id.is_none();
            let req_id = request.id.clone();

            let response = self.handle_request(request).await;

            if let Some(resp) = response {
                let json_line = serde_json::to_string(&resp)? + "\n";
                stdout.write_all(json_line.as_bytes()).await?;
                stdout.flush().await?;
            } else if !is_notification {
                // If it was a request expecting a response but handler returned None
                let fallback = JsonRpcResponse::success(req_id, json!({}));
                let json_line = serde_json::to_string(&fallback)? + "\n";
                stdout.write_all(json_line.as_bytes()).await?;
                stdout.flush().await?;
            }
        }

        info!("MCP Server stdin closed. Exiting.");
        Ok(())
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let method = request.method.as_str();
        let id = request.id.clone();

        match method {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    },
                    "serverInfo": {
                        "name": "annas-archive-mcp",
                        "version": env!("CARGO_PKG_VERSION"),
                        "description": "Unified Anna's Archive MCP Server (Search, SciDB/DOI Lookup, Fast Downloads, SLUM Mirror Discovery)"
                    },
                    "instructions": "Search and retrieve books, academic articles, DOIs, and metadata from Anna's Archive. Downloads require ANNAS_SECRET_KEY."
                });
                Some(JsonRpcResponse::success(id, result))
            }

            "notifications/initialized" | "initialized" => {
                debug!("Client initialized notification received");
                None
            }

            "ping" => Some(JsonRpcResponse::success(id, json!({}))),

            "tools/list" => {
                let tools = self.tool_manager.list_tools();
                let result = json!({
                    "tools": tools
                });
                Some(JsonRpcResponse::success(id, result))
            }

            "tools/call" => {
                let params = request.params.unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned();

                let result = self.tool_manager.call_tool(tool_name, arguments).await;
                Some(JsonRpcResponse::success(id, json!(result)))
            }

            other => {
                debug!("Unhandled method: {other}");
                if id.is_some() {
                    Some(JsonRpcResponse::error(
                        id,
                        -32601,
                        format!("Method not found: {other}"),
                    ))
                } else {
                    None
                }
            }
        }
    }
}
