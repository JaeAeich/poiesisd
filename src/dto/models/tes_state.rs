use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Task state as defined by the server
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
#[schema(
    description = "Task state as defined by the server",
    example = "RUNNING"
)]
pub enum TesState {
    #[serde(rename = "UNKNOWN")]
    #[default]
    Unknown,
    #[serde(rename = "QUEUED")]
    Queued,
    #[serde(rename = "INITIALIZING")]
    Initializing,
    #[serde(rename = "RUNNING")]
    Running,
    #[serde(rename = "PAUSED")]
    Paused,
    #[serde(rename = "COMPLETE")]
    Complete,
    #[serde(rename = "EXECUTOR_ERROR")]
    ExecutorError,
    #[serde(rename = "SYSTEM_ERROR")]
    SystemError,
    #[serde(rename = "CANCELED")]
    Canceled,
    #[serde(rename = "PREEMPTED")]
    Preempted,
    #[serde(rename = "CANCELING")]
    Canceling,
}

impl std::fmt::Display for TesState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "UNKNOWN"),
            Self::Queued => write!(f, "QUEUED"),
            Self::Initializing => write!(f, "INITIALIZING"),
            Self::Running => write!(f, "RUNNING"),
            Self::Paused => write!(f, "PAUSED"),
            Self::Complete => write!(f, "COMPLETE"),
            Self::ExecutorError => write!(f, "EXECUTOR_ERROR"),
            Self::SystemError => write!(f, "SYSTEM_ERROR"),
            Self::Canceled => write!(f, "CANCELED"),
            Self::Preempted => write!(f, "PREEMPTED"),
            Self::Canceling => write!(f, "CANCELING"),
        }
    }
}
