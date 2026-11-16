//! The window's thread (ESC-290). Tauri runs a command without `async` on it, and while one runs
//! the window neither draws nor answers: under WebKitGTK a command that reads a big folder, runs
//! git or waits for a lock froze the whole interface. Every command now leaves it, for a lane of
//! its own (`Lanes`): the calls of one command, or of one family (a terminal's writes, an
//! attachment's chunks), keep their order; a slow one holds back none of the others. A command
//! whose own part lasts more than `SLOW` is written in the log, and a watch on the window's thread
//! (`watch`) writes there any pause of more than `PAUSE`, with the commands running then. Never a
//! path nor a content: a command's name, how long, the ids it was called with.

pub use crate::lanes::{lane_of, Job, Lanes, Running};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::ipc::{Invoke, InvokeBody};
use tauri::Runtime;

/// A window's thread that answers nothing for this long is written in the log.
pub const PAUSE: Duration = Duration::from_secs(1);
/// How often the watch asks the window's thread for an answer.
const BEAT: Duration = Duration::from_millis(250);

/// The ids a command was called with, as the log may hold them: those of its project, agent or
/// ticket, an id being only letters, digits, `-` and `_` (`model::new_id`). A path, a text, a
/// content never: « project 3f2a9c81d0e4 ».
pub fn ids_of(body: &InvokeBody) -> String {
    let InvokeBody::Json(args) = body else {
        return String::new();
    };
    let is_id = |v: &serde_json::Value| {
        v.as_str()
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 40
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            })
            .map(str::to_string)
    };
    let mut out = String::new();
    for (key, told) in [
        ("projectId", "project"),
        ("project", "project"),
        ("agentId", "agent"),
        ("ticketId", "ticket"),
        ("id", "id"),
    ] {
        let Some(v) = args.get(key) else { continue };
        // `update_project` gets the project whole: its id only.
        let id = is_id(v).or_else(|| v.get("id").and_then(is_id));
        if let Some(id) = id {
            out.push_str(&format!(" {told} {id}"));
        }
    }
    if out.is_empty() {
        out
    } else {
        format!(" ({})", out.trim_start())
    }
}

/// The app's handler of the commands, `handler`, run away from the window's thread: each call on
/// the lane of its command (`lane_of`), timed there.
pub fn off_the_window<R: Runtime>(
    lanes: Arc<Lanes>,
    handler: impl Fn(Invoke<R>) -> bool + Send + Sync + 'static,
) -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    let handler = Arc::new(handler);
    move |invoke: Invoke<R>| {
        let name = invoke.message.command().to_string();
        // A launch's page (ESC-311) is the developed app, not Escouade: no command answers it.
        if crate::devpage::refuses(invoke.message.webview_ref().label()) {
            log::warn!("a launch's page called {name}: refused");
            invoke
                .resolver
                .reject(format!("Command {name} not allowed by ACL"));
            return true;
        }
        let (h, l) = (handler.clone(), lanes.clone());
        let lane = lane_of(&name).to_string();
        lanes.submit(
            &lane,
            Box::new(move || {
                let ids = ids_of(invoke.message.payload());
                let resolver = invoke.resolver.clone();
                let started = Instant::now();
                let handled = {
                    let _running = l.running.enter(&name);
                    h(invoke)
                };
                if !handled {
                    resolver.reject(format!("Command {name} not found"));
                }
                if let Some(line) = l.slow_line(&name, started.elapsed(), &ids) {
                    log::warn!("{line}");
                }
            }),
        );
        true
    }
}

/// What the watch saw of the window's thread.
#[derive(Debug, Clone, PartialEq)]
pub enum Pause {
    /// No answer for `PAUSE`: what ran on the lanes then.
    Began { running: String },
    /// The answer came back, `lasted` after it was asked for.
    Ended { lasted: Duration },
}

impl Pause {
    pub fn line(&self) -> String {
        match self {
            Pause::Began { running } if running.is_empty() => format!(
                "window thread: no answer for {} ms, no command running",
                PAUSE.as_millis()
            ),
            Pause::Began { running } => format!(
                "window thread: no answer for {} ms, commands running: {running}",
                PAUSE.as_millis()
            ),
            Pause::Ended { lasted } => {
                format!("window thread: paused {} ms", lasted.as_millis())
            }
        }
    }
}

/// Watches the window's thread until `post` fails (the app is gone): `post` runs its job there.
/// Every `BEAT` a job is posted; one that does not run within `PAUSE` is a pause, told to `tell`
/// when it begins and when it ends.
pub fn watch(
    post: impl Fn(Job) -> bool,
    running: &Running,
    tell: impl Fn(Pause),
    beat: Duration,
    pause: Duration,
) {
    loop {
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let asked = Instant::now();
        if !post(Box::new(move || {
            let _ = tx.send(());
        })) {
            return;
        }
        match rx.recv_timeout(pause) {
            Ok(()) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                tell(Pause::Began {
                    running: running.told(),
                });
                if rx.recv().is_err() {
                    return;
                }
                tell(Pause::Ended {
                    lasted: asked.elapsed(),
                });
            }
            // The job was dropped unrun: the loop of the window is gone.
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }
        std::thread::sleep(beat);
    }
}

/// Starts the watch of the app's window thread, its pauses written in the log.
pub fn watch_app<R: Runtime>(app: tauri::AppHandle<R>, running: Arc<Running>) {
    let spawned = std::thread::Builder::new()
        .name("window-watch".into())
        .spawn(move || {
            watch(
                |job| app.run_on_main_thread(job).is_ok(),
                &running,
                |p| log::warn!("{}", p.line()),
                BEAT,
                PAUSE,
            )
        });
    if let Err(e) = spawned {
        log::warn!("window thread not watched: {e}");
    }
}
