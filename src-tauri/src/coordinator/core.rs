// Coordinator Thread
//
// Aggregates events from all monitors and makes status decisions

use crate::session::{MonitorEvent, SessionState, current_timestamp, CleanupEvent};
use crate::session::finder::find_session_id_for_pid;
use crate::session::cleaner::is_process_alive;
use crate::status::hybrid::is_zombie_by_tty;
use crate::types::WorkingState;
use crate::notification;
use crate::event;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;

/// Start coordinator thread
pub fn start_coordinator(
    event_receiver: Receiver<MonitorEvent>,
    session_cache: Arc<Mutex<HashMap<u32, String>>>,
    shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        run_coordinator(event_receiver, session_cache, shared_sessions, None);
    })
}

/// Start coordinator thread with cleanup sender
pub fn start_coordinator_with_cleanup(
    event_receiver: Receiver<MonitorEvent>,
    session_cache: Arc<Mutex<HashMap<u32, String>>>,
    shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    cleanup_sender: Sender<CleanupEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        run_coordinator(event_receiver, session_cache, shared_sessions, Some(cleanup_sender));
    })
}

fn run_coordinator(
    event_receiver: Receiver<MonitorEvent>,
    session_cache: Arc<Mutex<HashMap<u32, String>>>,
    shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    cleanup_sender: Option<Sender<CleanupEvent>>,
) {
    let mut sessions: HashMap<String, SessionState> = HashMap::new();
    let mut pid_to_session: HashMap<u32, String> = HashMap::new();
    let mut event_count = 0;
    let mut last_summary = current_timestamp();

    println!("[Coordinator] Started with cleanup support: {}", cleanup_sender.is_some());

    // Event loop
    loop {
        match event_receiver.recv() {
            Ok(MonitorEvent::Log(log_event)) => {
                event_count += 1;
                println!("[Coordinator] Received Log event (count: {})", event_count);
                handle_log_event(log_event, &mut sessions, &mut pid_to_session, &session_cache);
            }
            Ok(MonitorEvent::Cpu(cpu_event)) => {
                event_count += 1;
                println!("[Coordinator] Received CPU event (count: {})", event_count);
                handle_cpu_event(cpu_event, &mut sessions, &mut pid_to_session, &session_cache, &cleanup_sender);
            }
            Ok(MonitorEvent::Hook(hook_event)) => {
                event_count += 1;
                println!("[Coordinator] Received Hook event (count: {})", event_count);
                handle_hook_event(hook_event, &mut sessions);
            }
            Err(_) => {
                println!("[Coordinator] Channel disconnected, shutting down");
                break;
            }
        }

        // Update shared sessions (for get_miners command) - MERGE instead of REPLACE
        {
            let mut shared = shared_sessions.lock().unwrap();

            // First, add all local sessions to shared
            for (session_id, session) in sessions.iter() {
                shared.insert(session_id.clone(), session.clone());
            }

            // Then, remove from local any sessions that were deleted from shared
            // (This only applies to sessions that existed in both and were deleted from shared)
            let mut removed_ids = Vec::new();
            for session_id in sessions.keys() {
                if !shared.contains_key(session_id) {
                    removed_ids.push(session_id.clone());
                }
            }

            for id in removed_ids {
                sessions.remove(&id);
                println!("[Coordinator] Session {} was removed by cleaner", &id[..8.min(id.len())]);
            }
        }

        // Periodic summary (every 30 seconds)
        let now = current_timestamp();
        if now - last_summary >= 30 {
            println!("[Coordinator] === Status Summary ===");
            println!("[Coordinator] Total events processed: {}", event_count);
            println!("[Coordinator] Active sessions: {}", sessions.len());
            for (sid, state) in sessions.iter() {
                println!("[Coordinator]   Session {}: status={}, pid={}, has_terminal={}",
                    &sid[..8.min(sid.len())], state.current_status, state.pid, state.has_terminal);
            }
            println!("[Coordinator] =====================");
            last_summary = now;
        }

        // Periodic cleanup (every 100 events or so)
        if sessions.len() > 100 {
            cleanup_stale_sessions(&mut sessions, &mut pid_to_session);
        }
    }
}

fn handle_log_event(
    log_event: crate::session::LogEvent,
    sessions: &mut HashMap<String, SessionState>,
    pid_to_session: &mut HashMap<u32, String>,
    _session_cache: &Arc<Mutex<HashMap<u32, String>>>,
) {
    let session_id = log_event.session_id.clone();

    println!("[Coordinator] handle_log_event: session={}, pid={:?}", &session_id[..8], log_event.pid);

    // Try to find existing PID from temporary sessions
    let mut found_pid: Option<u32> = None;
    for (temp_id, temp_session) in sessions.iter() {
        if temp_id.starts_with("pid-") && temp_session.pid != 0 {
            // Check if this temporary session should be merged
            if !sessions.contains_key(&session_id) {
                found_pid = Some(temp_session.pid);
                println!("[Coordinator] Found PID {} from temporary session", found_pid.unwrap());
                break;
            }
        }
    }

    // Check if PID is dead before creating/updating session
    if let Some(pid) = found_pid.or(log_event.pid) {
        if pid != 0 && !is_process_alive(pid) {
            println!("[Coordinator] ⚠️ Ignoring log event for dead process: PID {} (session: {})",
                pid, &session_id[..8]);
            return;
        }
    }

    // Check if this is a new session
    let is_new_session = !sessions.contains_key(&session_id);

    // Get or create session state (Legacy type - from log files)
    let session = sessions.entry(session_id.clone()).or_insert_with(|| {
        let pid = found_pid.or(log_event.pid).unwrap_or(0);
        println!("[Coordinator] Creating LEGACY session {} with PID {}", &session_id[..8], pid);
        SessionState::new_legacy(pid, session_id.clone())
    });

    // Check if existing session has a dead PID (prevents zombie resurrection)
    // Don't remove the session, just skip updating it to prevent resurrection
    if session.pid != 0 && !is_process_alive(session.pid) {
        println!("[Coordinator] ⚠️ Existing session has dead PID: {} (session: {}), skipping update",
            session.pid, &session_id[..8]);
        return;  // Skip update but keep session for cleanup later
    }

    // Update PID if found
    let mut temp_id_to_remove: Option<String> = None;
    if let Some(pid) = found_pid {
        if session.pid == 0 {
            session.pid = pid;
            pid_to_session.insert(pid, session_id.clone());
            temp_id_to_remove = Some(format!("pid-{}", pid));
        }
    }

    // Store session PID for later checks (before we drop the mutable borrow)
    let session_pid = session.pid;

    // Update log event
    session.last_log_event = Some(log_event.clone());
    session.last_update = current_timestamp();

    println!("[Coordinator] Log event for session {}: state={:?}, approval_pending={}",
        &session_id[..8], log_event.state, log_event.has_approval_pending);

    // Decide new status (only update if changed)
    let old_status = session.current_status;
    let new_status = decide_status(session);
    let status_changed = new_status != old_status;
    if status_changed {
        println!("[Coordinator] Session {} status change: {} -> {}",
            &session.session_id[..8], old_status, new_status);
        session.current_status = new_status;
    }

    // Clone session for events (to avoid borrow issues)
    let session_clone = session.clone();

    // Drop the mutable borrow by ending the scope
    let _ = session; // Release mutable borrow

    // Now we can remove temporary session
    if let Some(temp_id) = temp_id_to_remove {
        sessions.remove(&temp_id);
        println!("[Coordinator] Merged temporary session {} into {}", &temp_id[..8], &session_id[..8]);
    }

    // Emit session-created event if new
    if is_new_session && session_pid != 0 {
        println!("[Coordinator] ⭐ New session created: {}", &session_id[..8]);
        event::emit_session_created(&session_clone);
    }

    // Emit status-changed event
    if status_changed {
        event::emit_session_status_changed(&session_clone);

        // Send notification when task completes (working → resting)
        if old_status == "working" && new_status == "resting" {
            notification::send_task_completion_notification(&session_clone);
        }
    }
}

fn handle_cpu_event(
    cpu_event: crate::session::CpuEvent,
    sessions: &mut HashMap<String, SessionState>,
    pid_to_session: &mut HashMap<u32, String>,
    session_cache: &Arc<Mutex<HashMap<u32, String>>>,
    cleanup_sender: &Option<Sender<CleanupEvent>>,
) {
    if let Some(session_id) = pid_to_session.get(&cpu_event.pid) {
        if let Some(session) = sessions.get_mut(session_id) {
            println!("[Coordinator] CPU event for session {}: pid={}, cpu={:.1}%",
                &session.session_id[..8], cpu_event.pid, cpu_event.cpu_percent);

            session.last_cpu_event = Some(cpu_event.clone());
            session.last_update = current_timestamp();

            // Update PID if it was placeholder
            if session.pid == 0 {
                session.pid = cpu_event.pid;
            }

            // Update last active timestamp if CPU is high
            if cpu_event.cpu_percent > 1.0 {
                session.last_active_timestamp = Some(current_timestamp());
            }

            // Check TTY for zombie detection (Legacy sessions only)
            if matches!(session.session_type, crate::session::SessionType::Legacy) {
                let is_zombie = is_zombie_by_tty(cpu_event.pid);
                let has_tty = !is_zombie;

                // Debug output for TTY status
                if is_zombie {
                    println!("[Coordinator]   TTY check: pid={} is ZOMBIE (TTY='?' or '??')", cpu_event.pid);
                }

                if session.has_terminal != has_tty {
                    println!("[Coordinator]   TTY changed: {} -> {} (pid={}, is_zombie={})",
                        session.has_terminal, has_tty, cpu_event.pid, is_zombie);
                    session.has_terminal = has_tty;

                    // If became zombie, force status update immediately
                    if is_zombie {
                        println!("[Coordinator]   Session became zombie due to TTY loss");
                        session.current_status = "zombie";

                        // Send cleanup event to check if process is actually dead
                        if let Some(sender) = cleanup_sender {
                            let _ = sender.send(CleanupEvent::SessionBecameZombie(session_id.clone()));
                            println!("[Coordinator]   Sent zombie cleanup event for session {}", &session_id[..8]);
                        }
                    }
                }

                // Double check: even if has_terminal didn't change, verify zombie status
                if is_zombie && session.current_status != "zombie" {
                    println!("[Coordinator]   Correcting status to zombie (pid={})", cpu_event.pid);
                    session.current_status = "zombie";

                    // Send cleanup event to check if process is actually dead
                    if let Some(sender) = cleanup_sender {
                        let _ = sender.send(CleanupEvent::SessionBecameZombie(session_id.clone()));
                        println!("[Coordinator]   Sent zombie cleanup event for session {}", &session_id[..8]);
                    }
                }
            }

            // Check for idle detection on CPU events
            if session.current_status == "working" && matches!(session.session_type, crate::session::SessionType::Legacy) {
                let old_status = session.current_status;
                let new_status = decide_status(session);

                if new_status != old_status {
                    println!("[Coordinator] Session {} status change (CPU idle): {} -> {}",
                        &session.session_id[..8], old_status, new_status);
                    session.current_status = new_status;

                    // Emit status-changed event
                    event::emit_session_status_changed(&*session);

                    // Send notification when task completes (working → resting)
                    if old_status == "working" && new_status == "resting" {
                        notification::send_task_completion_notification(session);
                    }
                }
            }
        }
    } else {
        // Unknown PID - try to find real session ID first
        println!("[Coordinator] CPU event for unknown PID: {}, cpu={:.1}%",
            cpu_event.pid, cpu_event.cpu_percent);

        // Try to find session ID from debug files
        let found_session_id = find_session_id_for_pid(cpu_event.pid, &mut session_cache.lock().unwrap());

        if let Some(session_id) = found_session_id {
            println!("[Coordinator] Found real session ID {} for PID {}", session_id, cpu_event.pid);

            // Get or create session for this PID (Legacy type - discovered from CPU)
            let session = sessions.entry(session_id.clone()).or_insert_with(|| {
                println!("[Coordinator] Creating LEGACY session: {}", session_id);
                let mut new_session = SessionState::new_legacy(cpu_event.pid, session_id.clone());
                // Set initial status based on current state
                new_session.current_status = "resting"; // Default to resting instead of unknown
                new_session
            });

            session.last_cpu_event = Some(cpu_event.clone());
            session.last_update = current_timestamp();

            // Check TTY for zombie detection
            let is_zombie = is_zombie_by_tty(cpu_event.pid);
            session.has_terminal = !is_zombie;

            if is_zombie {
                println!("[Coordinator] Session '{}' is ZOMBIE (TTY='?' or '??', pid={})",
                    &session_id[..8], cpu_event.pid);
                session.current_status = "zombie";
            }

            // Update pid_to_session map
            pid_to_session.insert(cpu_event.pid, session_id.clone());

            // Re-decide status
            let old_status = session.current_status;
            let new_status = decide_status(session);
            if new_status != old_status {
                println!("[Coordinator] Session {} status change (CPU): {} -> {}",
                    &session.session_id[..8], old_status, new_status);
                session.current_status = new_status;

                // Emit status-changed event
                event::emit_session_status_changed(&*session);
            }
        } else {
            // No session ID found - just log and ignore
            println!("[Coordinator] No session ID found for PID {}, ignoring CPU event", cpu_event.pid);
        }
    }
}


fn decide_status(session: &SessionState) -> &'static str {
    use crate::session::SessionType;

    // FIRST PRIORITY: Always check for zombie first
    // Check 1: has_terminal flag
    if !session.has_terminal {
        println!("[Coordinator] decide_status: session={}, no terminal flag -> ZOMBIE",
            &session.session_id[..8]);
        return "zombie";
    }

    // Check 2: Direct TTY verification
    if session.pid != 0 {
        let is_zombie = is_zombie_by_tty(session.pid);
        if is_zombie {
            println!("[Coordinator] decide_status: session={}, TTY='??' -> ZOMBIE (pid={})",
                &session.session_id[..8], session.pid);
            return "zombie";
        }
    }

    // Only after confirming NOT zombie, check other status
    let status = match session.session_type {
        SessionType::Legacy => decide_status_legacy(session),
        SessionType::Hook => decide_status_hook(session),
    };

    // Never return "unknown" - default to "resting"
    if status == "unknown" {
        println!("[Coordinator] decide_status: converting unknown -> resting for session {}",
            &session.session_id[..8]);
        "resting"
    } else {
        status
    }
}

/// Legacy session status decision: mtime + CPU + log content based
/// Logic: "Stream started - received first chunk" → working (with stricter conditions)
///        mtime stale (>15s) OR low CPU → resting
fn decide_status_legacy(session: &SessionState) -> &'static str {
    let now = current_timestamp();

    println!("[Coordinator] decide_status_legacy: session={}", &session.session_id[..8]);

    // Priority 0: Check zombie status first
    if !session.has_terminal {
        println!("[Coordinator]   no terminal (zombie) -> zombie");
        return "zombie";
    }

    // Check idle time for working sessions (IMPROVED DEBOUNCING)
    // If session has been working but CPU is near 0 for extended time, switch to resting
    // IMPORTANT: Use conservative thresholds to avoid false positives during thinking/waiting
    if session.current_status == "working" {
        if let Some(ref cpu) = session.last_cpu_event {
            let cpu_age = now.saturating_sub(cpu.timestamp);

            // If we have recent CPU data and it's VERY low (stricter threshold: 0.5%)
            if cpu_age < 10 && cpu.cpu_percent <= 0.5 {
                // Check if there's been any recent activity
                if let Some(ref log) = session.last_log_event {
                    let log_age = now.saturating_sub(log.file_mtime);

                    // INCREASED DEBOUNCING: 45 seconds (was 20s) to avoid false positives
                    // This prevents marking as "resting" when Claude is:
                    // - Thinking deeply about a problem
                    // - Waiting for tool execution
                    // - Waiting for user input
                    if log_age > 45 {
                        println!("[Coordinator]   Working but idle (CPU={:.1}%, log_age={}s) -> resting [DEBOUNCED]",
                            cpu.cpu_percent, log_age);
                        return "resting";
                    } else {
                        println!("[Coordinator]   Working, low CPU but within debounce window (log_age={}s < 45s)",
                            log_age);
                    }
                } else {
                    // No log event BUT require longer idle time (60s) before switching
                    // This handles edge case where log hasn't been created yet
                    let session_age = now.saturating_sub(session.last_update);
                    if session_age > 60 {
                        println!("[Coordinator]   Working but no activity (CPU={:.1}%, session_age={}s) -> resting",
                            cpu.cpu_percent, session_age);
                        return "resting";
                    }
                }
            }
        }
    }

    // Priority 1: Check if "Stream started - received first chunk" exists in log
    if let Some(ref log) = session.last_log_event {
        let mtime_age = now.saturating_sub(log.file_mtime);

        println!("[Coordinator]   mtime_age={}s, state={:?}", mtime_age, log.state);

        // If "Stream started - received first chunk" was found → check additional conditions
        if matches!(log.state, WorkingState::ActivelyWorking) {
            println!("[Coordinator]   Stream started detected, checking conditions...");

            // Check if it's stale (INCREASED: mtime > 30s) → transition to resting
            // Was 15s, now 30s for better debouncing
            if mtime_age >= 30 {
                println!("[Coordinator]   mtime stale (>30s) -> resting [DEBOUNCED]");
                return "resting";
            }

            // Check CPU to confirm still working
            // IMPORTANT: Don't immediately switch to resting on low CPU
            // Claude might be thinking or waiting for tool execution
            if let Some(ref cpu) = session.last_cpu_event {
                let cpu_age = now.saturating_sub(cpu.timestamp);

                // If CPU is recent and > 10%, definitely working
                if cpu_age < 10 && cpu.cpu_percent > 10.0 {
                    println!("[Coordinator]   Stream started + CPU > 10% ({:.1}%) -> working", cpu.cpu_percent);
                    return "working";
                }

                // Low CPU BUT mtime is fresh (< 30s) → keep working
                // This prevents false positives when Claude is thinking
                if cpu_age < 10 && cpu.cpu_percent <= 10.0 && mtime_age < 30 {
                    println!("[Coordinator]   Low CPU ({:.1}%) but fresh mtime ({}s) -> working [DEBOUNCING]",
                        cpu.cpu_percent, mtime_age);
                    return "working";
                }

                // Low CPU AND stale mtime (>= 30s) → resting
                if cpu_age < 10 && mtime_age >= 30 {
                    println!("[Coordinator]   low CPU ({:.1}%) + stale mtime ({}s) -> resting [DEBOUNCED]",
                        cpu.cpu_percent, mtime_age);
                    return "resting";
                }
            }

            // No CPU data - need to be more careful
            // Only trust "very fresh log" if we have a valid PID (can get CPU later)
            if session.pid != 0 && mtime_age < 5 {
                println!("[Coordinator]   very fresh log, valid PID but no CPU yet -> working");
                return "working";
            }

            // If PID is 0 or log is not that fresh, default to resting
            // This prevents PID=0 sessions from staying "working" forever
            if session.pid == 0 {
                println!("[Coordinator]   no PID, cannot track CPU -> resting");
            } else {
                println!("[Coordinator]   no supporting evidence -> resting");
            }
            return "resting";
        } else {
            // No "Stream started" pattern found → default to resting
            println!("[Coordinator]   No stream activity detected -> resting");
        }
    }

    // Priority 2: CPU usage (fallback for sessions without log)
    // CPU > 10% = working
    if let Some(ref cpu) = session.last_cpu_event {
        let cpu_age = now.saturating_sub(cpu.timestamp);
        if cpu_age < 10 && cpu.cpu_percent > 10.0 {
            println!("[Coordinator]   CPU > 10% ({:.1}%) -> working", cpu.cpu_percent);
            return "working";
        }
    }

    // Default: No recent activity = resting
    println!("[Coordinator]   no recent activity -> resting");
    "resting"
}

/// Hook session status decision: Hook events only
fn decide_status_hook(session: &SessionState) -> &'static str {
    println!("[Coordinator] decide_status_hook: session={}, current_status={}",
        &session.session_id[..8], session.current_status);

    // Hook sessions maintain their status set by Hook events
    // We don't change status here - only Hook events can change it
    session.current_status
}

fn find_pid_for_session(session_id: &str, session_cache: &Arc<Mutex<HashMap<u32, String>>>) -> Option<u32> {
    // Search through all PIDs (this is called rarely)
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();

    for (pid, _process) in sys.processes() {
        let pid_u32 = pid.as_u32();

        // Get session ID for this PID
        let mut cache = session_cache.lock().unwrap();
        if let Some(found_id) = find_session_id_for_pid(pid_u32, &mut *cache) {
            drop(cache); // Release lock
            if found_id == session_id {
                return Some(pid_u32);
            }
        } else {
            drop(cache);
        }
    }

    None
}

fn handle_hook_event(
    hook_event: crate::session::HookEvent,
    sessions: &mut HashMap<String, SessionState>,
) {
    let session_id = hook_event.sid.clone();

    println!("[Coordinator] handle_hook_event: session={}, evt={}",
        &session_id[..8.min(session_id.len())], hook_event.evt);

    match hook_event.evt.as_str() {
        "start" => {
            // Create or activate Hook session
            let is_new = !sessions.contains_key(&session_id);

            let session = sessions.entry(session_id.clone()).or_insert_with(|| {
                println!("[Coordinator] Creating HOOK session from Hook: {}", &session_id[..8]);
                SessionState::new_hook(session_id.clone())
            });

            // Upgrade Legacy to Hook if needed
            if session.upgrade_to_hook() {
                println!("[Coordinator] ✅ Session {} successfully upgraded to Hook", &session_id[..8]);
            }

            session.current_status = "resting"; // Just started, waiting for work
            session.last_update = current_timestamp();

            if is_new {
                println!("[Coordinator] ⭐ New session created via Hook: {}", &session_id[..8]);
                event::emit_session_created(&*session);
            }
        }
        "working" => {
            if let Some(session) = sessions.get_mut(&session_id) {
                // Upgrade Legacy to Hook if needed
                if session.upgrade_to_hook() {
                    println!("[Coordinator] ✅ Session {} upgraded to Hook on 'working' event", &session_id[..8]);
                }

                let old_status = session.current_status;
                session.current_status = "working";
                session.last_update = current_timestamp();

                if old_status != "working" {
                    println!("[Coordinator] Session {} status change (Hook): {} -> working",
                        &session.session_id[..8], old_status);

                    event::emit_session_status_changed(&*session);
                }
            }
        }
        "resting" => {
            if let Some(session) = sessions.get_mut(&session_id) {
                // Upgrade Legacy to Hook if needed
                if session.upgrade_to_hook() {
                    println!("[Coordinator] ✅ Session {} upgraded to Hook on 'resting' event", &session_id[..8]);
                }

                let old_status = session.current_status;
                session.current_status = "resting";
                session.last_update = current_timestamp();

                if old_status != "resting" {
                    println!("[Coordinator] Session {} status change (Hook): {} -> resting",
                        &session.session_id[..8], old_status);

                    event::emit_session_status_changed(&*session);

                    // Send notification when task completes (working → resting)
                    if old_status == "working" {
                        notification::send_task_completion_notification(session);
                    }
                }
            }
        }
        "end" => {
            if let Some(session) = sessions.remove(&session_id) {
                println!("[Coordinator] 💀 Session terminated via Hook: {}", &session_id[..8]);

                event::emit_session_terminated(&session);
            }
        }
        _ => {
            println!("[Coordinator] Unknown hook event: {}", hook_event.evt);
        }
    }
}

fn cleanup_stale_sessions(
    sessions: &mut HashMap<String, SessionState>,
    pid_to_session: &mut HashMap<u32, String>,
) {
    let now = current_timestamp();
    let stale_threshold = 3600; // 1 hour

    let mut removed_sessions = Vec::new();

    sessions.retain(|session_id, session| {
        let age = now.saturating_sub(session.last_update);
        if age > stale_threshold {
            println!("[Coordinator] 💀 Session terminated (stale): {}", &session_id[..8]);
            removed_sessions.push(session.clone());
            // Remove from PID mapping too
            pid_to_session.remove(&session.pid);
            false
        } else {
            true
        }
    });

    // Emit session-terminated events for all removed sessions
    for session in removed_sessions {
        event::emit_session_terminated(&session);
    }
}
