use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Create CORS middleware for web UI
pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Create tracing middleware for request logging
pub fn create_trace_layer() -> TraceLayer {
    TraceLayer::new_for_http()
}
