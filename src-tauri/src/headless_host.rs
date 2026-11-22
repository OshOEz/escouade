//! `escouade-server`: the core without a window (docs/REMOTE.md, « Le mode serveur »), its commands and the web build
//! served on 127.0.0.1 (`remoteapi::server`) to the paired devices (`remoteapi::devices`), stopped by SIGINT or SIGTERM.
//! `Headless` is its host. `pair` and `devices` ask the server that runs on the same data folder (`server::Control`), of
//! which there is one (`lockfile`). `install` writes its systemd unit of the user, `uninstall` removes it.

use crate::core::Core;
use crate::host::Host;
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
use crate::model::{Settings, UiEvent};
use crate::remoteapi::devices::{Device, Devices};
use crate::remoteapi::lockfile::Lock;
use crate::remoteapi::server::{Control, CONTROL};
use crate::{paths, remoteapi};
use escouade_base::rt;
use escouade_model::agentgroups;
use std::path::{Path, PathBuf};
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

const USAGE: &str = "usage: escouade-server (--socket <path> | --port <n>) [--web <folder>] [--origin <url>]...\n\
                     \x20        serve on a Unix socket (in a folder of yours alone: on a machine shared with others)\n\
                     \x20        or on 127.0.0.1's port (ESCOUADE_REMOTE_PORT without either), also for the pages\n\
                     \x20        of each origin (https://name, a proxy's) besides the loopback's\n\
                     \x20      escouade-server pair                  a code to pair a browser\n\
                     \x20      escouade-server devices               the paired devices\n\
                     \x20      escouade-server devices revoke <id>   forget one, its connections closed\n\
                     \x20      escouade-server install [--user] [--socket <path> | --port <n>] [--web <folder>] [--origin <url>]...\n\
                     \x20        write the systemd unit of the user (on %t/escouade/server.sock by default)\n\
                     \x20      escouade-server uninstall             remove it";

/// The unit `install` writes, in the user's systemd folder.
const UNIT: &str = "escouade-server.service";
/// The socket of the unit when `install` is given no other: in `$XDG_RUNTIME_DIR`, the user's alone.
const UNIT_SOCKET: &str = "%t/escouade/server.sock";

/// Where and what `escouade-server` serves, as its arguments say.
#[derive(Debug, Default)]
struct Serve {
    port: Option<u16>,
    socket: Option<PathBuf>,
    web: Option<PathBuf>,
    origins: Vec<String>,
}

/// The arguments of `escouade-server` and `install`; `Err` is what to print (code 2).
fn parse_serve(args: &[&str]) -> Result<Serve, String> {
    let (mut serve, mut rest) = (Serve::default(), args);
    let absolute =
        |p: &str| std::path::absolute(p).map_err(|e| format!("escouade-server: {p}: {e}"));
    while !rest.is_empty() {
        rest = match rest {
            ["--web", dir, tail @ ..] if serve.web.is_none() => {
                let dir = absolute(dir)?;
                if !dir.is_dir() {
                    return Err(format!(
                        "escouade-server: {} is not a folder",
                        dir.display()
                    ));
                }
                serve.web = Some(dir);
                tail
            }
            ["--origin", url, tail @ ..] => {
                let origin = remoteapi::server::parse_origin(url);
                serve
                    .origins
                    .push(origin.map_err(|why| format!("escouade-server: {why}"))?);
                tail
            }
            ["--port", port, tail @ ..] if serve.port.is_none() && serve.socket.is_none() => {
                let n = port.parse::<u16>().ok().filter(|p| *p != 0);
                serve.port = Some(n.ok_or(format!(
                    "escouade-server: --port {port:?} is not a port from 1 to 65535"
                ))?);
                tail
            }
            ["--socket", path, tail @ ..] if serve.port.is_none() && serve.socket.is_none() => {
                if cfg!(not(unix)) {
                    return Err(
                        "escouade-server: --socket is for Unix systems: listen on --port <n>"
                            .into(),
                    );
                }
                serve.socket = Some(absolute(path)?);
                tail
            }
            _ => return Err(USAGE.into()),
        };
    }
    Ok(serve)
}

/// The program: the exit code.
pub fn run(args: Vec<String>) -> i32 {
    // First, before any thread: no child process may inherit the token.
    let remote = remoteapi::server::take_port_from_env();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["pair"] | ["devices"] | ["devices", "revoke", _] => return command(&args),
        ["install", rest @ ..] => return install(rest),
        ["uninstall"] => return uninstall(),
        _ => {}
    }
    let serve = match parse_serve(&args) {
        Ok(serve) => serve,
        Err(why) => {
            eprintln!("{why}");
            return 2;
        }
    };
    let mut config = match remote {
        Ok(config) => config,
        Err(why) => {
            eprintln!("escouade-server: {why}");
            return 2;
        }
    };
    // The arguments before the environment's port.
    if let Some(port) = serve.port {
        config.port = port;
    }
    config.socket = serve.socket;
    if config.port == 0 && config.socket.is_none() {
        eprintln!("escouade-server: where to listen? --socket <path> (on a machine shared with others) or --port <n>");
        eprintln!("{USAGE}");
        return 2;
    }
    config.web = serve.web;
    config.origins = serve.origins;
    // One server per data folder, before anything is read or written in it: the lock goes with the process.
    let dir = paths::default_data_dir();
    let _lock = match Lock::take(&dir) {
        Ok(lock) => lock,
        Err(why) => {
            eprintln!("escouade-server: {why}");
            // Its own code: the unit does not restart for it (`RestartPreventExitStatus`).
            return 3;
        }
    };
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
    let data = paths::DataDir::new(dir);
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
    match &running.socket {
        Some(socket) => log::info!("remote API listening on {}", socket.display()),
        None => log::info!("remote API listening on {}", running.addr),
    }
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
            match (control.origins.first(), &control.socket) {
                (Some(origin), _) => println!("Open {origin}/ and enter the code:\n\n    {code}\n"),
                (None, Some(socket)) => println!(
                    "From your computer, a tunnel to this server's socket:\n\n    ssh -N -L 8765:{} <this server>\n\n\
                     then open http://127.0.0.1:8765/ and enter the code:\n\n    {code}\n",
                    socket.display()
                ),
                (None, None) => println!(
                    "Open http://127.0.0.1:{}/ and enter the code:\n\n    {code}\n",
                    control.port
                ),
            }
            println!("It pairs one device, once, within {minutes} minutes.");
            println!("Five wrong codes cancel it: then run `escouade-server pair` again.");
            if control.socket.is_none() {
                println!(
                    "\nThis server listens on a port of 127.0.0.1: another user of this machine may take it while the server\n\
                     restarts, and get your browser's token. On a machine shared with others, serve on --socket instead\n\
                     (docs/REMOTE.md)."
                );
            }
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
    let wait = Some(std::time::Duration::from_secs(30));
    // On the socket, any port of the loopback names the server (a tunnel's).
    let (port, at, mut stream): (u16, String, Box<dyn ReadWrite>) = match &control.socket {
        #[cfg(unix)]
        Some(socket) => {
            let unix = std::os::unix::net::UnixStream::connect(socket).map_err(|_| down())?;
            let _ = unix.set_read_timeout(wait);
            (80, socket.display().to_string(), Box::new(unix))
        }
        #[cfg(not(unix))]
        Some(_) => return Err(down()),
        None => {
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], control.port));
            let tcp =
                std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(5))
                    .map_err(|_| down())?;
            let _ = tcp.set_read_timeout(wait);
            (
                control.port,
                format!("port {}", control.port),
                Box::new(tcp),
            )
        }
    };
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {}\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        control.key,
        body.len()
    );
    let mut answer = Vec::new();
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.read_to_end(&mut answer))
        .map_err(|e| format!("the server on {at} did not answer: {e}"))?;
    let answer = String::from_utf8_lossy(&answer);
    let (head, json) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
    let status = head.split(' ').nth(1).unwrap_or_default();
    if status != "200" {
        // Another program on that port, or a server started since with another key.
        return Err(format!(
            "the server on {at} refused ({status}): is it this data folder's?"
        ));
    }
    let json = serde_json::from_str(json)
        .map_err(|e| format!("the server on {at} answered badly: {e}"))?;
    Ok((control, json))
}

/// A connection to the server, by its port or its socket.
trait ReadWrite: std::io::Read + std::io::Write {}
impl<T: std::io::Read + std::io::Write> ReadWrite for T {}

/// The user's unit: `~/.config/systemd/user/escouade-server.service` (or under `$XDG_CONFIG_HOME`).
fn unit_path() -> Result<PathBuf, String> {
    let config = dirs::config_dir().ok_or("escouade-server: no home folder to install in")?;
    Ok(config.join("systemd").join("user").join(UNIT))
}

/// `install [--user] [serve's arguments]`: the unit written, what to run then said; nothing run.
fn install(args: &[&str]) -> i32 {
    // The unit of the user is the only one: `--user` says it, and changes nothing.
    let args: Vec<&str> = args.iter().copied().filter(|a| *a != "--user").collect();
    if !cfg!(target_os = "linux") {
        eprintln!("escouade-server: install writes a unit of systemd, on Linux");
        return 2;
    }
    let written = parse_serve(&args).and_then(|serve| {
        let exe = std::env::current_exe().and_then(|e| e.canonicalize());
        let exe = exe.map_err(|e| format!("escouade-server: where am I? {e}"))?;
        let data = paths::sandbox_dir()
            .map(std::path::absolute)
            .transpose()
            .map_err(|e| format!("escouade-server: ESCOUADE_DATA_DIR: {e}"))?;
        let text = unit(&exe, &serve, data.as_deref())?;
        let path = unit_path()?;
        let cannot =
            |e: std::io::Error| format!("escouade-server: cannot write {}: {e}", path.display());
        std::fs::create_dir_all(path.parent().expect("a folder")).map_err(cannot)?;
        paths::write_private(&path, text.as_bytes()).map_err(cannot)?;
        Ok(path)
    });
    match written {
        Ok(path) => {
            println!("Wrote {}. Then:\n", path.display());
            println!("    systemctl --user daemon-reload");
            println!("    systemctl --user enable --now escouade-server");
            println!("    loginctl enable-linger    # it runs while you are logged out, and starts with the machine\n");
            println!(
                "Already running: `systemctl --user restart escouade-server` takes the new unit."
            );
            println!("Then `escouade-server pair` gives the code to pair a browser.");
            0
        }
        Err(why) => {
            eprintln!("{why}");
            2
        }
    }
}

/// `uninstall`: the unit removed, unless it is still enabled.
fn uninstall() -> i32 {
    let path = match unit_path() {
        Ok(path) => path,
        Err(why) => {
            eprintln!("{why}");
            return 1;
        }
    };
    let wants = path.with_file_name("default.target.wants").join(UNIT);
    if wants.symlink_metadata().is_ok() {
        eprintln!("escouade-server: the service is enabled: `systemctl --user disable --now escouade-server` first");
        return 1;
    }
    match std::fs::remove_file(&path) {
        Ok(()) => {
            println!(
                "Removed {}. Then: systemctl --user daemon-reload",
                path.display()
            );
            0
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("No service installed ({}).", path.display());
            0
        }
        Err(e) => {
            eprintln!("escouade-server: cannot remove {}: {e}", path.display());
            1
        }
    }
}

/// The unit of `exe` serving as `serve` says, on the data folder `data` if not the default one.
fn unit(exe: &Path, serve: &Serve, data: Option<&Path>) -> Result<String, String> {
    let mut exec = vec![word(text(exe)?, true)?];
    match (serve.port, &serve.socket) {
        (Some(port), _) => exec.extend(["--port".into(), port.to_string()]),
        (None, Some(socket)) => exec.extend(["--socket".into(), word(text(socket)?, true)?]),
        (None, None) => exec.extend(["--socket".into(), UNIT_SOCKET.into()]),
    }
    if let Some(web) = &serve.web {
        exec.extend(["--web".into(), word(text(web)?, true)?]);
    }
    for origin in &serve.origins {
        exec.extend(["--origin".into(), word(origin, true)?]);
    }
    let environment = match data {
        Some(dir) => {
            let assignment = format!("ESCOUADE_DATA_DIR={}", text(dir)?);
            format!("Environment={}\n", word(&assignment, false)?)
        }
        None => String::new(),
    };
    Ok(format!(
        "# Written by `escouade-server install`, removed by `escouade-server uninstall` (docs/REMOTE.md).\n\
         [Unit]\n\
         Description=Escouade server: the core and its web interface, for the browsers paired to it\n\
         \n\
         [Service]\n\
         {environment}\
         ExecStart={}\n\
         # On SIGTERM the server stops its agents itself, as the app does when it quits; what is left is then killed.\n\
         KillMode=mixed\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         # Wrong arguments (2) or the data folder another server's (3): restarting would not change it.\n\
         RestartPreventExitStatus=2 3\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exec.join(" ")
    ))
}

/// `p` as text, as a unit is written: a path that is not UTF-8 would be written wrong.
fn text(p: &Path) -> Result<&str, String> {
    p.to_str().ok_or_else(|| {
        format!(
            "escouade-server: {} is not UTF-8: a unit cannot name it",
            p.display()
        )
    })
}

/// `s` as one quoted word of a unit: its specifiers (`%`) and, in a command line (`command`), its variables (`$`)
/// written as themselves. A control character cannot be written.
fn word(s: &str, command: bool) -> Result<String, String> {
    if s.chars().any(char::is_control) {
        return Err(format!(
            "escouade-server: {s:?} cannot be written in a unit"
        ));
    }
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '\\' | '"' => out.extend(['\\', c]),
            '%' => out.push_str("%%"),
            '$' if command => out.push_str("$$"),
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(out)
}

#[cfg(test)]
#[path = "headless_host_tests.rs"]
mod tests;
