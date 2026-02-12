use super::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Input describes Task input files
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(description = "Input describes Task input files", example = json!({
    "name": "infile",
    "description": "Input file for processing",
    "url": "s3://my-bucket/input.txt",
    "path": "/tmp/input.txt",
    "type": "FILE",
    "streamable": false
}))]
pub struct TesInput {
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    #[schema(example = "infile")]
    pub name: Option<String>,
    #[serde(rename = "description", skip_serializing_if = "Option::is_none")]
    #[schema(example = "Input file for processing")]
    pub description: Option<String>,
    /// REQUIRED, unless "content" is set. URL in long term storage, for example:   - s3://my-object-store/file1   - gs://my-bucket/file2   - file:///path/to/my/file   - /path/to/my/file
    #[serde(rename = "url", skip_serializing_if = "Option::is_none")]
    #[schema(example = "s3://my-bucket/input.txt")]
    pub url: Option<String>,
    /// Path of the file inside the container.  Must be an absolute path.
    #[serde(rename = "path")]
    #[schema(example = "/tmp/input.txt")]
    pub path: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<tes_file_type::TesFileType>,
    /// File content literal.  Implementations should support a minimum of 128 KiB in this field and may define their own maximum.  UTF-8 encoded  If content is not empty, \"url\" must be ignored.
    #[serde(rename = "content", skip_serializing_if = "Option::is_none")]
    #[schema(example = "hello world")]
    pub content: Option<String>,
    /// Indicate that a file resource could be accessed using a streaming interface. The `streamable` flag does not guarantee that the receiver understands the streaming interface, only that the resource can be accessed via streaming.  Example: A large genomics file is stored with a `.tbi` (tabix) index file.  The tabix index allows a client to stream through large genomics files.  The client could choose to download the whole file or use the tabix index to stream through regions of the file.
    #[serde(rename = "streamable", skip_serializing_if = "Option::is_none")]
    #[schema(example = false)]
    pub streamable: Option<bool>,
}

impl TesInput {
    /// Input describes Task input files.
    pub fn new(path: String) -> TesInput {
        TesInput {
            name: None,
            description: None,
            url: None,
            path,
            r#type: None,
            content: None,
            streamable: None,
        }
    }
}
