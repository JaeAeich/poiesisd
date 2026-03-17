use crate::dto::TesState;

#[derive(Clone, Debug)]
pub struct TaskEvent {
    pub task_id: String,
    pub state: TesState,
}
