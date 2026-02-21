use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum FilerError {
    #[error("Invalid URL '{url}': {reason}")]
    InvalidUrl { url: String, reason: String },

    #[error("Unsupported URL scheme: {0}")]
    UnsupportedScheme(String),

    #[error("No backend registered for scheme: {0}")]
    BackendNotFound(String),

    #[error("Storage operation failed for '{url}': {source}")]
    StorageFailed { url: String, source: opendal::Error },

    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("Workspace path does not exist: {0}")]
    WorkspaceNotFound(PathBuf),

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

    pub fn storage_failed(url: impl Into<String>, source: opendal::Error) -> Self {
        Self::StorageFailed {
            url: url.into(),
            source,
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, FilerError>;
