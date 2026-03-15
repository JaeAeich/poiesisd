use crate::dto::TesInput;
use crate::dto::TesOutput;
use crate::dto::TesOutputFileLog;
use crate::filer::backend::{S3Backend, StorageBackend};
use crate::filer::config::BackendConfig;
use crate::filer::error::{FilerError, Result};
use crate::filer::input::stage_input;
use crate::filer::output::collect_output;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const DEFAULT_MAX_CONCURRENT: usize = 16;

pub struct Filer<B: StorageBackend> {
    backend: Arc<B>,
    scheme: String,
    bucket: String,
    semaphore: Arc<Semaphore>,
}

impl Filer<S3Backend> {
    pub fn from_config(config: &BackendConfig) -> Result<Self> {
        match config {
            BackendConfig::S3(s3_config) => {
                let backend = S3Backend::from_config(s3_config)?;
                Ok(Self {
                    backend: Arc::new(backend),
                    scheme: "s3".to_string(),
                    bucket: s3_config.bucket.clone(),
                    semaphore: Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT)),
                })
            }
        }
    }
}

impl<B: StorageBackend + 'static> Filer<B> {
    pub async fn stage_inputs(&self, inputs: &[TesInput], workspace: &Path) -> Result<()> {
        if inputs.is_empty() {
            return Ok(());
        }

        let mut tasks = JoinSet::new();

        for input in inputs {
            let backend = self.backend.clone();
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let input = input.clone();
            let workspace = workspace.to_path_buf();
            let bucket = self.bucket.clone();

            tasks.spawn(async move {
                let _permit = permit;
                stage_input(&*backend, &input, &workspace, &bucket).await
            });
        }

        collect_unit_results(tasks).await
    }

    pub async fn collect_outputs(
        &self,
        outputs: &[TesOutput],
        workspace: &Path,
    ) -> Result<Vec<TesOutputFileLog>> {
        if outputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut tasks = JoinSet::new();

        for output in outputs {
            let backend = self.backend.clone();
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let output = output.clone();
            let workspace = workspace.to_path_buf();
            let scheme = self.scheme.clone();
            let bucket = self.bucket.clone();

            tasks.spawn(async move {
                let _permit = permit;
                collect_output(&*backend, &output, &workspace, &scheme, &bucket).await
            });
        }

        collect_log_results(tasks).await
    }
}

async fn collect_unit_results(mut tasks: JoinSet<Result<()>>) -> Result<()> {
    let mut errors = Vec::new();

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Err(e)) => errors.push(e),
            Err(join_error) => {
                errors.push(FilerError::config(format!(
                    "task panicked or was cancelled: {join_error}",
                )));
            }
            Ok(Ok(())) => {}
        }
    }

    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.into_iter().next().unwrap()),
        _ => Err(FilerError::Multiple(errors)),
    }
}

async fn collect_log_results(
    mut tasks: JoinSet<Result<Vec<TesOutputFileLog>>>,
) -> Result<Vec<TesOutputFileLog>> {
    let mut errors = Vec::new();
    let mut logs = Vec::new();

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(file_logs)) => logs.extend(file_logs),
            Ok(Err(e)) => errors.push(e),
            Err(join_error) => {
                errors.push(FilerError::config(format!(
                    "task panicked or was cancelled: {join_error}",
                )));
            }
        }
    }

    match errors.len() {
        0 => Ok(logs),
        1 => Err(errors.into_iter().next().unwrap()),
        _ => Err(FilerError::Multiple(errors)),
    }
}
