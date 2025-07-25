//! Order Coffee - A state-managed HTTP server to control system suspension
//! 
//! This library provides functionality to manage system states that prevent
//! automatic suspension, including coffee mode and Ollama service management.

pub mod config;
pub mod state;
pub mod api;
pub mod services;
pub mod tasks;
pub mod utils;

// Re-export commonly used types
pub use config::Config;
pub use state::AppState;
pub use api::create_router;
pub use utils::signals::shutdown_signal;
