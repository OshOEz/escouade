//! `escouade-server`: the core without a window (docs/REMOTE.md, « Le mode serveur »), its commands and the web build
//! served on 127.0.0.1 (`remoteapi::server`) to the paired devices (`remoteapi::devices`), stopped by SIGINT or SIGTERM.
//! `Headless` is its host. `pair` and `devices` ask the server that runs on the same data folder (`server::Control`).

use crate::core::Core;
use crate::host::Host;
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
use crate::model::{Settings, UiEvent};
use crate::remoteapi::devices::{Device, Devices};
use crate::remoteapi::server::{Control, CONTROL};
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

const USAGE: &str = "usage: escouade-server [--web <folder>] [--origin <url>]...\n\
                     \x20        serve, on 127.0.0.1 at ESCOUADE_REMOTE_PORT, also for the pages of each origin\n\
                     \x20        (https://name, a proxy's) besides the loopback's\n\
                     \x20      escouade-server pair                  a code to pair a browser\n\
                     \x20      escouade-server devices               the paired devices\n\
                     \x20      escouade-server devices revoke <id>   forget one, its connections closed";

/// The program: the exit code.
pub fn run(args: Vec<String>) -> i32 {
    // First, before any thread: no child process may inherit the token.
    let remote = remoteapi::server::take_port_from_env();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    if let ["pair"] | ["devices"] | ["devices", "revoke", _] = args.as_slice() {
        return command(&args);
    }
    let (mut web, mut origins, mut rest) = (None, Vec::new(), args.as_slice());
    while !rest.is_empty() {
        match rest {
            ["--web", dir, tail @ ..] if web.is_none() => {
                web = Some(PathBuf::from(dir));
                rest = tail;
            }
            ["--origin", url, tail @ ..] => match remoteapi::server::parse_origin(url) {
                Ok(origin) => {
                    origins.push(origin);
                    rest = tail;
                }
                Err(why) => {
                    eprintln!("escouade-server: {why}");
                    return 2;
                }
            },
            _ => {
                eprintln!("{USAGE}");
                return 2;
            }
        }
    }
    let mut config = match remote {
        Ok(config) => config,
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
    config.origins = origins;
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
    for origin in &config.origins {
        log::info!("remote API: the pages of {origin} accepted");
        let host = origin.trim_start_matches("http://");
        let local = ["127.0.0.1", "localhost", "[::1]"]
            .iter()
            .any(|l| host == *l || host.starts_with(&format!("{l}:")));
        if origin.starts_with("http://") && !local {
            log::warn!("remote API: {origin} is plain http: the devices' tokens cross the network unencrypted, use https");
        }
    }
    let data = paths::DataDir::new(paths::default_data_dir());
    config.devices = Some(Arc::new(Devices::open(data.root())));
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

/// `pair`, `devices`, `devices revoke <id>`: asked of the server that runs on this data folder.
fn command(args: &[&str]) -> i32 {
    let dir = paths::default_data_dir();
    let asked = match args {
        ["pair"] => ask(&dir, "POST", "/admin/pair", "").map(|(control, answer)| {
            let code = answer["code"].as_str().unwrap_or_default();
            let minutes = answer["minutes"].as_u64().unwrap_or_default();
            let local = format!("http://127.0.0.1:{}", control.port);
            let address = control.origins.first().unwrap_or(&local);
            println!("Open {address}/ and enter the code:\n\n    {code}\n");
            println!("It pairs one device, once, within {minutes} minutes.");
            println!("Five wrong codes cancel it: then run `escouade-server pair` again.");
        }),
        ["devices"] => ask(&dir, "GET", "/admin/devices", "").map(|(_, answer)| {
            let devices: Vec<Device> = serde_json::from_value(answer).unwrap_or_default();
            if devices.is_empty() {
                println!("No device paired: `escouade-server pair` gives a code.");
            }
            let day = |t: chrono::DateTime<chrono::Utc>| t.format("%Y-%m-%d %H:%M UTC").to_string();
            for d in devices {
                let seen = d.last_seen.map_or("never".into(), day);
                println!(
                    "{}  {}  paired {}, last seen {seen}",
                    d.id,
                    d.name,
                    day(d.paired_at)
                );
            }
        }),
        ["devices", "revoke", id] => {
            ask(&dir, "POST", "/admin/revoke", id).and_then(|(_, answer)| {
                if let Some(why) = answer["unwritten"].as_str() {
                    return Err(format!(
                        "{id} is cut until the server restarts only ({why}): fix it and revoke again"
                    ));
                }
                if answer["revoked"] == true {
                    println!("{id} revoked: its connections are closed.");
                } else {
                    println!("No device {id}: `escouade-server devices` lists them.");
                }
                Ok(())
            })
        }
        _ => unreachable!("the commands `run` lets through"),
    };
    match asked {
        Ok(()) => 0,
        Err(why) => {
            eprintln!("escouade-server: {why}");
            1
        }
    }
}

/// `method path` with `body`, asked of the server of `dir` with its key: what it wrote of itself, and its answer.
// shortcut: plain HTTP/1.1 by hand, the server answers one request and closes; a client crate if this grows.
fn ask(
    dir: &std::path::Path,
    method: &str,
    path: &str,
    body: &str,
) -> Result<(Control, serde_json::Value), String> {
    use std::io::{Read, Write};
    let down = || {
        format!(
            "no escouade-server runs on {}: start it first",
            dir.display()
        )
    };
    let control = std::fs::read(dir.join(CONTROL)).map_err(|_| down())?;
    let control: Control = serde_json::from_slice(&control).map_err(|_| down())?;
    let port = control.port;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let mut tcp = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5))
        .map_err(|_| down())?;
    let _ = tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        control.key,
        body.len()
    );
    let mut answer = Vec::new();
    tcp.write_all(request.as_bytes())
        .and_then(|()| tcp.read_to_end(&mut answer))
        .map_err(|e| format!("the server on port {port} did not answer: {e}"))?;
    let answer = String::from_utf8_lossy(&answer);
    let (head, json) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
    let status = head.split(' ').nth(1).unwrap_or_default();
    if status != "200" {
        // Another program on that port, or a server started since with another key.
        return Err(format!(
            "the server on port {port} refused ({status}): is it this data folder's?"
        ));
    }
    let json = serde_json::from_str(json)
        .map_err(|e| format!("the server on port {port} answered badly: {e}"))?;
    Ok((control, json))
}
