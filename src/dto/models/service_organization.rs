use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Organization providing the service
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "Organization providing the service", example = json!({
    "name": "GA4GH",
    "url": "https://ga4gh.org"
}))]
pub struct ServiceOrganization {
    /// Name of the organization responsible for the service
    #[serde(rename = "name")]
    #[schema(example = "GA4GH")]
    pub name: String,
    /// URL of the website of the organization (RFC 3986 format)
    #[serde(rename = "url")]
    #[schema(example = "https://ga4gh.org")]
    pub url: String,
}

impl ServiceOrganization {
    /// Organization providing the service
    pub fn new(name: String, url: String) -> ServiceOrganization {
        ServiceOrganization { name, url }
    }
}
