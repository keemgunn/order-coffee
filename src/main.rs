use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde::{Deserialize, Serialize};
use signal_hook_tokio::Signals;
use futures::stream::StreamExt;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    net::TcpListener,
    process::Command,
    sync::{broadcast, watch},
    time::sleep,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn, debug};

// CLI argument parsing structure
#[derive(Parser)]
#[command(name = "order-coffee")]
#[command(about = "A state-managed HTTP server to control system suspension")]
#[command(version = "2.0.0")]
struct Args {
    /// Port to bind the server to
    #[arg(short, long, default_value = "20553")]
    port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Suspension timer duration in minutes
    #[arg(short, long, default_value = "10")]
    timer: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

// System state structure - holds all states that can prevent suspension
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemState {
    /// Manual sleep prevention state (controlled by /coffee and /chill endpoints)
    coffee: bool,
    /// Ollama service state (controlled by /ollama-on and /ollama-off endpoints)
    ollama: bool,
    /// List of current errors for client visibility
    errors: Vec<String>,
}

impl SystemState {
    /// Create a new SystemState with all states set to false
    fn new() -> Self {
        Self {
            coffee: false,
            ollama: false,
            errors: Vec::new(),
        }
    }

    /// Check if any state is active (true)
    fn any_active(&self) -> bool {
        self.coffee || self.ollama
    }

    /// Check if all states are inactive (false)
    fn all_inactive(&self) -> bool {
        !self.any_active()
    }
}

// API response structure for state change endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse {
    status: String,
    message: String,
    timestamp: DateTime<Utc>,
    states: SystemState,
}

// Enhanced status response with timer information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusResponse {
    states: SystemState,
    timer_active: bool,
    timer_remaining_seconds: Option<u64>,
    uptime: String,
    port: u16,
    host: String,
    last_action: Option<String>,
    last_action_time: Option<DateTime<Utc>>,
}

// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    timestamp: DateTime<Utc>,
    version: String,
}

// Timer state for tracking suspension countdown
#[derive(Debug, Clone)]
struct TimerState {
    active: bool,
    remaining_seconds: Option<u64>,
}

// Main application state that manages all system states and timer
#[derive(Debug)]
struct AppState {
    /// Current system states (coffee, ollama, errors)
    system_state: Arc<Mutex<SystemState>>,
    /// Timer configuration and state
    timer_duration_minutes: u64,
    timer_state: Arc<Mutex<TimerState>>,
    /// Server metadata
    start_time: Instant,
    port: u16,
    host: String,
    /// Last action tracking
    last_action: Arc<Mutex<Option<String>>>,
    last_action_time: Arc<Mutex<Option<DateTime<Utc>>>>,
    /// Channels for state change notifications
    state_change_tx: broadcast::Sender<SystemState>,
    /// Channel for timer updates
    timer_update_tx: watch::Sender<TimerState>,
    /// Keep the receiver alive to prevent channel closure
    _timer_update_rx: watch::Receiver<TimerState>,
}

impl AppState {
    /// Create a new AppState with default values
    fn new(port: u16, host: String, timer_duration_minutes: u64) -> Self {
        let (state_change_tx, _) = broadcast::channel(100);
        let (timer_update_tx, timer_update_rx) = watch::channel(TimerState {
            active: false,
            remaining_seconds: None,
        });

        Self {
            system_state: Arc::new(Mutex::new(SystemState::new())),
            timer_duration_minutes,
            timer_state: Arc::new(Mutex::new(TimerState {
                active: false,
                remaining_seconds: None,
            })),
            start_time: Instant::now(),
            port,
            host,
            last_action: Arc::new(Mutex::new(None)),
            last_action_time: Arc::new(Mutex::new(None)),
            state_change_tx,
            timer_update_tx,
            _timer_update_rx: timer_update_rx,
        }
    }

    /// Update a specific state and trigger state change notifications
    fn update_state<F>(&self, action: &str, updater: F) -> Result<SystemState, String>
    where
        F: FnOnce(&mut SystemState),
    {
        // Lock the system state and apply the update
        let mut state = self.system_state.lock()
            .map_err(|e| format!("Failed to lock system state: {}", e))?;
        
        updater(&mut *state);
        let new_state = state.clone();
        drop(state); // Release the lock early

        // Update last action tracking
        if let Ok(mut last_action) = self.last_action.lock() {
            *last_action = Some(action.to_string());
        }
        if let Ok(mut last_time) = self.last_action_time.lock() {
            *last_time = Some(Utc::now());
        }

        // Notify state change listeners (this will trigger timer logic)
        if let Err(e) = self.state_change_tx.send(new_state.clone()) {
            warn!("Failed to send state change notification: {}", e);
        }

        Ok(new_state)
    }

    /// Set the coffee state
    fn set_coffee(&self, active: bool) -> Result<SystemState, String> {
        info!("Setting coffee state to: {}", active);
        self.update_state(
            if active { "coffee" } else { "chill" },
            |state| state.coffee = active,
        )
    }

    /// Set the ollama state
    fn set_ollama(&self, active: bool) -> Result<SystemState, String> {
        info!("Setting ollama state to: {}", active);
        self.update_state(
            if active { "ollama-on" } else { "ollama-off" },
            |state| state.ollama = active,
        )
    }

    /// Add an error to the state
    fn add_error(&self, error: String) -> Result<(), String> {
        let mut state = self.system_state.lock()
            .map_err(|e| format!("Failed to lock system state: {}", e))?;
        
        warn!("Adding error to state: {}", error);
        state.errors.push(error);
        let new_state = state.clone();
        drop(state);

        // Notify listeners of the error update
        if let Err(e) = self.state_change_tx.send(new_state) {
            warn!("Failed to send error state notification: {}", e);
        }

        Ok(())
    }

    /// Clear errors for a specific component
    fn clear_errors_for(&self, component: &str) -> Result<(), String> {
        let mut state = self.system_state.lock()
            .map_err(|e| format!("Failed to lock system state: {}", e))?;
        
        // Remove errors that contain the component name
        let initial_count = state.errors.len();
        state.errors.retain(|error| !error.to_lowercase().contains(&component.to_lowercase()));
        
        if state.errors.len() != initial_count {
            info!("Cleared {} errors for component: {}", initial_count - state.errors.len(), component);
            let new_state = state.clone();
            drop(state);

            if let Err(e) = self.state_change_tx.send(new_state) {
                warn!("Failed to send error clear notification: {}", e);
            }
        }

        Ok(())
    }

    /// Get current system state
    fn get_system_state(&self) -> Result<SystemState, String> {
        self.system_state.lock()
            .map(|state| state.clone())
            .map_err(|e| format!("Failed to lock system state: {}", e))
    }

    /// Get current timer state
    fn get_timer_state(&self) -> Result<TimerState, String> {
        self.timer_state.lock()
            .map(|state| state.clone())
            .map_err(|e| format!("Failed to lock timer state: {}", e))
    }

    /// Update timer state
    fn update_timer_state(&self, active: bool, remaining_seconds: Option<u64>) -> Result<(), String> {
        let mut timer_state = self.timer_state.lock()
            .map_err(|e| format!("Failed to lock timer state: {}", e))?;
        
        timer_state.active = active;
        timer_state.remaining_seconds = remaining_seconds;
        let new_timer_state = timer_state.clone();
        drop(timer_state);

        // Notify timer state watchers
        if let Err(e) = self.timer_update_tx.send(new_timer_state) {
            warn!("Failed to send timer update: {}", e);
        }

        Ok(())
    }

    /// Calculate server uptime as a formatted string
    fn get_uptime(&self) -> String {
        let duration = self.start_time.elapsed();
        let hours = duration.as_secs() / 3600;
        let minutes = (duration.as_secs() % 3600) / 60;
        let seconds = duration.as_secs() % 60;
        
        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

// Ollama service management functions

/// Start the ollama.service using systemctl
async fn start_ollama_service() -> Result<(), String> {
    debug!("Attempting to start ollama.service");
    
    let output = Command::new("systemctl")
        .args(&["start", "ollama.service"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl start: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl start failed: {}", stderr));
    }

    info!("ollama.service started successfully");
    Ok(())
}

/// Stop the ollama.service using systemctl
async fn stop_ollama_service() -> Result<(), String> {
    debug!("Attempting to stop ollama.service");
    
    let output = Command::new("systemctl")
        .args(&["stop", "ollama.service"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl stop: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl stop failed: {}", stderr));
    }

    info!("ollama.service stopped successfully");
    Ok(())
}

/// Force kill ollama processes
async fn force_kill_ollama() -> Result<(), String> {
    debug!("Attempting to force kill ollama processes");
    
    let output = Command::new("pkill")
        .args(&["-f", "ollama"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute pkill: {}", e))?;

    // pkill returns non-zero if no processes were found, which is okay
    info!("Force kill ollama completed (exit code: {})", output.status.code().unwrap_or(-1));
    Ok(())
}

/// Reload systemd daemon
async fn reload_systemd_daemon() -> Result<(), String> {
    debug!("Reloading systemd daemon");
    
    let output = Command::new("systemctl")
        .args(&["daemon-reload"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl daemon-reload: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl daemon-reload failed: {}", stderr));
    }

    info!("systemd daemon reloaded successfully");
    Ok(())
}

/// Restart ollama.service using systemctl
async fn restart_ollama_service() -> Result<(), String> {
    debug!("Attempting to restart ollama.service");
    
    let output = Command::new("systemctl")
        .args(&["restart", "ollama.service"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl restart: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl restart failed: {}", stderr));
    }

    info!("ollama.service restarted successfully");
    Ok(())
}

/// Comprehensive ollama service recovery with escalating attempts
async fn recover_ollama_service() -> Result<(), String> {
    warn!("Starting ollama service recovery process");

    // Step 1: Try force kill and restart
    warn!("Recovery step 1: Force kill and start");
    if let Err(e) = force_kill_ollama().await {
        warn!("Force kill failed: {}", e);
    }
    
    // Wait a moment for processes to clean up
    sleep(Duration::from_secs(2)).await;
    
    if start_ollama_service().await.is_ok() {
        info!("Recovery successful after force kill");
        return Ok(());
    }

    // Step 2: Reload systemd and restart service
    warn!("Recovery step 2: Reload systemd and restart service");
    if let Err(e) = reload_systemd_daemon().await {
        warn!("Systemd reload failed: {}", e);
    }
    
    if restart_ollama_service().await.is_ok() {
        info!("Recovery successful after systemd reload and restart");
        return Ok(());
    }

    // All recovery attempts failed
    Err("All ollama service recovery attempts failed".to_string())
}

/// Execute system suspension
async fn execute_system_suspend() -> Result<(), String> {
    info!("Executing system suspension");
    
    let output = Command::new("systemctl")
        .args(&["suspend"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl suspend: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl suspend failed: {}", stderr));
    }

    info!("System suspension command executed");
    Ok(())
}

// HTTP endpoint handlers

/// Handle POST /coffee - Enable coffee state
async fn coffee_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    match state.set_coffee(true) {
        Ok(system_state) => {
            info!("Coffee endpoint called - coffee state enabled");
            Ok(Json(ApiResponse {
                status: "active".to_string(),
                message: "Coffee state enabled".to_string(),
                timestamp: Utc::now(),
                states: system_state,
            }))
        }
        Err(e) => {
            error!("Failed to enable coffee state: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle POST /chill - Disable coffee state
async fn chill_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    match state.set_coffee(false) {
        Ok(system_state) => {
            info!("Chill endpoint called - coffee state disabled");
            Ok(Json(ApiResponse {
                status: "inactive".to_string(),
                message: "Coffee state disabled".to_string(),
                timestamp: Utc::now(),
                states: system_state,
            }))
        }
        Err(e) => {
            error!("Failed to disable coffee state: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Handle POST /ollama-on - Enable ollama state and start service
async fn ollama_on_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
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
                    Ok(Json(ApiResponse {
                        status: "active".to_string(),
                        message: "Ollama state enabled and service started".to_string(),
                        timestamp: Utc::now(),
                        states: system_state,
                    }))
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
                            Ok(Json(ApiResponse {
                                status: "active".to_string(),
                                message: "Ollama state enabled after recovery".to_string(),
                                timestamp: Utc::now(),
                                states: system_state,
                            }))
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
                        Ok(system_state) => Ok(Json(ApiResponse {
                            status: "error".to_string(),
                            message: error_msg,
                            timestamp: Utc::now(),
                            states: system_state,
                        })),
                        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
                    }
                }
            }
        }
    }
}

/// Handle POST /ollama-off - Disable ollama state and stop service
async fn ollama_off_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
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
                    Ok(Json(ApiResponse {
                        status: "inactive".to_string(),
                        message: "Ollama state disabled and service stopped".to_string(),
                        timestamp: Utc::now(),
                        states: system_state,
                    }))
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
                    Ok(Json(ApiResponse {
                        status: "inactive".to_string(),
                        message: "Ollama state disabled (service stop attempted)".to_string(),
                        timestamp: Utc::now(),
                        states: system_state,
                    }))
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
async fn status_handler(State(state): State<Arc<AppState>>) -> Result<Json<StatusResponse>, StatusCode> {
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

    let last_action = state.last_action.lock().ok().and_then(|a| a.clone());
    let last_action_time = state.last_action_time.lock().ok().and_then(|t| *t);
    
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
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        timestamp: Utc::now(),
        version: "2.0.0".to_string(),
    })
}

// Background task for managing suspension timer

/// Background task that manages the suspension timer based on system state changes
async fn suspension_timer_task(state: Arc<AppState>) {
    info!("Starting suspension timer task");
    
    let mut state_rx = state.state_change_tx.subscribe();
    
    loop {
        // Wait for a state change notification
        match state_rx.recv().await {
            Ok(current_state) => {
                debug!("Timer task received state change: coffee={}, ollama={}", 
                       current_state.coffee, current_state.ollama);
                
                if current_state.all_inactive() {
                    // All states are inactive, start suspension timer
                    info!("All states inactive, starting suspension timer for {} minutes", 
                          state.timer_duration_minutes);
                    
                    // Update timer state to active
                    if let Err(e) = state.update_timer_state(true, Some(state.timer_duration_minutes * 60)) {
                        error!("Failed to update timer state: {}", e);
                        continue;
                    }
                    
                    // Start countdown
                    let timer_duration = Duration::from_secs(state.timer_duration_minutes * 60);
                    let start_time = Instant::now();
                    
                    // Create a timer that can be cancelled
                    let mut interval = tokio::time::interval(Duration::from_secs(1));
                    let mut cancelled = false;
                    
                    loop {
                        tokio::select! {
                            // Timer tick - update remaining time
                            _ = interval.tick() => {
                                let elapsed = start_time.elapsed();
                                if elapsed >= timer_duration {
                                    // Timer expired, trigger suspension
                                    info!("Suspension timer expired, triggering system suspension");
                                    
                                    // Update timer state to inactive
                                    if let Err(e) = state.update_timer_state(false, None) {
                                        error!("Failed to update timer state: {}", e);
                                    }
                                    
                                    // Execute system suspension
                                    if let Err(e) = execute_system_suspend().await {
                                        error!("Failed to suspend system: {}", e);
                                        if let Err(e) = state.add_error(format!("System suspension failed: {}", e)) {
                                            error!("Failed to add suspension error: {}", e);
                                        }
                                    }
                                    
                                    break;
                                } else {
                                    // Update remaining time
                                    let remaining = timer_duration - elapsed;
                                    if let Err(e) = state.update_timer_state(true, Some(remaining.as_secs())) {
                                        error!("Failed to update timer remaining time: {}", e);
                                    }
                                }
                            }
                            
                            // State change - check if we should cancel timer
                            Ok(new_state) = state_rx.recv() => {
                                if new_state.any_active() {
                                    // Some state became active, cancel timer
                                    info!("State became active, cancelling suspension timer");
                                    cancelled = true;
                                    
                                    // Update timer state to inactive
                                    if let Err(e) = state.update_timer_state(false, None) {
                                        error!("Failed to update timer state: {}", e);
                                    }
                                    
                                    break;
                                }
                            }
                        }
                    }
                    
                    if cancelled {
                        debug!("Timer was cancelled, continuing to wait for next state change");
                    }
                } else {
                    // Some states are active, ensure timer is inactive
                    debug!("Some states are active, ensuring timer is inactive");
                    if let Err(e) = state.update_timer_state(false, None) {
                        error!("Failed to update timer state: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Error receiving state change: {}", e);
                // Wait a bit before retrying
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

// Signal handling for graceful shutdown

/// Wait for shutdown signals (SIGTERM, SIGINT)
async fn shutdown_signal() {
    let mut signals = Signals::new(&[
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
    ]).expect("Failed to create signal handler");

    while let Some(signal) = signals.next().await {
        info!("Received signal: {}", signal);
        break;
    }
}

// Main function

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing with appropriate log level
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("order_coffee={},tower_http=info", log_level))
        .init();

    info!("Starting order-coffee server v2.0.0");
    info!("Configuration: host={}, port={}, timer={}min", 
          args.host, args.port, args.timer);

    // Check if systemctl is available (required for ollama service management and suspension)
    match Command::new("systemctl").arg("--version").output().await {
        Ok(_) => info!("systemctl is available"),
        Err(_) => {
            error!("systemctl is not available. This server requires systemd.");
            std::process::exit(1);
        }
    }

    // Create application state
    let state = Arc::new(AppState::new(args.port, args.host.clone(), args.timer));

    // Start the suspension timer background task
    let timer_state = Arc::clone(&state);
    tokio::spawn(async move {
        suspension_timer_task(timer_state).await;
    });

    // Create HTTP router with all endpoints
    let app = Router::new()
        .route("/coffee", post(coffee_handler))
        .route("/chill", post(chill_handler))
        .route("/ollama-on", post(ollama_on_handler))
        .route("/ollama-off", post(ollama_off_handler))
        .route("/status", get(status_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Bind to the specified address
    let addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("Server running on http://{}", addr);
    info!("Endpoints:");
    info!("  POST /coffee     - Enable coffee state");
    info!("  POST /chill      - Disable coffee state");
    info!("  POST /ollama-on  - Enable ollama state and start service");
    info!("  POST /ollama-off - Disable ollama state and stop service");
    info!("  GET  /status     - Check current status and timer");
    info!("  GET  /health     - Health check");

    // Setup graceful shutdown
    let server = axum::serve(listener, app);
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("Server error: {}", e);
            }
        }
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
        }
    }

    info!("Server shutdown complete");
    Ok(())
}
