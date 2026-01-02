mod api;
mod db;
mod disc;
mod jobs;

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use rustripper_core::Config;

use crate::api::{create_router, WebSocketState};
use crate::db::Database;
use crate::disc::DiscWatcher;
use crate::jobs::{JobExecutor, JobQueue};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustripper_backend=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting RustRipper Backend Server");

    // Load configuration
    let config = match Config::load() {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            info!("Using default configuration");
            Config::default()
        }
    };

    let config = Arc::new(RwLock::new(config));

    // Initialize database
    let database_path = shellexpand::tilde("~/.local/share/masterrustripper/rustripper.db")
        .to_string();
    
    let database = match Database::new(&database_path).await {
        Ok(db) => {
            info!("Database initialized at {}", database_path);
            db
        }
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            std::process::exit(1);
        }
    };

    // Create WebSocket state
    let ws_state = WebSocketState::new();

    // Create job queue
    let job_queue = JobQueue::new(database.clone(), ws_state.sender.clone());

    // Create job executor and start it
    let executor = JobExecutor::new(job_queue.clone(), config.read().await.clone());
    executor.start().await;
    info!("Job executor started");

    // Create shared state for disc presence tracking
    let disc_present = Arc::new(RwLock::new(false));
    let current_disc_label = Arc::new(RwLock::new(None));

    // Create disc watcher and start it
    let disc_watcher = DiscWatcher::new(
        config.clone(),
        job_queue.clone(),
        disc_present.clone(),
        current_disc_label.clone(),
    );
    disc_watcher.start().await;
    info!("Disc watcher started");

    // Create app state
    let state = api::routes::AppState {
        config,
        database,
        job_queue,
        ws_state: ws_state.clone(),
        disc_present,
        current_disc_label,
    };

    // Create router
    let app = create_router(state);

    // Start server
    let addr = "0.0.0.0:8081";
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            info!("Server listening on http://{}", addr);
            listener
        }
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    info!("RustRipper Backend API ready");
    info!("  REST API: http://localhost:8081/api");
    info!("  WebSocket: ws://localhost:8081/ws");
    info!("  Health: http://localhost:8081/api/system/health");

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {}", e);
        std::process::exit(1);
    }
}
