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
    Artifact, ServiceOrganization, TesCreateTaskResponse, TesServiceInfo, TesServiceType, TesTask,
    TesView,
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
