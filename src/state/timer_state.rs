//! Timer state structure and management

/// Timer state for tracking suspension countdown
#[derive(Debug, Clone)]
pub struct TimerState {
    pub active: bool,
    pub remaining_seconds: Option<u64>,
}

impl TimerState {
    /// Create a new inactive timer state
    pub fn new() -> Self {
        Self {
            active: false,
            remaining_seconds: None,
        }
    }

    /// Create an active timer state with remaining seconds
    pub fn active(remaining_seconds: u64) -> Self {
        Self {
            active: true,
            remaining_seconds: Some(remaining_seconds),
        }
    }

    /// Create an inactive timer state
    pub fn inactive() -> Self {
        Self {
            active: false,
            remaining_seconds: None,
        }
    }

    /// Check if the timer is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get remaining seconds if timer is active
    pub fn remaining_seconds(&self) -> Option<u64> {
        if self.active {
            self.remaining_seconds
        } else {
            None
        }
    }
}

impl Default for TimerState {
    fn default() -> Self {
        Self::new()
    }
}
