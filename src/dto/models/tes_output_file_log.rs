use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// OutputFileLog describes a single output file
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "OutputFileLog describes a single output file. This describes file details after the task has completed successfully, for logging purposes.", example = json!({
    "url": "s3://bucket/output.txt",
    "path": "/tmp/output.txt",
    "size_bytes": 1024
}))]
pub struct TesOutputFileLog {
    /// URL of the file in storage, e.g. s3://bucket/file.txt
    #[serde(rename = "url")]
    #[schema(example = "s3://bucket/output.txt")]
    pub url: String,
    /// Path of the file inside the container. Must be an absolute path.
    #[serde(rename = "path")]
    #[schema(example = "/tmp/output.txt")]
    pub path: String,
    /// Size of the file in bytes. Note, this is currently coded as a string because official JSON doesn't support int64 numbers.
    #[serde(rename = "size_bytes")]
    #[schema(example = "1024")]
    pub size_bytes: String,
}

impl TesOutputFileLog {
    /// OutputFileLog describes a single output file. This describes file details after the task has completed successfully, for logging purposes.
    pub fn new(url: String, path: String, size_bytes: String) -> TesOutputFileLog {
        TesOutputFileLog {
            url,
            path,
            size_bytes,
        }
    }
}
