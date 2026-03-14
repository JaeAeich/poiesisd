use axum::Json;
use axum::extract::State;
use sqlx::SqlitePool;

use crate::api::error::ApiError;
use crate::database::insert_task;
use crate::dto::{TesCreateTaskResponse, TesTask};

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
