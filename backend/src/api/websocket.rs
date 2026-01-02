use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade, State},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info};

use crate::jobs::JobStage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketEvent {
    #[serde(rename = "disc_inserted")]
    DiscInserted {
        disc_label: String,
        disc_type: String,
    },
    #[serde(rename = "disc_ejected")]
    DiscEjected,
    #[serde(rename = "job_started")]
    JobStarted {
        job_id: i64,
        disc_label: String,
        title: Option<String>,
        year: Option<i32>,
    },
    #[serde(rename = "job_progress")]
    JobProgress {
        job_id: i64,
        progress: f64,
        stage: Option<JobStage>,
        eta: Option<u64>,
    },
    #[serde(rename = "job_completed")]
    JobCompleted {
        job_id: i64,
        output_path: String,
    },
    #[serde(rename = "job_failed")]
    JobFailed {
        job_id: i64,
        error: String,
    },
    #[serde(rename = "system_status")]
    SystemStatus {
        active_jobs: usize,
        queue_length: usize,
        disc_present: bool,
    },
}

#[derive(Clone)]
pub struct WebSocketState {
    pub sender: broadcast::Sender<WebSocketEvent>,
}

impl WebSocketState {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WebSocketEvent> {
        self.sender.subscribe()
    }

    pub fn send(&self, event: WebSocketEvent) {
        let _ = self.sender.send(event);
    }
}

/// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(ws_state): State<WebSocketState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, ws_state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, ws_state: WebSocketState) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_receiver = ws_state.subscribe();

    info!("WebSocket client connected");

    // Spawn task to forward events to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            let msg = match serde_json::to_string(&event) {
                Ok(json) => axum::extract::ws::Message::Text(json),
                Err(e) => {
                    error!("Failed to serialize event: {}", e);
                    continue;
                }
            };

            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Spawn task to handle incoming messages (ping/pong)
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, axum::extract::ws::Message::Close(_)) {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("WebSocket client disconnected");
}
