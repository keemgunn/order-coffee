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
    process::{Child, Command},
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};

#[derive(Parser)]
#[command(name = "order-coffee")]
#[command(about = "A simple HTTP server to prevent system sleep on Pop!_OS/Ubuntu")]
#[command(version = "1.0.0")]
struct Args {
    /// Port to bind the server to
    #[arg(short, long, default_value = "20553")]
    port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse {
    status: String,
    message: String,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusResponse {
    status: String,
    uptime: String,
    port: u16,
    host: String,
    inhibitor_active: bool,
    last_action: Option<String>,
    last_action_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    timestamp: DateTime<Utc>,
    version: String,
}

#[derive(Debug)]
struct AppState {
    inhibitor: Arc<Mutex<Option<Child>>>,
    start_time: Instant,
    port: u16,
    host: String,
    last_action: Arc<Mutex<Option<String>>>,
    last_action_time: Arc<Mutex<Option<DateTime<Utc>>>>,
}

impl AppState {
    fn new(port: u16, host: String) -> Self {
        Self {
            inhibitor: Arc::new(Mutex::new(None)),
            start_time: Instant::now(),
            port,
            host,
            last_action: Arc::new(Mutex::new(None)),
            last_action_time: Arc::new(Mutex::new(None)),
        }
    }

    fn is_inhibitor_active(&self) -> bool {
        if let Ok(inhibitor) = self.inhibitor.lock() {
            inhibitor.is_some()
        } else {
            false
        }
    }

    fn start_inhibitor(&self) -> Result<(), String> {
        let mut inhibitor = self.inhibitor.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        // If already active, don't start another one
        if inhibitor.is_some() {
            return Ok(());
        }

        info!("Starting sleep inhibitor");
        
        let child = Command::new("systemd-inhibit")
            .args(&[
                "--what=sleep:idle",
                "--who=order-coffee",
                "--why=Remote work session active",
                "--mode=block",
                "sleep", "infinity"
            ])
            .spawn()
            .map_err(|e| format!("Failed to start systemd-inhibit: {}", e))?;

        *inhibitor = Some(child);
        
        // Update last action
        if let Ok(mut last_action) = self.last_action.lock() {
            *last_action = Some("coffee".to_string());
        }
        if let Ok(mut last_time) = self.last_action_time.lock() {
            *last_time = Some(Utc::now());
        }

        info!("Sleep inhibitor started successfully");
        Ok(())
    }

    fn stop_inhibitor(&self) -> Result<(), String> {
        let mut inhibitor = self.inhibitor.lock().map_err(|e| format!("Lock error: {}", e))?;
        
        if let Some(mut child) = inhibitor.take() {
            info!("Stopping sleep inhibitor");
            
            if let Err(e) = child.kill() {
                warn!("Failed to kill inhibitor process: {}", e);
            }
            
            if let Err(e) = child.wait() {
                warn!("Failed to wait for inhibitor process: {}", e);
            }
            
            // Update last action
            if let Ok(mut last_action) = self.last_action.lock() {
                *last_action = Some("chill".to_string());
            }
            if let Ok(mut last_time) = self.last_action_time.lock() {
                *last_time = Some(Utc::now());
            }

            info!("Sleep inhibitor stopped successfully");
        }
        
        Ok(())
    }

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

async fn coffee_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    match state.start_inhibitor() {
        Ok(()) => {
            info!("Coffee endpoint called - sleep prevention enabled");
            Ok(Json(ApiResponse {
                status: "active".to_string(),
                message: "Sleep prevention enabled".to_string(),
                timestamp: Utc::now(),
            }))
        }
        Err(e) => {
            error!("Failed to enable sleep prevention: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn chill_handler(State(state): State<Arc<AppState>>) -> Result<Json<ApiResponse>, StatusCode> {
    match state.stop_inhibitor() {
        Ok(()) => {
            info!("Chill endpoint called - sleep prevention disabled");
            Ok(Json(ApiResponse {
                status: "inactive".to_string(),
                message: "Sleep prevention disabled".to_string(),
                timestamp: Utc::now(),
            }))
        }
        Err(e) => {
            error!("Failed to disable sleep prevention: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn status_handler(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let last_action = state.last_action.lock().ok().and_then(|a| a.clone());
    let last_action_time = state.last_action_time.lock().ok().and_then(|t| *t);
    
    Json(StatusResponse {
        status: if state.is_inhibitor_active() { "active".to_string() } else { "inactive".to_string() },
        uptime: state.get_uptime(),
        port: state.port,
        host: state.host.clone(),
        inhibitor_active: state.is_inhibitor_active(),
        last_action,
        last_action_time,
    })
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        timestamp: Utc::now(),
        version: "1.0.0".to_string(),
    })
}

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("order_coffee={},tower_http=info", log_level))
        .init();

    info!("Starting order-coffee server v1.0.0");
    info!("Binding to {}:{}", args.host, args.port);

    // Check if systemd-inhibit is available
    match Command::new("systemd-inhibit").arg("--version").output() {
        Ok(_) => info!("systemd-inhibit is available"),
        Err(_) => {
            error!("systemd-inhibit is not available. This server requires systemd.");
            std::process::exit(1);
        }
    }

    let state = Arc::new(AppState::new(args.port, args.host.clone()));

    // Create router
    let app = Router::new()
        .route("/coffee", post(coffee_handler))
        .route("/chill", post(chill_handler))
        .route("/status", get(status_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Bind to address
    let addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&addr).await?;
    
    info!("Server running on http://{}", addr);
    info!("Endpoints:");
    info!("  POST /coffee - Prevent system sleep");
    info!("  POST /chill  - Allow system sleep");
    info!("  GET  /status - Check current status");
    info!("  GET  /health - Health check");

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

    // Cleanup: stop any active inhibitor
    info!("Cleaning up...");
    if let Err(e) = state.stop_inhibitor() {
        warn!("Error during cleanup: {}", e);
    }

    info!("Server shutdown complete");
    Ok(())
}
