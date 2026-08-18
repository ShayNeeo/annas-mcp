use std::path::PathBuf;
use std::sync::Arc;
use clap::{Parser, Subcommand};
use tracing::info;

use crate::client::AnnaClient;
use crate::config::AppConfig;
use crate::mcp::McpServer;

#[derive(Parser, Debug)]
#[command(
    name = "annas-mcp",
    author = "Shayneeo (credits to iosifache, Abhinav Prabhakar, Remi Kalbe)",
    version,
    about = "Unified Anna's Archive MCP Server and CLI Tool with SLUM mirror resolver, SciDB/DOI lookups, and fast downloads."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, global = true, help = "Custom Anna's Archive mirror URL")]
    pub base_url: Option<String>,

    #[arg(long, global = true, help = "Anna's Archive Secret Key for fast downloads")]
    pub secret_key: Option<String>,

    #[arg(long, global = true, help = "Download destination folder")]
    pub download_path: Option<PathBuf>,

    #[arg(long, global = true, help = "HTTP request timeout in seconds")]
    pub timeout: Option<u64>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Start the Model Context Protocol (MCP) server over stdio")]
    Mcp,

    #[command(about = "Search for books by title, author, topic, or ISBN")]
    BookSearch {
        #[arg(help = "Search query")]
        query: String,

        #[arg(short, long, help = "Maximum number of results to return (default: 10)")]
        limit: Option<usize>,
    },

    #[command(about = "Download a book by its MD5 hash")]
    BookDownload {
        #[arg(help = "MD5 hash of the book")]
        hash: String,

        #[arg(
            short,
            long,
            help = "Target filename including extension (e.g. 'book.epub')"
        )]
        filename: Option<String>,

        #[arg(short, long, help = "Target output folder")]
        output_dir: Option<PathBuf>,
    },

    #[command(about = "Search for academic articles or look up a DOI")]
    ArticleSearch {
        #[arg(help = "Search query or DOI (e.g. '10.1038/nature12373')")]
        query: String,

        #[arg(short, long, help = "Maximum number of results to return")]
        limit: Option<usize>,
    },

    #[command(about = "Download an academic article or paper by its DOI")]
    ArticleDownload {
        #[arg(help = "DOI of the paper (e.g. '10.1038/nature12373')")]
        doi: String,

        #[arg(short, long, help = "Target output folder")]
        output_dir: Option<PathBuf>,
    },

    #[command(about = "Get deep Elasticsearch record metadata by MD5 hash")]
    Details {
        #[arg(help = "MD5 hash of the item")]
        hash: String,
    },

    #[command(about = "Check active mirror and SLUM candidate health status")]
    MirrorStatus,
}

pub async fn run_cli(cli: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut config = AppConfig::from_env();

    if let Some(base_url) = cli.base_url {
        config.base_url = Some(base_url);
    }
    if let Some(secret_key) = cli.secret_key {
        config.secret_key = Some(secret_key);
    }
    if let Some(download_path) = cli.download_path {
        config.download_path = download_path;
    }
    if let Some(timeout) = cli.timeout {
        config.timeout_secs = timeout;
    }

    let client = Arc::new(AnnaClient::new(config)?);

    match cli.command {
        Commands::Mcp => {
            let server = McpServer::new(client);
            server.run_stdio().await?;
        }

        Commands::BookSearch { query, limit } => {
            info!("Searching books for: '{}'", query);
            let books = client.search_books(&query, limit).await?;
            if books.is_empty() {
                println!("No books found.");
            } else {
                for (i, book) in books.iter().enumerate() {
                    println!("Book #{}:\n{}", i + 1, book.display_summary());
                    if i + 1 < books.len() {
                        println!("\n----------------------------------------\n");
                    }
                }
            }
        }

        Commands::BookDownload {
            hash,
            filename,
            output_dir,
        } => {
            let (title, format) = if let Some(ref fname) = filename {
                let p = std::path::Path::new(fname);
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("bin");
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("untitled");
                (stem.to_string(), ext.to_string())
            } else {
                (hash.clone(), "bin".to_string())
            };

            let out_dir = output_dir.as_deref();
            println!("Downloading book MD5: {} ...", hash);
            let result = client.download_book(&hash, &title, &format, out_dir).await?;
            println!(
                "Downloaded successfully!\nFile: {}\nSize: {} bytes\nSaved to: {}",
                result.filename,
                result.bytes_written,
                result.file_path.display()
            );
        }

        Commands::ArticleSearch { query, limit } => {
            info!("Searching articles for: '{}'", query);
            let papers = client.search_articles(&query, limit).await?;
            if papers.is_empty() {
                println!("No articles found.");
            } else {
                for (i, paper) in papers.iter().enumerate() {
                    println!("Article #{}:\n{}", i + 1, paper.display_summary());
                    if i + 1 < papers.len() {
                        println!("\n----------------------------------------\n");
                    }
                }
            }
        }

        Commands::ArticleDownload { doi, output_dir } => {
            let out_dir = output_dir.as_deref();
            println!("Downloading article DOI: {} ...", doi);
            let result = client.download_paper(&doi, out_dir).await?;
            println!(
                "Article downloaded successfully!\nFile: {}\nSize: {} bytes\nSaved to: {}",
                result.filename,
                result.bytes_written,
                result.file_path.display()
            );
        }

        Commands::Details { hash } => {
            println!("Fetching record details for MD5: {} ...", hash);
            let details = client.get_item_details(&hash).await?;
            let json_str = serde_json::to_string_pretty(&details)?;
            println!("{json_str}");
        }

        Commands::MirrorStatus => {
            let active = match client.get_active_mirror().await {
                Ok(m) => m,
                Err(e) => format!("Error resolving: {e}"),
            };

            let candidates = client
                .resolver()
                .fetch_and_rank_candidates()
                .await
                .unwrap_or_default();

            println!("=== Anna's Archive Mirror Status ===");
            println!("Active Mirror: https://{}", active);
            println!("Discovered SLUM Candidates: {}\n", candidates.len());

            for (i, c) in candidates.iter().enumerate() {
                let score = c.score();
                let status_str = if score.last_status == 1 {
                    "UP"
                } else if score.last_status == 0 {
                    "DOWN"
                } else {
                    "UNKNOWN"
                };

                println!(
                    "{:2}. https://{:<24} [{}] Success: {:5.1}% | Avg Ping: {:4}ms | Samples: {}",
                    i + 1,
                    c.base_url,
                    status_str,
                    score.success_rate * 100.0,
                    score.average_ping,
                    score.sample_count
                );
            }
        }
    }

    Ok(())
}
