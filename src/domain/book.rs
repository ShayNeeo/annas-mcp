use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub hash: String,
    pub title: String,
    pub authors: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub format: Option<String>,
    pub size: Option<String>,
    pub year: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Book {
    pub fn display_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Title: {}", self.title));
        if let Some(ref authors) = self.authors {
            lines.push(format!("Authors: {}", authors));
        }
        if let Some(ref publisher) = self.publisher {
            lines.push(format!("Publisher: {}", publisher));
        }
        if let Some(ref lang) = self.language {
            lines.push(format!("Language: {}", lang));
        }
        if let Some(ref format) = self.format {
            lines.push(format!("Format: {}", format));
        }
        if let Some(ref size) = self.size {
            lines.push(format!("Size: {}", size));
        }
        if let Some(ref year) = self.year {
            lines.push(format!("Year: {}", year));
        }
        lines.push(format!("URL: {}", self.url));
        lines.push(format!("MD5 Hash: {}", self.hash));
        lines.join("\n")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub doi: String,
    pub title: Option<String>,
    pub authors: Option<String>,
    pub journal: Option<String>,
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    pub download_url: Option<String>,
    pub page_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scihub_url: Option<String>,
}

impl Paper {
    pub fn display_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("DOI: {}", self.doi));
        if let Some(ref title) = self.title {
            lines.push(format!("Title: {}", title));
        }
        if let Some(ref authors) = self.authors {
            lines.push(format!("Authors: {}", authors));
        }
        if let Some(ref journal) = self.journal {
            lines.push(format!("Journal: {}", journal));
        }
        if let Some(ref size) = self.size {
            lines.push(format!("Size: {}", size));
        }
        if let Some(ref hash) = self.hash {
            lines.push(format!("MD5 Hash: {}", hash));
        }
        if let Some(ref dl) = self.download_url {
            lines.push(format!("Download URL: {}", dl));
        }
        lines.push(format!("Page: {}", self.page_url));
        lines.join("\n")
    }
}
