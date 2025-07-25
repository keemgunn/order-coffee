//! API response structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::SystemState;

/// API response structure for state change endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub states: SystemState,
}

impl ApiResponse {
    /// Create a new API response
    pub fn new(status: String, message: String, states: SystemState) -> Self {
        Self {
            status,
            message,
            timestamp: Utc::now(),
            states,
        }
    }

    /// Create an active response
    pub fn active(message: String, states: SystemState) -> Self {
        Self::new("active".to_string(), message, states)
    }

    /// Create an inactive response
    pub fn inactive(message: String, states: SystemState) -> Self {
        Self::new("inactive".to_string(), message, states)
    }

    /// Create an error response
    pub fn error(message: String, states: SystemState) -> Self {
        Self::new("error".to_string(), message, states)
    }
}

/// Enhanced status response with timer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub states: SystemState,
    pub timer_active: bool,
    pub timer_remaining_seconds: Option<u64>,
    pub uptime: String,
    pub port: u16,
    pub host: String,
    pub last_action: Option<String>,
    pub last_action_time: Option<DateTime<Utc>>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

impl HealthResponse {
    /// Create a new health response
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            timestamp: Utc::now(),
            version: "2.0.0".to_string(),
        }
    }
}
