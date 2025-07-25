//! Background tasks module
//! 
//! This module contains background tasks that run alongside the HTTP server.

pub mod suspension_timer;
pub mod wake_up_recovery;

// Re-export main functions
pub use suspension_timer::suspension_timer_task;
pub use wake_up_recovery::wake_up_recovery_task;
