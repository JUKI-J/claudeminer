// Hook Manager
//
// Manages Claude Code hook registration in settings.json
// Automatically registers hooks on app startup
//

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::PathBuf;

const PIPE_PATH: &str = "/tmp/claudeminer_pipe";

/// Hook configuration for Claude Code
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HookConfig {
    pub matcher: String,
    pub hooks: Vec<Hook>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Hook {
    #[serde(rename = "type")]
    pub hook_type: String,
    pub command: String,
}

/// Claude Code settings structure
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeSettings {
    #[serde(default)]
    pub hooks: HookEvents,
    #[serde(flatten)]
    pub other: Value, // Preserve other settings
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HookEvents {
    #[serde(rename = "SessionStart")]
    pub session_start: Vec<HookConfig>,
    #[serde(rename = "UserPromptSubmit")]
    pub user_prompt_submit: Vec<HookConfig>,
    #[serde(rename = "Stop")]
    pub stop: Vec<HookConfig>,
    #[serde(rename = "SessionEnd")]
    pub session_end: Vec<HookConfig>,
}

/// Get Claude settings.json path
pub fn get_settings_path() -> PathBuf {
    dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".claude")
        .join("settings.json")
}

/// Check if ClaudeMiner hooks are already registered
pub fn has_claudeminer_hooks(settings: &ClaudeSettings) -> bool {
    // Check if any hook contains our pipe path
    let check_hooks = |configs: &[HookConfig]| {
        configs.iter().any(|config| {
            config.hooks.iter().any(|hook| {
                hook.command.contains(PIPE_PATH)
            })
        })
    };

    check_hooks(&settings.hooks.session_start) ||
    check_hooks(&settings.hooks.user_prompt_submit) ||
    check_hooks(&settings.hooks.stop) ||
    check_hooks(&settings.hooks.session_end)
}

/// Read Claude settings.json
pub fn read_settings() -> io::Result<ClaudeSettings> {
    let path = get_settings_path();

    if !path.exists() {
        // Create default settings if not exists
        let default_settings = ClaudeSettings {
            hooks: HookEvents::default(),
            other: json!({}),
        };
        return Ok(default_settings);
    }

    let contents = fs::read_to_string(&path)?;

    // Parse JSON, preserving unknown fields
    let settings: ClaudeSettings = serde_json::from_str(&contents)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(settings)
}

/// Write Claude settings.json with backup
pub fn write_settings(settings: &ClaudeSettings) -> io::Result<()> {
    let path = get_settings_path();

    // Create backup if file exists
    if path.exists() {
        let backup_path = path.with_extension("json.backup");
        fs::copy(&path, &backup_path)?;
        println!("[HookManager] Created backup at {:?}", backup_path);
    }

    // Ensure .claude directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write settings with pretty formatting
    let json_str = serde_json::to_string_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(&path, json_str)?;
    println!("[HookManager] Updated settings.json at {:?}", path);

    Ok(())
}

/// Create ClaudeMiner hook commands
fn create_hook_command(event_name: &str) -> String {
    format!(
        "echo '{{\"sid\":\"$SESSION_ID\",\"evt\":\"{}\"}}' > {}",
        event_name, PIPE_PATH
    )
}

/// Register ClaudeMiner hooks
pub fn register_hooks() -> io::Result<()> {
    println!("[HookManager] Registering ClaudeMiner hooks...");

    let mut settings = read_settings()?;

    // Create our hook config
    let claudeminer_hooks = vec![
        Hook {
            hook_type: "command".to_string(),
            command: String::new(), // Will be set per event
        }
    ];

    // Helper to add or update hook
    let mut add_hook = |configs: &mut Vec<HookConfig>, event_name: &str| {
        // Remove existing ClaudeMiner hooks if any
        configs.retain(|config| {
            !config.hooks.iter().any(|h| h.command.contains(PIPE_PATH))
        });

        // Add new hook
        let mut hook = claudeminer_hooks[0].clone();
        hook.command = create_hook_command(event_name);

        configs.push(HookConfig {
            matcher: "*".to_string(), // Apply to all tools
            hooks: vec![hook],
        });
    };

    // Register hooks for each event
    add_hook(&mut settings.hooks.session_start, "start");
    add_hook(&mut settings.hooks.user_prompt_submit, "working");
    add_hook(&mut settings.hooks.stop, "resting");
    add_hook(&mut settings.hooks.session_end, "end");

    // Write updated settings
    write_settings(&settings)?;

    println!("[HookManager] Successfully registered ClaudeMiner hooks");
    Ok(())
}

/// Unregister ClaudeMiner hooks (for cleanup)
pub fn unregister_hooks() -> io::Result<()> {
    println!("[HookManager] Unregistering ClaudeMiner hooks...");

    let mut settings = read_settings()?;

    // Helper to remove ClaudeMiner hooks
    let remove_hooks = |configs: &mut Vec<HookConfig>| {
        configs.retain(|config| {
            !config.hooks.iter().any(|h| h.command.contains(PIPE_PATH))
        });
    };

    // Remove hooks from each event
    remove_hooks(&mut settings.hooks.session_start);
    remove_hooks(&mut settings.hooks.user_prompt_submit);
    remove_hooks(&mut settings.hooks.stop);
    remove_hooks(&mut settings.hooks.session_end);

    // Write updated settings
    write_settings(&settings)?;

    println!("[HookManager] Successfully unregistered ClaudeMiner hooks");
    Ok(())
}

/// Ensure hooks are registered (idempotent)
pub fn ensure_hooks_registered() -> io::Result<()> {
    let settings = read_settings()?;

    if has_claudeminer_hooks(&settings) {
        println!("[HookManager] ClaudeMiner hooks already registered");
        Ok(())
    } else {
        println!("[HookManager] ClaudeMiner hooks not found, registering...");
        register_hooks()
    }
}

/// Verify hook registration by checking settings
pub fn verify_hooks() -> io::Result<bool> {
    let settings = read_settings()?;
    let registered = has_claudeminer_hooks(&settings);

    if registered {
        println!("[HookManager] ✓ Hooks are properly registered");
    } else {
        println!("[HookManager] ✗ Hooks are not registered");
    }

    Ok(registered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hook_registration() {
        // Create temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.json");

        // Test with mock settings path (would need to refactor to accept custom path)
        // For now, just test the hook command creation
        let cmd = create_hook_command("start");
        assert!(cmd.contains("\"evt\":\"start\""));
        assert!(cmd.contains(PIPE_PATH));
    }

    #[test]
    fn test_has_claudeminer_hooks() {
        let mut settings = ClaudeSettings {
            hooks: HookEvents::default(),
            other: json!({}),
        };

        // Initially no hooks
        assert!(!has_claudeminer_hooks(&settings));

        // Add a ClaudeMiner hook
        settings.hooks.session_start.push(HookConfig {
            matcher: "*".to_string(),
            hooks: vec![Hook {
                hook_type: "command".to_string(),
                command: format!("echo 'test' > {}", PIPE_PATH),
            }],
        });

        // Now should have hooks
        assert!(has_claudeminer_hooks(&settings));
    }
}