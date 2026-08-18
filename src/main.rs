use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use annas_mcp::cli::{run_cli, Cli};

#[tokio::main]
async fn main() {
    // Direct all tracing logs to stderr to keep stdout strictly for JSON-RPC in MCP mode
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("annas_mcp=info,warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stderr).with_target(false))
        .init();

    let cli = Cli::parse();

    if let Err(e) = run_cli(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
