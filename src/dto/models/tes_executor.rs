use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Executor describes a command to be executed, and its environment
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "Executor describes a command to be executed, and its environment", example = json!({
    "image": "alpine:latest",
    "command": ["echo", "hello world"],
    "workdir": "/tmp",
    "stdin": "/tmp/input.txt",
    "stdout": "/tmp/output.txt",
    "stderr": "/tmp/error.txt"
}))]
pub struct TesExecutor {
    /// Name of the container image. The string will be passed as the image argument to the containerization run command. Examples:    - `ubuntu`    - `quay.io/aptible/ubuntu`    - `gcr.io/my-org/my-image`    - `myregistryhost:5000/fedora/httpd:version1.0`
    #[serde(rename = "image")]
    #[schema(example = "alpine:latest")]
    pub image: String,
    /// A sequence of program arguments to execute, where the first argument is the program to execute (i.e. argv). Example: ``` {   \"command\" : [\"/bin/md5\", \"/data/file1\"] } ```
    #[serde(rename = "command")]
    #[schema(
        example = json!(["echo", "hello world"])
    )]
    pub command: Vec<String>,
    /// The working directory that the command will be executed in. If not defined, the system will default to the directory set by the container image.
    #[serde(rename = "workdir", skip_serializing_if = "Option::is_none")]
    #[schema(example = "/tmp")]
    pub workdir: Option<String>,
    /// Path inside the container to a file which will be piped to the executor's stdin. This must be an absolute path. This mechanism could be used in conjunction with the input declaration to process a data file using a tool that expects STDIN.  For example, to get the MD5 sum of a file by reading it into stdin: ``` {   \"command\" : [ \"md5sum\" ],   \"stdin\" : \"/tmp/test-file\" } ```
    #[serde(rename = "stdin", skip_serializing_if = "Option::is_none")]
    #[schema(example = "/tmp/input.txt")]
    pub stdin: Option<String>,
    /// Path inside the container to a file where the executor's stdout will be written to. Must be an absolute path. Example: ``` {   \"stdout\" : \"/tmp/stdout.log\" } ```
    #[serde(rename = "stdout", skip_serializing_if = "Option::is_none")]
    #[schema(example = "/tmp/output.txt")]
    pub stdout: Option<String>,
    /// Path inside the container to a file where the executor's stderr will be written to. Must be an absolute path. Example: ``` {   \"stderr\" : \"/tmp/stderr.log\" } ```
    #[serde(rename = "stderr", skip_serializing_if = "Option::is_none")]
    #[schema(example = "/tmp/error.txt")]
    pub stderr: Option<String>,
    /// Environmental variables to set within the container. Example: ``` {   \"env\" : {     \"ENV_CONFIG_PATH\" : \"/data/config.file\",     \"BLASTDB\" : \"/data/GRC38\",     \"HMMERDB\" : \"/data/hmmer\"   } } ```
    #[serde(rename = "env", skip_serializing_if = "Option::is_none")]
    #[schema(
        example = json!({"PATH": "/usr/local/bin:/usr/bin:/bin", "HOME": "/tmp"})
    )]
    pub env: Option<std::collections::HashMap<String, String>>,
    /// Default behavior of running an array of executors is that execution stops on the first error. If `ignore_error` is `True`, then the runner will record error exit codes, but will continue on to the next tesExecutor.
    #[serde(rename = "ignore_error", skip_serializing_if = "Option::is_none")]
    #[schema(example = false)]
    pub ignore_error: Option<bool>,
}

impl TesExecutor {
    /// Executor describes a command to be executed, and its environment.
    pub fn new(image: String, command: Vec<String>) -> TesExecutor {
        TesExecutor {
            image,
            command,
            workdir: None,
            stdin: None,
            stdout: None,
            stderr: None,
            env: None,
            ignore_error: None,
        }
    }
}
