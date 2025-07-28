//! HTTP endpoint handlers

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use tracing::{error, info, warn};

use crate::{
    services::{start_systemd_service, stop_systemd_service, force_kill_process, recover_systemd_service, ServiceConfig},
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

/// Handle POST /service/{service_name}/start - Start a systemd service
pub async fn service_start_handler(
    Path(service_name): Path<String>,
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse>, StatusCode> {
    // Get service configuration
    let service_config = match ServiceConfig::from_name(&service_name) {
        Some(config) => config,
        None => {
            warn!("Unknown service requested: {}", service_name);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Clear any previous errors for this service
    if let Err(e) = state.clear_errors_for(&service_name) {
        warn!("Failed to clear {} errors: {}", service_name, e);
    }

    // Try to start the service
    match start_systemd_service(&service_config.service_name).await {
        Ok(()) => {
            // Service started successfully, update state
            match state.set_service(&service_name, true) {
                Ok(system_state) => {
                    info!("{} service started successfully", service_name);
                    Ok(Json(ApiResponse::active(
                        format!("{} service started", service_name),
                        system_state,
                    )))
                }
                Err(e) => {
                    error!("Failed to update {} state: {}", service_name, e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            // Service start failed, try recovery if enabled
            if service_config.recovery_enabled {
                warn!("Failed to start {} service: {}, attempting recovery", service_name, e);
                
                match recover_systemd_service(&service_config).await {
                    Ok(()) => {
                        match state.set_service(&service_name, true) {
                            Ok(system_state) => {
                                info!("{} service recovered and started successfully", service_name);
                                Ok(Json(ApiResponse::active(
                                    format!("{} service started after recovery", service_name),
                                    system_state,
                                )))
                            }
                            Err(e) => {
                                error!("Failed to update {} state after recovery: {}", service_name, e);
                                Err(StatusCode::INTERNAL_SERVER_ERROR)
                            }
                        }
                    }
                    Err(recovery_error) => {
                        // Recovery failed, handle error
                        let error_msg = format!("{} service failed to start: {}", service_name, recovery_error);
                        
                        if let Err(e) = state.set_service(&service_name, false) {
                            error!("Failed to set {} state to false: {}", service_name, e);
                        }
                        
                        if let Err(e) = state.add_error(error_msg.clone()) {
                            error!("Failed to add error to state: {}", e);
                        }

                        match state.get_system_state() {
                            Ok(system_state) => Ok(Json(ApiResponse::error(error_msg, system_state))),
                            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                        }
                    }
                }
            } else {
                // No recovery, just return error
                let error_msg = format!("{} service failed to start: {}", service_name, e);
                if let Err(e) = state.add_error(error_msg.clone()) {
                    error!("Failed to add error to state: {}", e);
                }
                
                match state.get_system_state() {
                    Ok(system_state) => Ok(Json(ApiResponse::error(error_msg, system_state))),
                    Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                }
            }
        }
    }
}

/// Handle POST /service/{service_name}/stop - Stop a systemd service
pub async fn service_stop_handler(
    Path(service_name): Path<String>,
    State(state): State<Arc<AppState>>
) -> Result<Json<ApiResponse>, StatusCode> {
    // Get service configuration
    let service_config = match ServiceConfig::from_name(&service_name) {
        Some(config) => config,
        None => {
            warn!("Unknown service requested: {}", service_name);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Clear any previous errors for this service
    if let Err(e) = state.clear_errors_for(&service_name) {
        warn!("Failed to clear {} errors: {}", service_name, e);
    }

    // Try to stop the service
    match stop_systemd_service(&service_config.service_name).await {
        Ok(()) => {
            match state.set_service(&service_name, false) {
                Ok(system_state) => {
                    info!("{} service stopped successfully", service_name);
                    Ok(Json(ApiResponse::inactive(
                        format!("{} service stopped", service_name),
                        system_state,
                    )))
                }
                Err(e) => {
                    error!("Failed to update {} state: {}", service_name, e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            // Service stop failed, try force kill if available
            if let Some(process_name) = &service_config.process_name {
                warn!("Failed to stop {} service: {}, attempting force kill", service_name, e);
                
                if let Err(kill_error) = force_kill_process(process_name).await {
                    warn!("Force kill also failed: {}", kill_error);
                    
                    let error_msg = format!("{} service stop failed: {}", service_name, e);
                    if let Err(e) = state.add_error(error_msg.clone()) {
                        error!("Failed to add error to state: {}", e);
                    }
                }
            }

            // Always set state to false when turning off, even if stop failed
            match state.set_service(&service_name, false) {
                Ok(system_state) => {
                    info!("{} state set to false (service stop may have failed)", service_name);
                    Ok(Json(ApiResponse::inactive(
                        format!("{} service stop attempted", service_name),
                        system_state,
                    )))
                }
                Err(e) => {
                    error!("Failed to update {} state: {}", service_name, e);
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
