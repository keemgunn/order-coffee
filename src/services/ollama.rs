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
