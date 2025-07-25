//! HTTP API module
//! 
//! This module contains all HTTP endpoint handlers and response structures.

pub mod handlers;
pub mod responses;

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;
use handlers::*;

/// Create the HTTP router with all endpoints
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/coffee", post(coffee_handler))
        .route("/chill", post(chill_handler))
        .route("/ollama-on", post(ollama_on_handler))
        .route("/ollama-off", post(ollama_off_handler))
        .route("/status", get(status_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
