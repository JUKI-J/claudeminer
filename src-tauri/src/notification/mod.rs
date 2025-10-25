// Notification Module - User notification management
//
// This module handles all notification functionality for ClaudeMiner
// using a singleton pattern for AppHandle management

pub mod sender;

// Re-export public API
pub use sender::{
    init,
    send_task_completion_notification,
    // send_session_created_notification, // Unused
    send_zombie_killed_notification,
    send_test_notification,
};
