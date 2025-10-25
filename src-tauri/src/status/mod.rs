// Status Detection Module
//
// Provides intelligent multi-layered status detection for Claude Code processes

pub mod debouncer;
pub mod hybrid;
pub mod file_lock;

// pub use debouncer::apply_debouncing; // Unused
// pub use hybrid::is_zombie_by_tty; // Used directly via crate::status::hybrid::is_zombie_by_tty
// pub use hybrid::{LogActivityTracker, determine_hybrid_status}; // Unused
// pub use file_lock::{is_file_opened, is_file_opened_by_pid, get_pid_with_file_opened}; // Unused
