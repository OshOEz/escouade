//! The core's commands served over a WebSocket on 127.0.0.1 (docs/REMOTE.md, « La fondation commune », points 2 and
//! 6): off unless `ESCOUADE_REMOTE_PORT` and `ESCOUADE_REMOTE_TOKEN` are set, both taken out of the environment at
//! start-up so that no child process inherits the token; the desktop app reads them in debug builds only.
//! `escouade-server` reads the port always and the token in debug builds only (its tests): its clients are the devices
//! paired by a code (`devices`, `POST /pair`), each with its own token, and its own commands (`pair`, `devices`) come
//! with the key it writes beside them (`CONTROL`, `/admin/…`); it may listen on a Unix socket instead of the port, in a
//! folder of the user's alone, where no other user of the machine can take its place. Every request names this server; one from a page comes
//! from the page it serves (`--web`, `web`). A connection brings a token, by `Authorization: Bearer` or, from a page,
//! by the WebSocket's subprotocol `bearer.<token>`: never a cookie, which a browser would send to every port of the
//! host. A device revoked has its connections closed at once. A connection's slot is taken before its request, so a local process may hold all 16 for up to 10 s each (acceptable for now). Its calls
//! of one family keep their order, as the window's do (`windowthread`). The eight commands with a flow (`subscribe`
//! and the terminals) are served here, the others by the table. A client's stream commands start one at a time, each
//! waiting at most 10 s for half of its 16 MiB budget of terminal bytes, and give up when the connection ends; a
//! client that does not read is closed: a queue that fills, or a socket write that hangs, closes it. A remote `subscribe` adds the connection to the hub's
//! subscribers, the window keeping its own, until the connection ends; a second one takes the place of the first. A
//! terminal's stream reads its output beside the window, from what it kept, until the connection ends. A remote
//! `save_settings` retunes the desktop's conversation. The bytes of files go beside the WebSocket, by HTTP (`files`).
use super::devices::{self, Devices};
use super::table::{self, CallError, Ctx, Entry, ResultTag};
use super::{files, web, wire};
use crate::commands::{self, CoreState};
use crate::core::Core;
use crate::hub::Subscription;
use crate::lanes::{lane_of, Lanes};
use crate::model::UiEvent;
use crate::output::{Reader, Viewer};
use crate::sink::{Closed, Sink};
use escouade_base::rt;
use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full, Limited};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::OnUpgrade;
use hyper_util::rt::{TokioIo, TokioTimer};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio::sync::{watch, Notify, OwnedSemaphorePermit, Semaphore};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::http::{header, HeaderMap, Method, StatusCode};
use tokio_tungstenite::tungstenite::protocol::{Role, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

type Request = hyper::Request<Incoming>;
/// A WebSocket's connection once upgraded, and the device it comes from (none: the environment's token).
type Upgrade = Mutex<Option<(OnUpgrade, Option<String>)>>;
type Response = hyper::Response<Full<Bytes>>;

const PORT_VAR: &str = "ESCOUADE_REMOTE_PORT";
const TOKEN_VAR: &str = "ESCOUADE_REMOTE_TOKEN";
/// The shortest token taken: 32 characters (`escouade_mcp::new_token` makes 43).
const MIN_TOKEN: usize = 32;
/// Clients connected at once; the next ones are closed at once.
const MAX_CONNECTIONS: usize = 16;
/// The most a request weighs: a command's call, as the MCP server takes one without attachments.
const MAX_REQUEST: usize = escouade_mcp::http::MAX_TEXT_BODY;
/// A connection's requests under way; the next ones wait in the socket.
const MAX_IN_FLIGHT: usize = 64;
/// All connections' calls under way: each may hold two blocking threads, far below tokio's 512 that would jam the app.
const MAX_CALLS: usize = 64;
/// A connection's terminal streams open at once.
const MAX_STREAMS: usize = 256;
/// Answers and events waiting for the socket; full, the connection is closed.
const TEXT_QUEUE: usize = 1024;
/// The terminals' bytes waiting for the socket (16 MiB, four full replays of a terminal's kept output); past it, the
/// connection is closed. A client's stream commands start one at a time, each once half of it is free: a whole replay
/// then fits, with as much again for the live output of its terminals.
const BYTES_BUDGET: usize = 4 * crate::output::OUTPUT_KEPT;
/// What a frame of a terminal's bytes counts for, beyond its bytes: tiny reads cannot queue without bound.
const FRAME_COST: usize = 64;
/// How long one frame may take to reach the socket before the connection is closed.
const WRITE: Duration = Duration::from_secs(10);
/// How long a connection has to send its request's head.
const HANDSHAKE: Duration = Duration::from_secs(10);
/// How long a request may take, answer included (a file of the web build through a slow tunnel).
const EXCHANGE: Duration = Duration::from_secs(60);
/// The most `POST /pair` and `/admin/…` read: a code and a name.
const MAX_SMALL_BODY: usize = 1024;
/// The subprotocol a page names, beside its token's (`bearer.<token>`): the one the server answers.
const SUBPROTOCOL: &str = "escouade";
/// The file, in the devices' folder, that tells `escouade-server`'s own commands the port and the key (`Control`).
pub const CONTROL: &str = "server.json";
/// The commands with a flow, which this server serves itself.
const STREAMS: [&str; 8] = [
    "subscribe",
    "account_login",
    "agent_terminal",
    "term_spawn",
    "term_attach",
    "run_start",
    "run_attach",
    "test_run_start",
];

/// Where the server listens, the tokens its clients bring, and the web build it serves, if any.
pub struct Config {
    /// 127.0.0.1's port, unless `socket` is given.
    pub port: u16,
    /// A Unix socket to listen on instead (`--socket`), in a folder of this user's alone: on a machine shared with
    /// other users, none of them can take the place of the server while it restarts, as one could take a port.
    pub socket: Option<PathBuf>,
    /// `ESCOUADE_REMOTE_TOKEN`'s: the desktop app's and the tests'.
    pub token: Option<String>,
    pub web: Option<PathBuf>,
    /// The paired devices, `escouade-server`'s.
    pub devices: Option<Arc<Devices>>,
    /// The origins declared by `--origin`, as `parse_origin` gives them.
    pub origins: Vec<String>,
}

/// What `escouade-server pair` and `devices` read to reach the server that runs: its port, and the key of `/admin/…`.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Control {
    pub port: u16,
    /// The socket it listens on instead of the port, if any.
    #[serde(default)]
    pub socket: Option<PathBuf>,
    pub key: String,
    /// The origins declared, the first of which `pair` gives as the address to open.
    #[serde(default)]
    pub origins: Vec<String>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("socket", &self.socket)
            .field("token", &self.token.as_ref().map(|_| "<hidden>"))
            .field("web", &self.web)
            .field("devices", &self.devices.is_some())
            .finish()
    }
}

/// The two variables, read and removed from the environment before any thread starts: `Err` says which is wrong. The
/// desktop app's.
#[cfg_attr(not(feature = "desktop"), allow(dead_code))]
pub fn take_from_env() -> Result<Option<Config>, String> {
    let (port, token) = take_vars();
    Config::parse(port.as_deref(), token.as_deref())
}

/// `escouade-server`'s: the port, if set (0 if not: `--port` or `--socket` then says where to listen); the token read
/// in debug builds only (its tests), removed in all.
pub fn take_port_from_env() -> Result<Config, String> {
    let (port, token) = take_vars();
    let token = env_token(token, cfg!(debug_assertions));
    Ok(Config {
        port: port.as_deref().map(parse_port).transpose()?.unwrap_or(0),
        socket: None,
        token: token.as_deref().map(parse_token).transpose()?,
        web: None,
        devices: None,
        origins: Vec::new(),
    })
}

/// `ESCOUADE_REMOTE_TOKEN`'s value kept by a `debug` build alone; another says it ignores it.
fn env_token(token: Option<String>, debug: bool) -> Option<String> {
    if token.is_some() && !debug {
        eprintln!("escouade-server: {TOKEN_VAR} is for tests, read by debug builds only: ignored");
    }
    token.filter(|_| debug)
}

fn take_vars() -> (Option<String>, Option<String>) {
    let vars = (std::env::var(PORT_VAR).ok(), std::env::var(TOKEN_VAR).ok());
    std::env::remove_var(PORT_VAR);
    std::env::remove_var(TOKEN_VAR);
    vars
}

/// A port from 1 to 65535; `Err` names the variable, never its value.
fn parse_port(port: &str) -> Result<u16, String> {
    let port = port.trim().parse::<u16>().ok().filter(|p| *p != 0);
    port.ok_or(format!("{PORT_VAR} is not a port from 1 to 65535"))
}

/// `--origin`'s URL as a browser writes the `Origin` of its pages: `http` or `https`, a host, its port unless it is the
/// scheme's, in lower case; no user, path, query or fragment. `Err` says why.
pub fn parse_origin(url: &str) -> Result<String, String> {
    let wrong = || {
        format!("--origin {url:?} is not an origin, such as https://escouade.example.com (a name in punycode, xn--…, if it is not ASCII)")
    };
    let uri = url
        .parse::<hyper::Uri>()
        .ok()
        .filter(|_| !url.contains(['@', '?', '#']))
        .filter(|u| matches!(u.path(), "" | "/"))
        .ok_or_else(wrong)?;
    let (scheme, default) = match uri.scheme_str().map(str::to_ascii_lowercase).as_deref() {
        Some("http") => ("http", 80),
        Some("https") => ("https", 443),
        _ => return Err(wrong()),
    };
    let authority = uri.authority().ok_or_else(wrong)?;
    let host = authority.host().to_ascii_lowercase();
    let name = host
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
    let ipv6 = host.starts_with('[')
        && host.ends_with(']')
        && host[1..host.len() - 1]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
    if host.is_empty() || !(name || ipv6) {
        return Err(wrong());
    }
    // The port as written: `port_u16` would take one out of range for none.
    let port = match authority.as_str().strip_prefix(authority.host()) {
        Some("") => default,
        Some(port) => port
            .strip_prefix(':')
            .filter(|p| p.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|p| p.parse::<u16>().ok())
            .filter(|p| *p != 0)
            .ok_or_else(wrong)?,
        None => return Err(wrong()),
    };
    if port == default {
        Ok(format!("{scheme}://{host}"))
    } else {
        Ok(format!("{scheme}://{host}:{port}"))
    }
}

/// `origin`'s host and port, as `Host` names them.
fn authority(origin: &str) -> &str {
    origin.split_once("://").map_or(origin, |(_, a)| a)
}

/// A token of 32 characters or more.
fn parse_token(token: &str) -> Result<String, String> {
    if token.chars().count() < MIN_TOKEN {
        return Err(format!("{TOKEN_VAR} is shorter than 32 characters"));
    }
    Ok(token.to_string())
}

#[cfg_attr(not(feature = "desktop"), allow(dead_code))]
impl Config {
    /// `Ok(None)` without either value; `Err` names the variable that is wrong, never its value.
    pub fn parse(port: Option<&str>, token: Option<&str>) -> Result<Option<Config>, String> {
        let wrong = |var: &str, why: &str| Err(format!("{var} {why}"));
        let (port, token) = match (port, token) {
            (None, None) => return Ok(None),
            (None, Some(_)) => return wrong(PORT_VAR, "is not set"),
            (Some(_), None) => return wrong(TOKEN_VAR, "is not set"),
            (Some(port), Some(token)) => (port, token),
        };
        Ok(Some(Config {
            port: parse_port(port)?,
            socket: None,
            token: Some(parse_token(token)?),
            web: None,
            devices: None,
            origins: Vec::new(),
        }))
    }
}

/// The server listening, stopped when dropped.
pub struct Running {
    /// Its address on 127.0.0.1; port 0 when it listens on a socket.
    pub addr: SocketAddr,
    /// The socket it listens on, if any: removed when it stops.
    pub socket: Option<PathBuf>,
    stop: watch::Sender<bool>,
    /// `CONTROL`'s path and the key written in it, if any.
    control: Option<(PathBuf, String)>,
    /// The socket's lock, held until dropped: no other server binds the socket meanwhile, so the one `stop` removes
    /// is this one's.
    _socket_lock: Option<std::fs::File>,
}

impl Running {
    /// Stops listening and closes the connections; the core goes on. `CONTROL` goes with it, unless another server
    /// started since on the same folder wrote its own: `pair` then finds no stale port and key to send to whoever
    /// listens there next.
    pub fn stop(&self) {
        self.stop.send_replace(true);
        // Still ours: its lock is held until this is dropped.
        if let Some(socket) = &self.socket {
            let _ = std::fs::remove_file(socket);
        }
        if let Some((path, key)) = &self.control {
            let ours = std::fs::read(path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Control>(&b).ok())
                .is_some_and(|c| c.key == *key);
            if ours {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Serves `entries` of `core` on 127.0.0.1, at `config.port`, or on `config.socket`.
pub fn start(
    core: Arc<Core>,
    entries: &'static [Entry],
    config: Config,
) -> std::io::Result<Running> {
    start_with(core, entries, config, MAX_CALLS)
}

/// The same, with at most `calls` calls under way across all connections.
fn start_with(
    core: Arc<Core>,
    entries: &'static [Entry],
    config: Config,
    calls: usize,
) -> std::io::Result<Running> {
    let (listener, addr, socket, socket_lock) = match &config.socket {
        #[cfg(unix)]
        Some(path) => {
            let (listener, bound, lock) = bind_socket(path)?;
            let _on = rt::handle().enter();
            let unspecified = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
            (
                Listener::Unix(tokio::net::UnixListener::from_std(listener)?),
                unspecified,
                Some(bound),
                Some(lock),
            )
        }
        #[cfg(not(unix))]
        Some(_) => {
            let why = "a Unix socket is for Unix systems: listen on --port";
            return Err(std::io::Error::new(std::io::ErrorKind::Unsupported, why));
        }
        None => {
            let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, config.port))?;
            listener.set_nonblocking(true)?;
            let _on = rt::handle().enter();
            let listener = TcpListener::from_std(listener)?;
            let addr = listener.local_addr()?;
            (Listener::Tcp(listener), addr, None, None)
        }
    };
    let tokens = escouade_mcp::Tokens::default();
    *tokens.external.write() = config.token;
    let admin = escouade_mcp::Tokens::default();
    let mut written = None;
    if let Some(devices) = &config.devices {
        let key = escouade_mcp::new_token();
        let control = Control {
            port: addr.port(),
            socket: socket.clone(),
            key: key.clone(),
            origins: config.origins.clone(),
        };
        let bytes = serde_json::to_vec(&control).expect("plain data");
        let path = devices.dir().join(CONTROL);
        crate::paths::write_private(&path, &bytes)?;
        written = Some((path, key.clone()));
        *admin.external.write() = Some(key);
    }
    let server = Arc::new(Server {
        core,
        entries,
        port: addr.port(),
        tokens,
        devices: config.devices,
        admin,
        conns: Mutex::default(),
        web: config.web,
        origins: config.origins,
        calls: Arc::new(Semaphore::new(calls)),
        uploads: Arc::new(Semaphore::new(files::MAX_UPLOADS)),
        transfers: Arc::new(Semaphore::new(files::MAX_TRANSFERS)),
    });
    let (stop, stopped) = watch::channel(false);
    rt::spawn(server.accept(listener, stopped));
    Ok(Running {
        addr,
        socket,
        stop,
        control: written,
        _socket_lock: socket_lock,
    })
}

/// Bound in a folder of this user's alone (made 0700 if missing), the socket itself 0600: no other user can reach it,
/// nor put one of theirs in its place. The folder's real path is checked, then bound: no folder above it that another
/// user may change (but a sticky one of root's, as /tmp, where no one renames another's), so no link put in the way
/// later moves it. Held by `<socket>.lock` while the server lives: one left by a server that died is replaced, one in
/// use is not. The real path, and the lock.
#[cfg(unix)]
fn bind_socket(
    path: &std::path::Path,
) -> std::io::Result<(std::os::unix::net::UnixListener, PathBuf, std::fs::File)> {
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
    use std::os::unix::net::UnixListener;
    let Some(name) = path.file_name() else {
        let why = format!("{} names no socket", path.display());
        return Err(Error::new(ErrorKind::InvalidInput, why));
    };
    let folder = path.parent().filter(|p| !p.as_os_str().is_empty());
    let folder = folder.unwrap_or(std::path::Path::new("."));
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(folder)?;
    let folder = std::fs::canonicalize(folder)?;
    // SAFETY: geteuid has no precondition.
    let me = unsafe { libc::geteuid() };
    let meta = std::fs::symlink_metadata(&folder)?;
    if !meta.is_dir() || meta.uid() != me || meta.mode() & 0o077 != 0 {
        let why = format!(
            "{} must be a folder of this user's alone (0700) to hold the socket",
            folder.display()
        );
        return Err(Error::new(ErrorKind::PermissionDenied, why));
    }
    for above in folder.ancestors().skip(1) {
        let m = std::fs::symlink_metadata(above)?;
        let trusted = m.uid() == me || m.uid() == 0;
        let sticky_roots = m.uid() == 0 && m.mode() & 0o1000 != 0;
        if !m.is_dir() || !trusted || (m.mode() & 0o022 != 0 && !sticky_roots) {
            let why = format!(
                "{} may be changed by other users: the socket's path must be this user's and root's alone",
                above.display()
            );
            return Err(Error::new(ErrorKind::PermissionDenied, why));
        }
    }
    let path = folder.join(name);
    let mut lock = path.clone().into_os_string();
    lock.push(".lock");
    let Some(lock) = super::lockfile::acquire(std::path::Path::new(&lock))? else {
        let why = format!("another server listens on {}", path.display());
        return Err(Error::new(ErrorKind::AddrInUse, why));
    };
    if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_socket()) {
        std::fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    Ok((listener, path, lock))
}

/// Where the clients come from.
enum Listener {
    Tcp(TcpListener),
    #[cfg(unix)]
    Unix(tokio::net::UnixListener),
}

/// A client's connection, by TCP or by the socket.
trait Duplex: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send> Duplex for T {}
type Stream = std::pin::Pin<Box<dyn Duplex>>;

impl Listener {
    /// The next client; `None` for a peer off the loopback, which cannot come to a port bound there and is closed if
    /// it does. A socket's clients are this user's: its folder lets no other in.
    async fn accept(&self) -> std::io::Result<Option<Stream>> {
        match self {
            Listener::Tcp(listener) => {
                let (tcp, peer) = listener.accept().await?;
                Ok(peer.ip().is_loopback().then(|| Box::pin(tcp) as Stream))
            }
            #[cfg(unix)]
            Listener::Unix(listener) => Ok(Some(Box::pin(listener.accept().await?.0))),
        }
    }
}

struct Server {
    core: Arc<Core>,
    entries: &'static [Entry],
    /// Its port on 127.0.0.1; 0 on a socket, where every page comes through a tunnel.
    port: u16,
    /// `ESCOUADE_REMOTE_TOKEN`'s, if any, compared in constant time.
    tokens: escouade_mcp::Tokens,
    /// The paired devices, if any.
    devices: Option<Arc<Devices>>,
    /// The key of `/admin/…`, written in `CONTROL`, with the devices only.
    admin: escouade_mcp::Tokens,
    /// The devices' connections, closed when their device is revoked.
    conns: Mutex<Vec<(String, Weak<Conn>)>>,
    /// The web build's folder.
    web: Option<PathBuf>,
    /// The origins declared by `--origin`, besides the loopback's.
    origins: Vec<String>,
    /// The calls under way, all connections together.
    calls: Arc<Semaphore>,
    /// The files being sent, all connections together (`files::MAX_UPLOADS`).
    uploads: Arc<Semaphore>,
    /// The requests of files, all connections together (`files::MAX_TRANSFERS`).
    transfers: Arc<Semaphore>,
}

impl Server {
    async fn accept(self: Arc<Self>, listener: Listener, mut stopped: watch::Receiver<bool>) {
        let connections = Arc::new(Semaphore::new(MAX_CONNECTIONS));
        loop {
            let accepted = tokio::select! {
                _ = stopped.wait_for(|s| *s) => return,
                accepted = listener.accept() => accepted,
            };
            let tcp = match accepted {
                Ok(Some(tcp)) => tcp,
                Ok(None) => continue,
                Err(e) => {
                    log::debug!("remote API: accept failed: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            let Ok(permit) = connections.clone().try_acquire_owned() else {
                continue;
            };
            let (me, stopped) = (self.clone(), stopped.clone());
            rt::spawn(async move {
                me.serve(tcp, stopped).await;
                drop(permit);
            });
        }
    }

    /// Whether `headers` name this server: one `Host`, its own, and no `Origin` but its own page's. Its own: the loopback
    /// on its port or, for `ssh -L`, on another (the token is asked all the same), and the origins declared. No
    /// `X-Forwarded-*` is read: anyone can write one.
    fn mine(&self, uri: &hyper::Uri, headers: &HeaderMap) -> Result<(), StatusCode> {
        let port = self.port;
        let loopback = |host: &str| {
            host.rsplit_once(':').is_some_and(|(name, p)| {
                ["127.0.0.1", "localhost"]
                    .iter()
                    .any(|n| n.eq_ignore_ascii_case(name))
                    && p.parse::<u16>().is_ok_and(|n| n != 0 && n.to_string() == p)
            })
        };
        let declared = |host: &str| {
            self.origins
                .iter()
                .any(|o| authority(o).eq_ignore_ascii_case(host))
        };
        let mine = |host: &str| loopback(host) || declared(host);
        // A header given twice could be read differently by another reader: refused.
        let once = |name| headers.get_all(name).iter().count() == 1;
        let host = headers
            .get(header::HOST)
            .filter(|_| once(header::HOST))
            .and_then(|h| h.to_str().ok());
        let target = uri.authority().map(|a| a.as_str());
        if !host.is_some_and(mine) || target.is_some_and(|a| !mine(a)) {
            return Err(StatusCode::FORBIDDEN);
        }
        if let (true, Some(host)) = (headers.contains_key(header::ORIGIN), host) {
            // A tunnel's page is at the loopback's address it names; another port's page is another site.
            let page = loopback(host).then(|| format!("http://{}", host.to_ascii_lowercase()));
            // On a socket, no port of its own: every page is a tunnel's.
            let own = (port != 0).then(|| {
                [
                    format!("http://127.0.0.1:{port}"),
                    format!("http://localhost:{port}"),
                ]
            });
            let mut own = own.iter().flatten().chain(&self.origins).chain(&page);
            let origin = headers.get(header::ORIGIN).filter(|_| once(header::ORIGIN));
            if !origin.is_some_and(|o| own.any(|own| o.as_bytes() == own.as_bytes())) {
                return Err(StatusCode::FORBIDDEN);
            }
        }
        Ok(())
    }

    /// Who `headers` come from: the environment's token (`None`) or a device (its id). A token comes by one
    /// `Authorization: Bearer`, else, for a WebSocket, by one subprotocol `bearer.<token>`.
    fn admit(&self, headers: &HeaderMap) -> Result<Option<String>, StatusCode> {
        let token = if headers.contains_key(header::AUTHORIZATION) {
            bearer(headers).ok_or(StatusCode::UNAUTHORIZED)?
        } else {
            let mut offered = subprotocols(headers).filter_map(|p| p.strip_prefix("bearer."));
            match (offered.next(), offered.next()) {
                (Some(token), None) => token.to_string(),
                _ => return Err(StatusCode::UNAUTHORIZED),
            }
        };
        if self.tokens.caller(&token).is_some() {
            return Ok(None);
        }
        let device = self.devices.as_ref().and_then(|d| d.check(&token));
        device.map(Some).ok_or(StatusCode::UNAUTHORIZED)
    }

    /// Closes the connections of `id`, revoked.
    fn cut(&self, id: &str) {
        self.conns.lock().retain(|(device, conn)| {
            let Some(conn) = conn.upgrade() else {
                return false;
            };
            if device == id {
                conn.close("was revoked: closed");
            }
            device != id
        });
    }

    /// One request per connection: a file, a session, or the WebSocket, which then holds the connection.
    async fn serve(self: Arc<Self>, tcp: Stream, stopped: watch::Receiver<bool>) {
        let upgrade: Arc<Upgrade> = Arc::default();
        // A request of `files::PREFIX`: it may take longer than `EXCHANGE`, its bytes coming at a pace of their own.
        let long = Arc::new(AtomicBool::new(false));
        let (me, upgraded, slow) = (self.clone(), upgrade.clone(), long.clone());
        let service = service_fn(move |req: Request| {
            let (me, upgraded, slow) = (me.clone(), upgraded.clone(), slow.clone());
            async move {
                let mut answer = if req.uri().path().starts_with(files::PREFIX) {
                    slow.store(true, Ordering::Relaxed);
                    me.transfer(req).await
                } else {
                    let answer = me.answer(req, &upgraded).await;
                    answer.map(|b| b.map_err(|e| match e {}).boxed_unsync())
                };
                // The slot is held from the start: the next request comes on a connection of its own.
                if answer.status() != StatusCode::SWITCHING_PROTOCOLS {
                    let close = header::HeaderValue::from_static("close");
                    answer.headers_mut().insert(header::CONNECTION, close);
                }
                Ok::<_, Infallible>(answer)
            }
        });
        let conn = http1::Builder::new()
            .timer(TokioTimer::new())
            .header_read_timeout(HANDSHAKE)
            .serve_connection(TokioIo::new(tcp), service)
            .with_upgrades();
        tokio::pin!(conn);
        let served = match tokio::time::timeout(EXCHANGE, &mut conn).await {
            Ok(served) => served,
            // A file sent stops at its first pause of `files::IDLE`; one fetched by a client that stopped reading
            // holds its slot until `files::TRANSFER`.
            Err(_) if long.load(Ordering::Relaxed) => {
                let more = files::TRANSFER.saturating_sub(EXCHANGE);
                match tokio::time::timeout(more, &mut conn).await {
                    Ok(served) => served,
                    Err(_) => return,
                }
            }
            Err(_) => return,
        };
        let Ok(()) = served else {
            return;
        };
        let Some((on_upgrade, device)) = upgrade.lock().take() else {
            return;
        };
        let Ok(Ok(io)) = tokio::time::timeout(HANDSHAKE, on_upgrade).await else {
            return;
        };
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_REQUEST))
            .max_frame_size(Some(MAX_REQUEST));
        let ws =
            WebSocketStream::from_raw_socket(TokioIo::new(io), Role::Server, Some(config)).await;
        self.session(ws, stopped, device).await;
    }

    /// The answer to `req`; a WebSocket's leaves its connection in `upgrade`.
    async fn answer(&self, req: Request, upgrade: &Upgrade) -> Response {
        if let Err(status) = self.mine(req.uri(), req.headers()) {
            return empty(status);
        }
        let wants_ws = req
            .headers()
            .get(header::UPGRADE)
            .is_some_and(|v| v.as_bytes().eq_ignore_ascii_case(b"websocket"));
        if wants_ws {
            return self.upgrade(req, upgrade);
        }
        match (req.method(), req.uri().path()) {
            (&Method::POST, "/pair") => self.pair(req).await,
            (&Method::GET, "/device") => match self.admit(req.headers()) {
                Ok(_) => empty(StatusCode::NO_CONTENT),
                Err(status) => empty(status),
            },
            (_, path) if path.starts_with("/admin/") => self.control(req).await,
            (&Method::GET | &Method::HEAD, path) => self.file(&req, path.to_string()).await,
            _ => empty(StatusCode::METHOD_NOT_ALLOWED),
        }
    }

    /// A request of `files::PREFIX`: from this server's own pages, with a token.
    async fn transfer(&self, req: Request) -> files::Answer {
        let admitted = self.mine(req.uri(), req.headers());
        if let Err(status) = admitted.and_then(|()| self.admit(req.headers()).map(drop)) {
            return files::status(status);
        }
        let Ok(slot) = self.transfers.clone().try_acquire_owned() else {
            return files::busy();
        };
        let answer = files::serve(self.core.clone(), self.uploads.clone(), req).await;
        // Held until its body is sent, or dropped with the connection: a download holds it as long as it is read.
        // (`map_err` keeps the body's size, which `map_frame` would lose.)
        answer.map(|body| {
            body.map_err(move |e| {
                let _held = &slot;
                e
            })
            .boxed_unsync()
        })
    }

    /// The WebSocket's handshake, its token or session checked.
    fn upgrade(&self, mut req: Request, upgrade: &Upgrade) -> Response {
        let device = match self.admit(req.headers()) {
            Ok(device) => device,
            Err(status) => return empty(status),
        };
        let headers = req.headers();
        let version = headers.get(header::SEC_WEBSOCKET_VERSION);
        let key = headers.get(header::SEC_WEBSOCKET_KEY);
        let (Some(key), true) = (key, version.is_some_and(|v| v == "13")) else {
            return empty(StatusCode::BAD_REQUEST);
        };
        if req.method() != Method::GET {
            return empty(StatusCode::BAD_REQUEST);
        }
        let accept = derive_accept_key(key.as_bytes());
        // A page that names subprotocols is answered with ours, never with its token's.
        let named = subprotocols(req.headers()).any(|p| p == SUBPROTOCOL);
        *upgrade.lock() = Some((hyper::upgrade::on(&mut req), device));
        let mut answer = hyper::Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::CONNECTION, "Upgrade")
            .header(header::UPGRADE, "websocket")
            .header(header::SEC_WEBSOCKET_ACCEPT, accept);
        if named {
            answer = answer.header(header::SEC_WEBSOCKET_PROTOCOL, SUBPROTOCOL);
        }
        answer.body(Full::default()).expect("a valid answer")
    }

    /// `POST /pair`, `{code, name}`: the new device's token, which the page keeps (`{token, id}`).
    async fn pair(&self, req: Request) -> Response {
        #[derive(serde::Deserialize)]
        struct Asked {
            code: String,
            name: String,
        }
        let Some(devices) = self.devices.clone() else {
            return empty(StatusCode::NOT_FOUND);
        };
        let Some(body) = small_body(req).await else {
            return empty(StatusCode::PAYLOAD_TOO_LARGE);
        };
        let Ok(asked) = serde_json::from_slice::<Asked>(&body) else {
            return empty(StatusCode::BAD_REQUEST);
        };
        if devices::clean(&asked.name).is_empty() {
            return empty(StatusCode::BAD_REQUEST);
        }
        let redeemed = rt::spawn_blocking(move || {
            let token = devices.redeem(&asked.code, &asked.name).ok()?;
            Some((devices.check(&token)?, token))
        })
        .await;
        match redeemed {
            Ok(Some((id, token))) => json(serde_json::json!({ "token": token, "id": id })),
            _ => empty(StatusCode::FORBIDDEN),
        }
    }

    /// `escouade-server`'s own commands, with the key of `CONTROL`: a code, the devices, a revocation.
    async fn control(&self, req: Request) -> Response {
        let (Some(devices), Some(key)) = (self.devices.clone(), bearer(req.headers())) else {
            return empty(StatusCode::UNAUTHORIZED);
        };
        if self.admin.caller(&key).is_none() {
            return empty(StatusCode::UNAUTHORIZED);
        }
        match (req.method(), req.uri().path()) {
            (&Method::POST, "/admin/pair") => {
                let code = devices.pair_code();
                let minutes = code.expires_in.as_secs() / 60;
                json(serde_json::json!({ "code": code.code, "minutes": minutes }))
            }
            (&Method::GET, "/admin/devices") => json(serde_json::json!(devices.list())),
            (&Method::POST, "/admin/revoke") => {
                let Some(body) = small_body(req).await else {
                    return empty(StatusCode::PAYLOAD_TOO_LARGE);
                };
                let id = String::from_utf8_lossy(&body).trim().to_string();
                // Not written, it is cut all the same, and the answer says it would come back at the next start.
                let (revoked, unwritten) = match devices.revoke(&id) {
                    Ok(revoked) => (revoked, None),
                    Err(why) => (true, Some(why)),
                };
                if revoked {
                    self.cut(&id);
                }
                json(serde_json::json!({ "revoked": revoked, "unwritten": unwritten }))
            }
            _ => empty(StatusCode::NOT_FOUND),
        }
    }

    /// A file of the web build, none without one.
    async fn file(&self, req: &Request, path: String) -> Response {
        let Some(root) = self.web.clone() else {
            return empty(StatusCode::NOT_FOUND);
        };
        let file = rt::spawn_blocking(move || web::read(&root, &path)).await;
        let Ok(Some(file)) = file else {
            return empty(StatusCode::NOT_FOUND);
        };
        // The page's own host (checked above), for its WebSocket.
        let host = req
            .headers()
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();
        // Behind a proxy in `https`, the WebSocket is `wss`.
        let secure = self
            .origins
            .iter()
            .any(|o| o.starts_with("https://") && authority(o).eq_ignore_ascii_case(host));
        let ws = if secure { "wss" } else { "ws" };
        // Each declared origin's too, whatever `Host` says: a proxy that rewrites it to 127.0.0.1 still serves them.
        let own = format!("{ws}://{host}");
        let declared: String = self
            .origins
            .iter()
            .map(|o| format!("ws{}", o.strip_prefix("http").unwrap_or(o)))
            .filter(|o| !o.eq_ignore_ascii_case(&own))
            .map(|o| format!(" {o}"))
            .collect();
        let csp = format!(
            "default-src 'self'; img-src 'self' data: blob:; style-src 'self' 'unsafe-inline'; \
             font-src 'self' data:; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' {own}{declared}; \
             frame-ancestors 'none'; base-uri 'self'; form-action 'self'"
        );
        hyper::Response::builder()
            .header(header::CONTENT_TYPE, file.mime)
            .header(header::CACHE_CONTROL, file.cache)
            .header(header::CONTENT_SECURITY_POLICY, csp)
            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
            .body(Full::new(Bytes::from(file.body)))
            .expect("a valid answer")
    }

    /// A client's WebSocket, until it or the server ends.
    async fn session(
        self: Arc<Self>,
        ws: WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>,
        mut stopped: watch::Receiver<bool>,
        device: Option<String>,
    ) {
        let (mut out, mut input) = ws.split();
        let hello = Message::text(wire::hello(&self.core.host.version()));
        let Ok(Ok(())) = tokio::time::timeout(WRITE, out.send(hello)).await else {
            return;
        };
        log::info!("remote API: a client connected");
        let (conn, text, bytes) = Conn::new(TEXT_QUEUE, BYTES_BUDGET);
        if let (Some(id), Some(devices)) = (device, &self.devices) {
            let mut conns = self.conns.lock();
            // The connections gone since: each reconnection would leave its own otherwise.
            conns.retain(|(_, c)| c.strong_count() > 0);
            conns.push((id.clone(), Arc::downgrade(&conn)));
            drop(conns);
            // Revoked since its handshake: `cut` came before it was listed.
            if !devices.known(&id) {
                conn.close("was revoked: closed");
            }
        }
        let writer = rt::spawn(conn.clone().write(out, text, bytes));
        let in_flight = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
        let mut closed = conn.closed.subscribe();
        loop {
            let read = tokio::select! {
                _ = stopped.wait_for(|s| *s) => break,
                _ = closed.wait_for(|c| *c) => break,
                read = input.next() => read,
            };
            let request = match read {
                Some(Ok(Message::Text(text))) => wire::request(&text),
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
                // Closed, gone, or a frame that is no request.
                _ => None,
            };
            let Some(request) = request else { break };
            // The connection's slot, then the server's: neither wait outlives the connection or the server.
            let permits = async {
                let mine = in_flight.clone().acquire_owned().await;
                let all = self.calls.clone().acquire_owned().await;
                (mine.expect("never closed"), all.expect("never closed"))
            };
            let permits = tokio::select! {
                _ = stopped.wait_for(|s| *s) => break,
                _ = closed.wait_for(|c| *c) => break,
                permits = permits => permits,
            };
            self.dispatch(&conn, request, permits);
        }
        conn.close("left");
        drop(conn.subscription.lock().take());
        // Its terminals' readers leave with it.
        conn.streams.lock().clear();
        let _ = writer.await;
    }

    /// Runs `request` on its lane: after the calls of its family this connection made before it.
    fn dispatch(
        self: &Arc<Self>,
        conn: &Arc<Conn>,
        request: wire::Request,
        permits: (OwnedSemaphorePermit, OwnedSemaphorePermit),
    ) {
        let entry = self.entries.iter().find(|e| e.name == request.cmd);
        // An unknown name gets no lane of its own: the lanes are named by the table alone.
        let lane = entry.map_or("", |e| lane_of(e.name));
        let (me, conn, lanes) = (self.clone(), conn.clone(), conn.lanes.clone());
        lanes.submit(
            lane,
            Box::new(move || {
                let _permits = permits;
                // The client left: what it queued is dropped.
                if conn.is_closed() {
                    return;
                }
                let (id, on) = (request.id, conn.clone());
                let run = async move { me.run(&on, entry, request.args, request.stream).await };
                let answer = answer_of(id, rt::block_on(rt::spawn(run)));
                let _ = conn.push(&conn.text, Message::text(answer));
            }),
        );
    }

    async fn run(
        &self,
        conn: &Arc<Conn>,
        entry: Option<&'static Entry>,
        args: Value,
        stream: Option<u32>,
    ) -> Result<Value, CallError> {
        let entry = entry.ok_or(CallError::Unknown)?;
        if !STREAMS.contains(&entry.name) {
            let ctx = Ctx {
                core: self.core.clone(),
            };
            return table::call(self.entries, ctx, entry.name, args).await;
        }
        let core = self.core.clone();
        let failed = |e: tokio::task::JoinError| CallError::Failed(e.to_string().into());
        if entry.name == "subscribe" {
            // The first subscription leaves before the second comes: the client gets each event once.
            drop(conn.subscription.lock().take());
            let sink = conn.event_sink();
            let (state, subscription) = rt::spawn_blocking(move || {
                commands::subscribe_to(&core, sink, commands::Subscriber::Remote)
            })
            .await
            .map_err(failed)?;
            *conn.subscription.lock() = subscription;
            return table::ValueTag.reply(state);
        }
        let name = entry.name;
        let stream = stream
            .ok_or_else(|| CallError::Args(format!("command `{name}` needs a `stream` number")))?;
        // One stream command at a time per client, each with room for its replay.
        let _one = conn.streaming.lock().await;
        if conn.is_closed() {
            return Err(gone());
        }
        conn.room_for_a_replay().await?;
        let output = conn.open(stream)?;
        // Off the runtime's workers, as the table runs a synchronous command.
        rt::spawn_blocking(move || rt::block_on(start_stream(&core, name, &args, output)))
            .await
            .map_err(failed)?
    }
}

/// The answer to `id` once its run is joined: a panic in the server's own path is a failure, as the table answers a
/// command's.
fn answer_of(id: u64, joined: Result<Result<Value, CallError>, tokio::task::JoinError>) -> String {
    wire::answer(
        id,
        joined.unwrap_or_else(|e| Err(CallError::Failed(e.to_string().into()))),
    )
}

/// The command `name`, its bytes to `output`; its arguments read as Tauri reads them.
async fn start_stream(
    core: &Arc<Core>,
    name: &'static str,
    args: &Value,
    output: Viewer,
) -> Result<Value, CallError> {
    let core = CoreState::from(core);
    macro_rules! arg {
        ($key:literal) => {
            table::arg(args, name, $key)?
        };
    }
    let started = match name {
        "account_login" => {
            commands::account_login_to(core, arg!("id"), arg!("cols"), arg!("rows"), output)
        }
        "agent_terminal" => {
            let (id, cols, rows, manual) = (arg!("id"), arg!("cols"), arg!("rows"), arg!("manual"));
            commands::agent_terminal_to(core, id, cols, rows, manual, output).await
        }
        "term_spawn" => {
            let (project, agent, subdir) = (arg!("projectId"), arg!("agentId"), arg!("subdir"));
            let (shell, title, cols, rows) =
                (arg!("shell"), arg!("name"), arg!("cols"), arg!("rows"));
            commands::term_spawn_to(
                core, project, agent, subdir, shell, title, cols, rows, output,
            )
            .await
        }
        "term_attach" => {
            commands::term_attach_to(core, arg!("id"), arg!("cols"), arg!("rows"), output)
        }
        "run_attach" => {
            commands::run_attach_to(core, arg!("id"), arg!("cols"), arg!("rows"), output)
        }
        "run_start" => commands::run_start_to(
            core,
            arg!("projectId"),
            arg!("commandId"),
            arg!("agentId"),
            arg!("cols"),
            arg!("rows"),
            arg!("cursorRow"),
            output,
        ),
        "test_run_start" => commands::test_run_start_to(
            core,
            arg!("agentId"),
            arg!("kind"),
            arg!("index"),
            arg!("cols"),
            arg!("rows"),
            arg!("cursorRow"),
            output,
        ),
        _ => return Err(CallError::Unknown),
    };
    ResultTag.reply(started)
}

/// A client connected: what waits for its socket, its streams, its lanes.
struct Conn {
    /// Answers and events.
    text: mpsc::Sender<Message>,
    /// Terminals' bytes, behind the text.
    bytes: mpsc::Sender<Message>,
    /// What `bytes` holds, each frame counted with `FRAME_COST`, and the most it may.
    bytes_queued: AtomicUsize,
    bytes_budget: usize,
    /// Told when `bytes` gave some to the socket.
    room: Notify,
    /// Held by the stream command that starts, with its replay.
    streaming: tokio::sync::Mutex<()>,
    closed: watch::Sender<bool>,
    /// The stream numbers of its terminals still running, each with its place among the terminal's readers.
    streams: Mutex<HashMap<u32, Reader>>,
    lanes: Arc<Lanes>,
    /// Its place among the hub's subscribers, once it subscribed.
    subscription: Mutex<Option<Subscription>>,
}

type Queue = mpsc::Receiver<Message>;

impl Conn {
    fn new(text: usize, bytes_budget: usize) -> (Arc<Self>, Queue, Queue) {
        let (text, text_rx) = mpsc::channel(text);
        // As many frames as the budget can hold: the budget is the bound.
        let (bytes, bytes_rx) = mpsc::channel(bytes_budget / FRAME_COST + 1);
        let conn = Arc::new(Conn {
            text,
            bytes,
            bytes_queued: AtomicUsize::new(0),
            bytes_budget,
            room: Notify::new(),
            streaming: tokio::sync::Mutex::new(()),
            closed: watch::channel(false).0,
            streams: Mutex::default(),
            lanes: Arc::default(),
            subscription: Mutex::default(),
        });
        (conn, text_rx, bytes_rx)
    }

    /// Ends the connection, said once in the log.
    fn close(&self, why: &str) {
        if !self.closed.send_replace(true) {
            log::info!("remote API: a client {why}");
        }
    }

    fn is_closed(&self) -> bool {
        *self.closed.borrow()
    }

    /// Sends what waits, the text first, until the connection ends.
    async fn write(
        self: Arc<Self>,
        mut out: impl futures_util::Sink<Message> + Unpin,
        mut text: Queue,
        mut bytes: Queue,
    ) {
        let mut closed = self.closed.subscribe();
        loop {
            let message = tokio::select! {
                biased;
                _ = closed.wait_for(|c| *c) => break,
                Some(m) = text.recv() => m,
                Some(m) = bytes.recv() => {
                    // Its room given back once it leaves the queue.
                    self.bytes_queued.fetch_sub(m.len() + FRAME_COST, Ordering::AcqRel);
                    self.room.notify_waiters();
                    m
                }
                else => break,
            };
            // A client that reads nothing is closed, never waited for.
            let sent = tokio::select! {
                _ = closed.wait_for(|c| *c) => break,
                sent = tokio::time::timeout(WRITE, out.send(message)) => sent,
            };
            match sent {
                Ok(Ok(())) => {}
                Ok(Err(_)) => break,
                Err(_) => {
                    self.close("read too slowly: closed");
                    break;
                }
            }
        }
        self.close("left");
        let _ = tokio::time::timeout(WRITE, out.close()).await;
    }

    /// The core's events, to this client.
    fn event_sink(self: &Arc<Self>) -> Sink<UiEvent> {
        let conn = Arc::downgrade(self);
        Sink::new(move |event| {
            let conn = live(&conn)?;
            let Some(text) = wire::event(&event) else {
                return Ok(());
            };
            conn.push(&conn.text, Message::text(text))
        })
    }

    /// `message` queued without waiting: a full queue closes the connection.
    fn push(&self, queue: &mpsc::Sender<Message>, message: Message) -> Result<(), Closed> {
        match queue.try_send(message) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                self.close("read too slowly: closed");
                Err(Closed)
            }
            Err(TrySendError::Closed(_)) => Err(Closed),
        }
    }

    /// A terminal's frame queued without waiting: past the budget, the connection is closed.
    fn push_bytes(&self, message: Message) -> Result<(), Closed> {
        let cost = message.len() + FRAME_COST;
        if self.bytes_queued.fetch_add(cost, Ordering::AcqRel) + cost > self.bytes_budget {
            self.bytes_queued.fetch_sub(cost, Ordering::AcqRel);
            self.close("read too slowly: closed");
            return Err(Closed);
        }
        let pushed = self.push(&self.bytes, message);
        if pushed.is_err() {
            self.bytes_queued.fetch_sub(cost, Ordering::AcqRel);
        }
        pushed
    }

    /// Waits, up to `WRITE`, until half the bytes' budget is free: a whole replay fits; else the client is closed.
    /// A connection that ends meanwhile ends the wait: its writer is gone, nothing would free room.
    async fn room_for_a_replay(&self) -> Result<(), CallError> {
        let deadline = tokio::time::Instant::now() + WRITE;
        let mut closed = self.closed.subscribe();
        loop {
            let notified = self.room.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.bytes_queued.load(Ordering::Acquire) <= self.bytes_budget / 2 {
                return Ok(());
            }
            tokio::select! {
                _ = closed.wait_for(|c| *c) => return Err(gone()),
                woke = tokio::time::timeout_at(deadline, notified) => {
                    if woke.is_err() {
                        self.close("read too slowly: closed");
                        return Err(CallError::Failed("the client reads too slowly".into()));
                    }
                }
            }
        }
    }

    /// A terminal's bytes, under `stream`: refused while that number is in use.
    fn open(self: &Arc<Self>, stream: u32) -> Result<Viewer, CallError> {
        let mut streams = self.streams.lock();
        if streams.contains_key(&stream) {
            return Err(CallError::Args(format!("stream {stream} is in use")));
        }
        if streams.len() >= MAX_STREAMS {
            return Err(CallError::Args("too many streams".into()));
        }
        let reader = Reader::new();
        let open = Open {
            conn: Arc::downgrade(self),
            stream,
        };
        let sink = Sink::new(move |bytes: Vec<u8>| {
            // Called under the output's lock (`output::Output`): it never waits.
            let conn = live(&open.conn)?;
            conn.push_bytes(Message::binary(wire::frame(open.stream, &bytes)))
        });
        let viewer = Viewer::client(sink, &reader);
        streams.insert(stream, reader);
        Ok(viewer)
    }
}

/// The token of the one `Authorization: Bearer` of `headers`.
fn bearer(headers: &HeaderMap) -> Option<String> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return None;
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map(|(_, token)| token.trim().to_string())
}

/// The subprotocols a WebSocket's handshake names.
fn subprotocols(headers: &HeaderMap) -> impl Iterator<Item = &str> {
    headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
}

/// A body of `MAX_SMALL_BODY` at most; `None` past it.
async fn small_body(req: Request) -> Option<Bytes> {
    let body = Limited::new(req.into_body(), MAX_SMALL_BODY)
        .collect()
        .await;
    body.ok().map(|b| b.to_bytes())
}

/// A JSON answer that no cache keeps.
fn json(value: Value) -> Response {
    hyper::Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Full::new(Bytes::from(value.to_string())))
        .expect("a valid answer")
}

/// An answer with no body.
fn empty(status: StatusCode) -> Response {
    let mut answer = hyper::Response::new(Full::default());
    *answer.status_mut() = status;
    answer
}

/// What a stream command gets once its connection has ended.
fn gone() -> CallError {
    CallError::Failed("the connection is closed".into())
}

/// The connection, while it lasts.
fn live(conn: &Weak<Conn>) -> Result<Arc<Conn>, Closed> {
    conn.upgrade().filter(|c| !c.is_closed()).ok_or(Closed)
}

/// A stream number in use until its terminal's sink is dropped.
struct Open {
    conn: Weak<Conn>,
    stream: u32,
}

impl Drop for Open {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.upgrade() {
            conn.streams.lock().remove(&self.stream);
        }
    }
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
