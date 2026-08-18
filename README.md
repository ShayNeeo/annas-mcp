# Anna's Archive MCP Server & CLI (Rust)

A blazing-fast, robust, and unified **Model Context Protocol (MCP)** server and command-line interface for [Anna's Archive](https://annas-archive.li), written in pure Rust.

This project merges and synthesizes the best innovations from three prominent open-source Anna's Archive implementations:
1. **[iosifache/annas-mcp](https://github.com/iosifache/annas-mcp)** (Go) – Dynamic SLUM mirror discovery, SciDB DOI paper lookup and direct download, and CLI ergonomics.
2. **[Abhinav-Prabhakar/AnnasArchiveMCP](https://github.com/Abhinav-Prabhakar/AnnasArchiveMCP)** (Rust) – Rust MCP stdio design and structured domain tooling.
3. **[remikalbe/annas-archive-mcp](https://github.com/remikalbe/annas-archive-mcp)** (Rust) – Deep Elasticsearch record metadata parser (`/db/aarecord_elasticsearch/md5:{md5}.json`) and scraper heuristics.

---

## ✨ Features

- ⚡ **Model Context Protocol (MCP) Support**: Compatible with Claude Desktop, Cursor, Zed, Cline, Open WebUI, and any MCP-compliant client over stdio.
- 📖 **Authoritative Wikipedia & SLUM Mirror Resolver**: Automatically queries the official [Anna's Archive Wikipedia entry](https://en.wikipedia.org/wiki/Anna%27s_Archive) to fetch the latest active domains (`.pk`, `.gd`, `.gl`), cross-verifying with [open-slum.org](https://open-slum.org/) heartbeat telemetry and health probes.
- 📚 **Direct Book Search & Scraping**: Search books by title, author, topic, or ISBN with no third-party API subscription required.
- 🔄 **Automated Multi-Tier Fallback Downloads**: Automatically downloads books via Fast API if a key is provided, or seamlessly falls back across free decentralized **IPFS gateways** (Cloudflare, IPFS.io, Pinata, DWeb), **Libgen & Library.lol partner mirrors**, and **Anna's Archive Slow Download queue**.
- 🔬 **Academic Paper & SciDB DOI Engine**: Auto-detects DOIs (e.g. `10.1038/nature12373`), looks up publication details via Sci-Hub and SciDB, and enables direct paper downloads.
- 📦 **Safe Streaming Downloader**: Atomic `.tmp` writing, Content-Type & HTML challenge validation, and safe filename sanitization with path-traversal protection.
- 🔍 **Deep Record Metadata**: Query full metadata records including ISBN-10/13, ASIN, DOI, IPFS CIDs, torrent paths, and classifications.
- 💻 **Dual Mode**: Run either as a background MCP server (`annas-mcp mcp`) or as an interactive CLI utility.

---

## 🛠️ Requirements & Environment Variables

| Variable | Description | Required For |
|---|---|---|
| `ANNAS_SECRET_KEY` | Your secret API key from Anna's Archive (or `ANNAS_ARCHIVE_API_KEY`) | Fast book downloads |
| `ANNAS_DOWNLOAD_PATH` | Local directory where files will be saved (defaults to `~/Downloads`) | Downloads |
| `ANNAS_BASE_URL` | Override default mirror selection (e.g. `annas-archive.li`) | Optional |
| `ANNAS_TIMEOUT` | HTTP request timeout in seconds (default: `60`) | Optional |

> **Note**: Search and SciDB academic paper downloads work **100% free** without any API key or subscription. Fast book downloads require an active Anna's Archive membership key.

---

## 🚀 Installation & Building

```bash
# Clone the repository
git clone https://github.com/ShayNeeo/annas-mcp.git
cd annas-mcp

# Build release binary
cargo build --release

# Binary is located at target/release/annas-mcp
```

---

## 🤖 MCP Server Setup

### Claude Desktop Configuration

Add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "annas-archive": {
      "command": "/path/to/annas-mcp",
      "args": ["mcp"],
      "env": {
        "ANNAS_SECRET_KEY": "your_secret_key_here",
        "ANNAS_DOWNLOAD_PATH": "/path/to/downloads"
      }
    }
  }
}
```

### Available MCP Tools

1. `book_search`: Search Anna's Archive for books by keywords, author, or topic.
2. `book_download`: Download a book by its MD5 hash to `ANNAS_DOWNLOAD_PATH`.
3. `article_search`: Search academic papers and journal articles by keyword or DOI.
4. `article_download`: Download academic paper by DOI (via SciDB or Fast Download).
5. `get_item_details`: Fetch complete Elasticsearch record metadata (ISBN, IPFS CID, torrent paths).
6. `search_and_download_book`: Search and automatically download the top matching book.
7. `mirror_status`: Inspect current active mirror and SLUM health metrics.

---

## 💻 CLI Usage

```bash
# Start MCP server
annas-mcp mcp

# Search for books
annas-mcp book-search "Rust Programming" --limit 5

# Download a book by MD5
annas-mcp book-download 3f25b6a7b3... --filename "Rust_Book.epub"

# Search for articles or look up DOI
annas-mcp article-search "10.1038/nature12373"

# Download paper by DOI directly via SciDB
annas-mcp article-download "10.1038/nature12373"

# Get deep record details
annas-mcp details 3f25b6a7b3...

# Check live mirror health status from SLUM
annas-mcp mirror-status
```

---

## 📜 Credits & Acknowledgments

This unified implementation is built upon the invaluable contributions and ideas of the following creators and projects:

- **[Iosifache](https://github.com/iosifache)** for [iosifache/annas-mcp](https://github.com/iosifache/annas-mcp):
  - Original design of the SLUM dynamic mirror resolver, heartbeat parsing, and ranking algorithm.
  - SciDB DOI lookup and article download integration.
  - CLI architecture and workflow design.

- **[Abhinav Prabhakar](https://github.com/Abhinav-Prabhakar)** for [Abhinav-Prabhakar/AnnasArchiveMCP](https://github.com/Abhinav-Prabhakar/AnnasArchiveMCP):
  - Rust MCP server architecture, structured domain candidate ranking, and error design.

- **[Remi Kalbe](https://github.com/remikalbe)** for [remikalbe/annas-archive-mcp](https://github.com/remikalbe/annas-archive-mcp):
  - Elasticsearch JSON record metadata parsing (`/db/aarecord_elasticsearch/md5:{md5}.json`).
  - Scraper heuristics for robust HTML parsing.

---

## ⚖️ License

Distributed under the MIT License. See [LICENSE](LICENSE) for details.
