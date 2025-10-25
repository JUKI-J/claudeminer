// Status Debouncer
//
// Prevents flickering status changes by requiring multiple consecutive checks
// before actually changing the status display.
//
// Example:
// - Working → Resting: Requires 5 consecutive "resting" checks (5 seconds)
// - Resting → Working: Requires 3 consecutive "working" checks (3 seconds)

use std::collections::HashMap;

#[cfg(debug_assertions)]
use std::fs::OpenOptions;
#[cfg(debug_assertions)]
use std::io::Write;

/// Apply debouncing to status changes
///
/// # Arguments
/// * `pid` - Process ID
/// * `raw_status` - Current detected status
/// * `debouncer` - Debouncer state (status, count)
/// * `skip_debounce` - If true, immediately apply status change (for fresh hook events)
///
/// # Returns
/// Debounced status string
pub fn apply_debouncing(
    pid: u32,
    raw_status: &str,
    debouncer: &mut HashMap<u32, (String, u8)>,
    skip_debounce: bool,
) -> String {
    // Debug logging (only in debug builds)
    #[cfg(debug_assertions)]
    if let Ok(mut debug_file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/claudeminer_debug.log")
    {
        let _ = writeln!(debug_file, "  Debouncing: PID={}, raw={}, skip={}", pid, raw_status, skip_debounce);
    }

    // If hook event detected, skip debouncing for immediate response
    if skip_debounce {
        debouncer.insert(pid, (raw_status.to_string(), 0));
        #[cfg(debug_assertions)]
        if let Ok(mut debug_file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/claudeminer_debug.log")
        {
            let _ = writeln!(debug_file, "  Debouncing: Skipped, returning: {}", raw_status);
        }
        return raw_status.to_string();
    }

    const WORKING_THRESHOLD: u8 = 3;  // 3 consecutive checks
    const RESTING_THRESHOLD: u8 = 5;  // 5 consecutive checks

    let (current_status, count) = debouncer.get(&pid)
        .cloned()
        .unwrap_or(("resting".to_string(), 0));

    if raw_status == current_status {
        // Same status, reset counter
        debouncer.insert(pid, (current_status.clone(), 0));
        #[cfg(debug_assertions)]
        if let Ok(mut debug_file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/claudeminer_debug.log")
        {
            let _ = writeln!(debug_file, "  Debouncing: Same status, returning: {}", current_status);
        }
        return current_status;
    }

    // Different status, increment counter
    let new_count = count + 1;

    let threshold = if raw_status == "working" {
        WORKING_THRESHOLD
    } else {
        RESTING_THRESHOLD
    };

    if new_count >= threshold {
        // Threshold reached, change status
        debouncer.insert(pid, (raw_status.to_string(), 0));
        #[cfg(debug_assertions)]
        if let Ok(mut debug_file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/claudeminer_debug.log")
        {
            let _ = writeln!(debug_file, "  Debouncing: Threshold reached ({}/{}), changing: {} -> {}",
                new_count, threshold, current_status, raw_status);
        }
        raw_status.to_string()
    } else {
        // Keep current status, increment counter
        debouncer.insert(pid, (current_status.clone(), new_count));
        #[cfg(debug_assertions)]
        if let Ok(mut debug_file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/claudeminer_debug.log")
        {
            let _ = writeln!(debug_file, "  Debouncing: Counter {}/{}, keeping: {}",
                new_count, threshold, current_status);
        }
        current_status
    }
}
