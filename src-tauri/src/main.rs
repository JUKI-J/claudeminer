// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use sysinfo::{System, Pid};
use tauri::{Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, CustomMenuItem, AppHandle, Menu, MenuItem, Submenu};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Miner {
    pid: u32,
    cpu_usage: f32,
    memory: u64,
    status: String,
    has_terminal: bool,
    name: String,
}

// Global state for CPU measurements (cached)
type CpuCache = Arc<Mutex<HashMap<u32, f32>>>;

#[tauri::command]
fn get_miners(cpu_cache: tauri::State<CpuCache>) -> Vec<Miner> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut miners = Vec::new();

    // Write debug to file
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut debug_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("/tmp/claudeminer_debug.log")
        .unwrap();

    writeln!(debug_file, "=== Scanning for Claude processes ===").unwrap();
    writeln!(debug_file, "Total processes: {}", sys.processes().len()).unwrap();

    // Try to find specific Claude PIDs
    let claude_pids = vec![40369, 21151, 56165, 96842, 72747];
    writeln!(debug_file, "\nLooking for known Claude PIDs:").unwrap();
    for check_pid in claude_pids {
        let pid = Pid::from_u32(check_pid);
        if let Some(process) = sys.process(pid) {
            writeln!(debug_file, "  Found PID {}: name='{}', cmd={:?}",
                check_pid, process.name(), process.cmd()).unwrap();
        } else {
            writeln!(debug_file, "  PID {} not found in sysinfo", check_pid).unwrap();
        }
    }
    writeln!(debug_file, "").unwrap();

    for (pid, process) in sys.processes() {
        let name = process.name().to_string();

        // Check if this is a Node process running 'claude'
        let is_claude = if name.to_lowercase() == "node" {
            // Check command line arguments
            if let Some(cmd) = process.cmd().get(0) {
                cmd.to_lowercase() == "claude"
            } else {
                false
            }
        } else {
            false
        };

        // Log ALL Claude processes
        if is_claude {
            writeln!(debug_file, "Found Claude process: PID {}, cmd={:?}", pid, process.cmd()).unwrap();
        }

        // Match Claude Code sessions
        if is_claude {
            let pid_u32 = pid.as_u32();

            // Get cached CPU value (non-blocking)
            let cpu = {
                let cache = cpu_cache.lock().unwrap();
                cache.get(&pid_u32).copied().unwrap_or(0.0)
            };

            let disk_usage = process.disk_usage();

            // Calculate total disk activity (read + write bytes per second)
            let disk_activity = disk_usage.read_bytes + disk_usage.written_bytes;

            // Get memory usage
            let memory_mb = process.memory() / 1024 / 1024;

            // Determine status based on real-time system metrics
            // Working if ANY of these conditions are true:
            // 1. CPU usage above 3% (lowered threshold for better detection)
            // 2. Disk I/O above 5KB/s (lowered threshold)
            // 3. Memory usage growing significantly (> 100MB suggests active session)

            let is_working_by_cpu = cpu > 3.0;
            let is_working_by_disk = disk_activity > 5120; // 5KB/s
            let is_potentially_active = memory_mb > 100; // Active sessions typically use more memory

            // Simple OR logic: working if CPU OR Disk activity detected
            // Memory is just for info, not for status determination
            let status = if is_working_by_cpu || is_working_by_disk {
                "working".to_string()
            } else {
                "resting".to_string()
            };

            // On macOS, check if process has a terminal by checking TTY
            #[cfg(target_os = "macos")]
            let has_terminal = {
                use std::process::Command;
                let output = Command::new("ps")
                    .args(["-p", &pid_u32.to_string(), "-o", "tty="])
                    .output();

                if let Ok(output) = output {
                    let tty = String::from_utf8_lossy(&output.stdout);
                    let tty = tty.trim();
                    // If TTY is "??" or empty, process has no terminal
                    !tty.is_empty() && tty != "??"
                } else {
                    true  // Default to true if we can't determine
                }
            };

            #[cfg(target_os = "windows")]
            let has_terminal = true;  // Windows doesn't have TTY concept

            writeln!(debug_file, "  -> CPU: {:.1}%{}, Disk: {} KB/s{}, Memory: {} MB{}, TTY: {}, Status: {}",
                cpu,
                if is_working_by_cpu { " ✓" } else { "" },
                disk_activity / 1024,
                if is_working_by_disk { " ✓" } else { "" },
                memory_mb,
                if is_potentially_active { " ✓" } else { "" },
                has_terminal,
                status).unwrap();

            miners.push(Miner {
                pid: pid_u32,
                cpu_usage: cpu,
                memory: process.memory(),
                status,
                has_terminal,
                name: "Claude Code".to_string(),  // Display name
            });
        }
    }

    writeln!(debug_file, "=== Total miners found: {} ===", miners.len()).unwrap();
    miners
}

#[tauri::command]
fn kill_miner(pid: u32) -> Result<String, String> {
    let _sys_pid = Pid::from_u32(pid);

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output();

        match output {
            Ok(_) => Ok(format!("Process {} killed successfully", pid)),
            Err(e) => Err(format!("Failed to kill process {}: {}", pid, e)),
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output();

        match output {
            Ok(_) => Ok(format!("Process {} killed successfully", pid)),
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
    app_handle: AppHandle,
    total: u32,
    working: u32,
    resting: u32,
    zombie: u32
) -> Result<(), String> {
    use tauri::{SystemTrayMenu, SystemTrayMenuItem};

    let tray = app_handle.tray_handle();

    // Update tooltip
    tray.set_tooltip(&format!("ClaudeMiner - {} sessions", total))
        .map_err(|e| e.to_string())?;

    // Create new menu with stats
    let stats_label = CustomMenuItem::new("stats".to_string(),
        format!("📊 Active Sessions: {}", total)).disabled();
    let working_label = CustomMenuItem::new("working".to_string(),
        format!("⛏️  Working: {}", working)).disabled();
    let resting_label = CustomMenuItem::new("resting".to_string(),
        format!("😴 Resting: {}", resting)).disabled();
    let zombie_label = CustomMenuItem::new("zombie".to_string(),
        format!("👻 Zombie: {}", zombie)).disabled();

    let separator1 = SystemTrayMenuItem::Separator;
    let show = CustomMenuItem::new("show".to_string(), "Show Window");
    let separator2 = SystemTrayMenuItem::Separator;
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(stats_label)
        .add_item(working_label)
        .add_item(resting_label)
        .add_item(zombie_label)
        .add_native_item(separator1)
        .add_item(show)
        .add_native_item(separator2)
        .add_item(quit);

    tray.set_menu(tray_menu)
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn main() {
    // Create CPU cache shared between background thread and main thread
    let cpu_cache: CpuCache = Arc::new(Mutex::new(HashMap::new()));
    let cpu_cache_clone = cpu_cache.clone();

    // Spawn background thread for CPU measurement
    thread::spawn(move || {
        let mut sys = System::new_all();
        loop {
            // Refresh CPU usage for all processes
            sys.refresh_all();

            // Wait for CPU measurement to stabilize (sysinfo requires 2 refreshes)
            thread::sleep(Duration::from_millis(200));
            sys.refresh_all();

            // Update cache with fresh CPU measurements
            let mut cache = cpu_cache_clone.lock().unwrap();
            cache.clear();

            for (pid, process) in sys.processes() {
                let name = process.name().to_string();

                // Only cache Claude Code processes to save memory
                let is_claude = if name.to_lowercase() == "node" {
                    if let Some(cmd) = process.cmd().get(0) {
                        cmd.to_lowercase() == "claude"
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_claude {
                    cache.insert(pid.as_u32(), process.cpu_usage());
                }
            }
            drop(cache); // Release lock

            // Update every 1 second
            thread::sleep(Duration::from_millis(800));
        }
    });

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
        .manage(cpu_cache) // Register CPU cache as Tauri state
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
            uninstall_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
