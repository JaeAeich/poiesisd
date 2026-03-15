use std::collections::HashMap;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::dto::{
    TesExecutor, TesExecutorLog, TesInput, TesOutput, TesOutputFileLog, TesState, TesTask,
    TesTaskLog,
};

/// Full task data needed by the worker, reconstructed from all related tables.
pub struct FullTask {
    pub inputs: Vec<TesInput>,
    pub outputs: Vec<TesOutput>,
    pub executors: Vec<TesExecutor>,
    pub volumes: Vec<String>,
}

/// Controls how much data to return for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TesView {
    Minimal,
    Basic,
    Full,
}

pub async fn insert_task(pool: &SqlitePool, task: &TesTask) -> Result<String, ApiError> {
    let id = Uuid::new_v4().to_string();
    let creation_time = chrono::Utc::now().to_rfc3339();

    let mut tx = pool.begin().await?;

    // Extract resource fields
    let (cpu_cores, preemptible, ram_gb, disk_gb, zones, backend_parameters, bp_strict) =
        match &task.resources {
            Some(r) => (
                r.cpu_cores,
                r.preemptible.map(|b| b as i32),
                r.ram_gb,
                r.disk_gb,
                r.zones.as_ref().map(|z| serde_json::to_string(z).unwrap()),
                r.backend_parameters
                    .as_ref()
                    .map(|bp| serde_json::to_string(bp).unwrap()),
                r.backend_parameters_strict.map(|b| b as i32).unwrap_or(0),
            ),
            None => (None, None, None, None, None, None, 0),
        };

    sqlx::query!(
        "INSERT INTO tasks (id, name, description, state, creation_time, cpu_cores, preemptible, ram_gb, disk_gb, zones, backend_parameters, backend_parameters_strict)
         VALUES (?, ?, ?, 'QUEUED', ?, ?, ?, ?, ?, ?, ?, ?)",
        id,
        task.name,
        task.description,
        creation_time,
        cpu_cores,
        preemptible,
        ram_gb,
        disk_gb,
        zones,
        backend_parameters,
        bp_strict,
    )
    .execute(&mut *tx)
    .await?;

    // Inputs
    if let Some(inputs) = &task.inputs {
        for input in inputs {
            let file_type = input.r#type.as_ref().map(|t| t.to_string());
            let streamable = input.streamable.map(|b| b as i32);
            sqlx::query!(
                "INSERT INTO task_inputs (task_id, name, description, url, path, type, content, streamable)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                id,
                input.name,
                input.description,
                input.url,
                input.path,
                file_type,
                input.content,
                streamable,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    // Outputs
    if let Some(outputs) = &task.outputs {
        for output in outputs {
            let file_type = output.r#type.as_ref().map(|t| t.to_string());
            sqlx::query!(
                "INSERT INTO task_outputs (task_id, name, description, url, path, path_prefix, type)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                id,
                output.name,
                output.description,
                output.url,
                output.path,
                output.path_prefix,
                file_type,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    // Executors
    for (i, executor) in task.executors.iter().enumerate() {
        let command = serde_json::to_string(&executor.command).unwrap();
        let env = executor
            .env
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap());
        let ignore_error = executor.ignore_error.map(|b| b as i32);
        let idx = i as i32;
        sqlx::query!(
            "INSERT INTO task_executors (task_id, executor_index, image, command, workdir, stdin, stdout, stderr, env, ignore_error)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            id,
            idx,
            executor.image,
            command,
            executor.workdir,
            executor.stdin,
            executor.stdout,
            executor.stderr,
            env,
            ignore_error,
        )
        .execute(&mut *tx)
        .await?;
    }

    // Volumes
    if let Some(volumes) = &task.volumes {
        for vol in volumes {
            sqlx::query!(
                "INSERT INTO task_volumes (task_id, volume_path) VALUES (?, ?)",
                id,
                vol,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    // Tags
    if let Some(tags) = &task.tags {
        for (key, value) in tags {
            sqlx::query!(
                "INSERT INTO task_tags (task_id, tag_key, tag_value) VALUES (?, ?, ?)",
                id,
                key,
                value,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(id)
}

/// Atomically claim the oldest QUEUED task by setting its state to INITIALIZING.
/// Returns the task id if one was claimed, None if no queued tasks exist.
pub async fn claim_queued_task(pool: &SqlitePool) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query_scalar!(
        "UPDATE tasks SET state = 'INITIALIZING'
         WHERE id = (SELECT id FROM tasks WHERE state = 'QUEUED' ORDER BY creation_time LIMIT 1)
         RETURNING id"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.flatten())
}

/// Fetch a task by ID with the requested view level.
pub async fn get_task_by_id(
    pool: &SqlitePool,
    task_id: &str,
    view: TesView,
) -> Result<Option<TesTask>, sqlx::Error> {
    let row = sqlx::query!(
        "SELECT id, state, name, description, creation_time FROM tasks WHERE id = ?",
        task_id,
    )
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let state: TesState = row.state.parse().unwrap_or_default();

    if view == TesView::Minimal {
        return Ok(Some(TesTask {
            id: row.id,
            state: Some(state),
            ..Default::default()
        }));
    }

    // BASIC: fetch inputs, outputs, executors, volumes, tags
    let inputs = fetch_inputs(pool, task_id).await?;
    let outputs = fetch_outputs(pool, task_id).await?;
    let executors = fetch_executors(pool, task_id).await?;
    let volumes = fetch_volumes(pool, task_id).await?;
    let tags = fetch_tags(pool, task_id).await?;

    let mut task = TesTask {
        id: row.id,
        state: Some(state),
        name: row.name,
        description: row.description,
        creation_time: Some(row.creation_time),
        inputs: if inputs.is_empty() {
            None
        } else {
            Some(inputs)
        },
        outputs: if outputs.is_empty() {
            None
        } else {
            Some(outputs)
        },
        executors,
        volumes: if volumes.is_empty() {
            None
        } else {
            Some(volumes)
        },
        tags: if tags.is_empty() { None } else { Some(tags) },
        ..Default::default()
    };

    // FULL: also fetch logs
    if view == TesView::Full {
        task.logs = Some(fetch_task_logs(pool, task_id).await?);
    }

    Ok(Some(task))
}

/// Fetch complete task data from all related tables (used by the worker).
pub async fn get_task_full(pool: &SqlitePool, task_id: &str) -> Result<FullTask, sqlx::Error> {
    let inputs = fetch_inputs(pool, task_id).await?;
    let outputs = fetch_outputs(pool, task_id).await?;

    // Executors
    let exec_rows = sqlx::query!(
        "SELECT image, command, workdir, stdin, stdout, stderr, env, ignore_error
         FROM task_executors WHERE task_id = ? ORDER BY executor_index",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    let executors: Vec<TesExecutor> = exec_rows
        .iter()
        .map(|row| {
            let command: Vec<String> = serde_json::from_str(&row.command).unwrap_or_else(|e| {
                tracing::warn!(task_id, error = %e, "failed to parse command JSON");
                Vec::new()
            });
            let env: Option<HashMap<String, String>> = row.env.as_deref().and_then(|j| {
                serde_json::from_str(j).unwrap_or_else(|e| {
                    tracing::warn!(task_id, error = %e, "failed to parse env JSON");
                    None
                })
            });

            TesExecutor {
                image: row.image.clone(),
                command,
                workdir: row.workdir.clone(),
                stdin: row.stdin.clone(),
                stdout: row.stdout.clone(),
                stderr: row.stderr.clone(),
                env,
                ignore_error: row.ignore_error.map(|v| v != 0),
            }
        })
        .collect();

    // Volumes
    let vol_rows = sqlx::query!(
        "SELECT volume_path FROM task_volumes WHERE task_id = ? ORDER BY id",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    let volumes: Vec<String> = vol_rows.iter().map(|row| row.volume_path.clone()).collect();

    Ok(FullTask {
        inputs,
        outputs,
        executors,
        volumes,
    })
}

/// Update the state of a task.
pub async fn update_task_state(
    pool: &SqlitePool,
    task_id: &str,
    state: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE tasks SET state = ? WHERE id = ?", state, task_id,)
        .execute(pool)
        .await?;
    Ok(())
}

/// Insert a task log with executor logs and output file logs.
pub async fn insert_task_log(
    pool: &SqlitePool,
    task_id: &str,
    task_log: &TesTaskLog,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let metadata = task_log
        .metadata
        .as_ref()
        .map(|m| serde_json::to_string(m).unwrap());
    let system_logs = task_log
        .system_logs
        .as_ref()
        .map(|s| serde_json::to_string(s).unwrap());

    // Determine log_index (next available for this task)
    let next_idx = sqlx::query_scalar!(
        "SELECT COALESCE(MAX(log_index), -1) + 1 FROM task_logs WHERE task_id = ?",
        task_id,
    )
    .fetch_one(&mut *tx)
    .await?;
    let log_index = next_idx as i32;

    let task_log_id = sqlx::query_scalar!(
        "INSERT INTO task_logs (task_id, log_index, metadata, start_time, end_time, system_logs)
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        task_id,
        log_index,
        metadata,
        task_log.start_time,
        task_log.end_time,
        system_logs,
    )
    .fetch_one(&mut *tx)
    .await?;

    // Executor logs
    for (i, elog) in task_log.logs.iter().enumerate() {
        let idx = i as i32;
        sqlx::query!(
            "INSERT INTO executor_logs (task_log_id, executor_index, start_time, end_time, stdout, stderr, exit_code)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            task_log_id,
            idx,
            elog.start_time,
            elog.end_time,
            elog.stdout,
            elog.stderr,
            elog.exit_code,
        )
        .execute(&mut *tx)
        .await?;
    }

    // Output file logs
    for ofl in &task_log.outputs {
        sqlx::query!(
            "INSERT INTO output_file_logs (task_log_id, url, path, size_bytes) VALUES (?, ?, ?, ?)",
            task_log_id,
            ofl.url,
            ofl.path,
            ofl.size_bytes,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

// --- Shared fetch helpers ---

async fn fetch_inputs(pool: &SqlitePool, task_id: &str) -> Result<Vec<TesInput>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT name, description, url, path, type, content, streamable
         FROM task_inputs WHERE task_id = ? ORDER BY id",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| TesInput {
            name: row.name.clone(),
            description: row.description.clone(),
            url: row.url.clone(),
            path: row.path.clone(),
            r#type: row.r#type.as_deref().and_then(|t| match t {
                "FILE" => Some(crate::dto::TesFileType::File),
                "DIRECTORY" => Some(crate::dto::TesFileType::Directory),
                _ => None,
            }),
            content: row.content.clone(),
            streamable: row.streamable.map(|v| v != 0),
        })
        .collect())
}

async fn fetch_outputs(pool: &SqlitePool, task_id: &str) -> Result<Vec<TesOutput>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT name, description, url, path, path_prefix, type
         FROM task_outputs WHERE task_id = ? ORDER BY id",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| TesOutput {
            name: row.name.clone(),
            description: row.description.clone(),
            url: row.url.clone(),
            path: row.path.clone(),
            path_prefix: row.path_prefix.clone(),
            r#type: row.r#type.as_deref().and_then(|t| match t {
                "FILE" => Some(crate::dto::TesFileType::File),
                "DIRECTORY" => Some(crate::dto::TesFileType::Directory),
                _ => None,
            }),
        })
        .collect())
}

async fn fetch_executors(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<Vec<TesExecutor>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT image, command, workdir, stdin, stdout, stderr, env, ignore_error
         FROM task_executors WHERE task_id = ? ORDER BY executor_index",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            let command: Vec<String> = serde_json::from_str(&row.command).unwrap_or_else(|e| {
                tracing::warn!(task_id, error = %e, "failed to parse command JSON");
                Vec::new()
            });
            let env: Option<HashMap<String, String>> = row.env.as_deref().and_then(|j| {
                serde_json::from_str(j).unwrap_or_else(|e| {
                    tracing::warn!(task_id, error = %e, "failed to parse env JSON");
                    None
                })
            });

            TesExecutor {
                image: row.image.clone(),
                command,
                workdir: row.workdir.clone(),
                stdin: row.stdin.clone(),
                stdout: row.stdout.clone(),
                stderr: row.stderr.clone(),
                env,
                ignore_error: row.ignore_error.map(|v| v != 0),
            }
        })
        .collect())
}

async fn fetch_volumes(pool: &SqlitePool, task_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT volume_path FROM task_volumes WHERE task_id = ? ORDER BY id",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|row| row.volume_path.clone()).collect())
}

async fn fetch_tags(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<HashMap<String, String>, sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT tag_key, tag_value FROM task_tags WHERE task_id = ? ORDER BY id",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            (
                row.tag_key.clone(),
                row.tag_value.clone().unwrap_or_default(),
            )
        })
        .collect())
}

async fn fetch_task_logs(pool: &SqlitePool, task_id: &str) -> Result<Vec<TesTaskLog>, sqlx::Error> {
    let log_rows = sqlx::query!(
        "SELECT id, metadata, start_time, end_time, system_logs
         FROM task_logs WHERE task_id = ? ORDER BY log_index",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    let mut logs = Vec::new();
    for log_row in &log_rows {
        // Executor logs
        let exec_log_rows = sqlx::query!(
            "SELECT start_time, end_time, stdout, stderr, exit_code
             FROM executor_logs WHERE task_log_id = ? ORDER BY executor_index",
            log_row.id,
        )
        .fetch_all(pool)
        .await?;

        let executor_logs: Vec<TesExecutorLog> = exec_log_rows
            .iter()
            .map(|r| {
                let mut log = TesExecutorLog::new(r.exit_code as i32);
                log.start_time = r.start_time.clone();
                log.end_time = r.end_time.clone();
                log.stdout = r.stdout.clone();
                log.stderr = r.stderr.clone();
                log
            })
            .collect();

        // Output file logs
        let ofl_rows = sqlx::query!(
            "SELECT url, path, size_bytes FROM output_file_logs WHERE task_log_id = ? ORDER BY id",
            log_row.id,
        )
        .fetch_all(pool)
        .await?;

        let output_file_logs: Vec<TesOutputFileLog> = ofl_rows
            .iter()
            .map(|r| TesOutputFileLog::new(r.url.clone(), r.path.clone(), r.size_bytes.to_string()))
            .collect();

        let metadata: Option<HashMap<String, String>> = log_row.metadata.as_deref().and_then(|j| {
            serde_json::from_str(j).unwrap_or_else(|e| {
                tracing::warn!(task_id, error = %e, "failed to parse metadata JSON");
                None
            })
        });

        let system_logs: Option<Vec<String>> = log_row.system_logs.as_deref().and_then(|j| {
            serde_json::from_str(j).unwrap_or_else(|e| {
                tracing::warn!(task_id, error = %e, "failed to parse system_logs JSON");
                None
            })
        });

        let mut task_log = TesTaskLog::new(executor_logs, output_file_logs);
        task_log.metadata = metadata;
        task_log.start_time = log_row.start_time.clone();
        task_log.end_time = log_row.end_time.clone();
        task_log.system_logs = system_logs;

        logs.push(task_log);
    }

    Ok(logs)
}
