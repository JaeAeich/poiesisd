use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::task::JoinHandle;

use crate::database;
use crate::dto::{TesExecutor, TesExecutorLog, TesInput, TesOutput, TesOutputFileLog, TesTaskLog};
use crate::filer::util::resolve_workspace_path;
use crate::filer::{Filer, S3Backend};

use super::docker::{ContainerRunConfig, DockerExecutor};
use super::error::{ExecutionFailed, RunnerError};

/// Workspace root. When running inside Docker (DooD via socket mount), bind mounts
/// are resolved by the Docker daemon against the **host** filesystem. Set
/// `POIESISD_WORKSPACE_ROOT` to a host path that is also mounted into the poiesisd
/// container at the same path so both sides see the same files.
fn workspace_root() -> PathBuf {
    PathBuf::from(
        std::env::var("POIESISD_WORKSPACE_ROOT").unwrap_or_else(|_| "/tmp/poiesisd".to_string()),
    )
}

pub struct Worker;

impl Worker {
    pub fn spawn(
        pool: SqlitePool,
        filer: Arc<Filer<S3Backend>>,
        docker: DockerExecutor,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let docker = Arc::new(docker);
            loop {
                match database::claim_queued_task(&pool).await {
                    Ok(Some(task_id)) => {
                        tracing::info!(task_id = %task_id, "claimed task");
                        if let Err(e) = run_task(&pool, &filer, &docker, &task_id).await {
                            tracing::error!(task_id = %task_id, error = %e, "task failed");
                        }
                    }
                    Ok(None) => {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to claim task");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        })
    }
}

async fn run_task(
    pool: &SqlitePool,
    filer: &Filer<S3Backend>,
    docker: &DockerExecutor,
    task_id: &str,
) -> Result<(), RunnerError> {
    let task = database::get_task_full(pool, task_id).await?;
    let workspace = workspace_root().join(task_id);

    let result = run_task_inner(pool, filer, docker, task_id, &task, &workspace).await;

    match &result {
        Ok(()) => {
            database::update_task_state(pool, task_id, "COMPLETE").await?;
            tracing::info!(task_id = %task_id, "task complete");
        }
        Err(RunnerError::ExecutionFailed(_)) => {
            // Terminal state already set by run_task_inner, don't overwrite
            tracing::warn!(task_id = %task_id, "task finished with error (state already set)");
        }
        Err(e) => {
            let state = e.tes_state();
            let _ = database::update_task_state(pool, task_id, state).await;
            tracing::error!(task_id = %task_id, state = state, error = %e, "task errored");
        }
    }

    // Cleanup workspace
    if workspace.exists() {
        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

    // Map ExecutionFailed to Ok — it's a handled terminal state, not a crash
    match result {
        Err(RunnerError::ExecutionFailed(_)) => Ok(()),
        other => other,
    }
}

/// Record partial logs and update task state on failure.
async fn fail_task(
    pool: &SqlitePool,
    task_id: &str,
    start_time: &str,
    executor_logs: &[TesExecutorLog],
    system_logs: &[String],
    state: &str,
) -> Result<(), crate::database::DatabaseError> {
    insert_task_log(pool, task_id, start_time, executor_logs, &[], system_logs).await;
    database::update_task_state(pool, task_id, state).await
}

/// Inner execution flow, separated so we can always do cleanup in the outer fn.
async fn run_task_inner(
    pool: &SqlitePool,
    filer: &Filer<S3Backend>,
    docker: &DockerExecutor,
    task_id: &str,
    task: &database::FullTask,
    workspace: &Path,
) -> Result<(), RunnerError> {
    // Create workspace
    tokio::fs::create_dir_all(workspace).await?;

    // Create volume dirs
    for vol in &task.volumes {
        let vol_path = resolve_workspace_path(workspace, vol)
            .map_err(|e| RunnerError::Io(std::io::Error::other(e.to_string())))?;
        tokio::fs::create_dir_all(&vol_path).await?;
    }

    // Stage inputs
    if !task.inputs.is_empty() {
        filer.stage_inputs(&task.inputs, workspace).await?;
    }

    // Compute bind mounts
    let binds = compute_bind_mounts(workspace, &task.inputs, &task.outputs, &task.volumes);

    // Update state to RUNNING
    database::update_task_state(pool, task_id, "RUNNING").await?;

    let task_start_time = chrono::Utc::now().to_rfc3339();
    let mut executor_logs: Vec<TesExecutorLog> = Vec::new();
    let mut system_logs: Vec<String> = Vec::new();
    let mut had_error = false;

    // Run executors sequentially
    for (i, executor) in task.executors.iter().enumerate() {
        tracing::info!(task_id = %task_id, executor_index = i, image = %executor.image, "running executor");

        // Pull image
        if let Err(e) = docker.pull_image(&executor.image).await {
            let msg = format!(
                "executor[{i}]: failed to pull image '{}': {e}",
                executor.image
            );
            tracing::error!("{msg}");
            system_logs.push(msg);

            executor_logs.push(TesExecutorLog::new(-1));
            had_error = true;

            if !executor.ignore_error.unwrap_or(false) {
                fail_task(
                    pool,
                    task_id,
                    &task_start_time,
                    &executor_logs,
                    &system_logs,
                    "SYSTEM_ERROR",
                )
                .await?;
                return Err(e);
            }
            continue;
        }

        // Build container config
        let command = build_command(executor);
        let env: Vec<String> = executor
            .env
            .as_ref()
            .map(|m| m.iter().map(|(k, v)| format!("{k}={v}")).collect())
            .unwrap_or_default();

        let config = ContainerRunConfig {
            image: executor.image.clone(),
            command,
            env,
            workdir: executor.workdir.clone(),
            binds: binds.clone(),
        };

        match docker.run_container(config).await {
            Ok(result) => {
                let mut log = TesExecutorLog::new(result.exit_code as i32);
                log.start_time = Some(result.start_time);
                log.end_time = Some(result.end_time);
                log.stdout = if result.stdout.is_empty() {
                    None
                } else {
                    Some(result.stdout)
                };
                log.stderr = if result.stderr.is_empty() {
                    None
                } else {
                    Some(result.stderr)
                };
                executor_logs.push(log);

                if result.exit_code != 0 {
                    let msg = format!("executor[{i}]: exited with code {}", result.exit_code);
                    tracing::warn!("{msg}");
                    system_logs.push(msg);

                    if !executor.ignore_error.unwrap_or(false) {
                        fail_task(
                            pool,
                            task_id,
                            &task_start_time,
                            &executor_logs,
                            &system_logs,
                            "EXECUTOR_ERROR",
                        )
                        .await?;
                        return Err(RunnerError::ExecutionFailed(ExecutionFailed));
                    }
                    had_error = true;
                }
            }
            Err(e) => {
                let msg = format!("executor[{i}]: container run failed: {e}");
                tracing::error!("{msg}");
                system_logs.push(msg);
                executor_logs.push(TesExecutorLog::new(-1));

                if !executor.ignore_error.unwrap_or(false) {
                    fail_task(
                        pool,
                        task_id,
                        &task_start_time,
                        &executor_logs,
                        &system_logs,
                        "SYSTEM_ERROR",
                    )
                    .await?;
                    return Err(e);
                }
                had_error = true;
            }
        }
    }

    // Collect outputs
    let output_file_logs = if !task.outputs.is_empty() && !had_error {
        match filer.collect_outputs(&task.outputs, workspace).await {
            Ok(logs) => logs,
            Err(e) => {
                let msg = format!("output collection failed: {e}");
                tracing::error!("{msg}");
                system_logs.push(msg);
                insert_task_log(
                    pool,
                    task_id,
                    &task_start_time,
                    &executor_logs,
                    &[],
                    &system_logs,
                )
                .await;
                return Err(e.into());
            }
        }
    } else {
        Vec::new()
    };

    // Insert task log
    insert_task_log(
        pool,
        task_id,
        &task_start_time,
        &executor_logs,
        &output_file_logs,
        &system_logs,
    )
    .await;

    Ok(())
}

/// Insert task log into the database, ignoring errors (best effort).
async fn insert_task_log(
    pool: &SqlitePool,
    task_id: &str,
    start_time: &str,
    executor_logs: &[TesExecutorLog],
    output_file_logs: &[TesOutputFileLog],
    system_logs: &[String],
) {
    let end_time = chrono::Utc::now().to_rfc3339();

    let mut task_log = TesTaskLog::new(executor_logs.to_vec(), output_file_logs.to_vec());
    task_log.start_time = Some(start_time.to_string());
    task_log.end_time = Some(end_time);
    if !system_logs.is_empty() {
        task_log.system_logs = Some(system_logs.to_vec());
    }

    if let Err(e) = database::insert_task_log(pool, task_id, &task_log).await {
        tracing::error!(task_id = %task_id, error = %e, "failed to insert task log");
    }
}

/// Collect unique top-level directory components from inputs, outputs, and volumes
/// to create host bind mounts: `workspace/dir:/dir`.
fn compute_bind_mounts(
    workspace: &Path,
    inputs: &[TesInput],
    outputs: &[TesOutput],
    volumes: &[String],
) -> Vec<String> {
    let mut top_dirs = BTreeSet::new();

    for input in inputs {
        if let Some(dir) = first_path_component(&input.path) {
            top_dirs.insert(dir);
        }
    }

    for output in outputs {
        if let Some(dir) = first_path_component(&output.path) {
            top_dirs.insert(dir);
        }
    }

    for vol in volumes {
        if let Some(dir) = first_path_component(vol) {
            top_dirs.insert(dir);
        }
    }

    top_dirs
        .into_iter()
        .map(|dir| {
            let host_path = workspace.join(&dir);
            // Ensure host dir exists
            std::fs::create_dir_all(&host_path).ok();
            format!("{}:/{dir}", host_path.display())
        })
        .collect()
}

/// Extract the first path component after stripping leading `/`.
/// e.g. `/data/input.txt` → `"data"`, `/tmp/work` → `"tmp"`.
fn first_path_component(path: &str) -> Option<String> {
    let stripped = path.strip_prefix('/').unwrap_or(path);
    let component = stripped.split('/').next()?;
    if component.is_empty() {
        None
    } else {
        Some(component.to_string())
    }
}

/// Build the command vec for a container. If stdin/stdout/stderr redirects are set,
/// wrap in `/bin/sh -c "..."`.
fn build_command(executor: &TesExecutor) -> Vec<String> {
    let has_redirects =
        executor.stdout.is_some() || executor.stderr.is_some() || executor.stdin.is_some();

    if !has_redirects {
        return executor.command.clone();
    }

    // Build shell command with redirections
    let base_cmd = executor
        .command
        .iter()
        .map(|arg| shell_escape(arg))
        .collect::<Vec<_>>()
        .join(" ");

    let mut shell_cmd = base_cmd;

    if let Some(stdin_path) = &executor.stdin {
        shell_cmd = format!("{shell_cmd} < {}", shell_escape(stdin_path));
    }
    if let Some(stdout_path) = &executor.stdout {
        shell_cmd = format!("{shell_cmd} > {}", shell_escape(stdout_path));
    }
    if let Some(stderr_path) = &executor.stderr {
        shell_cmd = format!("{shell_cmd} 2> {}", shell_escape(stderr_path));
    }

    vec!["/bin/sh".to_string(), "-c".to_string(), shell_cmd]
}

/// Simple shell escaping: wrap in single quotes, escaping existing single quotes.
fn shell_escape(s: &str) -> String {
    if s.contains(|c: char| c.is_whitespace() || "\"'\\$`!#&|;(){}[]<>?*~".contains(c)) {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}
