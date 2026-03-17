pub mod error;
mod routes;

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::config::ServiceConfig;
use crate::events::TaskEvent;
use crate::ui;

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
        .route("/ga4gh/tes/v1/tasks/{id}", get(routes::get_task))
        .route("/ga4gh/tes/v1/service-info", get(routes::service_info))
        .merge(ui::ui_router())
        .layer(axum::Extension(Arc::new(service_config)))
        .layer(axum::Extension(event_tx))
        .with_state(pool)
}
