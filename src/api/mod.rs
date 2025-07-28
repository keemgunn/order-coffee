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
        // New generic service endpoints
        .route("/service/:service_name/start", post(service_start_handler))
        .route("/service/:service_name/stop", post(service_stop_handler))
        .route("/status", get(status_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
