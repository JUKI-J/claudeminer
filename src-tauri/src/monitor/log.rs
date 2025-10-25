// Log Watcher Thread
//
// Monitors ~/.claude/debug directory for log file changes using notify (inotify/FSEvents)

use crate::session::{MonitorEvent, LogEvent, current_timestamp};
use crate::session::analyzer::analyze_log_content;
use notify::{Watcher, RecursiveMode, Event, EventKind, event::ModifyKind};
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;

/// Start log watcher thread
pub fn start_log_watcher(event_sender: Sender<MonitorEvent>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Err(e) = run_log_watcher(event_sender) {
            eprintln!("[LogWatcher] Error: {}", e);
        }
    })
}

fn run_log_watcher(event_sender: Sender<MonitorEvent>) -> notify::Result<()> {
    // Get debug directory
    let debug_dir = get_debug_dir();

    println!("[LogWatcher] Watching: {}", debug_dir.display());

    // Create notify channel
    let (tx, rx) = channel();

    // Create recommended watcher
    let mut watcher = notify::recommended_watcher(tx)?;

    // Watch debug directory
    watcher.watch(&debug_dir, RecursiveMode::NonRecursive)?;

    // Debouncing: Track last processed time for each file (session_id -> timestamp)
    let mut last_processed: HashMap<String, u64> = HashMap::new();
    const DEBOUNCE_MS: u64 = 200; // Minimum 200ms between processing same file

    // Event loop
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(Event { kind: EventKind::Modify(ModifyKind::Data(_)), paths, .. })) => {
                let now = current_timestamp();

                // Only process data modification events
                for path in paths {
                    println!("[LogWatcher] File modified: {}", path.display());

                    if let Some(session_id) = extract_session_id(&path) {
                        println!("[LogWatcher] Extracted session_id: {} from path: {}",
                            session_id, path.display());

                        // Check debouncing
                        let last_time = last_processed.get(&session_id).copied().unwrap_or(0);
                        let elapsed_ms = (now - last_time) * 1000; // Convert to milliseconds

                        if elapsed_ms < DEBOUNCE_MS {
                            println!("[LogWatcher] Skipping session {} (debounced: {}ms < {}ms)",
                                &session_id[..8], elapsed_ms, DEBOUNCE_MS);
                            continue;
                        }

                        println!("[LogWatcher] Analyzing log file: {}", path.display());

                        if let Ok(log_event) = analyze_log_file(&path, &session_id) {
                            println!("[LogWatcher] Processing session {}: state={:?}, approval_pending={}",
                                &session_id[..8], log_event.state, log_event.has_approval_pending);

                            // Update last processed time
                            last_processed.insert(session_id.clone(), now);

                            // Send event to coordinator
                            if event_sender.send(MonitorEvent::Log(log_event)).is_err() {
                                println!("[LogWatcher] Failed to send event! Coordinator channel disconnected?");
                                break;
                            } else {
                                println!("[LogWatcher] Event sent successfully for session {}", &session_id[..8]);
                            }
                        } else {
                            println!("[LogWatcher] Failed to analyze log file: {}", path.display());
                        }
                    } else {
                        println!("[LogWatcher] Failed to extract session_id from path: {}", path.display());
                    }
                }
            }
            Ok(Ok(_)) => {}, // Ignore other events
            Ok(Err(e)) => {
                eprintln!("[LogWatcher] Watch error: {}", e);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Normal timeout, continue
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                println!("[LogWatcher] Channel disconnected, shutting down");
                break;
            }
        }
    }

    Ok(())
}

fn get_debug_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude/debug")
}

fn extract_session_id(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| s.len() == 36) // UUID length
        .map(|s| s.to_string())
}

fn analyze_log_file(path: &Path, session_id: &str) -> Result<LogEvent, std::io::Error> {
    // Get file metadata for mtime
    let metadata = fs::metadata(path)?;
    let file_mtime = metadata.modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Read only last 50 lines for efficiency
    let content = fs::read_to_string(path)?;
    let last_lines: String = content
        .lines()
        .rev()
        .take(50)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    // Analyze content
    let state = analyze_log_content(&last_lines);

    // Detect approval pending pattern
    let has_approval_pending =
        last_lines.contains("executePreToolHooks") &&
        last_lines.contains("Notification") &&
        !last_lines.contains("Tool execution");

    Ok(LogEvent {
        session_id: session_id.to_string(),
        pid: None, // Will be resolved by coordinator
        timestamp: current_timestamp(),
        state,
        has_approval_pending,
        file_mtime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_id() {
        let path = Path::new("/home/.claude/debug/286e962f-c045-4274-8f37-c4e41fb6104a.txt");
        assert_eq!(
            extract_session_id(path),
            Some("286e962f-c045-4274-8f37-c4e41fb6104a".to_string())
        );
    }
}
