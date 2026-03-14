use crate::filer::backend::StorageBackend;
use crate::filer::config::S3Config;
use crate::filer::error::{FilerError, Result};
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use bytes::Bytes;

pub struct S3Backend {
    client: Client,
    bucket: String,
}

impl S3Backend {
    pub fn from_config(config: &S3Config) -> Result<Self> {
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None,
            None,
            "env",
        );

        let mut s3_config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .force_path_style(true);

        if let Some(ref endpoint) = config.endpoint {
            s3_config = s3_config.endpoint_url(endpoint);
        }

        s3_config = s3_config.region(Region::new(
            config
                .region
                .clone()
                .unwrap_or_else(|| "us-east-1".to_string()),
        ));

        let client = Client::from_conf(s3_config.build());

        Ok(Self {
            client,
            bucket: config.bucket.clone(),
        })
    }
}

impl StorageBackend for S3Backend {
    async fn get(&self, key: &str) -> Result<Bytes> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| FilerError::backend(key, e))?;

        let data = resp
            .body
            .collect()
            .await
            .map_err(|e| FilerError::backend(key, e))?;

        Ok(data.into_bytes())
    }

    async fn put(&self, key: &str, data: Bytes) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await
            .map_err(|e| FilerError::backend(key, e))?;

        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);

            if let Some(token) = &continuation_token {
                req = req.continuation_token(token);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| FilerError::backend(prefix, e))?;

            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    keys.push(key.to_string());
                }
            }

            if resp.is_truncated() == Some(true) {
                continuation_token = resp.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(keys)
    }
}
