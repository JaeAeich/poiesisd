use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// TesFileType : Define if input/output element is a file or a directory. It is not required that the user provide this value, but it is required that the server fill in the value once the information is available at run time.
/// Define if input/output element is a file or a directory. It is not required that the user provide this value, but it is required that the server fill in the value once the information is available at run time.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    Default,
)]
pub enum TesFileType {
    #[serde(rename = "FILE")]
    #[default]
    File,
    #[serde(rename = "DIRECTORY")]
    Directory,
}

impl std::fmt::Display for TesFileType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "FILE"),
            Self::Directory => write!(f, "DIRECTORY"),
        }
    }
}
