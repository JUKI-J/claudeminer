// Hooks Module - Claude Code hook management
//
// This module handles registration and receiving of Claude Code hooks

pub mod manager;
pub mod receiver;
pub mod sender;

// pub use manager::{ensure_hooks_registered, HookConfig}; // Unused
pub use receiver::start_hook_receiver;
// pub use receiver::{start_hook_receiver_with_config, ReceiverConfig}; // Unused
// pub use sender::send_process_killed_event; // Unused