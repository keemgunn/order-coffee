//! Background tasks module
//! 
//! This module contains background tasks that run alongside the HTTP server.

pub mod suspension_timer;

// Re-export main functions
pub use suspension_timer::suspension_timer_task;
