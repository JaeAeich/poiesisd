use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum_extra::extract::Query as ExtraQuery;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::api::error::ApiError;
use crate::config::ServiceConfig;
use crate::database::{self, ListFilter, insert_task};
use crate::dto::{
    Artifact, ServiceOrganization, TesCreateTaskResponse, TesListTasksResponse, TesServiceInfo,
    TesServiceType, TesTask, TesView,
};
use crate::events::TaskEvent;

#[utoipa::path(get, path = "/ga4gh/tes/v1/service-info", responses((status = 200, body = TesServiceInfo)))]
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

#[utoipa::path(post, path = "/ga4gh/tes/v1/tasks", request_body = TesTask, responses((status = 200, body = TesCreateTaskResponse)))]
pub async fn create_task(
    State(pool): State<SqlitePool>,
    Json(task): Json<TesTask>,
) -> Result<Json<TesCreateTaskResponse>, ApiError> {
    if task.executors.is_empty() {
        return Err(ApiError::Validation(
            "executors must not be empty".to_string(),
        ));
    }

    // Reject unknown backend parameters when strict mode is set
    if let Some(ref resources) = task.resources {
        if resources.backend_parameters_strict == Some(true) {
            if let Some(ref params) = resources.backend_parameters {
                // poiesisD doesn't support any backend parameters
                if !params.is_empty() {
                    return Err(ApiError::Validation(format!(
                        "unknown backend parameters: {}",
                        params.keys().cloned().collect::<Vec<_>>().join(", ")
                    )));
                }
            }
        }
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
    pub name_prefix: Option<String>,
    pub state: Option<String>,
    #[serde(default)]
    pub tag_key: Vec<String>,
    #[serde(default)]
    pub tag_value: Vec<String>,
}

fn default_page_size() -> i64 {
    100
}

#[utoipa::path(
    get, path = "/ga4gh/tes/v1/tasks",
    params(
        ("page_size" = Option<i64>, Query, description = "Page size"),
        ("page_token" = Option<String>, Query, description = "Page token"),
        ("name_prefix" = Option<String>, Query, description = "Filter by name prefix"),
        ("state" = Option<String>, Query, description = "Filter by state"),
        ("tag_key" = Option<String>, Query, description = "Filter by tag keys (comma-separated)"),
        ("tag_value" = Option<String>, Query, description = "Filter by tag values (comma-separated)"),
    ),
    responses((status = 200, body = TesListTasksResponse))
)]
pub async fn list_tasks(
    State(pool): State<SqlitePool>,
    ExtraQuery(query): ExtraQuery<ListQuery>,
) -> Result<Json<TesListTasksResponse>, ApiError> {
    let tag_keys: Vec<&str> = query.tag_key.iter().map(String::as_str).collect();
    let tag_values: Vec<&str> = query.tag_value.iter().map(String::as_str).collect();

    let filter = ListFilter {
        name_prefix: query.name_prefix.as_deref(),
        state: query.state.as_deref(),
        tag_key: if tag_keys.is_empty() {
            None
        } else {
            Some(tag_keys)
        },
        tag_value: if tag_values.is_empty() {
            None
        } else {
            Some(tag_values)
        },
    };

    let (items, next_page_token) =
        database::list_tasks(&pool, query.page_size, query.page_token.as_deref(), &filter).await?;

    // For MINIMAL view, return lightweight list items.
    // For BASIC/FULL, fetch each task with the requested view level.
    let tasks: Vec<TesTask> = match query.view {
        TesView::Minimal => items
            .into_iter()
            .map(|item| {
                let mut task = TesTask::new(vec![]);
                task.id = Some(item.id);
                task.state = Some(item.state);
                task.name = item.name;
                task.creation_time = Some(item.creation_time);
                task
            })
            .collect(),
        _ => {
            let mut out = Vec::with_capacity(items.len());
            for item in &items {
                if let Some(task) = database::get_task_by_id(&pool, &item.id, query.view).await? {
                    out.push(task);
                }
            }
            out
        }
    };

    let mut resp = TesListTasksResponse::new(tasks);
    resp.next_page_token = next_page_token;
    Ok(Json(resp))
}

#[utoipa::path(get, path = "/ga4gh/tes/v1/tasks/{id}", params(("id" = String, Path, description = "Task ID"), ("view" = Option<String>, Query, description = "MINIMAL, BASIC, or FULL")), responses((status = 200, body = TesTask)))]
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

#[utoipa::path(post, path = "/ga4gh/tes/v1/tasks/{id}:cancel", params(("id" = String, Path, description = "Task ID")), responses((status = 200)))]
pub async fn cancel_task(
    State(pool): State<SqlitePool>,
    Path(id_raw): Path<String>,
    Extension(event_tx): Extension<tokio::sync::broadcast::Sender<TaskEvent>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Path captures "uuid:cancel" — strip the ":cancel" suffix
    let id = id_raw
        .strip_suffix(":cancel")
        .unwrap_or(&id_raw)
        .to_string();

    let canceled = database::cancel_task(&pool, &id).await?;
    if !canceled {
        return Err(ApiError::NotFound(format!(
            "task '{id}' not found or already in terminal state"
        )));
    }
    let _ = event_tx.send(TaskEvent {
        task_id: id,
        state: crate::dto::TesState::Canceled,
    });
    Ok(Json(serde_json::json!({})))
}
