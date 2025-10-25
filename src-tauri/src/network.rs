// Network Connection Monitoring
//
// This module provides functionality to check active network connections
// for Claude Code processes to detect API communication.

use std::collections::HashMap;

/// Count active ESTABLISHED connections to Anthropic API (:443)
pub fn count_network_connections(pid: u32) -> usize {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = match Command::new("lsof")
            .args(["-i", "-n", "-P"])
            .output() {
                Ok(o) => o,
                Err(_) => return 0,
            };

        let output_str = String::from_utf8_lossy(&output.stdout);
        let pid_str = pid.to_string();

        // Count ESTABLISHED connections for this specific PID
        output_str.lines()
            .filter(|line| {
                line.contains("node") &&
                line.contains(&pid_str) &&
                line.contains("ESTABLISHED") &&
                line.contains(":443")  // HTTPS connections (Anthropic API uses 443)
            })
            .count()
    }

    #[cfg(not(target_os = "macos"))]
    {
        0  // Not implemented for other platforms
    }
}

/// Apply network debouncing - need 5+ connections (filter keep-alive)
pub fn is_network_active(
    pid: u32,
    connection_count: usize,
    network_debouncer: &mut HashMap<u32, u8>
) -> bool {
    const MIN_CONNECTIONS: usize = 5;  // At least 5 ESTABLISHED connections

    // Immediate detection when connections >= 5
    if connection_count >= MIN_CONNECTIONS {
        network_debouncer.insert(pid, 1);
        true
    } else {
        // Reset counter when connections drop
        network_debouncer.insert(pid, 0);
        false
    }
}
