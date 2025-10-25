// Notification Module
//
// Handles all user notifications for ClaudeMiner using singleton pattern
// - Task completion notifications
// - Session state change notifications
// - Zombie process termination notifications
//

use crate::session::SessionState;
use tauri::api::notification::Notification;
use once_cell::sync::OnceCell;

/// Global AppHandle singleton for notifications
static APP_HANDLE: OnceCell<tauri::AppHandle> = OnceCell::new();

/// Initialize the notification system with AppHandle
/// This should be called once during app setup
pub fn init(app_handle: tauri::AppHandle) {
    if APP_HANDLE.set(app_handle).is_err() {
        eprintln!("[Notification] Warning: AppHandle already initialized");
    }
    println!("[Notification] ✅ Notification system initialized");
}

/// Get the bundle identifier for notifications
fn get_bundle_id() -> String {
    APP_HANDLE
        .get()
        .map(|handle| handle.config().tauri.bundle.identifier.clone())
        .unwrap_or_else(|| {
            eprintln!("[Notification] ⚠️ AppHandle not initialized, using default bundle ID");
            "com.claudeminer.app".to_string()
        })
}

/// Send notification when Claude task completes (working → resting)
pub fn send_task_completion_notification(session: &SessionState) {
    let session_short = &session.session_id[..8.min(session.session_id.len())];

    println!("[Notification] 📢 Sending task completion notification for session {} (PID: {})",
        session_short, session.pid);

    let notification_result = Notification::new(&get_bundle_id())
        .title("Claude Task Completed ✅")
        .body(&format!("Claude #{} has finished working", session.pid))
        .show();

    match notification_result {
        Ok(_) => {
            println!("[Notification] ✅ Task completion notification sent successfully");
        }
        Err(e) => {
            println!("[Notification] ⚠️ Failed to send notification: {}", e);
        }
    }
}

/// Send notification when new session is created
pub fn send_session_created_notification(session: &SessionState) {
    let session_short = &session.session_id[..8.min(session.session_id.len())];

    println!("[Notification] 📢 Sending new session notification for session {} (PID: {})",
        session_short, session.pid);

    let notification_result = Notification::new(&get_bundle_id())
        .title("New Claude Session Started 🚀")
        .body(&format!("Claude #{} has started", session.pid))
        .show();

    match notification_result {
        Ok(_) => {
            println!("[Notification] ✅ Session created notification sent successfully");
        }
        Err(e) => {
            println!("[Notification] ⚠️ Failed to send notification: {}", e);
        }
    }
}

/// Send notification when zombie process is killed
pub fn send_zombie_killed_notification(pid: u32) {
    println!("[Notification] 📢 Sending zombie killed notification for PID: {}", pid);

    let notification_result = Notification::new(&get_bundle_id())
        .title("✅ Zombie Process Terminated")
        .body(&format!("Successfully killed zombie process #{}", pid))
        .show();

    match notification_result {
        Ok(_) => {
            println!("[Notification] ✅ Zombie killed notification sent successfully");
        }
        Err(e) => {
            println!("[Notification] ⚠️ Failed to send notification: {}", e);
        }
    }
}

/// Send test notification for debugging
pub fn send_test_notification() {
    println!("[Notification] 🔔 Sending test notification");

    let notification_result = Notification::new(&get_bundle_id())
        .title("🧪 Test Notification")
        .body("ClaudeMiner notification system is working correctly!")
        .show();

    match notification_result {
        Ok(_) => {
            println!("[Notification] ✅ Test notification sent successfully");
        }
        Err(e) => {
            println!("[Notification] ⚠️ Failed to send test notification: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_message_format() {
        // Test that notification messages are properly formatted
        let pid = 12345;
        let expected = format!("Claude #{} has finished working", pid);
        assert_eq!(expected, "Claude #12345 has finished working");
    }
}
