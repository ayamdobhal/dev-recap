use std::path::PathBuf;
use thiserror::Error;

/// Main error type for dev-recap
#[derive(Error, Debug)]
pub enum DevRecapError {
    /// Git-related errors
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// TOML parsing errors
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    /// TOML serialization errors
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    /// HTTP/API errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Claude API errors
    #[error("Claude API error: {0}")]
    ClaudeApi(String),

    /// Caching errors
    #[error("Cache error: {0}")]
    Cache(#[from] sled::Error),

    /// Repository not found
    #[error("Repository not found at path: {0}")]
    #[allow(dead_code)]
    RepositoryNotFound(PathBuf),

    /// No commits found
    #[error("No commits found for author {author} in timespan")]
    NoCommitsFound { author: String },

    /// Invalid timespan
    #[error("Invalid timespan: {0}")]
    #[allow(dead_code)]
    InvalidTimespan(String),

    /// Missing configuration
    #[error("Missing required configuration: {0}")]
    MissingConfig(String),

    /// Regex errors
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Generic error
    #[error("{0}")]
    #[allow(dead_code)]
    Other(String),
}

/// Result type alias for dev-recap operations
pub type Result<T> = std::result::Result<T, DevRecapError>;

impl DevRecapError {
    /// Create a new configuration error
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }

    /// Create a new Claude API error
    pub fn claude_api<S: Into<String>>(msg: S) -> Self {
        Self::ClaudeApi(msg.into())
    }

    /// Create a new generic error
    #[allow(dead_code)]
    pub fn other<S: Into<String>>(msg: S) -> Self {
        Self::Other(msg.into())
    }
}
