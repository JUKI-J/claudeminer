// ClaudeMiner Type Definitions
//
// This module contains all shared data structures and type aliases
// used throughout the application.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Represents a Claude Code process (miner)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Miner {
    pub pid: u32,
    pub cpu_usage: f32,
    pub memory: u64,
    pub status: String,
    pub has_terminal: bool,
    pub name: String,
}

/// Working state of a Claude Code session
#[derive(Debug, Clone, Copy, Serialize)]
#[allow(dead_code)]
pub enum WorkingState {
    ActivelyWorking,      // Tool execution detected
    GeneratingResponse,   // Stream only (text generation)
    Idle,                 // Only hook checks
    Unknown,              // Cannot determine
}

// Type aliases for shared state (kept for future use)
#[allow(dead_code)]
pub type CpuCache = Arc<Mutex<HashMap<u32, f32>>>;
#[allow(dead_code)]
pub type SessionCache = Arc<Mutex<HashMap<u32, String>>>; // PID -> session_id
#[allow(dead_code)]
pub type StatusDebouncer = Arc<Mutex<HashMap<u32, (String, u8)>>>; // PID -> (status, count)
#[allow(dead_code)]
pub type NetworkDebouncer = Arc<Mutex<HashMap<u32, u8>>>; // PID -> network_count
