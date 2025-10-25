// Monitor Events
//
// Event types for multi-threaded monitoring system

use crate::types::WorkingState;
use serde::{Serialize, Deserialize};

/// Unified monitor event
#[derive(Debug, Clone)]
pub enum MonitorEvent {
    Log(LogEvent),
    Cpu(CpuEvent),
    Hook(HookEvent),
}

/// Log file change event
#[derive(Debug, Clone, Serialize)]
pub struct LogEvent {
    pub session_id: String,
    pub pid: Option<u32>,
    pub timestamp: u64,
    pub state: WorkingState,
    pub has_approval_pending: bool,
    pub file_mtime: u64,  // File modification time (Unix timestamp)
}

/// CPU usage change event
#[derive(Debug, Clone, Serialize)]
pub struct CpuEvent {
    pub pid: u32,
    pub timestamp: u64,
    pub cpu_percent: f32,
}

/// Hook event from Claude Code hooks (via named pipe)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub sid: String,      // session_id
    pub evt: String,      // start|working|resting|end
}

/// Session type: Legacy (pre-app start) or Hook (post-app start)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SessionType {
    Legacy,  // Pre-app start: managed by mtime, CPU, log analysis
    Hook,    // Post-app start: managed by hook events
}

/// Session state aggregated from all events
#[derive(Debug, Clone, Serialize)]
pub struct SessionState {
    pub pid: u32,
    pub session_id: String,
    pub session_type: SessionType,
    pub last_log_event: Option<LogEvent>,
    pub last_cpu_event: Option<CpuEvent>,
    pub current_status: &'static str,
    pub has_terminal: bool,
    pub last_update: u64,
    pub last_active_timestamp: Option<u64>,  // For idle detection
}

impl SessionState {
    /// Create new Legacy session (pre-app start)
    pub fn new_legacy(pid: u32, session_id: String) -> Self {
        Self {
            pid,
            session_id,
            session_type: SessionType::Legacy,
            last_log_event: None,
            last_cpu_event: None,
            current_status: "unknown",
            has_terminal: true,
            last_update: current_timestamp(),
            last_active_timestamp: None,
        }
    }

    /// Create new Hook session (post-app start)
    pub fn new_hook(session_id: String) -> Self {
        Self {
            pid: 0,  // PID will be discovered later
            session_id,
            session_type: SessionType::Hook,
            last_log_event: None,
            last_cpu_event: None,
            current_status: "resting",
            has_terminal: true,
            last_update: current_timestamp(),
            last_active_timestamp: None,
        }
    }

    /// Upgrade Legacy session to Hook session (승격)
    /// Returns true if upgrade was successful, false otherwise
    pub fn upgrade_to_hook(&mut self) -> bool {
        if self.session_type == SessionType::Legacy {
            // 검증 1: UUID 형식의 세션 ID인지 확인 (36자)
            // 검증 2: 임시 세션(pid-XXXXX)이 아닌지 확인
            // 검증 3: 잘못된 세션($SESSION_ID)이 아닌지 확인
            if self.session_id.len() == 36 &&
               !self.session_id.starts_with("pid-") &&
               !self.session_id.starts_with("$") {

                println!("[SessionState] 🔼 Upgrading session {} from Legacy to Hook",
                    &self.session_id[..8]);
                self.session_type = SessionType::Hook;
                // Keep existing PID, status, and data
                return true;
            } else {
                println!("[SessionState] ⚠️ Cannot upgrade session '{}': not a valid UUID session (temporary or invalid)",
                    self.session_id);
                return false;
            }
        }

        // Already Hook session
        if self.session_type == SessionType::Hook {
            return true;
        }

        false
    }
}

/// Get current Unix timestamp in seconds
pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
