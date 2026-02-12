use super::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// GA4GH service
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "GA4GH service", example = json!({
    "id": "org.ga4gh.myservice",
    "name": "My GA4GH Service",
    "type": {
        "group": "org.ga4gh",
        "artifact": "service-type",
        "version": "1.0.0"
    },
    "description": "This is a sample GA4GH service",
    "organization": {
        "name": "GA4GH",
        "url": "https://ga4gh.org"
    },
    "version": "1.0.0"
}))]
pub struct Service {
    /// Unique ID of this service. Reverse domain name notation is recommended, though not required. The identifier should attempt to be globally unique so it can be used in downstream aggregator services e.g. Service Registry.
    #[serde(rename = "id")]
    #[schema(example = "org.ga4gh.myservice")]
    pub id: String,
    /// Name of this service. Should be human readable.
    #[serde(rename = "name")]
    #[schema(example = "My GA4GH Service")]
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: Box<service_type::ServiceType>,
    /// Description of the service. Should be human readable and provide information about the service.
    #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
    #[schema(example = "This is a sample GA4GH service")]
    pub description: Option<String>,
    #[serde(rename = "organization")]
    pub organization: Box<service_organization::ServiceOrganization>,
    /// URL of the contact for the provider of this service, e.g. a link to a contact form (RFC 3986 format), or an email (RFC 2368 format).
    #[serde(rename = "contactUrl", skip_serializing_if = "Option::is_none")]
    #[schema(example = "mailto:contact@ga4gh.org")]
    pub contact_url: Option<String>,
    /// URL of the documentation of this service (RFC 3986 format). This should help someone learn how to use your service, including any specifics required to access data, e.g. authentication.
    #[serde(rename = "documentationUrl", skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://ga4gh.org/docs")]
    pub documentation_url: Option<String>,
    /// Timestamp describing when the service was first deployed and available (RFC 3339 format)
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2023-01-01T00:00:00Z")]
    pub created_at: Option<String>,
    /// Timestamp describing when the service was last updated (RFC 3339 format)
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2023-12-01T00:00:00Z")]
    pub updated_at: Option<String>,
    /// Environment the service is running in. Use this to distinguish between production, development and testing/staging deployments. Suggested values are prod, test, dev, staging. However this is advised and not enforced.
    #[serde(rename = "environment", skip_serializing_if = "Option::is_none")]
    #[schema(example = "prod")]
    pub environment: Option<String>,
    /// Version of the service being described. Semantic versioning is recommended, but other identifiers, such as dates or commit hashes, are also allowed. The version should be changed whenever the service is updated.
    #[serde(rename = "version")]
    #[schema(example = "1.0.0")]
    pub version: String,
}

impl Service {
    /// GA4GH service
    pub fn new(
        id: String,
        name: String,
        r#type: service_type::ServiceType,
        organization: service_organization::ServiceOrganization,
        version: String,
    ) -> Service {
        Service {
            id,
            name,
            r#type: Box::new(r#type),
            description: None,
            organization: Box::new(organization),
            contact_url: None,
            documentation_url: None,
            created_at: None,
            updated_at: None,
            environment: None,
            version,
        }
    }
}
