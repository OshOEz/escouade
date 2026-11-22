//! `escouade-server`: the core without a window (docs/REMOTE.md, « Le mode serveur »), its commands and the web build
//! served on 127.0.0.1 (`remoteapi::server`), stopped by SIGINT or SIGTERM. `Headless` is its host.

use crate::core::Core;
use crate::host::Host;
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
use crate::model::{Settings, UiEvent};
use crate::{paths, remoteapi};
use escouade_base::rt;
use escouade_model::agentgroups;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, Weak};

/// The host of a core with no window, tray, sound or voice: what the clients are told goes through the hub.
#[derive(Default)]
pub struct Headless {
    /// The core, once loaded: its hub carries the toasts.
    core: OnceLock<Weak<Core>>,
}

impl Host for Headless {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    // shortcut: no plugin seed beside the program, until the server ships one.
    fn seed_dir(&self) -> Option<PathBuf> {
        None
    }

    fn webview_version(&self) -> Option<String> {
        None
    }

    fn window_attended(&self) -> bool {
        false
    }

    fn attention(&self) {}

    fn chime(&self) {}

    /// Said to the clients in passing, as the window says the background's news; nothing to click.
    fn toast(&self, title: &str, body: &str, _: Box<dyn Fn() + Send + Sync>) {
        if let Some(core) = self.core.get().and_then(Weak::upgrade) {
            let text = format!("{title} — {body}");
            core.hub.emit(UiEvent::Toast { text });
        }
    }

    fn waiting(&self, _: usize) {}

    fn relabel(&self, _: Lang, _: usize) {}

    fn stop_voice(&self) {}

    fn cancel_dictation(&self) {}

    fn tune_conversation(&self, _: &Settings, _: &dyn SecretStore) {}
}

const USAGE: &str = "usage: escouade-server [--web <folder>]\n\
                     ESCOUADE_REMOTE_PORT and ESCOUADE_REMOTE_TOKEN (32 characters or more) must be set.";

/// The program: the exit code.
pub fn run(args: Vec<String>) -> i32 {
    // First, before any thread: no child process may inherit the token.
    let remote = remoteapi::server::take_from_env();
    let web = match args.as_slice() {
        [] => None,
        [flag, dir] if flag == "--web" => Some(PathBuf::from(dir)),
        _ => {
            eprintln!("{USAGE}");
            return 2;
        }
    };
    let mut config = match remote {
        Ok(Some(config)) => config,
        Ok(None) => {
            eprintln!("escouade-server: no token\n{USAGE}");
            return 2;
        }
        Err(why) => {
            eprintln!("escouade-server: {why}");
            return 2;
        }
    };
    if let Some(dir) = &web {
        if !dir.is_dir() {
            eprintln!("escouade-server: {} is not a folder", dir.display());
            return 2;
        }
    }
    config.web = web;
    // Caught before anything starts: a signal during the start-up still stops the core. Listened to once PATH is written,
    // which no other thread may read meanwhile: catching makes none.
    let caught = catch_stop();
    #[cfg(unix)]
    crate::shellenv::adopt_login_path();
    #[cfg(windows)]
    crate::syspath::refresh();
    let stop = caught.listen();
    crate::init_logging();
    for note in paths::take_migration_notes() {
        log::info!("migration: {note}");
    }
    log::info!("escouade-server {} starting", env!("CARGO_PKG_VERSION"));
    let data = paths::DataDir::new(paths::default_data_dir());
    // The process groups of agents a killed run left behind, stopped before any agent starts.
    agentgroups::start(data.root().join("agent-groups.json"));
    let host = Arc::new(Headless::default());
    let (core, git_rx) = Core::load(host.clone(), data);
    let _ = host.core.set(Arc::downgrade(&core));
    core.start(git_rx);
    let running = match remoteapi::server::start(core.clone(), remoteapi::table::entries(), config)
    {
        Ok(running) => running,
        Err(e) => {
            log::error!("remote API not started: {e}");
            eprintln!("escouade-server: cannot listen: {e}");
            core.shutdown();
            return 1;
        }
    };
    log::info!("remote API listening on {}", running.addr);
    // The Principals of settings saved before 1.8.1 kept or dropped, once (ESC-128).
    let migrating = core.clone();
    rt::spawn(async move { migrating.migrate_accounts().await });
    let warm = core.clone();
    rt::spawn_blocking(move || warm.warm_claude());
    core.read_all_account_models();
    // Claude Code updated by Escouade, apart from its agents (ESC-133).
    crate::selfupdate::watch(core.clone());
    let signal = rt::block_on(stop).unwrap_or("?");
    log::info!("escouade-server stopping on {signal}");
    running.stop();
    core.shutdown();
    log::info!("core stopped");
    0
}

/// SIGINT and SIGTERM caught, on a runtime of their own with no thread: none is told until `listen`.
#[cfg(unix)]
struct Caught {
    runtime: tokio::runtime::Runtime,
    int: tokio::signal::unix::Signal,
    term: tokio::signal::unix::Signal,
}

#[cfg(unix)]
fn catch_stop() -> Caught {
    use tokio::signal::unix::{signal, SignalKind};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("the signals' runtime");
    let (int, term) = {
        let _in = runtime.enter();
        (
            signal(SignalKind::interrupt()).expect("the SIGINT handler"),
            signal(SignalKind::terminate()).expect("the SIGTERM handler"),
        )
    };
    Caught { runtime, int, term }
}

#[cfg(unix)]
impl Caught {
    /// The first signal, caught since `catch_stop`, told on the receiver; a second, while stopping, ends the process at
    /// once (130).
    fn listen(self) -> tokio::sync::oneshot::Receiver<&'static str> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let Caught {
            runtime,
            mut int,
            mut term,
        } = self;
        std::thread::spawn(move || {
            runtime.block_on(async move {
                let mut tx = Some(tx);
                loop {
                    let name = tokio::select! {
                        _ = int.recv() => "SIGINT",
                        _ = term.recv() => "SIGTERM",
                    };
                    let Some(tx) = tx.take() else {
                        eprintln!(
                            "escouade-server: {name} again, leaving without stopping the core"
                        );
                        std::process::exit(130);
                    };
                    eprintln!("escouade-server: stopping on {name}");
                    let _ = tx.send(name);
                }
            })
        });
        rx
    }
}

/// Elsewhere, Ctrl+C, listened to on the app's runtime from `listen`.
#[cfg(not(unix))]
struct Caught;

#[cfg(not(unix))]
fn catch_stop() -> Caught {
    Caught
}

#[cfg(not(unix))]
impl Caught {
    fn listen(self) -> tokio::sync::oneshot::Receiver<&'static str> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        rt::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            eprintln!("escouade-server: stopping on Ctrl+C");
            let _ = tx.send("Ctrl+C");
            let _ = tokio::signal::ctrl_c().await;
            eprintln!("escouade-server: Ctrl+C again, leaving without stopping the core");
            std::process::exit(130);
        });
        rx
    }
}
