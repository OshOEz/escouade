//! The core's remote API: the same commands as the window's, called with JSON without Tauri (docs/REMOTE.md,
//! « La fondation commune », point 2), and served over a WebSocket on 127.0.0.1 (`server`, point 6).

// Served in debug builds only (`lib.rs`): release builds compile it without starting it.
#[cfg_attr(not(debug_assertions), allow(dead_code))]
pub mod server;
pub mod table;
pub mod wire;
