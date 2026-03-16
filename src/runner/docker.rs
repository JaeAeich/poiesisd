use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use futures_util::StreamExt;

use super::error::RunnerError;

/// Max bytes of stdout/stderr to capture from docker logs for the executor log snippet.
const LOG_TRUNCATE_BYTES: usize = 10 * 1024;

pub struct ContainerRunConfig {
    pub image: String,
    pub command: Vec<String>,
    pub env: Vec<String>,
    pub workdir: Option<String>,
    pub binds: Vec<String>,
}

pub struct ContainerResult {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
    pub start_time: String,
    pub end_time: String,
}

pub struct DockerExecutor {
    client: Docker,
}

impl DockerExecutor {
    pub fn new() -> Result<Self, RunnerError> {
        let client = Docker::connect_with_local_defaults()?;
        Ok(Self { client })
    }

    pub async fn pull_image(&self, image: &str) -> Result<(), RunnerError> {
        let opts = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(opts), None, None);
        while let Some(result) = stream.next().await {
            result?;
        }
        Ok(())
    }

    pub async fn run_container(
        &self,
        config: ContainerRunConfig,
    ) -> Result<ContainerResult, RunnerError> {
        let host_config = HostConfig {
            binds: Some(config.binds),
            ..Default::default()
        };

        let container_config = Config {
            image: Some(config.image.clone()),
            cmd: Some(config.command.clone()),
            env: Some(config.env.clone()),
            working_dir: config.workdir.clone(),
            host_config: Some(host_config),
            ..Default::default()
        };

        let create_opts = CreateContainerOptions::<String> {
            ..Default::default()
        };

        let container = self
            .client
            .create_container(Some(create_opts), container_config)
            .await?;
        let id = &container.id;

        let start_time = chrono::Utc::now().to_rfc3339();

        self.client
            .start_container(id, None::<StartContainerOptions<String>>)
            .await?;

        // Wait for container to finish
        let wait_opts = WaitContainerOptions {
            condition: "not-running",
        };
        let mut wait_stream = self.client.wait_container(id, Some(wait_opts));
        let mut exit_code: i64 = -1;
        while let Some(result) = wait_stream.next().await {
            match result {
                Ok(response) => {
                    exit_code = response.status_code;
                }
                Err(e) => {
                    // Bollard returns an error when container exits with non-zero,
                    // but we still need to handle it gracefully
                    if let bollard::errors::Error::DockerContainerWaitError { code, .. } = &e {
                        exit_code = *code;
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }

        let end_time = chrono::Utc::now().to_rfc3339();

        // Capture logs
        let (stdout, stderr) = self.capture_logs(id).await?;

        // Cleanup container
        self.cleanup_container(id).await?;

        Ok(ContainerResult {
            exit_code,
            stdout,
            stderr,
            start_time,
            end_time,
        })
    }

    async fn capture_logs(&self, id: &str) -> Result<(String, String), RunnerError> {
        let log_opts = LogsOptions::<String> {
            follow: false,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let mut stream = self.client.logs(id, Some(log_opts));
        while let Some(result) = stream.next().await {
            match result? {
                bollard::container::LogOutput::StdOut { message } => {
                    if stdout_buf.len() < LOG_TRUNCATE_BYTES {
                        stdout_buf.extend_from_slice(&message);
                    }
                }
                bollard::container::LogOutput::StdErr { message } => {
                    if stderr_buf.len() < LOG_TRUNCATE_BYTES {
                        stderr_buf.extend_from_slice(&message);
                    }
                }
                _ => {}
            }
        }

        stdout_buf.truncate(LOG_TRUNCATE_BYTES);
        stderr_buf.truncate(LOG_TRUNCATE_BYTES);

        Ok((
            String::from_utf8_lossy(&stdout_buf).into_owned(),
            String::from_utf8_lossy(&stderr_buf).into_owned(),
        ))
    }

    pub async fn cleanup_container(&self, id: &str) -> Result<(), RunnerError> {
        let opts = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };
        // Ignore errors on cleanup — container may already be removed
        let _ = self.client.remove_container(id, Some(opts)).await;
        Ok(())
    }
}
