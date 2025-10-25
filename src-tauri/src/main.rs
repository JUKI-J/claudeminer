// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]

// Refactored modules
mod types;
mod network;
mod session;
mod status;
mod monitor;
mod hooks;
mod coordinator;
mod notification;
mod event;

use types::Miner;
use session::SessionState;
use sysinfo::{System, Pid};
use tauri::{Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, CustomMenuItem, Menu, MenuItem, Submenu};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// Type alias for shared sessions
type SharedSessions = Arc<Mutex<HashMap<String, SessionState>>>;

#[tauri::command]
fn get_miners(
    shared_sessions: tauri::State<SharedSessions>,
) -> Vec<Miner> {
    println!("[get_miners] ===== CALLED =====");

    // Get sessions from Coordinator's real-time monitoring
    let sessions = shared_sessions.lock().unwrap();

    let mut miners = Vec::new();

    // Get fresh process info for memory
    let mut sys = System::new_all();
    sys.refresh_all();

    println!("[get_miners] Retrieved {} sessions from Coordinator", sessions.len());

    if sessions.is_empty() {
        println!("[get_miners] WARNING: No sessions found! Coordinator may not be detecting sessions.");
    }

    // Convert SessionState to Miner for each session
    for (session_id, session_state) in sessions.iter() {
        // Skip only truly invalid sessions ($SESSION_ID or sessions with PID=0 that never got a real PID)
        if session_id == "$SESSION_ID" {
            println!("[get_miners] Skipping invalid session: {} (pid={})", session_id, session_state.pid);
            continue;
        }

        // Skip sessions with PID=0 only if they're not working (PID=0 means we haven't discovered the PID yet)
        if session_state.pid == 0 && session_state.current_status != "working" {
            println!("[get_miners] Skipping session without PID: {} (status={})", session_id, session_state.current_status);
            continue;
        }

        println!("[get_miners] Processing session: {}", session_id);
        println!("[get_miners]   - PID: {}", session_state.pid);
        println!("[get_miners]   - Status: {}", session_state.current_status);
        println!("[get_miners]   - Has terminal: {} (zombie={})",
            session_state.has_terminal,
            session_state.current_status == "zombie");

        let pid = Pid::from_u32(session_state.pid);

        // Get memory from sysinfo
        let memory = sys.process(pid)
            .map(|p| {
                let mem = p.memory();
                println!("[get_miners]   - Memory: {} bytes", mem);
                mem
            })
            .unwrap_or_else(|| {
                println!("[get_miners]   - Memory: 0 (process not found in sysinfo)");
                0
            });

        // Get CPU from last CPU event
        let cpu = session_state.last_cpu_event.as_ref()
            .map(|e| {
                println!("[get_miners]   - CPU (from event): {:.1}%", e.cpu_percent);
                e.cpu_percent
            })
            .unwrap_or_else(|| {
                println!("[get_miners]   - CPU: 0.0% (no CPU event)");
                0.0
            });


        println!("[get_miners]   Session {}: pid={}, status={}, cpu={:.1}%, mem={}KB, has_terminal={}",
            &session_id[..8], session_state.pid, session_state.current_status, cpu, memory/1024, session_state.has_terminal);

        miners.push(Miner {
            pid: session_state.pid,
            cpu_usage: cpu,
            memory,
            status: session_state.current_status.to_string(),
            has_terminal: session_state.has_terminal,
            name: "Claude Code".to_string(),
        });
    }

    println!("[get_miners] Returning {} miners", miners.len());
    println!("[get_miners] Miners by status:");
    let working = miners.iter().filter(|m| m.status == "working").count();
    let resting = miners.iter().filter(|m| m.status == "resting").count();
    let zombie = miners.iter().filter(|m| m.status == "zombie").count();
    println!("[get_miners]   - working: {}", working);
    println!("[get_miners]   - resting: {}", resting);
    println!("[get_miners]   - zombie: {}", zombie);
    println!("[get_miners] ===== END =====");

    miners
}

#[tauri::command]
fn kill_miner(pid: u32) -> Result<String, String> {
    let _sys_pid = Pid::from_u32(pid);

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Kill process
        let output = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    println!("[kill_miner] Successfully killed PID {}", pid);

                    // Send notification directly
                    notification::send_zombie_killed_notification(pid);

                    Ok(format!("Process {} killed successfully", pid))
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Err(format!("Failed to kill process {}: {}", pid, stderr))
                }
            }
            Err(e) => Err(format!("Failed to execute kill command: {}", e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();

        match output {
            Ok(_) => {
                println!("[kill_miner] Successfully killed PID {}", pid);
                Ok(format!("Process {} killed successfully", pid))
            }
            Err(e) => Err(format!("Failed to kill process {}: {}", pid, e)),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Unsupported platform".to_string())
    }
}

#[tauri::command]
fn send_notification(_title: String, _body: String) -> Result<(), String> {
    // Notification will be handled by Tauri's notification API on the frontend
    Ok(())
}

#[tauri::command]
fn uninstall_app() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Get the app bundle path
        let app_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get app path: {}", e))?
            .ancestors()
            .nth(3)  // Go up from MacOS/ClaudeMiner to ClaudeMiner.app
            .ok_or("Failed to find app bundle")?
            .to_path_buf();

        // Create AppleScript to show confirmation dialog and delete app
        let script = format!(
            r#"
            set appPath to POSIX file "{}"
            display dialog "Are you sure you want to uninstall ClaudeMiner?" buttons {{"Cancel", "Uninstall"}} default button "Cancel" with icon caution
            if button returned of result is "Uninstall" then
                do shell script "rm -rf " & quoted form of POSIX path of appPath with administrator privileges
                return "uninstalled"
            else
                return "cancelled"
            end if
            "#,
            app_path.display()
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| format!("Failed to run uninstall script: {}", e))?;

        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if result == "uninstalled" {
            std::process::exit(0);
        } else {
            Ok("Uninstall cancelled".to_string())
        }
    }

    #[cfg(target_os = "windows")]
    {
        Err("Uninstall feature not implemented for Windows. Please use Windows Settings > Apps to uninstall.".to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Uninstall feature not supported on this platform".to_string())
    }
}

#[tauri::command]
fn update_tray_menu(
    total: u32,
    working: u32,
    resting: u32,
    zombie: u32
) -> Result<(), String> {
    // Delegate to event module (singleton pattern)
    event::update_tray_menu(total, working, resting, zombie)
}

#[tauri::command]
fn send_test_notification() -> Result<String, String> {
    println!("[TestNotification] 🔔 Sending test notification...");
    notification::send_test_notification();
    Ok("Test notification sent!".to_string())
}

fn main() {
    // Create session cache for monitor system
    let session_cache = Arc::new(Mutex::new(HashMap::new()));

    // Create shared sessions for real-time monitoring
    let shared_sessions = Arc::new(Mutex::new(HashMap::new()));
    let shared_sessions_for_command = shared_sessions.clone();

    // Create system tray menu
    let show = CustomMenuItem::new("show".to_string(), "Show Window");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_item(quit);

    let tray = SystemTray::new().with_menu(tray_menu);

    // Create app menu
    let app_menu = Menu::new()
        .add_submenu(Submenu::new(
            "ClaudeMiner",
            Menu::new()
                .add_native_item(MenuItem::About("ClaudeMiner".to_string(), Default::default()))
                .add_native_item(MenuItem::Separator)
                .add_item(CustomMenuItem::new("uninstall".to_string(), "Uninstall ClaudeMiner"))
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Hide)
                .add_native_item(MenuItem::HideOthers)
                .add_native_item(MenuItem::ShowAll)
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Quit),
        ))
        .add_submenu(Submenu::new(
            "Edit",
            Menu::new()
                .add_native_item(MenuItem::Undo)
                .add_native_item(MenuItem::Redo)
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Cut)
                .add_native_item(MenuItem::Copy)
                .add_native_item(MenuItem::Paste)
                .add_native_item(MenuItem::SelectAll),
        ))
        .add_submenu(Submenu::new(
            "Window",
            Menu::new()
                .add_native_item(MenuItem::Minimize)
                .add_native_item(MenuItem::Zoom),
        ));

    tauri::Builder::default()
        .manage(shared_sessions_for_command) // Register shared sessions from Coordinator
        .menu(app_menu)
        .on_menu_event(|event| {
            match event.menu_item_id() {
                "uninstall" => {
                    // Trigger uninstall directly
                    let _ = uninstall_app();
                }
                _ => {}
            }
        })
        .system_tray(tray)
        .on_system_tray_event(|app, event| match event {
            // Removed LeftClick handler to allow default menu behavior on macOS
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "show" => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_miners,
            kill_miner,
            send_notification,
            update_tray_menu,
            uninstall_app,
            send_test_notification
        ])
        .setup(move |app| {
            // Start multi-threaded monitoring system with app_handle
            let app_handle = app.handle();

            // Initialize notification system (singleton pattern)
            notification::init(app_handle.clone());

            // Initialize event emitter (singleton pattern)
            event::init(app_handle.clone());

            // Ensure hooks are registered in Claude Code settings.json
            if let Err(e) = hooks::ensure_hooks_registered() {
                eprintln!("[Main] Failed to register hooks: {}", e);
            }

            // Create communication channels
            use std::sync::mpsc::channel;
            let (event_sender, event_receiver) = channel();

            // Create shared PID set for monitors
            use std::collections::HashSet;
            let claude_pids = Arc::new(Mutex::new(HashSet::new()));

            // Start all monitoring threads
            let _cpu_monitor = monitor::start_cpu_monitor(event_sender.clone(), claude_pids.clone());
            let _log_watcher = monitor::start_log_watcher(event_sender.clone());

            // Start hook receiver (no app_handle needed - uses notification module)
            let _hook_receiver = hooks::start_hook_receiver(event_sender.clone());

            // Start session cleaner (returns handle and sender)
            let (_cleaner_handle, cleanup_sender) = session::start_session_cleaner(
                shared_sessions.clone(),
                event_sender.clone(),
            );

            // Start coordinator with cleanup support (no app_handle needed - uses event module)
            let _coordinator = coordinator::start_coordinator_with_cleanup(
                event_receiver,
                session_cache,
                shared_sessions,
                cleanup_sender,
            );

            println!("[Main] Multi-threaded monitoring system started with Tauri events");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
