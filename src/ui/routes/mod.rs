mod dashboard;
mod partials;
mod sse;
mod static_files;
mod task_create;
mod task_detail;

pub use dashboard::dashboard;
pub use partials::{executor_field_partial, task_rows_partial};
pub use sse::task_events;
pub use static_files::static_file;
pub use task_create::{create_task, task_form};
pub use task_detail::task_detail;
