use askama::Template;
use axum::response::Html;

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate;

pub async fn dashboard() -> Html<String> {
    let tmpl = DashboardTemplate;
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}
