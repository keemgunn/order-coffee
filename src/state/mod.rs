//! State management module
//! 
//! This module contains all state-related structures and their management logic.

pub mod system_state;
pub mod app_state;
pub mod timer_state;

// Re-export main types
pub use system_state::SystemState;
pub use app_state::AppState;
pub use timer_state::TimerState;
