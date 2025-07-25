//! HTTP endpoint handlers

use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use tracing::{error, info, warn};

use crate::{
    services::{
        start_ollama_service, stop_ollama_service, force_kill_ollama, recover_ollama_service,
    },
    state::AppState,
};
use super::responses::{ApiResponse, StatusResponse, HealthResponse};

/// Handle POST /coffee - Enable coffee state
pub async fn coffee_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    match state.set_coffee(true) {
        Ok(system_state) => {
            info!("Coffee endpoint called - coffee state enabled");
            Ok(Json(ApiResponse::active(
                "Coffee state enabled".to_string(),
                system_state,
            )))
        }
        Err(e) => {
            error!("Failed to enable coffee state: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle POST /chill - Disable coffee state
pub async fn chill_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    match state.set_coffee(false) {
        Ok(system_state) => {
            info!("Chill endpoint called - coffee state disabled");
            Ok(Json(ApiResponse::inactive(
                "Coffee state disabled".to_string(),
                system_state,
            )))
        }
        Err(e) => {
            error!("Failed to disable coffee state: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle POST /ollama-on - Enable ollama state and start service
pub async fn ollama_on_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    // Clear any previous ollama errors
    if let Err(e) = state.clear_errors_for("ollama") {
        warn!("Failed to clear ollama errors: {}", e);
    }

    // Try to start ollama service
    match start_ollama_service().await {
        Ok(()) => {
            // Service started successfully, update state
            match state.set_ollama(true) {
                Ok(system_state) => {
                    info!("Ollama-on endpoint called - ollama state enabled and service started");
                    Ok(Json(ApiResponse::active(
                        "Ollama state enabled and service started".to_string(),
                        system_state,
                    )))
                }
                Err(e) => {
                    error!("Failed to update ollama state: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            // Service start failed, try recovery
            warn!("Failed to start ollama service: {}, attempting recovery", e);
            
            match recover_ollama_service().await {
                Ok(()) => {
                    // Recovery successful, update state
                    match state.set_ollama(true) {
                        Ok(system_state) => {
                            info!("Ollama service recovered and started successfully");
                            Ok(Json(ApiResponse::active(
                                "Ollama state enabled after recovery".to_string(),
                                system_state,
                            )))
                        }
                        Err(e) => {
                            error!("Failed to update ollama state after recovery: {}", e);
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                }
                Err(recovery_error) => {
                    // All recovery attempts failed, set state to false and add error
                    let error_msg = format!("Ollama service failed to start: {}", recovery_error);
                    
                    if let Err(e) = state.set_ollama(false) {
                        error!("Failed to set ollama state to false: {}", e);
                    }
                    
                    if let Err(e) = state.add_error(error_msg.clone()) {
                        error!("Failed to add error to state: {}", e);
                    }

                    error!("Ollama service start failed completely: {}", recovery_error);
                    
                    // Return the current state with error
                    match state.get_system_state() {
                        Ok(system_state) => Ok(Json(ApiResponse::error(error_msg, system_state))),
                        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                    }
                }
            }
        }
    }
}

/// Handle POST /ollama-off - Disable ollama state and stop service
pub async fn ollama_off_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    // Clear any previous ollama errors
    if let Err(e) = state.clear_errors_for("ollama") {
        warn!("Failed to clear ollama errors: {}", e);
    }

    // Try to stop ollama service
    match stop_ollama_service().await {
        Ok(()) => {
            // Service stopped successfully, update state
            match state.set_ollama(false) {
                Ok(system_state) => {
                    info!("Ollama-off endpoint called - ollama state disabled and service stopped");
                    Ok(Json(ApiResponse::inactive(
                        "Ollama state disabled and service stopped".to_string(),
                        system_state,
                    )))
                }
                Err(e) => {
                    error!("Failed to update ollama state: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            // Service stop failed, try force kill
            warn!("Failed to stop ollama service: {}, attempting force kill", e);
            
            if let Err(kill_error) = force_kill_ollama().await {
                warn!("Force kill also failed: {}", kill_error);
                
                // Add error but still set state to false (we tried our best)
                let error_msg = format!("Ollama service stop failed: {}", e);
                if let Err(e) = state.add_error(error_msg.clone()) {
                    error!("Failed to add error to state: {}", e);
                }
            }

            // Always set state to false when turning off, even if stop failed
            match state.set_ollama(false) {
                Ok(system_state) => {
                    info!("Ollama state set to false (service stop may have failed)");
                    Ok(Json(ApiResponse::inactive(
                        "Ollama state disabled (service stop attempted)".to_string(),
                        system_state,
                    )))
                }
                Err(e) => {
                    error!("Failed to update ollama state: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    }
}

/// Handle GET /status - Return current system status
pub async fn status_handler(State(state): State<Arc<AppState>>) -> Result<Json<StatusResponse>, StatusCode> {
    let system_state = match state.get_system_state() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to get system state: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let timer_state = match state.get_timer_state() {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to get timer state: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let (last_action, last_action_time) = state.get_last_action();
    
    Ok(Json(StatusResponse {
        states: system_state,
        timer_active: timer_state.active,
        timer_remaining_seconds: timer_state.remaining_seconds,
        uptime: state.get_uptime(),
        port: state.port,
        host: state.host.clone(),
        last_action,
        last_action_time,
    }))
}

/// Handle GET /health - Health check endpoint
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse::ok())
}
