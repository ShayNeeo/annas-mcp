use std::path::PathBuf;
use std::sync::Arc;
use serde_json::{json, Value};
use tracing::{error, info};

use crate::client::AnnaClient;
use crate::mcp::protocol::{ToolCallResult, ToolDescriptor};

#[derive(Clone)]
pub struct ToolManager {
    client: Arc<AnnaClient>,
    descriptors: Vec<ToolDescriptor>,
}

impl ToolManager {
    pub fn new(client: Arc<AnnaClient>) -> Self {
        let descriptors = vec![
            ToolDescriptor {
                name: "book_search".to_string(),
                description: "Search Anna's Archive for books by title, author, or topic. Returns metadata including MD5 hash for downloading.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search keywords (e.g., book title, author, topic, ISBN)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Optional maximum number of results to return (default 10)"
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDescriptor {
                name: "book_download".to_string(),
                description: "Download a book by its MD5 hash from search results. Requires ANNAS_SECRET_KEY in environment or configuration.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "hash": {
                            "type": "string",
                            "description": "MD5 hash of the book from search results"
                        },
                        "title": {
                            "type": "string",
                            "description": "Book title to use for the saved file name"
                        },
                        "format": {
                            "type": "string",
                            "description": "File format/extension (e.g., epub, pdf, mobi)"
                        },
                        "download_dir": {
                            "type": "string",
                            "description": "Optional directory path to save the downloaded file (defaults to ANNAS_DOWNLOAD_PATH)"
                        }
                    },
                    "required": ["hash"]
                }),
            },
            ToolDescriptor {
                name: "article_search".to_string(),
                description: "Search for academic papers and journal articles by DOI or keywords. Auto-detects if input is a DOI (starts with 10.).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "DOI (e.g., '10.1038/nature12373') or search keywords"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Optional maximum number of results to return"
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDescriptor {
                name: "article_download".to_string(),
                description: "Download an academic article/paper by its DOI (uses SciDB direct download, or Fast Download if key is set).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "doi": {
                            "type": "string",
                            "description": "DOI of the academic paper (e.g., '10.1038/nature12373')"
                        },
                        "download_dir": {
                            "type": "string",
                            "description": "Optional directory path to save the downloaded paper (defaults to ANNAS_DOWNLOAD_PATH)"
                        }
                    },
                    "required": ["doi"]
                }),
            },
            ToolDescriptor {
                name: "get_item_details".to_string(),
                description: "Get detailed Elasticsearch metadata for an item by MD5 hash (ISBNs, DOIs, IPFS CIDs, torrent paths, classifications).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "md5": {
                            "type": "string",
                            "description": "MD5 hash of the item"
                        }
                    },
                    "required": ["md5"]
                }),
            },
            ToolDescriptor {
                name: "search_and_download_book".to_string(),
                description: "Search for a book by query and automatically download the top matching result.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query for the book"
                        },
                        "preferred_format": {
                            "type": "string",
                            "description": "Optional preferred format (e.g., 'epub' or 'pdf')"
                        },
                        "download_dir": {
                            "type": "string",
                            "description": "Optional directory to save the file"
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolDescriptor {
                name: "mirror_status".to_string(),
                description: "Check the active Anna's Archive mirror and SLUM health status of candidate mirrors.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];

        Self {
            client,
            descriptors,
        }
    }

    pub fn list_tools(&self) -> Vec<ToolDescriptor> {
        self.descriptors.clone()
    }

    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> ToolCallResult {
        let args = arguments.unwrap_or(Value::Object(serde_json::Map::new()));

        match name {
            "book_search" => self.handle_book_search(args).await,
            "book_download" => self.handle_book_download(args).await,
            "article_search" => self.handle_article_search(args).await,
            "article_download" => self.handle_article_download(args).await,
            "get_item_details" => self.handle_get_item_details(args).await,
            "search_and_download_book" => self.handle_search_and_download_book(args).await,
            "mirror_status" => self.handle_mirror_status(args).await,
            _ => ToolCallResult::error(format!("Unknown tool: {name}")),
        }
    }

    async fn handle_book_search(&self, args: Value) -> ToolCallResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => return ToolCallResult::error("Missing or empty 'query' parameter"),
        };

        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        info!("MCP Tool book_search called: query='{}'", query);

        match self.client.search_books(query, limit).await {
            Ok(books) => {
                if books.is_empty() {
                    ToolCallResult::text("No books found matching the search query.")
                } else {
                    let formatted = books
                        .iter()
                        .enumerate()
                        .map(|(i, b)| format!("--- Book #{} ---\n{}", i + 1, b.display_summary()))
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    ToolCallResult::text(formatted)
                }
            }
            Err(e) => {
                error!("book_search failed: {e}");
                ToolCallResult::error(format!("Search failed: {e}"))
            }
        }
    }

    async fn handle_book_download(&self, args: Value) -> ToolCallResult {
        let hash = match args.get("hash").and_then(|v| v.as_str()) {
            Some(h) if !h.trim().is_empty() => h.trim(),
            _ => return ToolCallResult::error("Missing or empty 'hash' parameter"),
        };

        let title = args
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("untitled");

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("bin");

        let custom_dir = args
            .get("download_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        info!("MCP Tool book_download called: hash='{}'", hash);

        match self
            .client
            .download_book(hash, title, format, custom_dir.as_deref())
            .await
        {
            Ok(info) => ToolCallResult::text(format!(
                "Book downloaded successfully!\nFile: {}\nSize: {} bytes\nSaved to: {}",
                info.filename,
                info.bytes_written,
                info.file_path.display()
            )),
            Err(e) => {
                error!("book_download failed: {e}");
                ToolCallResult::error(format!("Download failed: {e}"))
            }
        }
    }

    async fn handle_article_search(&self, args: Value) -> ToolCallResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => return ToolCallResult::error("Missing or empty 'query' parameter"),
        };

        let limit = args.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        info!("MCP Tool article_search called: query='{}'", query);

        match self.client.search_articles(query, limit).await {
            Ok(papers) => {
                if papers.is_empty() {
                    ToolCallResult::text("No articles or papers found matching query.")
                } else {
                    let formatted = papers
                        .iter()
                        .enumerate()
                        .map(|(i, p)| format!("--- Article #{} ---\n{}", i + 1, p.display_summary()))
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    ToolCallResult::text(formatted)
                }
            }
            Err(e) => {
                error!("article_search failed: {e}");
                ToolCallResult::error(format!("Article search failed: {e}"))
            }
        }
    }

    async fn handle_article_download(&self, args: Value) -> ToolCallResult {
        let doi = match args.get("doi").and_then(|v| v.as_str()) {
            Some(d) if !d.trim().is_empty() => d.trim(),
            _ => return ToolCallResult::error("Missing or empty 'doi' parameter"),
        };

        let custom_dir = args
            .get("download_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        info!("MCP Tool article_download called: doi='{}'", doi);

        match self
            .client
            .download_paper(doi, custom_dir.as_deref())
            .await
        {
            Ok(info) => ToolCallResult::text(format!(
                "Article downloaded successfully!\nFile: {}\nSize: {} bytes\nSaved to: {}",
                info.filename,
                info.bytes_written,
                info.file_path.display()
            )),
            Err(e) => {
                error!("article_download failed: {e}");
                ToolCallResult::error(format!("Article download failed: {e}"))
            }
        }
    }

    async fn handle_get_item_details(&self, args: Value) -> ToolCallResult {
        let md5 = match args.get("md5").and_then(|v| v.as_str()) {
            Some(h) if !h.trim().is_empty() => h.trim(),
            _ => return ToolCallResult::error("Missing or empty 'md5' parameter"),
        };

        info!("MCP Tool get_item_details called: md5='{}'", md5);

        match self.client.get_item_details(md5).await {
            Ok(details) => match serde_json::to_string_pretty(&details) {
                Ok(j) => ToolCallResult::text(j),
                Err(e) => ToolCallResult::error(format!("Serialization error: {e}")),
            },
            Err(e) => ToolCallResult::error(format!("Failed to retrieve item details: {e}")),
        }
    }

    async fn handle_search_and_download_book(&self, args: Value) -> ToolCallResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => return ToolCallResult::error("Missing or empty 'query' parameter"),
        };

        let pref_fmt = args
            .get("preferred_format")
            .and_then(|v| v.as_str())
            .map(|s| s.to_lowercase());

        let custom_dir = args
            .get("download_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);

        info!("MCP Tool search_and_download_book called: query='{}'", query);

        let books = match self.client.search_books(query, Some(5)).await {
            Ok(b) if !b.is_empty() => b,
            Ok(_) => return ToolCallResult::text(format!("No books found matching '{query}'.")),
            Err(e) => return ToolCallResult::error(format!("Search failed: {e}")),
        };

        let selected = if let Some(ref target_fmt) = pref_fmt {
            books
                .iter()
                .find(|b| {
                    b.format
                        .as_ref()
                        .map(|f| f.to_lowercase() == *target_fmt)
                        .unwrap_or(false)
                })
                .unwrap_or(&books[0])
        } else {
            &books[0]
        };

        let format = selected.format.as_deref().unwrap_or("pdf");
        match self
            .client
            .download_book(&selected.hash, &selected.title, format, custom_dir.as_deref())
            .await
        {
            Ok(info) => ToolCallResult::text(format!(
                "Successfully matched and downloaded book:\nTitle: {}\nFormat: {}\nHash: {}\nFile: {}\nPath: {}",
                selected.title,
                format,
                selected.hash,
                info.filename,
                info.file_path.display()
            )),
            Err(e) => ToolCallResult::error(format!(
                "Found book '{}' ({}) but download failed: {}",
                selected.title, selected.hash, e
            )),
        }
    }

    async fn handle_mirror_status(&self, _args: Value) -> ToolCallResult {
        let active = match self.client.get_active_mirror().await {
            Ok(m) => m,
            Err(e) => format!("Error resolving: {e}"),
        };

        let candidates = self
            .client
            .resolver()
            .fetch_and_rank_candidates()
            .await
            .unwrap_or_default();

        let mut lines = Vec::new();
        lines.push(format!("Active Mirror: https://{}", active));
        lines.push(format!("Discovered SLUM Candidates: {}", candidates.len()));
        lines.push(String::new());

        for (i, c) in candidates.iter().enumerate() {
            let score = c.score();
            lines.push(format!(
                "{}. https://{} (Success Rate: {:.1}%, Avg Ping: {}ms, Samples: {})",
                i + 1,
                c.base_url,
                score.success_rate * 100.0,
                score.average_ping,
                score.sample_count
            ));
        }

        ToolCallResult::text(lines.join("\n"))
    }
}
