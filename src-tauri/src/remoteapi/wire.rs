//! The remote API's wire: JSON in text frames (the greeting, the requests and their answers, the events), a terminal's
//! bytes in binary frames under the stream number the client chose (4 bytes, big-endian, then the bytes). An error's
//! `message` is a text, except for `failed`, where it is the command's error as the window gets it: a text, or a JSON
//! structure. A terminal's end is the `terminalExit` event; a frame that is no request closes the connection.
//!
//! A client that comes back after a cut opens a new connection and calls `subscribe`, then reattaches its terminals:
//! - the events that arrive before the `subscribe` answer are kept, then applied on top of its state; a second
//!   `subscribe` starts the view again;
//! - `terminals` is a snapshot: a terminal opened elsewhere raises no event, only `terminalExit` exists;
//! - an entry whose id is an agent's `terminalId` is that agent's view, read with `agent_terminal` and the agent's id
//!   (its size is required, resizes it for everyone and starts the interface if needed); otherwise `launch: true`
//!   is read with `run_attach`, the rest with `term_attach`, whose size is optional (absent: a reader that does not
//!   resize);
//! - a stream replays the kept bytes (up to 4 MiB, in pieces of at most 64 KiB, possibly from mid-screen), then the
//!   live ones; it ends with its terminal (`terminalExit`, which may come before its last bytes) or with the
//!   connection: there is no detach command;
//! - a client that reads slower than its terminals print loses their bytes past its budget: such a stream starts
//!   again with a full reset (`ESC c`), the modes of an interface, then the replay, and follows from there; the
//!   screen so drawn is the replay's, which may start mid-screen: a full-screen program in a shell (vim, htop), whose
//!   modes are not followed, may lose those older than the replay (alternate screen, cursor keys, mouse) until it draws
//!   or sets them again.

use super::table::CallError;
use crate::model::UiEvent;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The version of this wire, which the greeting gives.
pub const PROTOCOL: u32 = 1;

/// A command called by a client: `stream` names where a terminal's bytes go.
#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: u64,
    pub cmd: String,
    #[serde(default)]
    pub args: Value,
    pub stream: Option<u32>,
}

/// The request in `text`; `None` when it is none.
pub fn request(text: &str) -> Option<Request> {
    serde_json::from_str(text).ok()
}

/// The first frame of a connection.
pub fn hello(version: &str) -> String {
    json!({ "hello": { "version": version, "protocol": PROTOCOL } }).to_string()
}

/// The answer to the request `id`.
pub fn answer(id: u64, result: Result<Value, CallError>) -> String {
    let (kind, message) = match result {
        Ok(value) => return json!({ "id": id, "ok": value }).to_string(),
        Err(CallError::Unknown) => ("unknown", "no command has this name".into()),
        Err(CallError::DesktopOnly) => ("desktop", "the desktop app's alone".into()),
        Err(CallError::Args(message)) => ("args", Value::String(message)),
        Err(CallError::Failed(value)) => ("failed", value),
    };
    json!({ "id": id, "err": { "kind": kind, "message": message } }).to_string()
}

#[derive(Serialize)]
struct Pushed<'a> {
    event: &'a UiEvent,
}

/// An event of the core, pushed to a subscribed client.
pub fn event(event: &UiEvent) -> Option<String> {
    serde_json::to_string(&Pushed { event }).ok()
}

/// A terminal's bytes under its stream number, big-endian.
pub fn frame(stream: u32, bytes: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(4 + bytes.len());
    frame.extend_from_slice(&stream.to_be_bytes());
    frame.extend_from_slice(bytes);
    frame
}
