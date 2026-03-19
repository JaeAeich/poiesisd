use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use sqlx::SqlitePool;

use crate::database;
use crate::dto::{TesTask, TesView};

#[derive(Template)]
#[template(path = "task_detail.html")]
struct TaskDetailTemplate {
    task: TesTask,
}

pub async fn task_detail(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let task = database::get_task_by_id(&pool, &id, TesView::Full)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let tmpl = TaskDetailTemplate { task };
    Ok(Html(
        tmpl.render()
            .unwrap_or_else(|e| format!("Template error: {e}")),
    ))
}
