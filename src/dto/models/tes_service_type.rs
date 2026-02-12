use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// TES service type information
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "TES service type information", example = json!({
    "group": "org.ga4gh",
    "artifact": "tes",
    "version": "1.1.0"
}))]
pub struct TesServiceType {
    /// Namespace in reverse domain name format. Use `org.ga4gh` for implementations compliant with official GA4GH specifications. For services with custom APIs not standardized by GA4GH, or implementations diverging from official GA4GH specifications, use a different namespace (e.g. your organization's reverse domain name).
    #[serde(rename = "group")]
    #[schema(example = "org.ga4gh")]
    pub group: String,
    #[serde(rename = "artifact")]
    pub artifact: Artifact,
    /// Version of the API or specification. GA4GH specifications use semantic versioning.
    #[serde(rename = "version")]
    #[schema(example = "1.1.0")]
    pub version: String,
}

impl TesServiceType {
    pub fn new(group: String, artifact: Artifact, version: String) -> TesServiceType {
        TesServiceType {
            group,
            artifact,
            version,
        }
    }
}
/// Artifact type for TES service
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    Default,
)]
#[schema(description = "Artifact type for TES service", example = "tes")]
pub enum Artifact {
    #[serde(rename = "tes")]
    #[default]
    Tes,
}
