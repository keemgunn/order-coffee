//! Main application state management

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, watch};
use tracing::{info, warn};

use super::{SystemState, TimerState};

/// Main application state that manages all system states and timer
#[derive(Debug)]
pub struct AppState {
    /// Current system states (coffee, ollama, errors)
    pub system_state: Arc<Mutex<SystemState>>,
    /// Timer configuration and state
    pub timer_duration_minutes: u64,
    pub timer_state: Arc<Mutex<TimerState>>,
    /// Server metadata
    pub start_time: Instant,
    pub port: u16,
    pub host: String,
    /// Last action tracking
    pub last_action: Arc<Mutex<Option<String>>>,
    pub last_action_time: Arc<Mutex<Option<DateTime<Utc>>>>,
    /// Channels for state change notifications
    pub state_change_tx: broadcast::Sender<SystemState>,
    /// Channel for timer updates
    pub timer_update_tx: watch::Sender<TimerState>,
    /// Keep the receiver alive to prevent channel closure
    pub _timer_update_rx: watch::Receiver<TimerState>,
}

impl AppState {
    /// Create a new AppState with default values
    pub fn new(port: u16, host: String, timer_duration_minutes: u64) -> Self {
        let (state_change_tx, _) = broadcast::channel(100);
        let (timer_update_tx, timer_update_rx) = watch::channel(TimerState::new());

        Self {
            system_state: Arc::new(Mutex::new(SystemState::new())),
            timer_duration_minutes,
            timer_state: Arc::new(Mutex::new(TimerState::new())),
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
    pub fn update_state<F>(&self, action: &str, updater: F) -> Result<SystemState, String>
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
    pub fn set_coffee(&self, active: bool) -> Result<SystemState, String> {
        info!("Setting coffee state to: {}", active);
        self.update_state(
            if active { "coffee" } else { "chill" },
            |state| state.coffee = active,
        )
    }

    /// Set the ollama state
    pub fn set_ollama(&self, active: bool) -> Result<SystemState, String> {
        info!("Setting ollama state to: {}", active);
        self.update_state(
            if active { "ollama-on" } else { "ollama-off" },
            |state| state.ollama = active,
        )
    }

    /// Add an error to the state
    pub fn add_error(&self, error: String) -> Result<(), String> {
        let mut state = self.system_state.lock()
            .map_err(|e| format!("Failed to lock system state: {}", e))?;
        
        warn!("Adding error to state: {}", error);
        state.add_error(error);
        let new_state = state.clone();
        drop(state);

        // Notify listeners of the error update
        if let Err(e) = self.state_change_tx.send(new_state) {
            warn!("Failed to send error state notification: {}", e);
        }

        Ok(())
    }

    /// Clear errors for a specific component
    pub fn clear_errors_for(&self, component: &str) -> Result<(), String> {
        let mut state = self.system_state.lock()
            .map_err(|e| format!("Failed to lock system state: {}", e))?;
        
        let initial_count = state.errors.len();
        state.clear_errors_for(component);
        
        if state.errors.len() != initial_count {
            let new_state = state.clone();
            drop(state);

            if let Err(e) = self.state_change_tx.send(new_state) {
                warn!("Failed to send error clear notification: {}", e);
            }
        }

        Ok(())
    }

    /// Get current system state
    pub fn get_system_state(&self) -> Result<SystemState, String> {
        self.system_state.lock()
            .map(|state| state.clone())
            .map_err(|e| format!("Failed to lock system state: {}", e))
    }

    /// Get current timer state
    pub fn get_timer_state(&self) -> Result<TimerState, String> {
        self.timer_state.lock()
            .map(|state| state.clone())
            .map_err(|e| format!("Failed to lock timer state: {}", e))
    }

    /// Update timer state
    pub fn update_timer_state(&self, active: bool, remaining_seconds: Option<u64>) -> Result<(), String> {
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
    pub fn get_uptime(&self) -> String {
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

    /// Get last action information
    pub fn get_last_action(&self) -> (Option<String>, Option<DateTime<Utc>>) {
        let last_action = self.last_action.lock().ok().and_then(|a| a.clone());
        let last_action_time = self.last_action_time.lock().ok().and_then(|t| *t);
        (last_action, last_action_time)
    }

    /// Trigger an initial state check to start the suspension timer if needed
    pub fn trigger_state_check(&self) -> Result<(), String> {
        let current_state = self.get_system_state()?;
        
        // Send the current state to trigger timer logic
        if let Err(e) = self.state_change_tx.send(current_state) {
            warn!("Failed to send initial state check: {}", e);
            return Err(format!("Failed to trigger state check: {}", e));
        }
        
        info!("Initial state check triggered");
        Ok(())
    }

    /// Set the suspended state (internal use only)
    pub fn set_suspended(&self, suspended: bool) -> Result<(), String> {
        let mut state = self.system_state.lock()
            .map_err(|e| format!("Failed to lock system state: {}", e))?;
        
        state.set_suspended(suspended);
        info!("Suspended state set to: {}", suspended);
        Ok(())
    }

    /// Check if the system was suspended (internal use only)
    pub fn is_suspended(&self) -> Result<bool, String> {
        let state = self.system_state.lock()
            .map_err(|e| format!("Failed to lock system state: {}", e))?;
        
        Ok(state.is_suspended())
    }
}
