use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FilerError {
    #[error("Invalid URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },

    #[error("Unsupported URL scheme '{0}': only s3:// is supported")]
    UnsupportedScheme(String),

    #[error("Backend operation failed for '{key}': {message}")]
    Backend { key: String, message: String },

    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Input has neither URL nor content: path={0}")]
    MissingInputSource(String),

    #[error("Glob pattern error: {0}")]
    GlobPattern(String),

    #[error("Multiple errors occurred: {0:?}")]
    Multiple(Vec<Self>),
}

impl FilerError {
    pub fn invalid_url(url: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidUrl {
            url: url.into(),
            reason: reason.into(),
        }
    }

    pub fn backend(key: impl Into<String>, err: impl std::fmt::Display) -> Self {
        Self::Backend {
            key: key.into(),
            message: err.to_string(),
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, FilerError>;
