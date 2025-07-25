//! Utility functions module
//! 
//! This module contains utility functions used throughout the application.

pub mod signals;

// Re-export main functions
pub use signals::shutdown_signal;
