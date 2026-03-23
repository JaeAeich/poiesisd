use axum::Extension;
use axum::extract::Path;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::Stream;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::events::TaskEvent;

pub async fn task_events(
    Path(id): Path<String>,
    Extension(event_tx): Extension<broadcast::Sender<TaskEvent>>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = event_tx.subscribe();

    let stream = BroadcastStream::new(rx)
        .filter_map(move |result| match result {
            Ok(event) if event.task_id == id => Some(event),
            _ => None,
        })
        .map(move |event: TaskEvent| {
            let state = event.state;
            let state_lower = state.to_string().to_lowercase();
            let state_html = format!("<span class=\"badge badge-{state_lower}\">{state}</span>");

            Ok(Event::default().event("state").data(state_html))
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
