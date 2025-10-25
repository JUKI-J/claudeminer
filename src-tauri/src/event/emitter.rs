// Event Emitter Module
//
// Centralized Tauri event emission using singleton pattern
// - Session lifecycle events (created, status changed, terminated)
// - Tray menu updates
//

use crate::session::SessionState;
use once_cell::sync::OnceCell;
use tauri::Manager;

/// Global AppHandle singleton for event emission
static APP_HANDLE: OnceCell<tauri::AppHandle> = OnceCell::new();

/// Initialize the event emitter with AppHandle
/// This should be called once during app setup
pub fn init(app_handle: tauri::AppHandle) {
    if APP_HANDLE.set(app_handle).is_err() {
        eprintln!("[EventEmitter] Warning: AppHandle already initialized");
    }
    println!("[EventEmitter] ✅ Event emitter initialized");
}

/// Get the AppHandle (internal helper)
fn get_handle() -> Option<&'static tauri::AppHandle> {
    APP_HANDLE.get()
}

/// Emit session-created event to frontend
pub fn emit_session_created(session: &SessionState) {
    if let Some(handle) = get_handle() {
        if let Err(e) = handle.emit_all("session-created", session) {
            eprintln!("[EventEmitter] Failed to emit session-created: {}", e);
        } else {
            println!("[EventEmitter] 📡 Emitted session-created for session {}",
                &session.session_id[..8.min(session.session_id.len())]);
        }
    } else {
        eprintln!("[EventEmitter] ⚠️ Cannot emit session-created: AppHandle not initialized");
    }
}

/// Emit session-status-changed event to frontend
pub fn emit_session_status_changed(session: &SessionState) {
    if let Some(handle) = get_handle() {
        if let Err(e) = handle.emit_all("session-status-changed", session) {
            eprintln!("[EventEmitter] Failed to emit session-status-changed: {}", e);
        } else {
            println!("[EventEmitter] 📡 Emitted session-status-changed for session {} (status: {})",
                &session.session_id[..8.min(session.session_id.len())],
                session.current_status);
        }
    } else {
        eprintln!("[EventEmitter] ⚠️ Cannot emit session-status-changed: AppHandle not initialized");
    }
}

/// Emit session-terminated event to frontend
pub fn emit_session_terminated(session: &SessionState) {
    if let Some(handle) = get_handle() {
        if let Err(e) = handle.emit_all("session-terminated", session) {
            eprintln!("[EventEmitter] Failed to emit session-terminated: {}", e);
        } else {
            println!("[EventEmitter] 📡 Emitted session-terminated for session {}",
                &session.session_id[..8.min(session.session_id.len())]);
        }
    } else {
        eprintln!("[EventEmitter] ⚠️ Cannot emit session-terminated: AppHandle not initialized");
    }
}

/// Update tray menu with session statistics
pub fn update_tray_menu(total: u32, working: u32, resting: u32, zombie: u32) -> Result<(), String> {
    if let Some(handle) = get_handle() {
        use tauri::{SystemTrayMenu, SystemTrayMenuItem, CustomMenuItem};

        let tray = handle.tray_handle();

        // Update tray icon title with working count (macOS only)
        #[cfg(target_os = "macos")]
        {
            let title = if working > 0 {
                format!("⛏️ {}", working)
            } else {
                String::new() // Empty when no working sessions
            };
            let _ = tray.set_title(&title); // Ignore errors on other platforms
        }

        // Update tooltip
        tray.set_tooltip(&format!("ClaudeMiner - {} sessions", total))
            .map_err(|e| e.to_string())?;

        // Create new menu with stats
        let stats_label = CustomMenuItem::new("stats".to_string(),
            format!("📊 Active Sessions: {}", total)).disabled();
        let working_label = CustomMenuItem::new("working".to_string(),
            format!("⛏️  Working: {}", working)).disabled();
        let resting_label = CustomMenuItem::new("resting".to_string(),
            format!("😴 Resting: {}", resting)).disabled();
        let zombie_label = CustomMenuItem::new("zombie".to_string(),
            format!("🧟 Zombie: {}", zombie)).disabled();

        let separator1 = SystemTrayMenuItem::Separator;
        let show = CustomMenuItem::new("show".to_string(), "Show Window");
        let separator2 = SystemTrayMenuItem::Separator;
        let quit = CustomMenuItem::new("quit".to_string(), "Quit");

        let tray_menu = SystemTrayMenu::new()
            .add_item(stats_label)
            .add_item(working_label)
            .add_item(resting_label)
            .add_item(zombie_label)
            .add_native_item(separator1)
            .add_item(show)
            .add_native_item(separator2)
            .add_item(quit);

        tray.set_menu(tray_menu)
            .map_err(|e| e.to_string())?;

        println!("[EventEmitter] 🎯 Updated tray menu: {} sessions (working: {}, resting: {}, zombie: {})",
            total, working, resting, zombie);

        Ok(())
    } else {
        Err("AppHandle not initialized".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_emitter_without_init() {
        // Should not panic, just print warnings
        let session = SessionState::default();
        emit_session_created(&session);
        emit_session_status_changed(&session);
        emit_session_terminated(&session);
    }
}
