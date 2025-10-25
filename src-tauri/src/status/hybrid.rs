// Hybrid Status Detection Module
//
// Combines multiple detection methods for real-time status monitoring:
// 1. TTY check (zombie detection)
// 2. Log file activity monitoring (real-time changes)
// 3. CPU sampling (validation)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use std::fs;

/// Log activity tracker
/// Maps session_id -> last_modified_timestamp
pub type LogActivityTracker = Arc<Mutex<HashMap<String, u64>>>;

/// Check if process has a terminal (zombie detection via TTY and STAT)
/// Returns true if process is zombie (no terminal OR stopped process)
pub fn is_zombie_by_tty(pid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "tty=,stat="])
            .output();

        if let Ok(output) = output {
            let line = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.len() >= 2 {
                let tty = parts[0];
                let stat = parts[1];

                // Zombie conditions:
                // 1. TTY is "??" or "?" (no controlling terminal)
                // 2. STAT starts with 'T' (stopped process - unusable session)
                let is_zombie = tty.is_empty() || tty == "??" || tty == "?" || stat.starts_with('T');

                if is_zombie {
                    if stat.starts_with('T') {
                        println!("[is_zombie_by_tty] PID {} is zombie (STAT='{}' - Stopped)", pid, stat);
                    } else {
                        println!("[is_zombie_by_tty] PID {} is zombie (TTY='{}')", pid, tty);
                    }
                }

                return is_zombie;
            }

            false
        } else {
            false
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Monitor log file changes for real-time activity detection
/// Returns true if log was modified AND contains meaningful work activity
pub fn is_log_recently_active(
    session_id: &str,
    debug_dir: &PathBuf,
    log_tracker: &LogActivityTracker,
    threshold_secs: u64,
) -> bool {
    let log_file = debug_dir.join(format!("{}.txt", session_id));

    // Get current modification time
    let current_mtime = if let Ok(metadata) = fs::metadata(&log_file) {
        if let Ok(modified) = metadata.modified() {
            modified.duration_since(UNIX_EPOCH).unwrap().as_secs()
        } else {
            return false;
        }
    } else {
        return false;
    };

    // Update tracker
    let mut tracker = log_tracker.lock().unwrap();
    let _last_known_mtime = tracker.get(session_id).copied().unwrap_or(0);
    tracker.insert(session_id.to_string(), current_mtime);
    drop(tracker);

    // Check if file was recently modified
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let age = now.saturating_sub(current_mtime);

    // If file is old, definitely not active
    if age > threshold_secs {
        return false;
    }

    // File is recent, but check CONTENT to avoid false positives
    // Background hooks update the log even when idle
    if let Ok(content) = fs::read_to_string(&log_file) {
        let last_30_lines: Vec<&str> = content.lines().rev().take(30).collect();

        // Check for meaningful work activities
        let has_meaningful_work = last_30_lines.iter().any(|line| {
            line.contains("Stream started") ||
            line.contains("executePreToolHooks") ||
            line.contains("Tool execution") ||
            line.contains("Writing to temp file") ||
            line.contains("FileHistory: Making snapshot")
        });

        // Check if ONLY background hooks (idle state)
        let only_background = last_30_lines.iter().take(10).all(|line| {
            line.contains("Hooks: checkForNewResponses") ||
            line.contains("Hooks: getAsyncHookResponseAttachments") ||
            line.contains("Hooks: Found 0 total hooks") ||
            line.contains("Skills and commands") ||
            line.is_empty()
        });

        // Return true only if:
        // 1. File modified recently AND
        // 2. Has meaningful work (not just background hooks)
        has_meaningful_work && !only_background
    } else {
        // Can't read file, assume active if recently modified
        true
    }
}

/// Hybrid status determination
/// Priority:
/// 1. TTY check (zombie detection) - HIGHEST
/// 2. Log activity (real-time) - FAST
/// 3. CPU sampling (validation) - ACCURATE
pub fn determine_hybrid_status(
    pid: u32,
    session_id: Option<&str>,
    cpu: f32,
    debug_dir: &PathBuf,
    log_tracker: &LogActivityTracker,
) -> &'static str {
    // Priority 1: TTY check for zombie
    if is_zombie_by_tty(pid) {
        return "zombie";
    }

    // Priority 2: No session = orphaned process (zombie)
    let sid = match session_id {
        Some(s) => s,
        None => return "zombie",
    };

    // Priority 3: Log activity check (real-time response)
    // Check if log was modified in last 5 seconds
    if is_log_recently_active(sid, debug_dir, log_tracker, 5) {
        // Log is actively being written -> likely working
        // IMPORTANT: Don't require high CPU here!
        // AI might be waiting for API response (CPU=0 but still working)
        return "working";
    }

    // Priority 4: CPU validation (fallback)
    // If CPU is high, definitely working
    if cpu > 5.0 {
        return "working";
    }

    // Priority 5: Check if log is stale (zombie indicator)
    // If log hasn't been modified in 30 minutes, consider zombie
    let log_file = debug_dir.join(format!("{}.txt", sid));
    if let Ok(metadata) = fs::metadata(&log_file) {
        if let Ok(modified) = metadata.modified() {
            let mtime = modified.duration_since(UNIX_EPOCH).unwrap().as_secs();
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let age = now.saturating_sub(mtime);

            if age > 1800 && cpu < 0.5 {
                // Log stale for 30+ minutes AND low CPU = zombie
                return "zombie";
            }
        }
    }

    // Default: resting (waiting for input)
    "resting"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zombie_by_tty() {
        // This test requires actual PIDs, so it's mainly for documentation
        // In real usage: zombie PIDs should return true
        println!("TTY-based zombie detection test (requires manual verification)");
    }

    #[test]
    fn test_log_activity_tracking() {
        let tracker = Arc::new(Mutex::new(HashMap::new()));
        let debug_dir = PathBuf::from("/tmp");

        // First check should return false (no previous data)
        let result = is_log_recently_active("test-session", &debug_dir, &tracker, 5);
        println!("First check result: {}", result);
    }
}
