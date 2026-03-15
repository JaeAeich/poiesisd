use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum BackendConfig {
    #[serde(rename = "s3")]
    S3(S3Config),
}

#[derive(Debug, Deserialize)]
pub struct S3Config {
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let yaml = r#"
type: s3
endpoint: http://localhost:9000
access_key_id: adminadmin
secret_access_key: adminadmin
bucket: data
"#;
        let config: BackendConfig = serde_yaml::from_str(yaml).unwrap();
        match &config {
            BackendConfig::S3(s3) => {
                assert_eq!(s3.bucket, "data");
                assert_eq!(s3.endpoint.as_deref(), Some("http://localhost:9000"));
            }
        }
    }
}
