//! The remote API's wire: JSON in text frames (the greeting, the requests and their answers, the events), a terminal's
//! bytes in binary frames under the stream number the client chose (4 bytes, big-endian, then the bytes). An error's
//! `message` is a text, except for `failed`, where it is the command's error as the window gets it: a text, or a JSON
//! structure. A terminal's end is the `terminalExit` event; a frame that is no request closes the connection.

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
