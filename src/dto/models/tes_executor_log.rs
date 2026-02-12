use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// ExecutorLog describes logging information related to an Executor
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "ExecutorLog describes logging information related to an Executor", example = json!({
    "start_time": "2023-01-01T00:00:00Z",
    "end_time": "2023-01-01T00:05:00Z",
    "stdout": "Hello World\n",
    "stderr": "",
    "exit_code": 0
}))]
pub struct TesExecutorLog {
    /// Time the executor started, in RFC 3339 format.
    #[serde(rename = "start_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2023-01-01T00:00:00Z")]
    pub start_time: Option<String>,
    /// Time the executor ended, in RFC 3339 format.
    #[serde(rename = "end_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2023-01-01T00:05:00Z")]
    pub end_time: Option<String>,
    /// Stdout content. This is meant for convenience. No guarantees are made about the content. Implementations may chose different approaches: only the head, only the tail, a URL reference only, etc.  In order to capture the full stdout users should set Executor.stdout to a container file path, and use Task.outputs to upload that file to permanent storage.
    #[serde(rename = "stdout", skip_serializing_if = "Option::is_none")]
    #[schema(example = "Hello World\n")]
    pub stdout: Option<String>,
    /// Stderr content. This is meant for convenience. No guarantees are made about the content. Implementations may chose different approaches: only the head, only the tail, a URL reference only, etc.  In order to capture the full stderr users should set Executor.stderr to a container file path, and use Task.outputs to upload that file to permanent storage.
    #[serde(rename = "stderr", skip_serializing_if = "Option::is_none")]
    #[schema(example = "The program exited with an error")]
    pub stderr: Option<String>,
    /// Exit code.
    #[serde(rename = "exit_code")]
    #[schema(example = 0)]
    pub exit_code: i32,
}

impl TesExecutorLog {
    /// ExecutorLog describes logging information related to an Executor.
    pub fn new(exit_code: i32) -> TesExecutorLog {
        TesExecutorLog {
            start_time: None,
            end_time: None,
            stdout: None,
            stderr: None,
            exit_code,
        }
    }
}
