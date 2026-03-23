use axum::extract::Path;
use axum::http::{StatusCode, header};
use axum::response::IntoResponse;

pub async fn static_file(Path(path): Path<String>) -> impl IntoResponse {
    let (content, content_type) = match path.as_str() {
        "htmx.min.js" => (
            include_bytes!("../../../static/htmx.min.js").as_slice(),
            "application/javascript",
        ),
        "htmx-sse.js" => (
            include_bytes!("../../../static/htmx-sse.js").as_slice(),
            "application/javascript",
        ),
        "pico.min.css" => (
            include_bytes!("../../../static/pico.min.css").as_slice(),
            "text/css",
        ),
        "app.css" => (
            include_bytes!("../../../static/app.css").as_slice(),
            "text/css",
        ),
        "app.js" => (
            include_bytes!("../../../static/app.js").as_slice(),
            "application/javascript",
        ),
        _ => return Err(StatusCode::NOT_FOUND),
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        content,
    ))
}
