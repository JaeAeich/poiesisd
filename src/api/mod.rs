pub mod error;
pub(crate) mod routes;

use std::sync::Arc;

use axum::Router;
use axum::response::Html;
use axum::routing::get;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use utoipa::OpenApi;

use crate::config::ServiceConfig;
use crate::dto;
use crate::events::TaskEvent;
use crate::ui;

#[derive(OpenApi)]
#[openapi(
    info(title = "PoiesisD", version = "0.1.0", description = "TES API"),
    paths(
        crate::api::routes::tasks::service_info,
        crate::api::routes::tasks::create_task,
        crate::api::routes::tasks::list_tasks,
        crate::api::routes::tasks::get_task,
        crate::api::routes::tasks::cancel_task,
    ),
    components(schemas(
        dto::TesTask,
        dto::TesExecutor,
        dto::TesInput,
        dto::TesOutput,
        dto::TesResources,
        dto::TesState,
        dto::TesServiceInfo,
        dto::TesServiceType,
        dto::TesCreateTaskResponse,
        dto::TesListTasksResponse,
        dto::TesTaskLog,
        dto::TesExecutorLog,
        dto::TesOutputFileLog,
        dto::ServiceOrganization,
        dto::Artifact,
        dto::TesFileType,
    ))
)]
struct ApiDoc;

async fn serve_openapi() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}

async fn serve_docs() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>PoiesisD — API</title>
  <style>body { margin: 0; }</style>
</head>
<body>
  <script id="api-reference" data-url="/docs/openapi.json"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#,
    )
}

pub fn router(
    pool: SqlitePool,
    service_config: ServiceConfig,
    event_tx: broadcast::Sender<TaskEvent>,
) -> Router {
    Router::new()
        .route(
            "/ga4gh/tes/v1/tasks",
            get(routes::list_tasks).post(routes::create_task),
        )
        .route(
            "/ga4gh/tes/v1/tasks/{id}",
            get(routes::get_task).post(routes::cancel_task),
        )
        .route("/ga4gh/tes/v1/service-info", get(routes::service_info))
        .route("/docs", get(serve_docs))
        .route("/docs/openapi.json", get(serve_openapi))
        .merge(ui::ui_router())
        .layer(axum::Extension(Arc::new(service_config)))
        .layer(axum::Extension(event_tx))
        .with_state(pool)
}
