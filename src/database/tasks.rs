use sqlx::SqlitePool;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::dto::TesTask;

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

    sqlx::query(
        "INSERT INTO tasks (id, name, description, state, creation_time, cpu_cores, preemptible, ram_gb, disk_gb, zones, backend_parameters, backend_parameters_strict)
         VALUES (?, ?, ?, 'QUEUED', ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&task.name)
    .bind(&task.description)
    .bind(&creation_time)
    .bind(cpu_cores)
    .bind(preemptible)
    .bind(ram_gb)
    .bind(disk_gb)
    .bind(&zones)
    .bind(&backend_parameters)
    .bind(bp_strict)
    .execute(&mut *tx)
    .await?;

    // Inputs
    if let Some(inputs) = &task.inputs {
        for input in inputs {
            let file_type = input.r#type.as_ref().map(|t| t.to_string());
            let streamable = input.streamable.map(|b| b as i32);
            sqlx::query(
                "INSERT INTO task_inputs (task_id, name, description, url, path, type, content, streamable)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&input.name)
            .bind(&input.description)
            .bind(&input.url)
            .bind(&input.path)
            .bind(&file_type)
            .bind(&input.content)
            .bind(streamable)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Outputs
    if let Some(outputs) = &task.outputs {
        for output in outputs {
            let file_type = output.r#type.as_ref().map(|t| t.to_string());
            sqlx::query(
                "INSERT INTO task_outputs (task_id, name, description, url, path, path_prefix, type)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&output.name)
            .bind(&output.description)
            .bind(&output.url)
            .bind(&output.path)
            .bind(&output.path_prefix)
            .bind(&file_type)
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
        sqlx::query(
            "INSERT INTO task_executors (task_id, executor_index, image, command, workdir, stdin, stdout, stderr, env, ignore_error)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(i as i32)
        .bind(&executor.image)
        .bind(&command)
        .bind(&executor.workdir)
        .bind(&executor.stdin)
        .bind(&executor.stdout)
        .bind(&executor.stderr)
        .bind(&env)
        .bind(ignore_error)
        .execute(&mut *tx)
        .await?;
    }

    // Volumes
    if let Some(volumes) = &task.volumes {
        for vol in volumes {
            sqlx::query("INSERT INTO task_volumes (task_id, volume_path) VALUES (?, ?)")
                .bind(&id)
                .bind(vol)
                .execute(&mut *tx)
                .await?;
        }
    }

    // Tags
    if let Some(tags) = &task.tags {
        for (key, value) in tags {
            sqlx::query("INSERT INTO task_tags (task_id, tag_key, tag_value) VALUES (?, ?, ?)")
                .bind(&id)
                .bind(key)
                .bind(value)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    Ok(id)
}
