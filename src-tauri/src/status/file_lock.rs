// File Lock Checker
//
// Check if a log file is currently opened by a process using lsof

use std::process::Command;
use std::path::Path;

/// Check if a file is currently opened by any process
/// Returns true if file is being written to (working)
/// Returns false if file is closed (resting)
pub fn is_file_opened(file_path: &Path) -> bool {
    // Use lsof to check if file is opened
    let output = Command::new("lsof")
        .arg(file_path)
        .output();

    match output {
        Ok(result) => {
            // If lsof returns output, file is opened
            !result.stdout.is_empty()
        }
        Err(_) => {
            // If lsof fails, assume file is not opened
            false
        }
    }
}

/// Check if a file is opened by a specific PID
/// More precise check for session-to-PID mapping
pub fn is_file_opened_by_pid(file_path: &Path, pid: u32) -> bool {
    // Use lsof -p <pid> to check only specific process
    let output = Command::new("lsof")
        .arg("-p")
        .arg(pid.to_string())
        .arg(file_path)
        .output();

    match output {
        Ok(result) => {
            !result.stdout.is_empty()
        }
        Err(_) => {
            false
        }
    }
}

/// Get PID of process that has file opened (if any)
pub fn get_pid_with_file_opened(file_path: &Path) -> Option<u32> {
    let output = Command::new("lsof")
        .arg("-t")  // Output PIDs only
        .arg(file_path)
        .output();

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            // Parse first PID from output
            stdout.lines()
                .next()
                .and_then(|s| s.trim().parse::<u32>().ok())
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_not_opened() {
        let path = PathBuf::from("/tmp/nonexistent_file.txt");
        assert_eq!(is_file_opened(&path), false);
    }
}
