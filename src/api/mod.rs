pub mod error;
mod routes;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use sqlx::SqlitePool;

use crate::filer::ServiceConfig;

pub fn router(pool: SqlitePool, service_config: ServiceConfig) -> Router {
    Router::new()
        .route("/ga4gh/tes/v1/tasks", post(routes::create_task))
        .route("/ga4gh/tes/v1/tasks/{id}", get(routes::get_task))
        .route("/ga4gh/tes/v1/service-info", get(routes::service_info))
        .layer(axum::Extension(Arc::new(service_config)))
        .with_state(pool)
}
