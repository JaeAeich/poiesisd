use super::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// TesListTasksResponse : ListTasksResponse describes a response from the ListTasks endpoint.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TesListTasksResponse {
    /// List of tasks. These tasks will be based on the original submitted task document, but with other fields, such as the job state and logging info, added/changed as the job progresses.
    #[serde(rename = "tasks")]
    pub tasks: Vec<tes_task::TesTask>,
    /// Token used to return the next page of results. This value can be used in the `page_token` field of the next ListTasks request.
    #[serde(rename = "next_page_token", skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

impl TesListTasksResponse {
    /// ListTasksResponse describes a response from the ListTasks endpoint.
    pub fn new(tasks: Vec<tes_task::TesTask>) -> TesListTasksResponse {
        TesListTasksResponse {
            tasks,
            next_page_token: None,
        }
    }
}
