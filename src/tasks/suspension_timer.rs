//! Suspension timer background task

use std::{sync::Arc, time::{Duration, Instant}};
use tokio::time::sleep;
use tracing::{debug, error, info};

use crate::{
    services::execute_system_suspend,
    state::AppState,
};

/// Background task that manages the suspension timer based on system state changes
pub async fn suspension_timer_task(state: Arc<AppState>) {
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
                                    if let Err(e) = execute_system_suspend(Arc::clone(&state)).await {
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
