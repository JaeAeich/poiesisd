use serde::Deserialize;

use crate::filer::BackendConfig;

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
