use crate::dto::TesInput;
use crate::dto::TesOutput;
use crate::filer::backends::{LocalStorage, S3Storage};
use crate::filer::config::{FilerConfig, LocalConfig, S3Config};
use crate::filer::error::{FilerError, Result};
use crate::filer::input::stage_input;
use crate::filer::output::collect_output;
use crate::filer::storage::{Storage, StorageRegistry, extract_scheme};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const DEFAULT_MAX_CONCURRENT: usize = 16;

pub struct Filer {
    registry: StorageRegistry,
    semaphore: Arc<Semaphore>,
}

impl Filer {
    pub fn from_config(config: &FilerConfig) -> Result<Self> {
        Self::from_config_with_concurrency(config, DEFAULT_MAX_CONCURRENT)
    }

    pub fn from_config_with_concurrency(
        config: &FilerConfig,
        max_concurrent: usize,
    ) -> Result<Self> {
        let mut registry = StorageRegistry::new();

        if let Some(s3_config) = &config.storage.s3 {
            let storage = build_s3_storage(s3_config)?;
            registry.register("s3", storage);
        }

        if let Some(local_config) = &config.storage.local {
            let storage = build_local_storage(local_config)?;
            registry.register("local", storage.clone());
            registry.register("file", storage);
        }

        let permits = NonZeroUsize::new(max_concurrent.max(1))
            .ok_or_else(|| FilerError::config("concurrency must be at least 1"))?;

        Ok(Self {
            registry,
            semaphore: Arc::new(Semaphore::new(permits.get())),
        })
    }

    pub fn with_registry(registry: StorageRegistry) -> Self {
        Self {
            registry,
            semaphore: Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT)),
        }
    }

    pub async fn stage_inputs(&self, inputs: &[TesInput], workspace: &Path) -> Result<()> {
        if inputs.is_empty() {
            return Ok(());
        }

        let mut tasks = JoinSet::new();

        for input in inputs {
            let storage = self.resolve_storage_for_input(input)?;
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let input = input.clone();
            let workspace = workspace.to_path_buf();

            tasks.spawn(async move {
                let _permit = permit;
                stage_input(&*storage, &input, &workspace).await
            });
        }

        let errors = collect_errors(tasks).await;

        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(FilerError::Multiple(errors))
        }
    }

    pub async fn collect_outputs(&self, outputs: &[TesOutput], workspace: &Path) -> Result<()> {
        if outputs.is_empty() {
            return Ok(());
        }

        let mut tasks = JoinSet::new();

        for output in outputs {
            let storage = self.resolve_storage_for_output(output)?;
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let output = output.clone();
            let workspace = workspace.to_path_buf();

            tasks.spawn(async move {
                let _permit = permit;
                collect_output(&*storage, &output, &workspace).await
            });
        }

        let errors = collect_errors(tasks).await;

        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(FilerError::Multiple(errors))
        }
    }

    fn resolve_storage_for_input(&self, input: &TesInput) -> Result<Arc<dyn Storage>> {
        match &input.url {
            Some(url) => self.resolve_storage(url),
            None => {
                let storage = LocalStorage::new(Path::new("/"))?;
                Ok(Arc::new(storage))
            }
        }
    }

    fn resolve_storage_for_output(&self, output: &TesOutput) -> Result<Arc<dyn Storage>> {
        self.resolve_storage(&output.url)
    }

    fn resolve_storage(&self, url: &str) -> Result<Arc<dyn Storage>> {
        let scheme = extract_scheme(url)?;

        self.registry
            .get(scheme)
            .ok_or_else(|| FilerError::BackendNotFound(scheme.to_string()))
    }
}

fn build_s3_storage(config: &S3Config) -> Result<Arc<dyn Storage>> {
    let storage = S3Storage::new(
        config.endpoint.as_deref(),
        config.region.as_deref(),
        &config.access_key_id,
        &config.secret_access_key,
        config.allow_anonymous,
    )?;
    Ok(Arc::new(storage))
}

fn build_local_storage(config: &LocalConfig) -> Result<Arc<dyn Storage>> {
    let storage = LocalStorage::new(&config.root)?;
    Ok(Arc::new(storage))
}

async fn collect_errors(mut tasks: JoinSet<Result<()>>) -> Vec<FilerError> {
    let mut errors = Vec::new();

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Err(e)) => errors.push(e),
            Err(join_error) => {
                errors.push(FilerError::Config(format!(
                    "task join error: {}",
                    join_error
                )));
            }
            Ok(Ok(())) => {}
        }
    }

    errors
}
