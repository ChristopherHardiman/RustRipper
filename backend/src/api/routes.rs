use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use rustripper_core::Config;
use rustripper_metadata::{MetadataAggregator, MediaType};

use crate::api::{middleware, websocket, WebSocketState};
use crate::db::{Database, RipStats};
use crate::jobs::{Job, JobQueue};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub database: Database,
    pub job_queue: JobQueue,
    pub ws_state: WebSocketState,
    pub disc_present: Arc<RwLock<bool>>,
    pub current_disc_label: Arc<RwLock<Option<String>>>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // WebSocket
        .route("/ws", get(websocket::websocket_handler))
        // System
        .route("/api/status", get(get_status))
        .route("/api/system/health", get(health_check))
        // Jobs
        .route("/api/jobs", get(list_jobs).post(create_job))
        .route("/api/jobs/:id", get(get_job).delete(delete_job))
        .route("/api/jobs/:id/priority", put(update_job_priority))
        // History
        .route("/api/history", get(get_history))
        .route("/api/history/:id", get(get_history_item))
        .route("/api/history/stats", get(get_history_stats))
        // Config
        .route("/api/config", get(get_config).put(update_config))
        // Metadata
        .route("/api/metadata/search", post(search_metadata))
        .layer(middleware::create_cors_layer())
        .layer(middleware::create_trace_layer())
        .with_state(state)
}

// ============================================================================
// System Endpoints
// ============================================================================

#[derive(Serialize)]
struct StatusResponse {
    disc_present: bool,
    disc_label: Option<String>,
    active_jobs: usize,
    queue_length: usize,
    system_ready: bool,
}

async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let disc_present = *state.disc_present.read().await;
    let disc_label = state.current_disc_label.read().await.clone();
    let queue_length = state.job_queue.len().await;
    
    Json(StatusResponse {
        disc_present,
        disc_label,
        active_jobs: 0, // TODO: Track active jobs
        queue_length,
        system_ready: true,
    })
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

// ============================================================================
// Job Endpoints
// ============================================================================

#[derive(Deserialize)]
struct CreateJobRequest {
    disc_label: String,
    title: Option<String>,
    year: Option<i32>,
}

#[derive(Serialize)]
struct JobResponse {
    job_id: i64,
    message: String,
}

async fn create_job(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<JobResponse>, (StatusCode, String)> {
    let mut job = Job::new(request.disc_label.clone());
    
    if let Some(title) = request.title {
        job = job.with_metadata(title, request.year);
    }

    match state.job_queue.enqueue(job).await {
        Ok(job_id) => {
            info!("Job {} created via API", job_id);
            Ok(Json(JobResponse {
                job_id,
                message: "Job created successfully".to_string(),
            }))
        }
        Err(e) => {
            error!("Failed to create job: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

#[derive(Deserialize)]
struct ListJobsQuery {
    status: Option<String>,
}

async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    match state.database.get_jobs(query.status.as_deref()).await {
        Ok(jobs) => {
            let json_jobs: Vec<serde_json::Value> = jobs
                .into_iter()
                .map(|job| serde_json::to_value(job).unwrap())
                .collect();
            Ok(Json(json_jobs))
        }
        Err(e) => {
            error!("Failed to list jobs: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match state.database.get_job(id).await {
        Ok(Some(job)) => Ok(Json(serde_json::to_value(job).unwrap())),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Job not found".to_string())),
        Err(e) => {
            error!("Failed to get job: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

async fn delete_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, String)> {
    match state.job_queue.remove_job(id).await {
        Ok(_) => {
            info!("Job {} deleted via API", id);
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to delete job: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
    }
}

#[derive(Deserialize)]
struct UpdatePriorityRequest {
    priority: i32,
}

async fn update_job_priority(
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
    Json(_request): Json<UpdatePriorityRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // TODO: Implement priority queue reordering
    Ok(StatusCode::NOT_IMPLEMENTED)
}

// ============================================================================
// History Endpoints
// ============================================================================

#[derive(Deserialize)]
struct HistoryQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    50
}

async fn get_history(
    State(state): State<AppState>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    match state.database.get_rip_history(query.limit, query.offset).await {
        Ok(history) => {
            let json_history: Vec<serde_json::Value> = history
                .into_iter()
                .map(|item| serde_json::to_value(item).unwrap())
                .collect();
            Ok(Json(json_history))
        }
        Err(e) => {
            error!("Failed to get history: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

async fn get_history_item(
    State(_state): State<AppState>,
    Path(_id): Path<i64>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // TODO: Implement get single history item with job details
    Err((StatusCode::NOT_IMPLEMENTED, "Not implemented".to_string()))
}

async fn get_history_stats(
    State(state): State<AppState>,
) -> Result<Json<RipStats>, (StatusCode, String)> {
    match state.database.get_rip_stats().await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => {
            error!("Failed to get history stats: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

// ============================================================================
// Config Endpoints
// ============================================================================

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = state.config.read().await;
    Ok(Json(serde_json::to_value(&*config).unwrap()))
}

async fn update_config(
    State(state): State<AppState>,
    Json(new_config): Json<serde_json::Value>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Deserialize into Config struct
    let config: Config = serde_json::from_value(new_config)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid config: {}", e)))?;

    // Update in-memory config
    let mut current_config = state.config.write().await;
    *current_config = config;

    info!("Configuration updated via API");
    Ok(StatusCode::OK)
}

// ============================================================================
// Metadata Endpoints
// ============================================================================

#[derive(Deserialize)]
struct SearchMetadataRequest {
    query: String,
    year: Option<i32>,
    media_type: Option<String>,
}

async fn search_metadata(
    State(state): State<AppState>,
    Json(request): Json<SearchMetadataRequest>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, String)> {
    let config = state.config.read().await;
    
    // Create metadata aggregator
    let mut aggregator = MetadataAggregator::new();
    
    if let Some(api_key) = &config.metadata.tmdb_api_key {
        aggregator = aggregator.with_tmdb(api_key);
    }
    
    if let Some(api_key) = &config.metadata.omdb_api_key {
        aggregator = aggregator.with_omdb(api_key);
    }

    // Parse media type if provided
    let media_type = request.media_type.as_deref().and_then(|s| {
        match s.to_lowercase().as_str() {
            "movie" => Some(MediaType::Movie),
            "tv" | "tvshow" => Some(MediaType::TVShow),
            "anime" => Some(MediaType::Anime),
            _ => None,
        }
    });

    // Search all providers
    match aggregator.search_all(&request.query, request.year, media_type).await {
        Ok(results) => {
            let json_results: Vec<serde_json::Value> = results
                .into_iter()
                .map(|result| serde_json::to_value(result).unwrap())
                .collect();
            Ok(Json(json_results))
        }
        Err(e) => {
            error!("Metadata search failed: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{}", e)))
        }
    }
}
