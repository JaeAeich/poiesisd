mod routes;

use axum::Router;
use axum::routing::get;
use sqlx::SqlitePool;

pub fn ui_router() -> Router<SqlitePool> {
    Router::new()
        .route("/", get(routes::dashboard))
        .route(
            "/tasks/new",
            get(routes::task_form).post(routes::create_task),
        )
        .route("/tasks/{id}", get(routes::task_detail))
        .route("/tasks/{id}/events", get(routes::task_events))
        .route("/static/{*path}", get(routes::static_file))
        .route("/partials/task-rows", get(routes::task_rows_partial))
        .route(
            "/partials/executor-field",
            get(routes::executor_field_partial),
        )
}
