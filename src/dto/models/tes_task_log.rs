use super::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// TesTaskLog : TaskLog describes logging information related to a Task.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TesTaskLog {
    /// Logs for each executor
    #[serde(rename = "logs")]
    pub logs: Vec<tes_executor_log::TesExecutorLog>,
    /// Arbitrary logging metadata included by the implementation.
    #[serde(rename = "metadata", skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
    /// When the task started, in RFC 3339 format.
    #[serde(rename = "start_time", skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    /// When the task ended, in RFC 3339 format.
    #[serde(rename = "end_time", skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    /// Information about all output files. Directory outputs are flattened into separate items.
    #[serde(rename = "outputs")]
    pub outputs: Vec<tes_output_file_log::TesOutputFileLog>,
    /// System logs are any logs the system decides are relevant, which are not tied directly to an Executor process. Content is implementation specific: format, size, etc.  System logs may be collected here to provide convenient access.  For example, the system may include the name of the host where the task is executing, an error message that caused a SYSTEM_ERROR state (e.g. disk is full), etc.  System logs are only included in the FULL task view.
    #[serde(rename = "system_logs", skip_serializing_if = "Option::is_none")]
    pub system_logs: Option<Vec<String>>,
}

impl TesTaskLog {
    /// TaskLog describes logging information related to a Task.
    pub fn new(
        logs: Vec<tes_executor_log::TesExecutorLog>,
        outputs: Vec<tes_output_file_log::TesOutputFileLog>,
    ) -> TesTaskLog {
        TesTaskLog {
            logs,
            metadata: None,
            start_time: None,
            end_time: None,
            outputs,
            system_logs: None,
        }
    }
}
