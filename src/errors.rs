use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),

    #[error("HTML parsing error: {0}")]
    HtmlParse(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Mirror resolution error: {0}")]
    MirrorResolution(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Missing configuration: {0}")]
    Config(String),

    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
