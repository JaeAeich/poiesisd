use super::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Task describes an instance of a task
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "Task describes an instance of a task", example = json!({
    "id": "task-12345",
    "state": "COMPLETE",
    "name": "Hello World",
    "description": "A simple hello world task",
    "inputs": [
        {
            "name": "infile",
            "description": "Input file",
            "url": "s3://my-bucket/input.txt",
            "path": "/tmp/input.txt"
        }
    ],
    "outputs": [
        {
            "name": "outfile",
            "description": "Output file",
            "url": "s3://my-bucket/output.txt",
            "path": "/tmp/output.txt"
        }
    ],
    "executors": [
        {
            "image": "alpine:latest",
            "command": ["echo", "hello world"]
        }
    ],
    "resources": {
        "cpu_cores": 1,
        "ram_gb": 1.0,
        "disk_gb": 10.0
    },
    "creation_time": "2023-01-01T00:00:00Z"
}))]
pub struct TesTask {
    /// Task identifier assigned by the server.
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    #[schema(example = "task-12345")]
    pub id: Option<String>,
    #[serde(rename = "state", skip_serializing_if = "Option::is_none")]
    pub state: Option<tes_state::TesState>,
    /// User-provided task name.
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    #[schema(example = "Hello World")]
    pub name: Option<String>,
    /// Optional user-provided description of task for documentation purposes.
    #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
    #[schema(example = "A simple hello world task")]
    pub description: Option<String>,
    /// Input files that will be used by the task. Inputs will be downloaded and mounted into the executor container as defined by the task request document.
    #[serde(rename = "inputs", skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<tes_input::TesInput>>,
    /// Output files. Outputs will be uploaded from the executor container to long-term storage.
    #[serde(rename = "outputs", skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<tes_output::TesOutput>>,
    #[serde(rename = "resources", skip_serializing_if = "Option::is_none")]
    pub resources: Option<Box<tes_resources::TesResources>>,
    /// An array of executors to be run. Each of the executors will run one at a time sequentially. Each executor is a different command that will be run, and each can utilize a different docker image. But each of the executors will see the same mapped inputs and volumes that are declared in the parent CreateTask message.  Execution stops on the first error.
    #[serde(rename = "executors")]
    pub executors: Vec<tes_executor::TesExecutor>,
    /// Volumes are directories which may be used to share data between Executors. Volumes are initialized as empty directories by the system when the task starts and are mounted at the same path in each Executor.  For example, given a volume defined at `/vol/A`, executor 1 may write a file to `/vol/A/exec1.out.txt`, then executor 2 may read from that file.  (Essentially, this translates to a `docker run -v` flag where the container path is the same for each executor).
    #[serde(rename = "volumes", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(["/tmp/shared"]))]
    pub volumes: Option<Vec<String>>,
    /// A key-value map of arbitrary tags. These can be used to store meta-data and annotations about a task. Example: ``` {   \"tags\" : {       \"WORKFLOW_ID\" : \"cwl-01234\",       \"PROJECT_GROUP\" : \"alice-lab\"   } } ```
    #[serde(rename = "tags", skip_serializing_if = "Option::is_none")]
    #[schema(example = json!({"WORKFLOW_ID": "cwl-01234", "PROJECT_GROUP": "alice-lab"}))]
    pub tags: Option<std::collections::HashMap<String, String>>,
    /// Task logging information. Normally, this will contain only one entry, but in the case where a task fails and is retried, an entry will be appended to this list.
    #[serde(rename = "logs", skip_serializing_if = "Option::is_none")]
    pub logs: Option<Vec<tes_task_log::TesTaskLog>>,
    /// Date + time the task was created, in RFC 3339 format. This is set by the system, not the client.
    #[serde(rename = "creation_time", skip_serializing_if = "Option::is_none")]
    #[schema(example = "2023-01-01T00:00:00Z")]
    pub creation_time: Option<String>,
}

impl TesTask {
    /// Task describes an instance of a task.
    pub fn new(executors: Vec<tes_executor::TesExecutor>) -> TesTask {
        TesTask {
            id: None,
            state: None,
            name: None,
            description: None,
            inputs: None,
            outputs: None,
            resources: None,
            executors,
            volumes: None,
            tags: None,
            logs: None,
            creation_time: None,
        }
    }
}
