// Event Module - Centralized Tauri event management
//
// This module handles all Tauri event emission using singleton pattern

pub mod emitter;

// Re-export public API
pub use emitter::{
    init,
    emit_session_created,
    emit_session_status_changed,
    emit_session_terminated,
    update_tray_menu,
};
