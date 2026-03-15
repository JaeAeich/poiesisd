use std::fmt;

/// Signals that the task reached a terminal error state (already persisted to DB).
/// The outer `run_task` should not overwrite the state.
#[derive(Debug)]
pub struct ExecutionFailed;

#[derive(Debug)]
pub enum RunnerError {
    Docker(bollard::errors::Error),
    Filer(crate::filer::FilerError),
    Database(sqlx::Error),
    Io(std::io::Error),
    /// Terminal state already set in DB — not a crash, just a signal.
    ExecutionFailed(ExecutionFailed),
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Docker(e) => write!(f, "Docker error: {e}"),
            Self::Filer(e) => write!(f, "Filer error: {e}"),
            Self::Database(e) => write!(f, "Database error: {e}"),
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::ExecutionFailed(_) => write!(f, "execution failed (state already set)"),
        }
    }
}

impl std::error::Error for RunnerError {}

impl From<bollard::errors::Error> for RunnerError {
    fn from(e: bollard::errors::Error) -> Self {
        Self::Docker(e)
    }
}

impl From<crate::filer::FilerError> for RunnerError {
    fn from(e: crate::filer::FilerError) -> Self {
        Self::Filer(e)
    }
}

impl From<sqlx::Error> for RunnerError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<std::io::Error> for RunnerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl RunnerError {
    pub fn tes_state(&self) -> &'static str {
        "SYSTEM_ERROR"
    }
}
