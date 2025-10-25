// Session Log Analyzer
//
// Analyzes Claude Code debug logs to determine working state

use crate::session::finder::get_claude_debug_dir;
use crate::types::WorkingState;
use std::fs;
use std::time::UNIX_EPOCH;

/// Analyze log content to determine working state
/// For legacy sessions, checks for "Stream started" or "compacting" patterns
/// The transition from Working → Resting is handled by mtime + CPU check in the caller
pub fn analyze_log_content(log_content: &str) -> WorkingState {
    let last_100_lines: Vec<&str> = log_content.lines().rev().take(100).collect();

    // Check for "Stream started - received first chunk" pattern
    // This indicates Claude is actively working on a response
    let has_stream_started = last_100_lines.iter().any(|line| {
        line.contains("Stream started - received first chunk")
    });

    // Check for "compacting" pattern - also indicates working
    // Database compacting is a working operation
    let has_compacting = last_100_lines.iter().any(|line| {
        line.to_lowercase().contains("compacting")
    });

    if has_stream_started || has_compacting {
        WorkingState::ActivelyWorking
    } else {
        // Default to Unknown - caller will determine Resting based on mtime/CPU
        WorkingState::Unknown
    }
}

/// Check session activity based on log file
/// Returns (WorkingState, log_modification_time)
pub fn check_session_activity(session_id: &str) -> (WorkingState, u64) {
    let debug_dir = match get_claude_debug_dir() {
        Some(dir) => dir,
        None => return (WorkingState::Unknown, u64::MAX),
    };

    let log_file = debug_dir.join(format!("{}.txt", session_id));

    // Get file modification time
    let mtime = if let Ok(metadata) = fs::metadata(&log_file) {
        if let Ok(modified) = metadata.modified() {
            modified.duration_since(UNIX_EPOCH).unwrap().as_secs()
        } else {
            return (WorkingState::Unknown, u64::MAX);
        }
    } else {
        return (WorkingState::Unknown, u64::MAX);
    };

    // Read and analyze log content
    let working_state = if let Ok(content) = fs::read_to_string(&log_file) {
        analyze_log_content(&content)
    } else {
        WorkingState::Unknown
    };

    (working_state, mtime)
}
