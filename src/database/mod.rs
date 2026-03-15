mod tasks;

pub use tasks::{
    FullTask, TesView, claim_queued_task, get_task_by_id, get_task_full, insert_task,
    insert_task_log, update_task_state,
};

use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    name TEXT,
    description TEXT,
    state TEXT NOT NULL DEFAULT 'QUEUED'
        CHECK(state IN ('UNKNOWN','QUEUED','INITIALIZING','RUNNING','PAUSED',
              'COMPLETE','EXECUTOR_ERROR','SYSTEM_ERROR','CANCELED','PREEMPTED','CANCELING')),
    creation_time TEXT NOT NULL,
    cpu_cores INTEGER,
    preemptible INTEGER,
    ram_gb REAL,
    disk_gb REAL,
    zones TEXT,
    backend_parameters TEXT,
    backend_parameters_strict INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS task_inputs (
    id INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    name TEXT,
    description TEXT,
    url TEXT,
    path TEXT NOT NULL,
    type TEXT,
    content TEXT,
    streamable INTEGER
);

CREATE TABLE IF NOT EXISTS task_outputs (
    id INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    name TEXT,
    description TEXT,
    url TEXT NOT NULL,
    path TEXT NOT NULL,
    path_prefix TEXT,
    type TEXT
);

CREATE TABLE IF NOT EXISTS task_executors (
    id INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    executor_index INTEGER NOT NULL,
    image TEXT NOT NULL,
    command TEXT NOT NULL,
    workdir TEXT,
    stdin TEXT,
    stdout TEXT,
    stderr TEXT,
    env TEXT,
    ignore_error INTEGER,
    UNIQUE(task_id, executor_index)
);

CREATE TABLE IF NOT EXISTS task_volumes (
    id INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    volume_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_tags (
    id INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    tag_key TEXT NOT NULL,
    tag_value TEXT,
    UNIQUE(task_id, tag_key)
);

CREATE TABLE IF NOT EXISTS task_logs (
    id INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    log_index INTEGER NOT NULL,
    metadata TEXT,
    start_time TEXT,
    end_time TEXT,
    system_logs TEXT,
    UNIQUE(task_id, log_index)
);

CREATE TABLE IF NOT EXISTS executor_logs (
    id INTEGER PRIMARY KEY,
    task_log_id INTEGER NOT NULL REFERENCES task_logs(id) ON DELETE CASCADE,
    executor_index INTEGER NOT NULL,
    start_time TEXT,
    end_time TEXT,
    stdout TEXT,
    stderr TEXT,
    exit_code INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS output_file_logs (
    id INTEGER PRIMARY KEY,
    task_log_id INTEGER NOT NULL REFERENCES task_logs(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_state_creation ON tasks(state, creation_time);
CREATE INDEX IF NOT EXISTS idx_task_inputs_task_id ON task_inputs(task_id);
CREATE INDEX IF NOT EXISTS idx_task_outputs_task_id ON task_outputs(task_id);
CREATE INDEX IF NOT EXISTS idx_task_executors_task_id ON task_executors(task_id);
CREATE INDEX IF NOT EXISTS idx_task_logs_task_id ON task_logs(task_id);
"#;

pub async fn init_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite:poiesisd.db?mode=rwc")
        .await
        .expect("failed to connect to SQLite");

    for statement in SCHEMA.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed)
                .execute(&pool)
                .await
                .expect("failed to run schema migration");
        }
    }

    pool
}
