// Session ID Finder
//
// Locates Claude Code session IDs by searching debug log files

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Helper function to get Claude debug directory
pub fn get_claude_debug_dir() -> Option<PathBuf> {
    // Try HOME environment variable (Unix/Linux/macOS)
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".claude/debug"));
    }

    // Try USERPROFILE environment variable (Windows)
    if let Ok(home) = std::env::var("USERPROFILE") {
        return Some(PathBuf::from(home).join(".claude/debug"));
    }

    // If all else fails, return None (no hard-coded paths)
    None
}

/// Find session ID for a given PID by searching log files
pub fn find_session_id_for_pid(pid: u32, session_cache: &mut HashMap<u32, String>) -> Option<String> {
    use std::fs::OpenOptions;
    use std::io::Write as IoWrite;

    // Check cache first
    if let Some(session_id) = session_cache.get(&pid) {
        return Some(session_id.clone());
    }

    // Search for PID in debug log files
    // Claude logs contain patterns like ".tmp.{PID}." in file paths
    let debug_dir = match get_claude_debug_dir() {
        Some(dir) => dir,
        None => return None,
    };

    let entries = match fs::read_dir(&debug_dir) {
        Ok(e) => e,
        Err(_) => return None,
    };

    let search_pattern = format!(".tmp.{}.", pid);

    // Debug logging (best effort, ignore errors)
    if let Ok(mut debug_file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/claudeminer_session_debug.log")
    {
        let _ = writeln!(debug_file, "\n=== Searching session for PID {} ===", pid);
        let _ = writeln!(debug_file, "Search pattern: {}", search_pattern);
        let _ = writeln!(debug_file, "Debug dir: {:?}", debug_dir);
    }

    // Collect all matching files first, then pick the most recently modified
    let mut matching_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("txt") {
            // Use grep for faster search in large files
            #[cfg(target_os = "macos")]
            {
                use std::process::Command;

                let grep_result = Command::new("grep")
                    .arg("-l")  // Only output filename
                    .arg("-F")  // Fixed string (faster)
                    .arg(&search_pattern)
                    .arg(&path)
                    .output();

                if let Ok(output) = grep_result {
                    // Debug logging
                    if let Ok(mut debug_file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/claudeminer_session_debug.log")
                    {
                        let _ = writeln!(debug_file, "  Checking file: {:?}", path.file_name());
                        let _ = writeln!(debug_file, "  Grep exit code: {}", output.status.code().unwrap_or(-1));
                        let _ = writeln!(debug_file, "  Grep success: {}", output.status.success());
                    }

                    if output.status.success() {
                        // Found a matching file
                        if let Ok(mut debug_file) = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("/tmp/claudeminer_session_debug.log")
                        {
                            let _ = writeln!(debug_file, "  ✅ MATCH FOUND in {:?}", path.file_name());
                        }

                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                matching_files.push((path.clone(), modified));
                            }
                        }
                    }
                }
            }

            #[cfg(not(target_os = "macos"))]
            {
                // Fallback: read entire file
                if let Ok(content) = fs::read_to_string(&path) {
                    if content.contains(&search_pattern) {
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                matching_files.push((path.clone(), modified));
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by most recently modified and pick the first one
    matching_files.sort_by(|a, b| b.1.cmp(&a.1));

    if let Some((path, _)) = matching_files.first() {
        if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
            let session_id = file_name.to_string();
            session_cache.insert(pid, session_id.clone());
            return Some(session_id);
        }
    }

    None
}
