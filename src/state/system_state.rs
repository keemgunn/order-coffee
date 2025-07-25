//! System state structure and management

use serde::{Deserialize, Serialize};

/// System state structure - holds all states that can prevent suspension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    /// Manual sleep prevention state (controlled by /coffee and /chill endpoints)
    pub coffee: bool,
    /// Ollama service state (controlled by /ollama-on and /ollama-off endpoints)
    pub ollama: bool,
    /// List of current errors for client visibility
    pub errors: Vec<String>,
}

impl SystemState {
    /// Create a new SystemState with all states set to false
    pub fn new() -> Self {
        Self {
            coffee: false,
            ollama: false,
            errors: Vec::new(),
        }
    }

    /// Check if any state is active (true)
    pub fn any_active(&self) -> bool {
        self.coffee || self.ollama
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
}

impl Default for SystemState {
    fn default() -> Self {
        Self::new()
    }
}
