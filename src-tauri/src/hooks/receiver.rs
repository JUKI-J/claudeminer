// Hook Receiver Thread
//
// Improved named pipe communication with automatic reconnection,
// error recovery, and comprehensive monitoring
//

use crate::session::{MonitorEvent, HookEvent};
use crate::notification;
use std::sync::mpsc::Sender;
use std::thread;
use std::fs::{self, OpenOptions};
use std::io::{BufReader, BufRead};
use std::path::Path;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

const PIPE_PATH: &str = "/tmp/claudeminer_pipe";
const RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const PIPE_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Hook event with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEventWithTimestamp {
    pub sid: String,      // session_id
    pub evt: String,      // start|working|resting|end
    #[serde(default = "default_timestamp")]
    pub timestamp: u64,   // Unix timestamp
}

fn default_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl From<HookEventWithTimestamp> for HookEvent {
    fn from(evt_with_ts: HookEventWithTimestamp) -> Self {
        HookEvent {
            sid: evt_with_ts.sid,
            evt: evt_with_ts.evt,
        }
    }
}

/// Hook receiver statistics
#[derive(Debug)]
struct ReceiverStats {
    events_received: u64,
    parse_errors: u64,
    read_errors: u64,
    reconnects: u64,
    last_event_time: Option<Instant>,
    start_time: Instant,
}

impl ReceiverStats {
    fn new() -> Self {
        Self {
            events_received: 0,
            parse_errors: 0,
            read_errors: 0,
            reconnects: 0,
            last_event_time: None,
            start_time: Instant::now(),
        }
    }

    fn log_summary(&self) {
        let uptime = self.start_time.elapsed().as_secs();
        let hours = uptime / 3600;
        let minutes = (uptime % 3600) / 60;

        println!("[HookReceiver] === Statistics ===");
        println!("  Uptime: {}h {}m", hours, minutes);
        println!("  Events received: {}", self.events_received);
        println!("  Parse errors: {}", self.parse_errors);
        println!("  Read errors: {}", self.read_errors);
        println!("  Reconnections: {}", self.reconnects);

        if let Some(last_time) = self.last_event_time {
            let idle_time = last_time.elapsed().as_secs();
            println!("  Last event: {}s ago", idle_time);
        }
        println!("==================");
    }
}

/// Configuration for the hook receiver
pub struct ReceiverConfig {
    pub pipe_path: String,
    pub reconnect_delay: Duration,
    pub max_reconnects: u32,
    pub enable_stats: bool,
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self {
            pipe_path: PIPE_PATH.to_string(),
            reconnect_delay: RECONNECT_DELAY,
            max_reconnects: MAX_RECONNECT_ATTEMPTS,
            enable_stats: true,
        }
    }
}

/// Start hook receiver thread
pub fn start_hook_receiver(event_sender: Sender<MonitorEvent>) -> thread::JoinHandle<()> {
    start_hook_receiver_with_config(event_sender, ReceiverConfig::default())
}

/// Start hook receiver with custom configuration
pub fn start_hook_receiver_with_config(
    event_sender: Sender<MonitorEvent>,
    config: ReceiverConfig,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        println!("[HookReceiver] Starting hook receiver");
        let mut stats = ReceiverStats::new();
        let mut last_stats_log = Instant::now();

        loop {
            // Log statistics periodically
            if config.enable_stats && last_stats_log.elapsed() > Duration::from_secs(300) {
                stats.log_summary();
                last_stats_log = Instant::now();
            }

            match run_receiver_with_recovery(&event_sender, &config, &mut stats) {
                Ok(_) => {
                    println!("[HookReceiver] Receiver completed normally");
                    break;
                }
                Err(e) => {
                    eprintln!("[HookReceiver] Receiver error: {}", e);
                    stats.reconnects += 1;

                    // Exponential backoff
                    let delay = config.reconnect_delay * stats.reconnects.min(5) as u32;
                    thread::sleep(delay);
                }
            }
        }

        // Final statistics
        if config.enable_stats {
            stats.log_summary();
        }
    })
}

/// Run receiver with automatic recovery
fn run_receiver_with_recovery(
    event_sender: &Sender<MonitorEvent>,
    config: &ReceiverConfig,
    stats: &mut ReceiverStats,
) -> std::io::Result<()> {
    let mut consecutive_failures = 0;

    loop {
        // Ensure pipe exists and is healthy
        ensure_pipe_healthy(&config.pipe_path)?;

        match run_receiver_session(event_sender, config, stats) {
            Ok(_) => {
                let _ = consecutive_failures; // Suppress warning
                return Ok(());
            }
            Err(e) => {
                consecutive_failures += 1;

                if consecutive_failures >= config.max_reconnects {
                    println!("[HookReceiver] Max failures reached, recreating pipe...");
                    recreate_pipe(&config.pipe_path)?;
                    consecutive_failures = 0;
                }

                eprintln!("[HookReceiver] Session failed (attempt {}/{}): {}",
                    consecutive_failures, config.max_reconnects, e);

                thread::sleep(config.reconnect_delay * consecutive_failures);
            }
        }
    }
}

/// Run a single receiver session
fn run_receiver_session(
    event_sender: &Sender<MonitorEvent>,
    config: &ReceiverConfig,
    stats: &mut ReceiverStats,
) -> std::io::Result<()> {
    println!("[HookReceiver] Opening pipe: {}", config.pipe_path);

    // Open pipe with non-blocking read
    let file = open_pipe_robust(&config.pipe_path)?;
    let reader = BufReader::new(file);
    let mut last_activity = Instant::now();
    let mut buffer = String::new();

    println!("[HookReceiver] Pipe opened successfully, listening for events...");

    for line_result in reader.lines() {
        // Check for read timeout
        if last_activity.elapsed() > READ_TIMEOUT {
            println!("[HookReceiver] Read timeout, reconnecting...");
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "No data received within timeout period"
            ));
        }

        match line_result {
            Ok(line) => {
                last_activity = Instant::now();

                if line.trim().is_empty() {
                    continue;
                }

                // Handle potential multi-line JSON
                buffer.push_str(&line);

                // Try to parse JSON (check for killed event first)
                if let Ok(event) = serde_json::from_str::<serde_json::Value>(&buffer) {
                    // Check if this is a "killed" event
                    if let Some(evt) = event.get("evt").and_then(|v| v.as_str()) {
                        if evt == "killed" {
                            // Extract PID from sid (format: "PID-{pid}")
                            if let Some(sid) = event.get("sid").and_then(|v| v.as_str()) {
                                if sid.starts_with("PID-") {
                                    if let Some(pid_str) = sid.strip_prefix("PID-") {
                                        if let Ok(pid) = pid_str.parse::<u32>() {
                                            println!("[HookReceiver] 💀 Received process killed event for PID {}", pid);

                                            // Send notification via notification module
                                            notification::send_zombie_killed_notification(pid);

                                            buffer.clear();
                                            stats.events_received += 1;
                                            stats.last_event_time = Some(Instant::now());
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Try to parse as regular HookEvent
                match serde_json::from_str::<HookEventWithTimestamp>(&buffer) {
                    Ok(event_with_ts) => {
                        buffer.clear();
                        stats.events_received += 1;
                        stats.last_event_time = Some(Instant::now());

                        // Convert to standard HookEvent
                        let hook_event = HookEvent::from(event_with_ts.clone());

                        // Filter out invalid session IDs (like $SESSION_ID)
                        if hook_event.sid == "$SESSION_ID" || hook_event.sid.is_empty() {
                            println!("[HookReceiver] Ignoring event with invalid session ID: '{}'", hook_event.sid);
                            continue;
                        }

                        println!("[HookReceiver] Event #{}: session={}, type={}, time={}",
                            stats.events_received,
                            &hook_event.sid[..8.min(hook_event.sid.len())],
                            hook_event.evt,
                            event_with_ts.timestamp
                        );

                        // Send to coordinator
                        if event_sender.send(MonitorEvent::Hook(hook_event)).is_err() {
                            println!("[HookReceiver] Coordinator channel closed");
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        // Check if it might be incomplete JSON
                        if buffer.contains('{') && !buffer.contains('}') {
                            // Wait for more data
                            continue;
                        } else {
                            // Invalid JSON, log and clear buffer
                            stats.parse_errors += 1;
                            eprintln!("[HookReceiver] Parse error #{}: {} - Data: {}",
                                stats.parse_errors, e, buffer);
                            buffer.clear();
                        }
                    }
                }
            }
            Err(e) => {
                stats.read_errors += 1;
                eprintln!("[HookReceiver] Read error #{}: {}", stats.read_errors, e);

                // Check if pipe is broken
                if is_broken_pipe_error(&e) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "Pipe connection broken"
                    ));
                }
            }
        }
    }

    println!("[HookReceiver] Pipe closed by writer");
    Err(std::io::Error::new(
        std::io::ErrorKind::UnexpectedEof,
        "Pipe closed"
    ))
}

/// Open pipe with robust error handling
fn open_pipe_robust(path: &str) -> std::io::Result<fs::File> {
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 10;

    loop {
        attempts += 1;

        match OpenOptions::new()
            .read(true)
            .open(path)
        {
            Ok(file) => return Ok(file),
            Err(e) if attempts < MAX_ATTEMPTS => {
                eprintln!("[HookReceiver] Open attempt {}/{} failed: {}",
                    attempts, MAX_ATTEMPTS, e);
                thread::sleep(Duration::from_millis(100 * attempts as u64));
            }
            Err(e) => {
                return Err(std::io::Error::new(
                    e.kind(),
                    format!("Failed to open pipe after {} attempts: {}", MAX_ATTEMPTS, e)
                ));
            }
        }
    }
}

/// Ensure pipe exists and is healthy
fn ensure_pipe_healthy(path: &str) -> std::io::Result<()> {
    let pipe_path = Path::new(path);

    if pipe_path.exists() {
        let metadata = fs::metadata(pipe_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            if !metadata.file_type().is_fifo() {
                println!("[HookReceiver] Path exists but is not a FIFO, recreating...");
                fs::remove_file(pipe_path)?;
                create_named_pipe(path)?;
            } else {
                // Check if pipe is accessible
                match OpenOptions::new().read(true).open(pipe_path) {
                    Ok(_) => {
                        // Pipe is accessible
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("[HookReceiver] Pipe exists but not accessible: {}", e);
                        recreate_pipe(path)?;
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems, just check if file exists
            return Ok(());
        }
    } else {
        println!("[HookReceiver] Creating new pipe: {}", path);
        create_named_pipe(path)?;
    }

    Ok(())
}

/// Recreate the named pipe
fn recreate_pipe(path: &str) -> std::io::Result<()> {
    let pipe_path = Path::new(path);

    if pipe_path.exists() {
        println!("[HookReceiver] Removing old pipe...");
        fs::remove_file(pipe_path)?;
        thread::sleep(Duration::from_millis(100));
    }

    println!("[HookReceiver] Creating fresh pipe...");
    create_named_pipe(path)?;

    Ok(())
}

/// Check if error is a broken pipe
fn is_broken_pipe_error(e: &std::io::Error) -> bool {
    matches!(e.kind(),
        std::io::ErrorKind::BrokenPipe |
        std::io::ErrorKind::UnexpectedEof |
        std::io::ErrorKind::ConnectionAborted
    )
}

#[cfg(target_os = "macos")]
fn create_named_pipe(path: &str) -> std::io::Result<()> {
    use std::process::Command;

    let output = Command::new("mkfifo")
        .arg("-m")
        .arg("622")  // rw--w--w-
        .arg(path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("File exists") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("mkfifo failed: {}", stderr)
            ));
        }
    }

    println!("[HookReceiver] Named pipe created: {}", path);
    Ok(())
}

#[cfg(target_os = "linux")]
fn create_named_pipe(path: &str) -> std::io::Result<()> {
    use nix::sys::stat;
    use nix::unistd;

    match unistd::mkfifo(
        path,
        stat::Mode::S_IRUSR | stat::Mode::S_IWUSR | stat::Mode::S_IWGRP | stat::Mode::S_IWOTH
    ) {
        Ok(_) => {
            println!("[HookReceiver] Named pipe created: {}", path);
            Ok(())
        }
        Err(nix::errno::Errno::EEXIST) => {
            println!("[HookReceiver] Named pipe already exists: {}", path);
            Ok(())
        }
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("mkfifo failed: {}", e)
        ))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn create_named_pipe(_path: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Named pipes not supported on this platform"
    ))
}