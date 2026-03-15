use std::collections::HashMap;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::dto::{TesExecutor, TesInput, TesOutput, TesTask, TesTaskLog};

/// Full task data needed by the worker, reconstructed from all related tables.
pub struct FullTask {
    pub inputs: Vec<TesInput>,
    pub outputs: Vec<TesOutput>,
    pub executors: Vec<TesExecutor>,
    pub volumes: Vec<String>,
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

/// Fetch complete task data from all related tables.
pub async fn get_task_full(pool: &SqlitePool, task_id: &str) -> Result<FullTask, sqlx::Error> {
    // Inputs
    let input_rows = sqlx::query!(
        "SELECT name, description, url, path, type, content, streamable
         FROM task_inputs WHERE task_id = ? ORDER BY id",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    let inputs: Vec<TesInput> = input_rows
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
        .collect();

    // Outputs
    let output_rows = sqlx::query!(
        "SELECT name, description, url, path, path_prefix, type
         FROM task_outputs WHERE task_id = ? ORDER BY id",
        task_id,
    )
    .fetch_all(pool)
    .await?;

    let outputs: Vec<TesOutput> = output_rows
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
        .collect();

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
            let command: Vec<String> = serde_json::from_str(&row.command).unwrap_or_default();
            let env: Option<HashMap<String, String>> = row
                .env
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());

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
