use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Resources describes the resources requested by a task
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "Resources required by a task", example = json!({
    "cpu_cores": 2,
    "preemptible": false,
    "ram_gb": 4.0,
    "disk_gb": 100.0,
    "zones": ["us-west1-a", "us-west1-b"]
}))]
pub struct TesResources {
    /// Requested number of CPUs
    #[serde(rename = "cpu_cores", skip_serializing_if = "Option::is_none")]
    #[schema(example = 2)]
    pub cpu_cores: Option<i32>,
    /// Define if the task is allowed to run on preemptible compute instances, for example, AWS Spot. This option may have no effect when utilized on some backends that don't have the concept of preemptible jobs.
    #[serde(rename = "preemptible", skip_serializing_if = "Option::is_none")]
    pub preemptible: Option<bool>,
    /// Requested RAM required in gigabytes (GB)
    #[serde(rename = "ram_gb", skip_serializing_if = "Option::is_none")]
    #[schema(example = 4.0)]
    pub ram_gb: Option<f64>,
    /// Requested disk size in gigabytes (GB)
    #[serde(rename = "disk_gb", skip_serializing_if = "Option::is_none")]
    #[schema(example = 100.0)]
    pub disk_gb: Option<f64>,
    /// Request that the task be run in these compute zones. How this string is utilized will be dependent on the backend system. For example, a system based on a cluster queueing system may use this string to define priority queue to which the job is assigned.
    #[serde(rename = "zones", skip_serializing_if = "Option::is_none")]
    pub zones: Option<Vec<String>>,
    /// Key/value pairs for backend configuration. ServiceInfo shall return a list of keys that a backend supports. Keys are case insensitive. It is expected that clients pass all runtime or hardware requirement key/values that are not mapped to existing tesResources properties to backend_parameters. Backends shall log system warnings if a key is passed that is unsupported. Backends shall not store or return unsupported keys if included in a task. If backend_parameters_strict equals true, backends should fail the task if any key/values are unsupported, otherwise, backends should attempt to run the task Intended uses include VM size selection, coprocessor configuration, etc. Example: ``` {   \"backend_parameters\" : {     \"VmSize\" : \"Standard_D64_v3\"   } } ```
    #[serde(rename = "backend_parameters", skip_serializing_if = "Option::is_none")]
    pub backend_parameters: Option<std::collections::HashMap<String, String>>,
    /// If set to true, backends should fail the task if any backend_parameters key/values are unsupported, otherwise, backends should attempt to run the task
    #[serde(
        rename = "backend_parameters_strict",
        skip_serializing_if = "Option::is_none"
    )]
    pub backend_parameters_strict: Option<bool>,
}

impl TesResources {
    /// Resources describes the resources requested by a task.
    pub fn new() -> TesResources {
        TesResources {
            cpu_cores: None,
            preemptible: None,
            ram_gb: None,
            disk_gb: None,
            zones: None,
            backend_parameters: None,
            backend_parameters_strict: None,
        }
    }
}
