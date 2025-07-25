//! Ollama service management functions

use std::time::Duration;
use tokio::{process::Command, time::sleep};
use tracing::{debug, info, warn};

/// Start the ollama.service using systemctl
pub async fn start_ollama_service() -> Result<(), String> {
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
pub async fn stop_ollama_service() -> Result<(), String> {
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
pub async fn force_kill_ollama() -> Result<(), String> {
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
pub async fn reload_systemd_daemon() -> Result<(), String> {
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
pub async fn restart_ollama_service() -> Result<(), String> {
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

/// Check if ollama.service is currently active
pub async fn check_ollama_service_status() -> Result<bool, String> {
    debug!("Checking ollama.service status");
    
    let output = Command::new("systemctl")
        .args(&["is-active", "ollama.service"])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl is-active: {}", e))?;

    // systemctl is-active returns 0 if active, non-zero if inactive
    let is_active = output.status.success();
    debug!("ollama.service is {}", if is_active { "active" } else { "inactive" });
    
    Ok(is_active)
}

/// Initialize ollama service state on server startup
/// Ensures the service state matches the server's initial state (ollama: false)
pub async fn initialize_ollama_state() -> Result<(), String> {
    info!("Initializing ollama service state");
    
    match check_ollama_service_status().await {
        Ok(is_active) => {
            if is_active {
                info!("ollama.service is active, stopping to synchronize with server state");
                stop_ollama_service().await?;
                info!("ollama.service stopped successfully during initialization");
            } else {
                info!("ollama.service is already inactive, no action needed");
            }
            Ok(())
        }
        Err(e) => {
            warn!("Failed to check ollama.service status during initialization: {}", e);
            // Don't fail the entire server startup if we can't check the service status
            Ok(())
        }
    }
}

/// Comprehensive ollama service recovery with escalating attempts
pub async fn recover_ollama_service() -> Result<(), String> {
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
