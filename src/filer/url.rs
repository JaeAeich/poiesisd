use crate::filer::error::{FilerError, Result};
use tracing::warn;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ParsedUrl<'a> {
    pub scheme: &'a str,
    pub bucket: &'a str,
    pub key: &'a str,
}

/// Parses a storage URL like `s3://bucket/path/to/key` into its components.
pub fn parse_storage_url(url: &str) -> Result<ParsedUrl<'_>> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| FilerError::invalid_url(url, "missing :// scheme separator"))?;

    if scheme != "s3" {
        return Err(FilerError::UnsupportedScheme(scheme.to_string()));
    }

    let idx = rest
        .find('/')
        .ok_or_else(|| FilerError::invalid_url(url, "missing object key"))?;

    let bucket = &rest[..idx];
    let key = &rest[idx + 1..];

    if key.is_empty() {
        return Err(FilerError::invalid_url(url, "missing object key"));
    }

    Ok(ParsedUrl {
        scheme,
        bucket,
        key,
    })
}

/// Warns if the URL's bucket differs from the configured bucket.
pub fn warn_bucket_mismatch(url_bucket: &str, configured_bucket: &str) {
    if url_bucket != configured_bucket {
        warn!(
            url_bucket,
            configured_bucket,
            "URL references a different bucket than configured; using configured bucket"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_storage_url() {
        let parsed = parse_storage_url("s3://bucket/key").unwrap();
        assert_eq!(parsed.scheme, "s3");
        assert_eq!(parsed.bucket, "bucket");
        assert_eq!(parsed.key, "key");

        let parsed = parse_storage_url("s3://bucket/path/to/file.txt").unwrap();
        assert_eq!(parsed.key, "path/to/file.txt");

        assert!(parse_storage_url("s3://bucket/").is_err());
        assert!(parse_storage_url("s3://bucket").is_err());
        assert!(parse_storage_url("file:///path").is_err());
        assert!(parse_storage_url("noscheme").is_err());
    }
}
