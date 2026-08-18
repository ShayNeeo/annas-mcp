use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identifiers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn10: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isbn13: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asin: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_library: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_books: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goodreads: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsInfo {
    pub cid: String,
    pub from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSource {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDetails {
    pub md5: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<Identifiers>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipfs_cids: Option<Vec<IpfsInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_sources: Option<Vec<DownloadSource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torrent_paths: Option<Vec<String>>,
}
