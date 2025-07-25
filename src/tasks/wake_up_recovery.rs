//! Wake-up recovery background task

use std::{sync::Arc, time::Duration};
use tokio::time::interval;
use tracing::{info, warn};

use crate::state::AppState;

/// Background task that checks for system wake-up and triggers state recovery
pub async fn wake_up_recovery_task(state: Arc<AppState>) {
    info!("Starting wake-up recovery task");
    
    let mut interval = interval(Duration::from_secs(15));
    
    loop {
        interval.tick().await;
        
        // Check if system was suspended
        match state.is_suspended() {
            Ok(true) => {
                info!("System wake-up detected, triggering state check");
                
                // Trigger state check to restart timer if needed
                if let Err(e) = state.trigger_state_check() {
                    warn!("Failed to trigger wake-up state check: {}", e);
                }
                
                // Clear the suspended flag
                if let Err(e) = state.set_suspended(false) {
                    warn!("Failed to clear suspended state: {}", e);
                }
            }
            Ok(false) => {
                // System not suspended, continue monitoring
            }
            Err(e) => {
                warn!("Failed to check suspended state: {}", e);
            }
        }
    }
}
