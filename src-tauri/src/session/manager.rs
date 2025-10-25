// Session Manager
//
// Manages session state and lifecycle, separated from Coordinator
// Handles Legacy/Hook session logic and state transitions
//

use crate::session::{SessionState, SessionType, LogEvent, CpuEvent, HookEvent, current_timestamp};
use crate::types::WorkingState;
use crate::status::hybrid::is_zombie_by_tty;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
// use sysinfo::{System, Pid}; // Unused

const STALE_MTIME_THRESHOLD_SECS: u64 = 30;  // mtime older than 30s = stale
const LOW_CPU_THRESHOLD: f32 = 20.0;          // CPU < 20% = likely not working
const HIGH_CPU_THRESHOLD: f32 = 50.0;         // CPU > 50% = likely working
const CPU_AGE_THRESHOLD_SECS: u64 = 10;       // CPU data older than 10s = stale

/// Session Manager - Manages all session states and transitions
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    pid_to_session: Arc<Mutex<HashMap<u32, String>>>,
    app_start_time: u64,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            pid_to_session: Arc::new(Mutex::new(HashMap::new())),
            app_start_time: current_timestamp(),
        }
    }

    /// Get shared sessions reference (for other threads)
    pub fn get_shared_sessions(&self) -> Arc<Mutex<HashMap<String, SessionState>>> {
        self.sessions.clone()
    }

    /// Get all sessions snapshot
    pub fn get_all_sessions(&self) -> HashMap<String, SessionState> {
        self.sessions.lock().unwrap().clone()
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Option<SessionState> {
        self.sessions.lock().unwrap().get(session_id).cloned()
    }

    /// Get session by PID
    pub fn get_session_by_pid(&self, pid: u32) -> Option<SessionState> {
        let pid_map = self.pid_to_session.lock().unwrap();
        if let Some(session_id) = pid_map.get(&pid) {
            self.get_session(session_id)
        } else {
            None
        }
    }

    /// Process log event
    pub fn handle_log_event(&self, event: LogEvent) -> SessionUpdateResult {
        let session_id = event.session_id.clone();
        let mut result = SessionUpdateResult::default();

        // Lock sessions
        let mut sessions = self.sessions.lock().unwrap();
        let mut pid_map = self.pid_to_session.lock().unwrap();

        // Check if this is a new session
        let is_new = !sessions.contains_key(&session_id);
        result.is_new_session = is_new;

        // Get or create session (Legacy type from log)
        let session = sessions.entry(session_id.clone()).or_insert_with(|| {
            let pid = event.pid.unwrap_or(0);
            println!("[SessionManager] Creating LEGACY session from log: {} (PID: {})",
                &session_id[..8.min(session_id.len())], pid);

            if pid != 0 {
                pid_map.insert(pid, session_id.clone());
            }

            SessionState::new_legacy(pid, session_id.clone())
        });

        // Update session with log event
        session.last_log_event = Some(event.clone());
        session.last_update = current_timestamp();

        // Determine if this is a legacy session (created before app start)
        if session.session_type == SessionType::Legacy {
            // Check TTY status
            if session.pid != 0 {
                session.has_terminal = !is_zombie_by_tty(session.pid);
            }
        }

        // Decide new status
        let old_status = session.current_status;
        let new_status = self.decide_session_status(session);

        if new_status != old_status {
            println!("[SessionManager] Session {} status change: {} -> {}",
                &session_id[..8.min(session_id.len())], old_status, new_status);
            session.current_status = new_status;
            result.status_changed = true;
            result.new_status = Some(new_status.to_string());
        }

        result.session = session.clone();
        result
    }

    /// Process CPU event
    pub fn handle_cpu_event(&self, event: CpuEvent) -> SessionUpdateResult {
        let mut result = SessionUpdateResult::default();
        let mut sessions = self.sessions.lock().unwrap();
        let mut pid_map = self.pid_to_session.lock().unwrap();

        // Find session by PID
        let session_id = if let Some(sid) = pid_map.get(&event.pid).cloned() {
            sid
        } else {
            // Create temporary session for unknown PID
            let temp_id = format!("pid-{}", event.pid);
            println!("[SessionManager] Creating temporary session for PID {}", event.pid);

            sessions.insert(
                temp_id.clone(),
                SessionState::new_legacy(event.pid, temp_id.clone())
            );
            pid_map.insert(event.pid, temp_id.clone());
            temp_id
        };

        // Update session
        if let Some(session) = sessions.get_mut(&session_id) {
            session.last_cpu_event = Some(event.clone());
            session.last_update = current_timestamp();

            // Update PID if needed
            if session.pid == 0 {
                session.pid = event.pid;
                pid_map.insert(event.pid, session_id.clone());
            }

            // Check TTY for zombie detection (Legacy sessions only)
            if session.session_type == SessionType::Legacy {
                let has_tty = !is_zombie_by_tty(event.pid);
                if session.has_terminal != has_tty {
                    session.has_terminal = has_tty;

                    // Re-evaluate status if TTY changed
                    let old_status = session.current_status;
                    let new_status = self.decide_session_status(session);

                    if new_status != old_status {
                        session.current_status = new_status;
                        result.status_changed = true;
                        result.new_status = Some(new_status.to_string());
                    }
                }
            }

            result.session = session.clone();
        }

        result
    }

    /// Process hook event
    pub fn handle_hook_event(&self, event: HookEvent) -> SessionUpdateResult {
        let session_id = event.sid.clone();
        let mut result = SessionUpdateResult::default();

        let mut sessions = self.sessions.lock().unwrap();

        match event.evt.as_str() {
            "start" => {
                // Session start event
                let is_new = !sessions.contains_key(&session_id);
                result.is_new_session = is_new;

                let session = sessions.entry(session_id.clone()).or_insert_with(|| {
                    println!("[SessionManager] Creating HOOK session: {}",
                        &session_id[..8.min(session_id.len())]);
                    SessionState::new_hook(session_id.clone())
                });

                // Upgrade Legacy to Hook if needed
                if session.session_type == SessionType::Legacy {
                    if session.upgrade_to_hook() {
                        result.session_upgraded = true;
                        println!("[SessionManager] Session {} upgraded to Hook on 'start' event", &session_id[..8]);
                    }
                }

                session.current_status = "resting";
                session.last_update = current_timestamp();
                result.session = session.clone();
            }

            "working" => {
                if let Some(session) = sessions.get_mut(&session_id) {
                    // Upgrade Legacy to Hook if needed
                    if session.session_type == SessionType::Legacy {
                        if session.upgrade_to_hook() {
                            result.session_upgraded = true;
                            println!("[SessionManager] Session {} upgraded to Hook on 'working' event", &session_id[..8]);
                        }
                    }

                    let old_status = session.current_status;
                    session.current_status = "working";
                    session.last_update = current_timestamp();

                    if old_status != "working" {
                        result.status_changed = true;
                        result.new_status = Some("working".to_string());
                    }

                    result.session = session.clone();
                }
            }

            "resting" => {
                if let Some(session) = sessions.get_mut(&session_id) {
                    // Upgrade Legacy to Hook if needed
                    if session.session_type == SessionType::Legacy {
                        if session.upgrade_to_hook() {
                            result.session_upgraded = true;
                            println!("[SessionManager] Session {} upgraded to Hook on 'resting' event", &session_id[..8]);
                        }
                    }

                    let old_status = session.current_status;
                    session.current_status = "resting";
                    session.last_update = current_timestamp();

                    if old_status != "resting" {
                        result.status_changed = true;
                        result.new_status = Some("resting".to_string());
                    }

                    result.session = session.clone();
                }
            }

            "end" => {
                if let Some(session) = sessions.remove(&session_id) {
                    println!("[SessionManager] Session terminated via hook: {}",
                        &session_id[..8.min(session_id.len())]);

                    // Remove from PID mapping
                    if session.pid != 0 {
                        let mut pid_map = self.pid_to_session.lock().unwrap();
                        pid_map.remove(&session.pid);
                    }

                    result.session_terminated = true;
                    result.session = session;
                }
            }

            _ => {
                println!("[SessionManager] Unknown hook event: {}", event.evt);
            }
        }

        result
    }

    /// Decide session status based on type and available data
    fn decide_session_status(&self, session: &SessionState) -> &'static str {
        // Priority 0: TTY check for zombie detection
        if !session.has_terminal {
            return "zombie";
        }

        // Different logic based on session type
        match session.session_type {
            SessionType::Legacy => self.decide_legacy_status(session),
            SessionType::Hook => session.current_status, // Hook sessions maintain their status
        }
    }

    /// Decide status for Legacy sessions
    fn decide_legacy_status(&self, session: &SessionState) -> &'static str {
        let now = current_timestamp();

        // Check log event
        if let Some(ref log) = session.last_log_event {
            let mtime_age = now.saturating_sub(log.file_mtime);

            // If "Stream started - received first chunk" was found → working
            if matches!(log.state, WorkingState::ActivelyWorking) {
                // But check if it's stale
                if mtime_age >= STALE_MTIME_THRESHOLD_SECS {
                    return "resting";
                }

                // Also check CPU to confirm still working
                if let Some(ref cpu) = session.last_cpu_event {
                    let cpu_age = now.saturating_sub(cpu.timestamp);
                    if cpu_age < CPU_AGE_THRESHOLD_SECS && cpu.cpu_percent < LOW_CPU_THRESHOLD {
                        return "resting";
                    }
                }

                return "working";
            }
        }

        // Check CPU usage (fallback)
        if let Some(ref cpu) = session.last_cpu_event {
            let cpu_age = now.saturating_sub(cpu.timestamp);
            if cpu_age < CPU_AGE_THRESHOLD_SECS && cpu.cpu_percent > HIGH_CPU_THRESHOLD {
                return "working";
            }
        }

        // Default to resting
        "resting"
    }

    /// Remove stale sessions
    pub fn cleanup_stale_sessions(&self, threshold_secs: u64) -> Vec<SessionState> {
        let now = current_timestamp();
        let mut removed = Vec::new();

        let mut sessions = self.sessions.lock().unwrap();
        let mut pid_map = self.pid_to_session.lock().unwrap();

        sessions.retain(|session_id, session| {
            let age = now.saturating_sub(session.last_update);
            if age > threshold_secs {
                println!("[SessionManager] Removing stale session: {} (age: {}s)",
                    &session_id[..8.min(session_id.len())], age);

                // Remove from PID mapping
                if session.pid != 0 {
                    pid_map.remove(&session.pid);
                }

                removed.push(session.clone());
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get session statistics
    pub fn get_statistics(&self) -> SessionStatistics {
        let sessions = self.sessions.lock().unwrap();

        let mut stats = SessionStatistics::default();
        stats.total_sessions = sessions.len();

        for (_id, session) in sessions.iter() {
            match session.current_status {
                "working" => stats.working_count += 1,
                "resting" => stats.resting_count += 1,
                "zombie" => stats.zombie_count += 1,
                _ => stats.unknown_count += 1,
            }

            match session.session_type {
                SessionType::Legacy => stats.legacy_sessions += 1,
                SessionType::Hook => stats.hook_sessions += 1,
            }
        }

        stats
    }
}

/// Result of session update operation
#[derive(Debug)]
pub struct SessionUpdateResult {
    pub session: SessionState,
    pub is_new_session: bool,
    pub status_changed: bool,
    pub new_status: Option<String>,
    pub session_upgraded: bool,
    pub session_terminated: bool,
}

impl Default for SessionUpdateResult {
    fn default() -> Self {
        Self {
            session: SessionState::new_legacy(0, String::new()),
            is_new_session: false,
            status_changed: false,
            new_status: None,
            session_upgraded: false,
            session_terminated: false,
        }
    }
}

/// Session statistics
#[derive(Debug, Default)]
pub struct SessionStatistics {
    pub total_sessions: usize,
    pub working_count: usize,
    pub resting_count: usize,
    pub zombie_count: usize,
    pub unknown_count: usize,
    pub legacy_sessions: usize,
    pub hook_sessions: usize,
}

impl SessionStatistics {
    pub fn log_summary(&self) {
        println!("[SessionManager] === Session Statistics ===");
        println!("  Total sessions: {}", self.total_sessions);
        println!("  Status breakdown:");
        println!("    Working: {}", self.working_count);
        println!("    Resting: {}", self.resting_count);
        println!("    Zombie: {}", self.zombie_count);
        if self.unknown_count > 0 {
            println!("    Unknown: {}", self.unknown_count);
        }
        println!("  Session types:");
        println!("    Legacy: {}", self.legacy_sessions);
        println!("    Hook-enabled: {}", self.hook_sessions);
        println!("==============================");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_manager_creation() {
        let manager = SessionManager::new();
        let stats = manager.get_statistics();
        assert_eq!(stats.total_sessions, 0);
    }

    #[test]
    fn test_hook_event_handling() {
        let manager = SessionManager::new();

        // Test session start
        let start_event = HookEvent {
            sid: "test-session".to_string(),
            evt: "start".to_string(),
        };

        let result = manager.handle_hook_event(start_event);
        assert!(result.is_new_session);
        assert_eq!(result.session.session_type, SessionType::Hook);
        assert_eq!(result.session.current_status, "resting");

        // Test transition to working
        let working_event = HookEvent {
            sid: "test-session".to_string(),
            evt: "working".to_string(),
        };

        let result = manager.handle_hook_event(working_event);
        assert!(result.status_changed);
        assert_eq!(result.new_status, Some("working".to_string()));
    }

    #[test]
    fn test_legacy_to_hook_upgrade() {
        let manager = SessionManager::new();

        // Create legacy session via log event
        let log_event = LogEvent {
            session_id: "test-session".to_string(),
            pid: Some(1234),
            timestamp: current_timestamp(),
            state: WorkingState::MaybeWorking,
            has_approval_pending: false,
            file_mtime: current_timestamp(),
        };

        let result = manager.handle_log_event(log_event);
        assert_eq!(result.session.session_type, SessionType::Legacy);

        // Now send hook event for same session
        let hook_event = HookEvent {
            sid: "test-session".to_string(),
            evt: "working".to_string(),
        };

        let result = manager.handle_hook_event(hook_event);
        assert!(result.session_upgraded);
        assert_eq!(result.session.session_type, SessionType::Hook);
    }

    #[test]
    fn test_statistics() {
        let manager = SessionManager::new();

        // Add some sessions
        for i in 0..5 {
            let event = HookEvent {
                sid: format!("session-{}", i),
                evt: if i % 2 == 0 { "working" } else { "resting" }.to_string(),
            };

            // Start session first
            manager.handle_hook_event(HookEvent {
                sid: format!("session-{}", i),
                evt: "start".to_string(),
            });

            // Then set status
            manager.handle_hook_event(event);
        }

        let stats = manager.get_statistics();
        assert_eq!(stats.total_sessions, 5);
        assert_eq!(stats.hook_sessions, 5);
        assert!(stats.working_count > 0);
        assert!(stats.resting_count > 0);
    }
}