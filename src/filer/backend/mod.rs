mod s3;

pub use s3::S3Backend;

use crate::filer::error::Result;
use bytes::Bytes;

pub trait StorageBackend: Send + Sync {
    fn get(&self, key: &str) -> impl std::future::Future<Output = Result<Bytes>> + Send;
    fn put(&self, key: &str, data: Bytes) -> impl std::future::Future<Output = Result<()>> + Send;
    fn list(&self, prefix: &str) -> impl std::future::Future<Output = Result<Vec<String>>> + Send;
}
