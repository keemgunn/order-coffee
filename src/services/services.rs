//! Generic systemd service management functions

use std::time::Duration;
use tokio::{process::Command, time::sleep};
use tracing::{debug, info, warn};

/// Service configuration for different services
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub service_name: String,
    pub process_name: Option<String>, // For force kill operations
    pub recovery_enabled: bool,
}

impl ServiceConfig {
    pub fn ollama() -> Self {
        Self {
            service_name: "ollama.service".to_string(),
            process_name: Some("ollama".to_string()),
            recovery_enabled: true,
        }
    }
    
    pub fn comfy_unsafe() -> Self {
        Self {
            service_name: "comfy-unsafe.service".to_string(),
            process_name: Some("comfy-unsafe".to_string()),
            recovery_enabled: true,
        }
    }
    
    pub fn comfy_safe() -> Self {
        Self {
            service_name: "comfy-safe.service".to_string(),
            process_name: Some("comfy-safe".to_string()),
            recovery_enabled: true,
        }
    }
    
    // Method to get config by service name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ollama" => Some(Self::ollama()),
            "comfy-unsafe" => Some(Self::comfy_unsafe()),
            "comfy-safe" => Some(Self::comfy_safe()),
            _ => None,
        }
    }
}

/// Start a systemd service using systemctl
pub async fn start_systemd_service(service_name: &str) -> Result<(), String> {
    debug!("Attempting to start {}", service_name);
    
    let output = Command::new("systemctl")
        .args(&["start", service_name])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl start: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl start failed: {}", stderr));
    }

    info!("{} started successfully", service_name);
    Ok(())
}

/// Stop a systemd service using systemctl
pub async fn stop_systemd_service(service_name: &str) -> Result<(), String> {
    debug!("Attempting to stop {}", service_name);
    
    let output = Command::new("systemctl")
        .args(&["stop", service_name])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl stop: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl stop failed: {}", stderr));
    }

    info!("{} stopped successfully", service_name);
    Ok(())
}

/// Force kill processes by name
pub async fn force_kill_process(process_name: &str) -> Result<(), String> {
    debug!("Attempting to force kill {} processes", process_name);
    
    let output = Command::new("pkill")
        .args(&["-f", process_name])
        .output()
        .await
        .map_err(|e| format!("Failed to execute pkill: {}", e))?;

    // pkill returns non-zero if no processes were found, which is okay
    info!("Force kill {} completed (exit code: {})", process_name, output.status.code().unwrap_or(-1));
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

/// Restart a systemd service using systemctl
pub async fn restart_systemd_service(service_name: &str) -> Result<(), String> {
    debug!("Attempting to restart {}", service_name);
    
    let output = Command::new("systemctl")
        .args(&["restart", service_name])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl restart: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl restart failed: {}", stderr));
    }

    info!("{} restarted successfully", service_name);
    Ok(())
}

/// Check if a systemd service is currently active
pub async fn check_systemd_service_status(service_name: &str) -> Result<bool, String> {
    debug!("Checking {} status", service_name);
    
    let output = Command::new("systemctl")
        .args(&["is-active", service_name])
        .output()
        .await
        .map_err(|e| format!("Failed to execute systemctl is-active: {}", e))?;

    // systemctl is-active returns 0 if active, non-zero if inactive
    let is_active = output.status.success();
    debug!("{} is {}", service_name, if is_active { "active" } else { "inactive" });
    
    Ok(is_active)
}

/// Initialize service state on server startup
pub async fn initialize_service_state(config: &ServiceConfig, desired_state: bool) -> Result<(), String> {
    info!("Initializing {} service state", config.service_name);
    
    match check_systemd_service_status(&config.service_name).await {
        Ok(is_active) => {
            if is_active != desired_state {
                info!("{} is {}, {} to synchronize with server state", 
                    config.service_name, 
                    if is_active { "active" } else { "inactive" },
                    if desired_state { "starting" } else { "stopping" }
                );
                
                if desired_state {
                    start_systemd_service(&config.service_name).await?;
                } else {
                    stop_systemd_service(&config.service_name).await?;
                }
                
                info!("{} {} successfully during initialization", 
                    config.service_name, 
                    if desired_state { "started" } else { "stopped" }
                );
            } else {
                info!("{} is already {}, no action needed", 
                    config.service_name, 
                    if is_active { "active" } else { "inactive" }
                );
            }
            Ok(())
        }
        Err(e) => {
            warn!("Failed to check {} status during initialization: {}", config.service_name, e);
            // Don't fail the entire server startup if we can't check the service status
            Ok(())
        }
    }
}

/// Comprehensive service recovery with escalating attempts
pub async fn recover_systemd_service(config: &ServiceConfig) -> Result<(), String> {
    warn!("Starting {} service recovery process", config.service_name);

    // Step 1: Try force kill and restart
    warn!("Recovery step 1: Force kill and start");
    if let Some(process_name) = &config.process_name {
        if let Err(e) = force_kill_process(process_name).await {
            warn!("Force kill failed: {}", e);
        }
    }
    
    // Wait a moment for processes to clean up
    sleep(Duration::from_secs(2)).await;
    
    if start_systemd_service(&config.service_name).await.is_ok() {
        info!("Recovery successful after force kill");
        return Ok(());
    }

    // Step 2: Reload systemd and restart service
    warn!("Recovery step 2: Reload systemd and restart service");
    if let Err(e) = reload_systemd_daemon().await {
        warn!("Systemd reload failed: {}", e);
    }
    
    if restart_systemd_service(&config.service_name).await.is_ok() {
        info!("Recovery successful after systemd reload and restart");
        return Ok(());
    }

    // All recovery attempts failed
    Err(format!("All {} service recovery attempts failed", config.service_name))
}
