// CPU Monitor Thread
//
// Monitors Claude process CPU usage with adaptive polling

use crate::session::{MonitorEvent, CpuEvent, current_timestamp};
use sysinfo::{System, ProcessRefreshKind};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

/// Start CPU monitor thread
pub fn start_cpu_monitor(
    event_sender: Sender<MonitorEvent>,
    claude_pids: Arc<Mutex<HashSet<u32>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        run_cpu_monitor(event_sender, claude_pids);
    })
}

fn run_cpu_monitor(
    event_sender: Sender<MonitorEvent>,
    claude_pids: Arc<Mutex<HashSet<u32>>>,
) {
    let mut sys = System::new();
    let mut last_cpu: HashMap<u32, f32> = HashMap::new();
    let mut last_zombie_check: HashMap<u32, bool> = HashMap::new(); // Track zombie status

    println!("[CpuMonitor] Started");

    let mut scan_count = 0;
    loop {
        scan_count += 1;

        // Find Claude PIDs using ps command (returns PID -> (is_zombie))
        let current_pids_info = find_claude_pids_via_ps();
        let current_pids: HashSet<u32> = current_pids_info.keys().copied().collect();

        if !current_pids.is_empty() {
            // Refresh processes for CPU measurement
            sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());
            thread::sleep(Duration::from_millis(200));
            sys.refresh_processes_specifics(ProcessRefreshKind::new().with_cpu());
        }

        let mut claude_found = 0;
        for &pid_u32 in &current_pids {
            claude_found += 1;
            let is_zombie = current_pids_info.get(&pid_u32).copied().unwrap_or(false);

            if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid_u32)) {
                let cpu = process.cpu_usage();

                // Check if zombie status changed
                let zombie_changed = last_zombie_check.get(&pid_u32).copied().unwrap_or(false) != is_zombie;
                if zombie_changed {
                    last_zombie_check.insert(pid_u32, is_zombie);
                    if is_zombie {
                        println!("[CpuMonitor] ⚠️  PID {} became ZOMBIE (TTY='??') - sending immediate event", pid_u32);
                    } else {
                        println!("[CpuMonitor] ✅ PID {} recovered from zombie - sending immediate event", pid_u32);
                    }
                    // Force send event for zombie status change
                    let event = CpuEvent {
                        pid: pid_u32,
                        timestamp: current_timestamp(),
                        cpu_percent: cpu,
                    };
                    if event_sender.send(MonitorEvent::Cpu(event)).is_err() {
                        println!("[CpuMonitor] Channel disconnected, shutting down");
                        return;
                    }
                    continue; // Skip normal CPU change check
                }

                // Check if this is a new PID (not in last_cpu)
                let is_new_pid = !last_cpu.contains_key(&pid_u32);

                // Send event if CPU changed significantly OR if it's a new PID
                if is_new_pid || cpu_changed_significantly(pid_u32, cpu, &mut last_cpu) {
                    if is_new_pid {
                        println!("[CpuMonitor] New PID discovered: pid={}, cpu={:.1}%", pid_u32, cpu);
                    } else {
                        println!("[CpuMonitor] CPU change detected: pid={}, cpu={:.1}%", pid_u32, cpu);
                    }

                    let event = CpuEvent {
                        pid: pid_u32,
                        timestamp: current_timestamp(),
                        cpu_percent: cpu,
                    };

                    if event_sender.send(MonitorEvent::Cpu(event)).is_err() {
                        println!("[CpuMonitor] Channel disconnected, shutting down");
                        return;
                    }
                }
            }
        }

        // Update shared Claude PIDs set for network monitor
        {
            let mut pids = claude_pids.lock().unwrap();
            *pids = current_pids.clone();
        }

        // Log every 10 scans
        if scan_count % 10 == 0 {
            println!("[CpuMonitor] Scan #{}: claude_found={}, tracked_pids={:?}",
                scan_count, claude_found, current_pids);
        }

        // Adaptive polling interval
        let interval = adaptive_interval(&last_cpu);
        thread::sleep(interval);
    }
}

/// Find Claude PIDs using ps command (macOS-specific)
/// Returns map of PID -> is_zombie
#[cfg(target_os = "macos")]
fn find_claude_pids_via_ps() -> HashMap<u32, bool> {
    use std::process::Command;
    let mut pids_info = HashMap::new();

    // Use ps with specific fields and pipe to grep
    // Format: PID %CPU TTY STAT COMMAND
    let output = Command::new("sh")
        .arg("-c")
        .arg("ps -eo pid,%cpu,tty,stat,command | grep -E '\\bclaude\\b' | grep -v 'claude-miner'")
        .output();

    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                // parts[0] = PID, parts[1] = CPU%, parts[2] = TTY, parts[3] = STAT, parts[4..] = command
                if let Ok(pid) = parts[0].parse::<u32>() {
                    let cpu = parts[1];
                    let tty = parts[2];
                    let stat = parts[3];

                    println!("[CpuMonitor] Found: PID={}, CPU={}%, TTY={}, STAT={}", pid, cpu, tty, stat);

                    // Check if it's a zombie:
                    // 1. TTY = "??" or "?" (no controlling terminal)
                    // 2. STAT starts with 'T' (stopped process - unusable session)
                    let is_zombie = tty == "??" || tty == "?" || stat.starts_with('T');
                    if is_zombie {
                        if tty == "??" || tty == "?" {
                            println!("[CpuMonitor]   → Zombie process detected (TTY='{}')", tty);
                        } else if stat.starts_with('T') {
                            println!("[CpuMonitor]   → Zombie process detected (STAT='{}' - Stopped)", stat);
                        }
                    }

                    pids_info.insert(pid, is_zombie);
                }
            }
        }

        if pids_info.is_empty() {
            println!("[CpuMonitor] No Claude processes found");
        } else {
            println!("[CpuMonitor] Found {} Claude processes: {:?}", pids_info.len(), pids_info.keys());
        }
    } else {
        println!("[CpuMonitor] Failed to execute ps command");
    }

    pids_info
}

/// Fallback for non-macOS systems (not implemented yet)
#[cfg(not(target_os = "macos"))]
fn find_claude_pids_via_ps() -> HashMap<u32, bool> {
    HashMap::new()
}

fn cpu_changed_significantly(pid: u32, new_cpu: f32, last_cpu: &mut HashMap<u32, f32>) -> bool {
    let prev = last_cpu.get(&pid).copied().unwrap_or(0.0);

    // Threshold: 3% change or crossing important boundaries
    let threshold = 3.0;
    let changed = (new_cpu - prev).abs() > threshold ||
                  (prev < 5.0 && new_cpu >= 5.0) ||  // Crossed working threshold
                  (prev >= 5.0 && new_cpu < 5.0);    // Dropped below working

    if changed {
        last_cpu.insert(pid, new_cpu);
        true
    } else {
        false
    }
}

fn adaptive_interval(last_cpu: &HashMap<u32, f32>) -> Duration {
    // If any process has high CPU, poll faster (but not too fast to save resources)
    let max_cpu = last_cpu.values().copied().fold(0.0f32, f32::max);

    if max_cpu > 20.0 {
        Duration::from_millis(500)  // High activity: 0.5s (reduced from 0.3s)
    } else if max_cpu > 5.0 {
        Duration::from_secs(1)      // Medium activity: 1s (increased from 0.5s)
    } else {
        Duration::from_secs(2)      // Low activity: 2s (increased from 1s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_change_detection() {
        let mut last_cpu = HashMap::new();

        // First measurement
        assert!(cpu_changed_significantly(1234, 10.0, &mut last_cpu));

        // Small change (< 3%)
        assert!(!cpu_changed_significantly(1234, 11.5, &mut last_cpu));

        // Large change (> 3%)
        assert!(cpu_changed_significantly(1234, 15.0, &mut last_cpu));
    }

    #[test]
    fn test_adaptive_interval() {
        let mut last_cpu = HashMap::new();

        // Low CPU
        last_cpu.insert(1, 2.0);
        assert_eq!(adaptive_interval(&last_cpu), Duration::from_secs(1));

        // Medium CPU
        last_cpu.insert(1, 10.0);
        assert_eq!(adaptive_interval(&last_cpu), Duration::from_millis(500));

        // High CPU
        last_cpu.insert(1, 25.0);
        assert_eq!(adaptive_interval(&last_cpu), Duration::from_millis(300));
    }
}
