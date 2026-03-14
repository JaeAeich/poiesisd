pub mod error;
mod routes;

use axum::Router;
use axum::routing::post;
use sqlx::SqlitePool;

pub fn router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/ga4gh/tes/v1/tasks", post(routes::create_task))
        .with_state(pool)
}
