use std::sync::Arc;

use poiesisd::config::AppConfig;
use poiesisd::filer::Filer;
use poiesisd::runner::{DockerExecutor, Worker};
use poiesisd::{api, database};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

fn load_config() -> AppConfig {
    let path = std::env::var("POIESISD_CONFIG").unwrap_or_else(|_| "config.yaml".to_string());
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read config file '{path}': {e}"));
    serde_yaml::from_str(&contents)
        .unwrap_or_else(|e| panic!("failed to parse config file '{path}': {e}"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pool = database::init_db().await;

    // Init filer + docker + worker
    let config = load_config();
    let filer =
        Arc::new(Filer::from_config(&config.backend).expect("failed to create filer from config"));
    let docker = DockerExecutor::new().expect("failed to connect to Docker");
    let _worker = Worker::spawn(pool.clone(), filer, docker);
    tracing::info!("worker started");

    let app = api::router(pool, config.service).layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
