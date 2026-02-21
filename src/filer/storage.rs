use crate::filer::error::{FilerError, Result};
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn get(&self, path: &str) -> Result<Bytes>;
    async fn put(&self, path: &str, data: Bytes) -> Result<()>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn delete(&self, path: &str) -> Result<()>;
}

#[derive(Default)]
pub struct StorageRegistry {
    backends: HashMap<String, Arc<dyn Storage>>,
}

impl StorageRegistry {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    pub fn register(&mut self, scheme: &str, storage: Arc<dyn Storage>) {
        self.backends.insert(scheme.to_lowercase(), storage);
    }

    pub fn get(&self, scheme: &str) -> Option<Arc<dyn Storage>> {
        self.backends.get(&scheme.to_lowercase()).cloned()
    }

    pub fn schemes(&self) -> impl Iterator<Item = &String> {
        self.backends.keys()
    }

    pub fn len(&self) -> usize {
        self.backends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }
}

pub fn extract_scheme(url: &str) -> Result<&str> {
    url.split("://")
        .next()
        .filter(|s| !s.is_empty() && !s.contains('/'))
        .ok_or_else(|| FilerError::invalid_url(url, "missing or invalid scheme"))
}

pub fn extract_path(url: &str) -> Result<&str> {
    url.find("://")
        .map(|idx| &url[idx + 3..])
        .filter(|s| !s.is_empty())
        .ok_or_else(|| FilerError::invalid_url(url, "missing path component"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_scheme() {
        assert_eq!(extract_scheme("s3://bucket/key").unwrap(), "s3");
        assert_eq!(extract_scheme("file:///path/to/file").unwrap(), "file");
        assert!(extract_scheme("/no/scheme").is_err());
        assert!(extract_scheme("noscheme").is_err());
    }

    #[test]
    fn test_extract_path() {
        assert_eq!(extract_path("s3://bucket/key").unwrap(), "bucket/key");
        assert_eq!(
            extract_path("file:///path/to/file").unwrap(),
            "/path/to/file"
        );
        assert!(extract_path("s3://").is_err());
    }
}
