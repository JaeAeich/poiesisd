use crate::filer::error::{FilerError, Result};
use crate::filer::storage::Storage;
use async_trait::async_trait;
use bytes::Bytes;
use opendal::layers::RetryLayer;
use opendal::{Operator, services::Fs};
use std::path::Path;

pub struct LocalStorage {
    operator: Operator,
}

impl LocalStorage {
    pub fn new(root: &Path) -> Result<Self> {
        let root_str = root
            .to_str()
            .ok_or_else(|| FilerError::config("root path contains invalid UTF-8"))?;

        let builder = Fs::default().root(root_str);

        let operator = Operator::new(builder)
            .map_err(|e| FilerError::config(format!("failed to create local operator: {}", e)))?
            .layer(RetryLayer::new())
            .finish();

        Ok(Self { operator })
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn get(&self, path: &str) -> Result<Bytes> {
        let data = self
            .operator
            .read(path)
            .await
            .map_err(|e| FilerError::storage_failed(path, e))?;
        Ok(data.to_bytes())
    }

    async fn put(&self, path: &str, data: Bytes) -> Result<()> {
        self.operator
            .write(path, data)
            .await
            .map_err(|e| FilerError::storage_failed(path, e))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let mut entries = Vec::new();
        let mut lister = self
            .operator
            .lister(prefix)
            .await
            .map_err(|e| FilerError::storage_failed(prefix, e))?;

        use futures::StreamExt;
        while let Some(entry) = lister.next().await {
            let entry = entry.map_err(|e| FilerError::storage_failed(prefix, e))?;
            entries.push(entry.path().to_string());
        }

        Ok(entries)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let meta = self.operator.stat(path).await;
        match meta {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(FilerError::storage_failed(path, e)),
        }
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.operator
            .delete(path)
            .await
            .map_err(|e| FilerError::storage_failed(path, e))
    }
}
