// Session Module - Session state management
//
// This module handles all session-related functionality

pub mod analyzer;
pub mod finder;
pub mod manager;
pub mod cleaner;
pub mod state;

// Core types
pub use state::{SessionState, SessionType, MonitorEvent, LogEvent, CpuEvent, HookEvent, current_timestamp};

// Session management
// pub use manager::{SessionManager, SessionUpdateResult, SessionStatistics}; // Unused
pub use cleaner::{start_session_cleaner, CleanupEvent};

// Session utilities
// pub use analyzer::{analyze_log_content, check_session_activity}; // Unused
// pub use finder::{find_session_id_for_pid, get_claude_debug_dir}; // Unused