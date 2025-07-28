//! System state structure and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// System state structure - holds all states that can prevent suspension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    /// Manual sleep prevention state (controlled by /coffee and /chill endpoints)
    pub coffee: bool,
    /// Generic services state (replaces ollama: bool)
    pub services: HashMap<String, bool>,
    /// Internal flag to track if system was suspended (not exposed in API)
    #[serde(skip)]
    suspended: bool,
    /// List of current errors for client visibility
    pub errors: Vec<String>,
}

impl SystemState {
    /// Create a new SystemState with all states set to false
    pub fn new() -> Self {
        let mut services = HashMap::new();
        services.insert("ollama".to_string(), false); // Initialize ollama
        
        Self {
            coffee: false,
            services,
            suspended: false,
            errors: Vec::new(),
        }
    }

    /// Check if any state is active (true)
    pub fn any_active(&self) -> bool {
        self.coffee || self.services.values().any(|&active| active)
    }

    /// Check if all states are inactive (false)
    pub fn all_inactive(&self) -> bool {
        !self.any_active()
    }

    /// Add an error to the state
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Clear errors for a specific component
    pub fn clear_errors_for(&mut self, component: &str) {
        let initial_count = self.errors.len();
        self.errors.retain(|error| !error.to_lowercase().contains(&component.to_lowercase()));
        
        if self.errors.len() != initial_count {
            tracing::info!("Cleared {} errors for component: {}", initial_count - self.errors.len(), component);
        }
    }

    /// Set a service state
    pub fn set_service(&mut self, service_name: &str, active: bool) {
        self.services.insert(service_name.to_string(), active);
    }

    /// Get a service state
    pub fn get_service(&self, service_name: &str) -> bool {
        self.services.get(service_name).copied().unwrap_or(false)
    }

    /// Set the suspended state (internal use only)
    pub(crate) fn set_suspended(&mut self, suspended: bool) {
        self.suspended = suspended;
    }

    /// Check if the system was suspended (internal use only)
    pub(crate) fn is_suspended(&self) -> bool {
        self.suspended
    }
}

impl Default for SystemState {
    fn default() -> Self {
        Self::new()
    }
}
