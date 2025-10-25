// Monitor Module - Pure monitoring functionality
//
// This module handles CPU and log file monitoring

pub mod cpu;
pub mod log;

// Re-export monitoring functions
pub use cpu::start_cpu_monitor;
pub use log::start_log_watcher;