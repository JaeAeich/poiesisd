use crate::filer::error::{FilerError, Result};
use crate::filer::storage::Storage;
use async_trait::async_trait;
use bytes::Bytes;
use opendal::layers::RetryLayer;
use opendal::{Operator, services::S3};

pub struct S3Storage {
    endpoint: Option<String>,
    region: Option<String>,
    access_key_id: String,
    secret_access_key: String,
    allow_anonymous: bool,
}

impl S3Storage {
    pub fn new(
        endpoint: Option<&str>,
        region: Option<&str>,
        access_key_id: &str,
        secret_access_key: &str,
        allow_anonymous: bool,
    ) -> Result<Self> {
        Ok(Self {
            endpoint: endpoint.map(|s| s.to_string()),
            region: region.map(|s| s.to_string()),
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret_access_key.to_string(),
            allow_anonymous,
        })
    }

    fn parse_bucket_and_key<'a>(&self, path: &'a str) -> Result<(&'a str, &'a str)> {
        let (bucket, key) = path
            .split_once('/')
            .ok_or_else(|| FilerError::invalid_url(path, "expected bucket/key format"))?;

        if bucket.is_empty() {
            return Err(FilerError::invalid_url(path, "bucket name is empty"));
        }

        Ok((bucket, key))
    }

    fn create_operator(&self, bucket: &str) -> Result<Operator> {
        let mut builder = S3::default().bucket(bucket);

        if let Some(ref endpoint) = self.endpoint {
            builder = builder.endpoint(endpoint);
        }

        if let Some(ref region) = self.region {
            builder = builder.region(region);
        }

        if !self.allow_anonymous {
            builder = builder
                .access_key_id(&self.access_key_id)
                .secret_access_key(&self.secret_access_key);
        }

        let operator = Operator::new(builder)
            .map_err(|e| {
                FilerError::config(format!(
                    "failed to create S3 operator for bucket '{}': {}",
                    bucket, e
                ))
            })?
            .layer(RetryLayer::new())
            .finish();

        Ok(operator)
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn get(&self, path: &str) -> Result<Bytes> {
        let (bucket, key) = self.parse_bucket_and_key(path)?;
        let operator = self.create_operator(bucket)?;

        let data = operator
            .read(key)
            .await
            .map_err(|e| FilerError::storage_failed(path, e))?;

        Ok(data.to_bytes())
    }

    async fn put(&self, path: &str, data: Bytes) -> Result<()> {
        let (bucket, key) = self.parse_bucket_and_key(path)?;
        let operator = self.create_operator(bucket)?;

        operator
            .write(key, data)
            .await
            .map_err(|e| FilerError::storage_failed(path, e))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let (bucket, key_prefix) = self.parse_bucket_and_key(prefix)?;
        let operator = self.create_operator(bucket)?;

        let mut entries = Vec::new();
        let mut lister = operator
            .lister(key_prefix)
            .await
            .map_err(|e| FilerError::storage_failed(prefix, e))?;

        use futures::StreamExt;
        while let Some(entry) = lister.next().await {
            let entry = entry.map_err(|e| FilerError::storage_failed(prefix, e))?;
            entries.push(format!("{}/{}", bucket, entry.path()));
        }

        Ok(entries)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let (bucket, key) = self.parse_bucket_and_key(path)?;
        let operator = self.create_operator(bucket)?;

        match operator.stat(key).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == opendal::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(FilerError::storage_failed(path, e)),
        }
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let (bucket, key) = self.parse_bucket_and_key(path)?;
        let operator = self.create_operator(bucket)?;

        operator
            .delete(key)
            .await
            .map_err(|e| FilerError::storage_failed(path, e))
    }
}
