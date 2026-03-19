use askama::Template;
use axum::extract::{Query, State};
use axum::response::Html;
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::database::{self, TaskListItem};

#[derive(Template)]
#[template(path = "partials/task_row.html")]
struct TaskRowTemplate<'a> {
    task: &'a TaskListItem,
}

#[derive(Template)]
#[template(path = "partials/executor_field.html")]
struct ExecutorFieldTemplate {
    index: usize,
}

#[derive(Deserialize)]
pub struct TaskRowsQuery {
    pub page_token: Option<String>,
}

pub async fn task_rows_partial(
    State(pool): State<SqlitePool>,
    Query(query): Query<TaskRowsQuery>,
) -> Html<String> {
    let (tasks, next_page_token) = database::list_tasks(&pool, 25, query.page_token.as_deref())
        .await
        .unwrap_or_default();

    let mut html = String::new();
    for task in &tasks {
        let tmpl = TaskRowTemplate { task };
        if let Ok(rendered) = tmpl.render() {
            html.push_str(&rendered);
        }
    }

    // Render load-more into #side-footer via out-of-band swap.
    // The button appends the next page below existing tasks.
    if let Some(token) = next_page_token {
        html.push_str(&format!(
            "<div id=\"side-footer\" hx-swap-oob=\"innerHTML\">\
             <button class=\"load-more-btn\" \
             hx-get=\"/partials/task-rows?page_token={token}\" \
             hx-target=\"#explorer\" hx-swap=\"beforeend\" \
             >Load more</button></div>"
        ));
    } else {
        // Clear load-more when no more pages
        html.push_str("<div id=\"side-footer\" hx-swap-oob=\"innerHTML\"></div>");
    }

    Html(html)
}

#[derive(Deserialize)]
pub struct ExecutorFieldQuery {
    pub index: Option<usize>,
}

pub async fn executor_field_partial(Query(query): Query<ExecutorFieldQuery>) -> Html<String> {
    // Count existing executor groups by checking what index we should use next
    // The frontend should pass the next index, defaulting to 1
    let index = query.index.unwrap_or(1);
    let tmpl = ExecutorFieldTemplate { index };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}
