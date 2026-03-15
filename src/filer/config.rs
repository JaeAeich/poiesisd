use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub backend: BackendConfig,
    #[serde(default)]
    pub service: ServiceConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    #[serde(default = "default_service_id")]
    pub id: String,
    #[serde(default = "default_service_name")]
    pub name: String,
    #[serde(default = "default_org_name")]
    pub org_name: String,
    #[serde(default = "default_org_url")]
    pub org_url: String,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            id: default_service_id(),
            name: default_service_name(),
            org_name: default_org_name(),
            org_url: default_org_url(),
        }
    }
}

fn default_service_id() -> String {
    "poiesisd".to_string()
}
fn default_service_name() -> String {
    "poiesisD TES".to_string()
}
fn default_org_name() -> String {
    "poiesisD".to_string()
}
fn default_org_url() -> String {
    "https://github.com/poiesisd".to_string()
}

#[derive(Debug, Deserialize)]
pub struct FilerConfig {
    pub backend: BackendConfig,
}

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
backend:
  type: s3
  endpoint: http://localhost:9000
  access_key_id: adminadmin
  secret_access_key: adminadmin
  bucket: data
"#;
        let config: FilerConfig = serde_yaml::from_str(yaml).unwrap();
        match &config.backend {
            BackendConfig::S3(s3) => {
                assert_eq!(s3.bucket, "data");
                assert_eq!(s3.endpoint.as_deref(), Some("http://localhost:9000"));
            }
        }
    }
}
