use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// CreateTaskResponse describes a response from the CreateTask endpoint
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "CreateTaskResponse describes a response from the CreateTask endpoint. It will include the task ID that can be used to look up the status of the job.", example = json!({
    "id": "task-12345"
}))]
pub struct TesCreateTaskResponse {
    /// Task identifier assigned by the server.
    #[serde(rename = "id")]
    #[schema(example = "task-12345")]
    pub id: String,
}

impl TesCreateTaskResponse {
    /// CreateTaskResponse describes a response from the CreateTask endpoint. It will include the task ID that can be used to look up the status of the job.
    pub fn new(id: String) -> TesCreateTaskResponse {
        TesCreateTaskResponse { id }
    }
}
