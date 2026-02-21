use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct FilerConfig {
    pub storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub s3: Option<S3Config>,
    pub local: Option<LocalConfig>,
}

#[derive(Debug, Deserialize)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(default)]
    pub allow_anonymous: bool,
}

#[derive(Debug, Deserialize)]
pub struct LocalConfig {
    pub root: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let yaml = r#"
storage:
  s3:
    endpoint: http://localhost:9000
    access_key_id: minioadmin
    secret_access_key: minioadmin
  local:
    root: /data
"#;
        let config: FilerConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.storage.s3.is_some());
        assert!(config.storage.local.is_some());
    }
}
