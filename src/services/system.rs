//! System operations like suspension

use tokio::process::Command;
use tracing::info;

/// Execute system suspension
pub async fn execute_system_suspend() -> Result<(), String> {
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

/// Check if systemctl is available on the system
pub async fn check_systemctl_available() -> Result<(), String> {
    Command::new("systemctl")
        .arg("--version")
        .output()
        .await
        .map_err(|_| "systemctl is not available. This server requires systemd.".to_string())?;
    
    info!("systemctl is available");
    Ok(())
}
