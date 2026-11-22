//! The core's remote API: the same commands as the window's, called with JSON without Tauri (docs/REMOTE.md,
//! « La fondation commune », point 2), and served over a WebSocket on 127.0.0.1 (`server`, point 6), beside the files
//! of the web build (`web`).

pub mod devices;
pub mod lockfile;
pub mod server;
pub mod table;
pub mod web;
pub mod wire;
