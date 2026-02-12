use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Type of a GA4GH service
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "Type of a GA4GH service", example = json!({
    "group": "org.ga4gh",
    "artifact": "tes",
    "version": "1.1.0"
}))]
pub struct ServiceType {
    /// Namespace in reverse domain name format. Use `org.ga4gh` for implementations compliant with official GA4GH specifications. For services with custom APIs not standardized by GA4GH, or implementations diverging from official GA4GH specifications, use a different namespace (e.g. your organization's reverse domain name).
    #[serde(rename = "group")]
    #[schema(example = "org.ga4gh")]
    pub group: String,
    /// Name of the API or GA4GH specification implemented. Official GA4GH types should be assigned as part of standards approval process. Custom artifacts are supported.
    #[serde(rename = "artifact")]
    pub artifact: String,
    /// Version of the API or specification. GA4GH specifications use semantic versioning.
    #[serde(rename = "version")]
    #[schema(example = "1.1.0")]
    pub version: String,
}

impl ServiceType {
    /// Type of a GA4GH service
    pub fn new(group: String, artifact: String, version: String) -> ServiceType {
        ServiceType {
            group,
            artifact,
            version,
        }
    }
}
