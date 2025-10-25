// Session Cleaner Thread
//
// Event-driven session cleanup system
// Responds immediately to process termination events
//

use crate::session::{MonitorEvent, SessionState};
use std::sync::mpsc::{Sender, Receiver, channel};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use sysinfo::{System, Pid};

/// Cleanup events that trigger immediate action
#[derive(Debug, Clone)]
pub enum CleanupEvent {
    ProcessTerminated(u32),           // PID that terminated
    SessionBecameZombie(String),      // Session ID that became zombie
    CheckDeadSessions,                // Check all sessions for dead processes
    ForceCleanup(String),             // Force cleanup specific session
    CleanupZombies,                   // Clean all zombie sessions
}

/// Session cleaner that responds to events
pub struct SessionCleaner {
    shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    event_sender: Sender<MonitorEvent>,
    cleanup_receiver: Receiver<CleanupEvent>,
    cleanup_sender: Sender<CleanupEvent>,
}

impl SessionCleaner {
    pub fn new(
        shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>,
        event_sender: Sender<MonitorEvent>,
    ) -> (Self, Sender<CleanupEvent>) {
        let (cleanup_sender, cleanup_receiver) = channel();
        let sender_clone = cleanup_sender.clone();

        let cleaner = Self {
            shared_sessions,
            event_sender,
            cleanup_receiver,
            cleanup_sender,
        };

        (cleaner, sender_clone)
    }

    pub fn run(mut self) {
        println!("[SessionCleaner] Started in event-driven mode");

        loop {
            // Wait for cleanup events
            match self.cleanup_receiver.recv() {
                Ok(event) => {
                    self.handle_cleanup_event(event);
                }
                Err(_) => {
                    println!("[SessionCleaner] Channel closed, shutting down");
                    break;
                }
            }
        }
    }

    fn handle_cleanup_event(&mut self, event: CleanupEvent) {
        match event {
            CleanupEvent::ProcessTerminated(pid) => {
                self.cleanup_terminated_process(pid);
            }
            CleanupEvent::SessionBecameZombie(session_id) => {
                self.cleanup_zombie_session(&session_id);
            }
            CleanupEvent::CheckDeadSessions => {
                self.check_and_cleanup_dead_sessions();
            }
            CleanupEvent::ForceCleanup(session_id) => {
                self.force_cleanup_session(&session_id);
            }
            CleanupEvent::CleanupZombies => {
                self.cleanup_all_zombies();
            }
        }
    }

    /// Clean up a specific terminated process
    fn cleanup_terminated_process(&mut self, pid: u32) {
        println!("[SessionCleaner] Cleaning up terminated process: PID {}", pid);

        let mut sessions = self.shared_sessions.lock().unwrap();
        let mut sessions_to_remove = Vec::new();

        // Find all sessions with this PID
        for (session_id, session) in sessions.iter() {
            if session.pid == pid {
                // Verify process is really dead
                if !is_process_alive(pid) {
                    println!("[SessionCleaner] Process {} confirmed dead, removing session: {}",
                        pid, &session_id[..8.min(session_id.len())]);
                    sessions_to_remove.push(session_id.clone());
                }
            }
        }

        // Remove dead sessions
        for session_id in sessions_to_remove {
            sessions.remove(&session_id);
            println!("[SessionCleaner] Removed dead session: {}",
                &session_id[..8.min(session_id.len())]);
        }
    }

    /// Clean up a zombie session
    fn cleanup_zombie_session(&mut self, session_id: &str) {
        println!("[SessionCleaner] Checking zombie session: {}",
            &session_id[..8.min(session_id.len())]);

        let mut sessions = self.shared_sessions.lock().unwrap();

        if let Some(session) = sessions.get(session_id) {
            // Skip sessions with PID=0 (Hook sessions waiting for PID discovery)
            if session.pid == 0 {
                println!("[SessionCleaner] Skipping zombie check for session with PID=0: {}",
                    &session_id[..8.min(session_id.len())]);
                return;
            }

            // If process doesn't exist, remove immediately
            if !is_process_alive(session.pid) {
                println!("[SessionCleaner] Zombie process {} is dead, removing session",
                    session.pid);
                sessions.remove(session_id);
            }
        }
    }

    /// Check all sessions and cleanup dead ones
    fn check_and_cleanup_dead_sessions(&mut self) {
        println!("[SessionCleaner] Checking all sessions for dead processes");

        let mut sessions = self.shared_sessions.lock().unwrap();
        let mut dead_sessions = Vec::new();

        for (session_id, session) in sessions.iter() {
            // Skip sessions with PID=0 (Hook sessions waiting for PID discovery)
            if session.pid == 0 {
                continue;
            }

            if !is_process_alive(session.pid) {
                println!("[SessionCleaner] Found dead process: PID {} (session: {})",
                    session.pid, &session_id[..8.min(session_id.len())]);
                dead_sessions.push(session_id.clone());
            }
        }

        // Remove all dead sessions
        for session_id in &dead_sessions {
            sessions.remove(session_id);
            println!("[SessionCleaner] Removed dead session: {}",
                &session_id[..8.min(session_id.len())]);
        }

        if !dead_sessions.is_empty() {
            println!("[SessionCleaner] Cleaned up {} dead sessions", dead_sessions.len());
        }
    }

    /// Force cleanup a specific session
    fn force_cleanup_session(&mut self, session_id: &str) {
        println!("[SessionCleaner] Force cleaning session: {}",
            &session_id[..8.min(session_id.len())]);

        let mut sessions = self.shared_sessions.lock().unwrap();
        if sessions.remove(session_id).is_some() {
            println!("[SessionCleaner] Force removed session: {}",
                &session_id[..8.min(session_id.len())]);
        }
    }

    /// Clean up all zombie sessions
    fn cleanup_all_zombies(&mut self) {
        println!("[SessionCleaner] Cleaning all zombie sessions");

        let mut sessions = self.shared_sessions.lock().unwrap();
        let mut zombie_sessions = Vec::new();

        for (session_id, session) in sessions.iter() {
            // Remove all temporary zombie sessions (they shouldn't exist)
            if session_id.starts_with("pid-") && session.current_status == "zombie" {
                println!("[SessionCleaner] Found temporary zombie: {} (pid={})",
                    &session_id[..8.min(session_id.len())], session.pid);
                zombie_sessions.push(session_id.clone());
                continue;
            }

            if session.current_status == "zombie" {
                // Check if process is actually dead
                if session.pid == 0 || !is_process_alive(session.pid) {
                    zombie_sessions.push(session_id.clone());
                }
            }
        }

        // Remove dead zombie sessions
        for session_id in &zombie_sessions {
            sessions.remove(session_id);
            println!("[SessionCleaner] Removed zombie session: {}",
                &session_id[..8.min(session_id.len())]);
        }

        if !zombie_sessions.is_empty() {
            println!("[SessionCleaner] Cleaned up {} zombie sessions", zombie_sessions.len());
        }
    }
}

/// Start session cleaner thread with event-driven architecture
pub fn start_session_cleaner(
    shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    event_sender: Sender<MonitorEvent>,
) -> (thread::JoinHandle<()>, Sender<CleanupEvent>) {
    let (cleaner, cleanup_sender) = SessionCleaner::new(shared_sessions.clone(), event_sender);
    let cleanup_sender_clone = cleanup_sender.clone();

    // Start cleaner thread
    let handle = thread::spawn(move || {
        cleaner.run();
    });

    // Also start a periodic dead session checker (fallback)
    let cleanup_sender_periodic = cleanup_sender_clone.clone();
    let sessions_for_periodic = shared_sessions;
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(15)); // Check every 15 seconds for zombies

            // Send event to check dead sessions
            if cleanup_sender_periodic.send(CleanupEvent::CheckDeadSessions).is_err() {
                break;
            }

            // Also periodically clean zombies
            if cleanup_sender_periodic.send(CleanupEvent::CleanupZombies).is_err() {
                break;
            }
        }
    });

    (handle, cleanup_sender_clone)
}

/// Check if a process is still alive
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }

    let mut sys = System::new();
    sys.refresh_process(Pid::from_u32(pid));
    let exists = sys.process(Pid::from_u32(pid)).is_some();

    if !exists {
        println!("[SessionCleaner] Process {} is NOT alive", pid);
    }

    exists
}

/// Force cleanup of all sessions (for emergency use)
pub fn force_cleanup_all(shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>) {
    println!("[SessionCleaner] FORCE CLEANUP: Removing all sessions");

    let mut sessions = shared_sessions.lock().unwrap();
    let count = sessions.len();
    sessions.clear();

    println!("[SessionCleaner] Force cleaned {} sessions", count);
}

/// Cleanup sessions by criteria
pub fn cleanup_by_status(
    shared_sessions: Arc<Mutex<HashMap<String, SessionState>>>,
    status: &str,
) -> usize {
    println!("[SessionCleaner] Cleaning sessions with status: {}", status);

    let mut sessions = shared_sessions.lock().unwrap();
    let mut removed_count = 0;

    sessions.retain(|_id, session| {
        if session.current_status == status {
            // Also check if zombie processes are actually dead
            if status == "zombie" && session.pid != 0 && !is_process_alive(session.pid) {
                removed_count += 1;
                false
            } else if status != "zombie" && session.current_status == status {
                removed_count += 1;
                false
            } else {
                true
            }
        } else {
            true
        }
    });

    println!("[SessionCleaner] Removed {} sessions with status '{}'", removed_count, status);
    removed_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::events::SessionType;

    #[test]
    fn test_cleanup_by_status() {
        let sessions = Arc::new(Mutex::new(HashMap::new()));

        // Add test sessions
        {
            let mut s = sessions.lock().unwrap();
            let mut session1 = SessionState::new_legacy(1, "test1".to_string());
            session1.current_status = "zombie";
            s.insert("test1".to_string(), session1);

            let mut session2 = SessionState::new_legacy(2, "test2".to_string());
            session2.current_status = "working";
            s.insert("test2".to_string(), session2);

            let mut session3 = SessionState::new_legacy(3, "test3".to_string());
            session3.current_status = "zombie";
            s.insert("test3".to_string(), session3);
        }

        // Clean zombie sessions
        let removed = cleanup_by_status(sessions.clone(), "zombie");
        assert_eq!(removed, 2);

        // Check remaining sessions
        let s = sessions.lock().unwrap();
        assert_eq!(s.len(), 1);
        assert!(s.contains_key("test2"));
    }

    #[test]
    fn test_force_cleanup() {
        let sessions = Arc::new(Mutex::new(HashMap::new()));

        // Add test sessions
        {
            let mut s = sessions.lock().unwrap();
            s.insert("test1".to_string(), SessionState::new_legacy(1, "test1".to_string()));
            s.insert("test2".to_string(), SessionState::new_hook("test2".to_string()));
        }

        // Force cleanup
        force_cleanup_all(sessions.clone());

        // Check all sessions removed
        let s = sessions.lock().unwrap();
        assert_eq!(s.len(), 0);
    }
}