use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::api::error::ApiError;
use crate::config::ServiceConfig;
use crate::database::{self, insert_task};
use crate::dto::{
    Artifact, ServiceOrganization, TesCreateTaskResponse, TesListTasksResponse, TesServiceInfo,
    TesServiceType, TesTask, TesView,
};

pub async fn service_info(
    Extension(config): Extension<Arc<ServiceConfig>>,
) -> Json<TesServiceInfo> {
    Json(TesServiceInfo::new(
        config.id.clone(),
        config.name.clone(),
        TesServiceType::new("org.ga4gh".into(), Artifact::Tes, "1.1.0".into()),
        ServiceOrganization::new(config.org_name.clone(), config.org_url.clone()),
        env!("CARGO_PKG_VERSION").into(),
    ))
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

#[derive(Deserialize)]
pub struct ViewQuery {
    #[serde(default)]
    pub view: TesView,
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub page_token: Option<String>,
    #[serde(default)]
    pub view: TesView,
}

fn default_page_size() -> i64 {
    100
}

pub async fn list_tasks(
    State(pool): State<SqlitePool>,
    Query(query): Query<ListQuery>,
) -> Result<Json<TesListTasksResponse>, ApiError> {
    let (items, next_page_token) =
        database::list_tasks(&pool, query.page_size, query.page_token.as_deref()).await?;

    let tasks: Vec<TesTask> = items
        .into_iter()
        .map(|item| {
            let mut task = TesTask::new(vec![]);
            task.id = Some(item.id);
            task.state = Some(item.state);
            task.name = item.name;
            task.creation_time = Some(item.creation_time);
            task
        })
        .collect();

    let mut resp = TesListTasksResponse::new(tasks);
    resp.next_page_token = next_page_token;
    Ok(Json(resp))
}

pub async fn get_task(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
    Query(query): Query<ViewQuery>,
) -> Result<Json<TesTask>, ApiError> {
    let task = database::get_task_by_id(&pool, &id, query.view)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("task '{id}' not found")))?;

    Ok(Json(task))
}
