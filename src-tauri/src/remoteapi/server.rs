//! The core's commands served over a WebSocket on 127.0.0.1 (docs/REMOTE.md, « La fondation commune », points 2 and
//! 6): off unless `ESCOUADE_REMOTE_PORT` and `ESCOUADE_REMOTE_TOKEN` are set, both taken out of the environment at
//! start-up so that no child process inherits the token; release builds never read them nor listen. A connection
//! brings the token, comes from no web page and names this server; its slot is taken before the handshake, so a local
//! process may hold all 16 for up to 10 s each (acceptable for now). Its calls of one family keep their order, as the
//! window's do (`windowthread`). The eight commands with a flow (`subscribe` and the terminals) are served here, the
//! others by the table. A client's stream commands start one at a time, each waiting at most 10 s for half of its
//! 16 MiB budget of terminal bytes, and give up when the connection ends; a client that does not read is closed: a
//! queue that fills, or a socket write that hangs, closes it. A remote `subscribe` adds the connection to the hub's
//! subscribers, the window keeping its own, until the connection ends; a second one takes the place of the first. A
//! terminal's stream reads its output beside the window, from what it kept, until the connection ends. A remote
//! `save_settings` retunes the desktop's conversation.
use super::table::{self, CallError, Ctx, Entry, ResultTag};
use super::wire;
use crate::commands::{self, CoreState};
use crate::core::Core;
use crate::hub::Subscription;
use crate::lanes::{lane_of, Lanes};
use crate::model::UiEvent;
use crate::output::{Reader, Viewer};
use crate::sink::{Closed, Sink};
use escouade_base::rt;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, error::TrySendError};
use tokio::sync::{watch, Notify, OwnedSemaphorePermit, Semaphore};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{header, StatusCode};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

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
/// How long a connection has to make its handshake.
const HANDSHAKE: Duration = Duration::from_secs(10);
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

/// Where the server listens and the token its clients bring.
pub struct Config {
    pub port: u16,
    pub token: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("token", &"<hidden>")
            .finish()
    }
}

/// The two variables, read and removed from the environment before any thread starts: `Err` says which is wrong.
pub fn take_from_env() -> Result<Option<Config>, String> {
    let port = std::env::var(PORT_VAR).ok();
    let token = std::env::var(TOKEN_VAR).ok();
    std::env::remove_var(PORT_VAR);
    std::env::remove_var(TOKEN_VAR);
    Config::parse(port.as_deref(), token.as_deref())
}

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
        let Some(port) = port.trim().parse::<u16>().ok().filter(|p| *p != 0) else {
            return wrong(PORT_VAR, "is not a port from 1 to 65535");
        };
        if token.chars().count() < MIN_TOKEN {
            return wrong(TOKEN_VAR, "is shorter than 32 characters");
        }
        Ok(Some(Config {
            port,
            token: token.to_string(),
        }))
    }
}

/// The server listening, stopped when dropped.
pub struct Running {
    pub addr: SocketAddr,
    stop: watch::Sender<bool>,
}

impl Running {
    /// Stops listening and closes the connections; the core goes on.
    pub fn stop(&self) {
        self.stop.send_replace(true);
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Serves `entries` of `core` on 127.0.0.1, at `config.port`.
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
    let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, config.port))?;
    listener.set_nonblocking(true)?;
    let listener = {
        let _on = rt::handle().enter();
        TcpListener::from_std(listener)?
    };
    let addr = listener.local_addr()?;
    let tokens = escouade_mcp::Tokens::default();
    *tokens.external.write() = Some(config.token);
    let server = Arc::new(Server {
        core,
        entries,
        port: addr.port(),
        tokens,
        calls: Arc::new(Semaphore::new(calls)),
    });
    let (stop, stopped) = watch::channel(false);
    rt::spawn(server.accept(listener, stopped));
    Ok(Running { addr, stop })
}

struct Server {
    core: Arc<Core>,
    entries: &'static [Entry],
    port: u16,
    /// The one token, compared in constant time.
    tokens: escouade_mcp::Tokens,
    /// The calls under way, all connections together.
    calls: Arc<Semaphore>,
}

impl Server {
    async fn accept(self: Arc<Self>, listener: TcpListener, mut stopped: watch::Receiver<bool>) {
        let connections = Arc::new(Semaphore::new(MAX_CONNECTIONS));
        loop {
            let accepted = tokio::select! {
                _ = stopped.wait_for(|s| *s) => return,
                accepted = listener.accept() => accepted,
            };
            let (tcp, peer) = match accepted {
                Ok(accepted) => accepted,
                Err(e) => {
                    log::debug!("remote API: accept failed: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            // Bound to the loopback: another peer cannot come, and is closed if it does.
            if !peer.ip().is_loopback() {
                continue;
            }
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

    /// Whether the handshake `req` may open: one `Host`, this server's; no `Origin`; one `Authorization`, the token.
    fn admit(&self, req: &Request) -> Result<(), StatusCode> {
        let port = self.port;
        let mine = |host: &str| {
            [format!("127.0.0.1:{port}"), format!("localhost:{port}")]
                .iter()
                .any(|h| h.eq_ignore_ascii_case(host))
        };
        let headers = req.headers();
        // A header given twice could be read differently by another reader: refused.
        let once = |name| headers.get_all(name).iter().count() == 1;
        let host = headers
            .get(header::HOST)
            .filter(|_| once(header::HOST))
            .and_then(|h| h.to_str().ok());
        let authority = req.uri().authority().map(|a| a.as_str());
        if !host.is_some_and(mine) || authority.is_some_and(|a| !mine(a)) {
            return Err(StatusCode::FORBIDDEN);
        }
        if headers.contains_key(header::ORIGIN) {
            return Err(StatusCode::FORBIDDEN);
        }
        if !once(header::AUTHORIZATION) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .map_or("", |(_, token)| token.trim());
        if self.tokens.caller(token).is_none() {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(())
    }

    async fn serve(self: Arc<Self>, tcp: TcpStream, mut stopped: watch::Receiver<bool>) {
        #[allow(clippy::result_large_err)] // The shape of tungstenite's callback.
        let check = |req: &Request, ok: Response| -> Result<Response, ErrorResponse> {
            self.admit(req).map(|()| ok).map_err(|status| {
                let mut refused = ErrorResponse::new(None);
                *refused.status_mut() = status;
                refused
            })
        };
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_REQUEST))
            .max_frame_size(Some(MAX_REQUEST));
        let handshake = tokio_tungstenite::accept_hdr_async_with_config(tcp, check, Some(config));
        let Ok(Ok(ws)) = tokio::time::timeout(HANDSHAKE, handshake).await else {
            return;
        };
        let (mut out, mut input) = ws.split();
        let hello = Message::text(wire::hello(&self.core.host.version()));
        let Ok(Ok(())) = tokio::time::timeout(WRITE, out.send(hello)).await else {
            return;
        };
        log::info!("remote API: a client connected");
        let (conn, text, bytes) = Conn::new(TEXT_QUEUE, BYTES_BUDGET);
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
