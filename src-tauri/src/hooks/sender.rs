// Named Pipe Sender - Send events to ClaudeMiner
//
// Sends process termination events to the named pipe for ClaudeMiner to pick up

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const PIPE_PATH: &str = "/tmp/claudeminer_pipe";

/// Send a process killed event to the named pipe
pub fn send_process_killed_event(pid: u32) -> Result<(), String> {
    send_named_pipe_message(&format!("PROCESS_KILLED:{}", pid))
}

/// Send a raw message to the named pipe
fn send_named_pipe_message(message: &str) -> Result<(), String> {
    // Check if pipe exists
    if !Path::new(PIPE_PATH).exists() {
        return Err(format!("Named pipe does not exist: {}", PIPE_PATH));
    }

    // Open pipe for writing (non-blocking)
    match OpenOptions::new()
        .write(true)
        .open(PIPE_PATH)
    {
        Ok(mut pipe) => {
            // Write message
            match writeln!(pipe, "{}", message) {
                Ok(_) => {
                    println!("[PipeSender] Sent message: {}", message);
                    Ok(())
                }
                Err(e) => {
                    Err(format!("Failed to write to pipe: {}", e))
                }
            }
        }
        Err(e) => {
            Err(format!("Failed to open pipe: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_process_killed_event() {
        // This test requires the named pipe to exist
        // In real usage, the receiver should be running
        let result = send_process_killed_event(12345);
        println!("Send result: {:?}", result);
    }
}
