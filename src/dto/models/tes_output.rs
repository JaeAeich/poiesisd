use super::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Output describes Task output files
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "Output describes Task output files", example = json!({
    "name": "outfile",
    "description": "Output file from processing",
    "url": "s3://my-bucket/output.txt",
    "path": "/tmp/output.txt",
    "type": "FILE"
}))]
pub struct TesOutput {
    /// User-provided name of output file
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    #[schema(example = "outfile")]
    pub name: Option<String>,
    /// Optional users provided description field, can be used for documentation.
    #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
    #[schema(example = "Output file from processing")]
    pub description: Option<String>,
    /// URL at which the TES server makes the output accessible after the task is complete. When tesOutput.path contains wildcards, it must be a directory; see `tesOutput.path_prefix` for details on how output URLs are constructed in this case. For Example:  - `s3://my-object-store/file1`  - `gs://my-bucket/file2`  - `file:///path/to/my/file`
    #[serde(rename = "url")]
    #[schema(example = "s3://my-bucket/output.txt")]
    pub url: String,
    /// Absolute path of the file inside the container. May contain pattern matching wildcards to select multiple outputs at once, but mind implications for `tesOutput.url` and `tesOutput.path_prefix`. Only wildcards defined in IEEE Std 1003.1-2017 (POSIX), 12.3 are supported; see https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_13
    #[serde(rename = "path")]
    #[schema(example = "/tmp/output.txt")]
    pub path: String,
    /// Prefix to be removed from matching outputs if `tesOutput.path` contains wildcards; output URLs are constructed by appending pruned paths to the directory specified in `tesOutput.url`. Required if `tesOutput.path` contains wildcards, ignored otherwise.
    #[serde(rename = "path_prefix", skip_serializing_if = "Option::is_none")]
    #[schema(example = "/tmp")]
    pub path_prefix: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<tes_file_type::TesFileType>,
}

impl TesOutput {
    /// Output describes Task output files.
    pub fn new(url: String, path: String) -> TesOutput {
        TesOutput {
            name: None,
            description: None,
            url,
            path,
            path_prefix: None,
            r#type: None,
        }
    }
}
