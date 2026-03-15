use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::extract::{Path, State};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::api::error::ApiError;
use crate::database::insert_task;
use crate::dto::{TesCreateTaskResponse, TesState, TesTask};
use crate::filer::ServiceConfig;

pub async fn service_info(Extension(config): Extension<Arc<ServiceConfig>>) -> Json<Value> {
    Json(json!({
        "id": config.id,
        "name": config.name,
        "type": {
            "group": "org.ga4gh",
            "artifact": "tes",
            "version": "1.1.0"
        },
        "organization": {
            "name": config.org_name,
            "url": config.org_url
        },
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn create_task(
    State(pool): State<SqlitePool>,
    Json(task): Json<TesTask>,
) -> Result<Json<TesCreateTaskResponse>, ApiError> {
    if task.executors.is_empty() {
        return Err(ApiError::Validation(
            "executors must not be empty".to_string(),
        ));
    }

    let id = insert_task(&pool, &task).await?;
    Ok(Json(TesCreateTaskResponse::new(id)))
}

pub async fn get_task(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Json<TesTask>, ApiError> {
    let row = sqlx::query!(
        "SELECT id, state, name, description, creation_time FROM tasks WHERE id = ?",
        id,
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| ApiError::Validation(format!("task '{id}' not found")))?;

    let state = match row.state.as_str() {
        "UNKNOWN" => TesState::Unknown,
        "QUEUED" => TesState::Queued,
        "INITIALIZING" => TesState::Initializing,
        "RUNNING" => TesState::Running,
        "PAUSED" => TesState::Paused,
        "COMPLETE" => TesState::Complete,
        "EXECUTOR_ERROR" => TesState::ExecutorError,
        "SYSTEM_ERROR" => TesState::SystemError,
        "CANCELED" => TesState::Canceled,
        "PREEMPTED" => TesState::Preempted,
        "CANCELING" => TesState::Canceling,
        _ => TesState::Unknown,
    };

    let task = TesTask {
        id: row.id,
        state: Some(state),
        name: row.name,
        description: row.description,
        creation_time: Some(row.creation_time),
        executors: Vec::new(),
        ..Default::default()
    };

    Ok(Json(task))
}
