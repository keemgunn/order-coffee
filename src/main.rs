//! Order Coffee - A state-managed HTTP server to control system suspension
//! 
//! This is the main entry point for the order-coffee application.

use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use order_coffee::{
    config::Config,
    state::AppState,
    api::create_router,
    services::check_systemctl_available,
    tasks::suspension_timer_task,
    utils::shutdown_signal,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();

    // Initialize tracing with appropriate log level
    tracing_subscriber::fmt()
        .with_env_filter(format!("order_coffee={},tower_http=info", config.log_level()))
        .init();

    info!("Starting order-coffee server v2.0.0");
    info!("Configuration: host={}, port={}, timer={}min", 
          config.host, config.port, config.timer);

    // Check if systemctl is available (required for ollama service management and suspension)
    if let Err(e) = check_systemctl_available().await {
        tracing::error!("{}", e);
        std::process::exit(1);
    }

    // Create application state
    let state = Arc::new(AppState::new(config.port, config.host.clone(), config.timer));

    // Start the suspension timer background task
    let timer_state = Arc::clone(&state);
    tokio::spawn(async move {
        suspension_timer_task(timer_state).await;
    });

    // Create HTTP router with all endpoints
    let app = create_router(state);

    // Bind to the specified address
    let addr = config.address();
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
                tracing::error!("Server error: {}", e);
            }
        }
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
        }
    }

    info!("Server shutdown complete");
    Ok(())
}
