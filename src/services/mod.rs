//! External service management module
//! 
//! This module contains functions for managing external services like Ollama
//! and system operations like suspension.

pub mod ollama;
pub mod system;

// Re-export main functions
pub use ollama::*;
pub use system::*;
