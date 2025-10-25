// Coordinator Module - Central event coordination
//
// This module handles event routing and session state decisions

pub mod core;

pub use core::start_coordinator_with_cleanup;
// pub use core::start_coordinator; // Unused - use start_coordinator_with_cleanup instead