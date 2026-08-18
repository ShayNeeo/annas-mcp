use std::path::PathBuf;
use directories::UserDirs;

pub const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
pub const DEFAULT_WIKIPEDIA_URL: &str = "https://en.wikipedia.org/wiki/Anna%27s_Archive";
pub const DEFAULT_SLUM_STATUS_URL: &str = "https://open-slum.org/";
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;
pub const DEFAULT_FALLBACK_MIRROR: &str = "annas-archive.gl";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub secret_key: Option<String>,
    pub download_path: PathBuf,
    pub base_url: Option<String>,
    pub wikipedia_url: String,
    pub slum_status_url: String,
    pub timeout_secs: u64,
    pub user_agent: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        let secret_key = std::env::var("ANNAS_SECRET_KEY")
            .or_else(|_| std::env::var("ANNAS_ARCHIVE_API_KEY"))
            .ok()
            .and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() { None } else { Some(trimmed) }
            });

        let download_path = std::env::var("ANNAS_DOWNLOAD_PATH")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                UserDirs::new()
                    .and_then(|dirs| dirs.download_dir().map(PathBuf::from))
                    .unwrap_or_else(|| PathBuf::from("."))
            });

        let base_url = std::env::var("ANNAS_BASE_URL")
            .ok()
            .and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() { None } else { Some(trimmed) }
            });

        let wikipedia_url = std::env::var("ANNAS_WIKI_URL")
            .unwrap_or_else(|_| DEFAULT_WIKIPEDIA_URL.to_string());

        let slum_status_url = std::env::var("ANNAS_SLUM_URL")
            .unwrap_or_else(|_| DEFAULT_SLUM_STATUS_URL.to_string());

        let timeout_secs = std::env::var("ANNAS_TIMEOUT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        Self {
            secret_key,
            download_path,
            base_url,
            wikipedia_url,
            slum_status_url,
            timeout_secs,
            user_agent: DEFAULT_USER_AGENT.to_string(),
        }
    }
}
