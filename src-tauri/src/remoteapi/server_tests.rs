//! The server on a core loaded on the fake host, port 0, with a client of `tokio_tungstenite`: terminals of the system's
//! shell, no harness.

use super::*;
use crate::commands::Subscriber;
use crate::model::UiState;
use crate::paths::{test_dir, DataDir};
use escouade_macros::command;
use serde_json::json;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::WebSocketStream;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// A core on the fake host, its terminals and processes stopped with the test.
struct Loaded {
    core: Arc<Core>,
    dir: PathBuf,
}

impl Drop for Loaded {
    fn drop(&mut self) {
        self.core.pty.kill_all();
        self.core.stop_for_good();
    }
}

fn load(name: &str) -> Loaded {
    let dir = test_dir(name);
    let data = DataDir::new(dir.join("data"));
    Loaded {
        core: Core::load(crate::host::Fake::new(), data).0,
        dir,
    }
}

fn serve(loaded: &Loaded) -> Running {
    serve_table(loaded, table::entries())
}

fn serve_table(loaded: &Loaded, entries: &'static [Entry]) -> Running {
    let config = Config {
        port: 0,
        token: TOKEN.into(),
    };
    start(loaded.core.clone(), entries, config).unwrap()
}

/// A command of the `term_` family, run on that lane, that panics.
#[command]
fn term_breaks() {
    panic!("boom");
}

const BREAKS: &[Entry] = &[__REMOTE_TERM_BREAKS, commands::__REMOTE_PR_REVIEWS];

/// The slow calls running now, and the most seen at once.
static SLOW: AtomicUsize = AtomicUsize::new(0);
static SLOW_PEAK: AtomicUsize = AtomicUsize::new(0);

/// A synchronous call that holds its blocking thread a while.
fn slow() {
    let now = SLOW.fetch_add(1, SeqCst) + 1;
    SLOW_PEAK.fetch_max(now, SeqCst);
    std::thread::sleep(Duration::from_millis(400));
    SLOW.fetch_sub(1, SeqCst);
}

// Three names: three lanes, which nothing but the server's cap keeps from running at once.
#[command]
fn slow_a() {
    slow()
}

#[command]
fn slow_b() {
    slow()
}

#[command]
fn slow_c() {
    slow()
}

const SLOW_TABLE: &[Entry] = &[__REMOTE_SLOW_A, __REMOTE_SLOW_B, __REMOTE_SLOW_C];

/// A client: its socket, and the bytes of each stream read while waiting for a text.
struct Ws {
    ws: WebSocketStream<TcpStream>,
    bytes: std::collections::HashMap<u32, Vec<u8>>,
}

/// A client with the token, past the greeting.
async fn connect(addr: SocketAddr) -> Ws {
    let mut ws = connect_greeted(addr).await;
    let hello = text(&mut ws).await;
    assert!(hello.get("hello").is_some(), "{hello}");
    ws
}

async fn connect_greeted(addr: SocketAddr) -> Ws {
    let tcp = TcpStream::connect(addr).await.unwrap();
    let mut req = format!("ws://127.0.0.1:{}/", addr.port())
        .into_client_request()
        .unwrap();
    let bearer = format!("Bearer {TOKEN}").parse().unwrap();
    req.headers_mut().insert(header::AUTHORIZATION, bearer);
    let ws = tokio_tungstenite::client_async(req, tcp).await.unwrap().0;
    Ws {
        ws,
        bytes: Default::default(),
    }
}

/// The next frame: a text as JSON; a stream's bytes kept, `None`.
async fn next(ws: &mut Ws) -> Option<Value> {
    let frame = tokio::time::timeout(Duration::from_secs(20), ws.ws.next())
        .await
        .expect("a frame in time")
        .expect("the connection open")
        .unwrap();
    match frame {
        Message::Text(t) => Some(serde_json::from_str(&t).unwrap()),
        Message::Binary(b) => {
            let stream = u32::from_be_bytes(b[..4].try_into().unwrap());
            ws.bytes
                .entry(stream)
                .or_default()
                .extend_from_slice(&b[4..]);
            None
        }
        _ => None,
    }
}

/// The next text frame, as JSON.
async fn text(ws: &mut Ws) -> Value {
    loop {
        if let Some(v) = next(ws).await {
            return v;
        }
    }
}

async fn send(ws: &mut Ws, id: u64, cmd: &str, args: Value, stream: Option<u32>) {
    let mut request = json!({ "id": id, "cmd": cmd, "args": args });
    if let Some(n) = stream {
        request["stream"] = n.into();
    }
    ws.ws
        .send(Message::text(request.to_string()))
        .await
        .unwrap();
}

/// The answer to `id`, the texts before it skipped.
async fn answer(ws: &mut Ws, id: u64) -> Value {
    loop {
        let v = text(ws).await;
        if v["id"] == id {
            return v;
        }
    }
}

async fn call(ws: &mut Ws, id: u64, cmd: &str, args: Value) -> Value {
    send(ws, id, cmd, args, None).await;
    answer(ws, id).await
}

/// The bytes of `stream` so far, once `until` holds of them.
async fn read_stream(ws: &mut Ws, stream: u32, until: impl Fn(&str) -> bool) -> String {
    loop {
        let got =
            String::from_utf8_lossy(ws.bytes.get(&stream).map_or(&[][..], |b| b)).into_owned();
        if until(&got) {
            return got;
        }
        next(ws).await;
    }
}

/// A project of a new repository, and the first shell of this system that runs the tests' POSIX lines: on Windows,
/// Git Bash, not PowerShell (which comes first there and reads none of them).
async fn project(loaded: &Loaded) -> (String, String) {
    let repo = crate::core_tests::repo(&loaded.dir);
    let p = loaded
        .core
        .create_project(
            &repo.to_string_lossy(),
            "demo",
            "oklch(0.72 0.12 48)",
            false,
            None,
            true,
        )
        .await
        .unwrap();
    let shell = crate::pty::detect_shells(&loaded.core.settings.read())
        .into_iter()
        .find(|s| cfg!(unix) || s.id == "bash")
        .expect("a POSIX shell (on Windows, Git Bash)")
        .id;
    (p.id, shell)
}

/// A terminal of the project under `stream`: its id.
async fn spawn(ws: &mut Ws, id: u64, project: &str, shell: &str, stream: u32) -> String {
    let args = json!({ "projectId": project, "shell": shell, "name": "t", "cols": 80, "rows": 24 });
    send(ws, id, "term_spawn", args, Some(stream)).await;
    let a = answer(ws, id).await;
    a["ok"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{a}"))
        .to_string()
}

async fn write(ws: &mut Ws, id: u64, term: &str, data: &str) {
    let a = call(ws, id, "term_write", json!({ "id": term, "data": data })).await;
    assert!(a.get("ok").is_some(), "{a}");
}

#[test]
fn the_server_greets_with_its_version_and_protocol() {
    let loaded = load("remote-server-hello");
    let running = serve(&loaded);
    rt::block_on(async {
        let mut ws = connect_greeted(running.addr).await;
        let version = loaded.core.host.version();
        assert_eq!(
            text(&mut ws).await,
            json!({ "hello": { "version": version, "protocol": 1 } })
        );
    });
}

#[test]
fn a_command_is_answered_under_its_id() {
    let loaded = load("remote-server-answer");
    let running = serve(&loaded);
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        let ui = UiState {
            view: "board".into(),
            ..Default::default()
        };
        assert_eq!(
            call(&mut ws, 1, "set_ui", json!({ "ui": ui })).await,
            json!({ "id": 1, "ok": null })
        );
        // Sent one after the other, before any answer.
        send(&mut ws, 2, "subscribe", json!({}), None).await;
        send(&mut ws, 3, "pr_reviews", json!({ "projectId": "p1" }), None).await;
        let (mut two, mut three) = (None, None);
        while two.is_none() || three.is_none() {
            let v = text(&mut ws).await;
            match v["id"].as_u64() {
                Some(2) => two = Some(v),
                Some(3) => three = Some(v),
                _ => {}
            }
        }
        assert_eq!(two.unwrap()["ok"]["ui"]["view"], "board");
        assert_eq!(three.unwrap(), json!({ "id": 3, "ok": [] }));
    });
}

#[test]
fn an_event_reaches_a_subscribed_client() {
    let loaded = load("remote-server-event");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect(running.addr).await;
        let state = call(&mut ws, 1, "subscribe", json!({})).await;
        assert_eq!(state["ok"]["version"], loaded.core.host.version());
        // The terminal's end is the core's event, as the window learns it.
        let term = spawn(&mut ws, 2, &project, &shell, 1).await;
        write(&mut ws, 3, &term, "exit\n").await;
        loop {
            let v = text(&mut ws).await;
            if v["event"]["type"] == "terminalExit" {
                assert_eq!(v["event"]["id"], term.as_str());
                break;
            }
        }
    });
}

#[test]
fn a_terminal_stream_arrives_in_binary_frames_under_its_number() {
    let loaded = load("remote-server-stream");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect(running.addr).await;
        let term = spawn(&mut ws, 1, &project, &shell, 7).await;
        write(&mut ws, 2, &term, "printf 'remote-%s\\n' line\n").await;
        let got = read_stream(&mut ws, 7, |s| s.contains("remote-line")).await;
        assert!(got.contains("remote-line"), "{got}");
        // A number still in use is refused.
        let args =
            json!({ "projectId": project, "shell": shell, "name": "t", "cols": 80, "rows": 24 });
        send(&mut ws, 3, "term_spawn", args, Some(7)).await;
        assert_eq!(answer(&mut ws, 3).await["err"]["kind"], "args");
    });
}

#[test]
fn the_calls_of_a_family_keep_their_order() {
    let loaded = load("remote-server-order");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect(running.addr).await;
        let term = spawn(&mut ws, 1, &project, &shell, 3).await;
        // `cat` gives back each line as it gets it; the quotes keep the command's echo from saying READY.
        write(&mut ws, 2, &term, "stty -echo; echo RE''ADY; exec cat\n").await;
        read_stream(&mut ws, 3, |s| s.contains("READY")).await;
        for n in 0..200u64 {
            let args = json!({ "id": term, "data": format!("<{n}>\n") });
            send(&mut ws, 10 + n, "term_write", args, None).await;
        }
        let got = read_stream(&mut ws, 3, |s| s.contains("<199>")).await;
        let after = got.split("READY").last().unwrap();
        let mut seen = Vec::new();
        for part in after.split('<').skip(1) {
            let n: u64 = part.split('>').next().unwrap().parse().unwrap();
            if !seen.contains(&n) {
                seen.push(n);
            }
        }
        assert_eq!(seen, (0..200).collect::<Vec<_>>());
    });
}

/// The status line of a handshake whose headers are `headers`.
fn handshake(addr: SocketAddr, headers: &str) -> String {
    handshake_at(addr, "/", headers)
}

/// The same, for the request target `target`.
fn handshake_at(addr: SocketAddr, target: &str, headers: &str) -> String {
    let mut tcp = std::net::TcpStream::connect(addr).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let request = format!(
        "GET {target} HTTP/1.1\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n{headers}\r\n"
    );
    tcp.write_all(request.as_bytes()).unwrap();
    // Up to the end of the answer's head: an accepted connection stays open.
    let (mut got, mut buf) = (Vec::new(), [0u8; 1024]);
    while !got.windows(4).any(|w| w == b"\r\n\r\n") {
        match tcp.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
        }
    }
    let got = String::from_utf8_lossy(&got).into_owned();
    assert!(!got.contains(TOKEN));
    got.lines().next().unwrap_or_default().to_string()
}

fn host(addr: SocketAddr) -> String {
    format!("Host: 127.0.0.1:{}\r\n", addr.port())
}

fn bearer(token: &str) -> String {
    format!("Authorization: Bearer {token}\r\n")
}

#[test]
fn the_right_handshake_is_let_in() {
    let loaded = load("remote-server-in");
    let running = serve(&loaded);
    let line = handshake(running.addr, &(host(running.addr) + &bearer(TOKEN)));
    assert!(line.contains(" 101 "), "{line}");
    let localhost = format!("Host: LOCALHOST:{}\r\n", running.addr.port());
    let line = handshake(running.addr, &(localhost + &bearer(TOKEN)));
    assert!(line.contains(" 101 "), "{line}");
}

#[test]
fn a_wrong_token_is_refused() {
    let loaded = load("remote-server-wrong-token");
    let running = serve(&loaded);
    let wrong = TOKEN.replace('0', "1");
    let line = handshake(running.addr, &(host(running.addr) + &bearer(&wrong)));
    assert!(line.contains(" 401 "), "{line}");
}

#[test]
fn a_missing_token_is_refused() {
    let loaded = load("remote-server-no-token");
    let running = serve(&loaded);
    let line = handshake(running.addr, &host(running.addr));
    assert!(line.contains(" 401 "), "{line}");
}

#[test]
fn a_foreign_host_is_refused() {
    let loaded = load("remote-server-host");
    let running = serve(&loaded);
    for foreign in [
        "Host: evil.example\r\n".to_string(),
        format!(
            "Host: 127.0.0.1:{}\r\n",
            running.addr.port().wrapping_add(1)
        ),
        String::new(),
        // Given twice, whichever comes first.
        host(running.addr) + "Host: evil.example\r\n",
        "Host: evil.example\r\n".to_string() + &host(running.addr),
        host(running.addr) + &host(running.addr),
        // A trailing dot names the same machine, not this server's name.
        format!("Host: 127.0.0.1.:{}\r\n", running.addr.port()),
        format!("Host: localhost.:{}\r\n", running.addr.port()),
    ] {
        let line = handshake(running.addr, &(foreign.clone() + &bearer(TOKEN)));
        assert!(line.contains(" 403 "), "{foreign:?}: {line}");
    }
    // An absolute target names its own host.
    let line = handshake_at(
        running.addr,
        "ws://evil.example/",
        &(host(running.addr) + &bearer(TOKEN)),
    );
    assert!(line.contains(" 403 "), "{line}");
}

#[test]
fn a_token_given_twice_is_refused() {
    let loaded = load("remote-server-two-tokens");
    let running = serve(&loaded);
    let twice = host(running.addr) + &bearer(TOKEN) + &bearer(TOKEN);
    let line = handshake(running.addr, &twice);
    assert!(line.contains(" 401 "), "{line}");
}

#[test]
fn any_origin_is_refused() {
    let loaded = load("remote-server-origin");
    let running = serve(&loaded);
    for origin in [
        format!("Origin: http://127.0.0.1:{}\r\n", running.addr.port()),
        "Origin: null\r\n".to_string(),
        "Origin:\r\n".to_string(),
    ] {
        let line = handshake(
            running.addr,
            &(host(running.addr) + &origin + &bearer(TOKEN)),
        );
        assert!(line.contains(" 403 "), "{origin:?}: {line}");
    }
}

#[test]
fn a_command_that_panics_on_a_lane_is_answered() {
    let loaded = load("remote-server-panics");
    let running = serve_table(&loaded, BREAKS);
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        for id in [1, 2] {
            let a = call(&mut ws, id, "term_breaks", json!({})).await;
            assert_eq!(a["err"]["kind"], "failed", "{a}");
            assert!(
                a["err"]["message"].as_str().unwrap().contains("boom"),
                "{a}"
            );
        }
        let a = call(&mut ws, 3, "pr_reviews", json!({ "projectId": "p1" })).await;
        assert_eq!(a["ok"], json!([]));
    });
}

#[test]
fn a_panic_in_the_servers_own_path_is_answered_as_a_failure() {
    async fn breaks() -> Result<Value, CallError> {
        panic!("boom")
    }
    let answer: Value =
        serde_json::from_str(&answer_of(7, rt::block_on(rt::spawn(breaks())))).unwrap();
    assert_eq!(answer["id"], 7);
    assert_eq!(answer["err"]["kind"], "failed", "{answer}");
    assert!(
        answer["err"]["message"].as_str().unwrap().contains("boom"),
        "{answer}"
    );
}

#[test]
fn the_calls_of_all_connections_stay_under_the_servers_cap() {
    let loaded = load("remote-server-cap");
    let config = Config {
        port: 0,
        token: TOKEN.into(),
    };
    // Two at once across the server, three asked on two connections.
    let running = start_with(loaded.core.clone(), SLOW_TABLE, config, 2).unwrap();
    rt::block_on(async {
        let (mut one, mut two) = (connect(running.addr).await, connect(running.addr).await);
        send(&mut one, 1, "slow_a", json!({}), None).await;
        send(&mut one, 2, "slow_b", json!({}), None).await;
        send(&mut two, 3, "slow_c", json!({}), None).await;
        while SLOW.load(SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        // The server full, the app's own blocking work still runs.
        let app = tokio::time::timeout(Duration::from_secs(5), rt::spawn_blocking(|| 42));
        assert_eq!(app.await.expect("in time").unwrap(), 42);
        let mut ids = Vec::new();
        while ids.len() < 2 {
            let v = text(&mut one).await;
            assert_eq!(v.get("ok"), Some(&Value::Null), "{v}");
            ids.push(v["id"].as_u64().unwrap());
        }
        assert_eq!(answer(&mut two, 3).await, json!({ "id": 3, "ok": null }));
        ids.sort_unstable();
        assert_eq!(ids, [1, 2]);
    });
    assert_eq!(SLOW_PEAK.load(SeqCst), 2);
}

#[test]
fn a_desktop_command_says_so() {
    let loaded = load("remote-server-desktop");
    let running = serve(&loaded);
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        let a = call(&mut ws, 4, "quit_app", json!({})).await;
        assert_eq!(a["err"]["kind"], "desktop", "{a}");
    });
}

#[test]
fn an_unknown_command_says_so() {
    let loaded = load("remote-server-unknown");
    let running = serve(&loaded);
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        let a = call(&mut ws, 5, "no_such_command", json!({})).await;
        assert_eq!(a["err"]["kind"], "unknown", "{a}");
    });
}

#[test]
fn a_client_that_leaves_mid_stream_stops_nothing() {
    let loaded = load("remote-server-leaves");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect(running.addr).await;
        let term = spawn(&mut ws, 1, &project, &shell, 1).await;
        write(
            &mut ws,
            2,
            &term,
            "while :; do echo tick; sleep 0.01; done\n",
        )
        .await;
        read_stream(&mut ws, 1, |s| s.matches("tick").count() > 3).await;
        drop(ws);
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(loaded.core.pty.list().iter().any(|t| t.id == term));
        let mut again = connect(running.addr).await;
        let a = call(&mut again, 1, "pr_reviews", json!({ "projectId": project })).await;
        assert_eq!(a["ok"], json!([]));
        assert!(loaded.core.pty.list().iter().any(|t| t.id == term));
    });
}

fn exit(id: &str) -> UiEvent {
    UiEvent::TerminalExit {
        id: id.into(),
        code: None,
    }
}

#[test]
fn the_window_keeps_its_events_while_a_remote_client_comes_and_goes() {
    let loaded = load("remote-server-window-again");
    let running = serve(&loaded);
    let got = Arc::new(Mutex::new(Vec::new()));
    let to = got.clone();
    let window = Sink::new(move |e: UiEvent| {
        to.lock()
            .push(serde_json::to_value(&e).unwrap()["id"].clone());
        Ok(())
    });
    commands::subscribe_to(&loaded.core, window.clone(), Subscriber::Window);
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        call(&mut ws, 1, "subscribe", json!({})).await;
        // A second subscribe takes the place of the first: each event comes once.
        call(&mut ws, 2, "subscribe", json!({})).await;
        loaded.core.hub.emit(exit("during"));
        loaded.core.hub.emit(exit("next"));
        let mut seen = Vec::new();
        while seen.last() != Some(&json!("next")) {
            let v = text(&mut ws).await;
            if v.get("event").is_some() {
                seen.push(v["event"]["id"].clone());
            }
        }
        assert_eq!(seen, [json!("during"), json!("next")]);
        drop(ws);
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    // The connection gone, its subscription left with it.
    loaded.core.hub.emit(exit("gone"));
    assert_eq!(*got.lock(), [json!("during"), json!("next"), json!("gone")]);
    // A window that reloads takes its own place again.
    got.lock().clear();
    commands::subscribe_to(&loaded.core, window, Subscriber::Window);
    loaded.core.hub.emit(exit("after"));
    assert_eq!(*got.lock(), [json!("after")]);
    // A remote sink after its client left says so, each time, without waiting.
    let (conn, _text, _bytes) = Conn::new(4, 4);
    let remote = conn.event_sink();
    conn.close("left");
    assert_eq!(remote.send(exit("x")), Err(Closed));
    assert_eq!(remote.send(exit("x")), Err(Closed));
}

#[test]
fn a_remote_subscribe_undoes_nothing_of_the_others() {
    let dir = test_dir("remote-server-undoes-nothing");
    let host = crate::host::Fake::new();
    let (core, _rx) = Core::load(host.clone(), DataDir::new(dir.join("data")));
    commands::subscribe_to(&core, Sink::new(|_| Ok(())), Subscriber::Window);
    assert_eq!(*host.dictations_cancelled.lock(), 1);
    // What the window holds: a message for an agent, a file left unsaved, a crash to tell.
    core.held.lock().insert("a1".into());
    core.unsaved.store(1, SeqCst);
    let crash = crate::model::PageCrash {
        project_id: Some("p1".into()),
        agent_id: None,
    };
    *core.page_crash.lock() = Some(crash.clone());
    let (state, sub) = commands::subscribe_to(&core, Sink::new(|_| Ok(())), Subscriber::Remote);
    assert!(sub.is_some());
    assert_eq!(*host.dictations_cancelled.lock(), 1);
    assert!(core.held.lock().contains("a1"));
    assert_eq!(core.unsaved.load(SeqCst), 1);
    assert!(serde_json::to_value(&state).unwrap()["pageCrash"].is_null());
    assert_eq!(*core.page_crash.lock(), Some(crash));
}

#[test]
fn nothing_is_lost_between_the_initial_state_and_the_events() {
    let loaded = load("remote-server-no-gap");
    let core = loaded.core.clone();
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    // As the core does: the value changed, then its event emitted.
    let emitter = {
        let (core, stop) = (core.clone(), stop.clone());
        std::thread::spawn(move || {
            let mut n = 0u64;
            while !stop.load(SeqCst) {
                n += 1;
                core.board_issues.lock().insert("p".into(), n.to_string());
                core.hub.emit(UiEvent::BoardIssue {
                    project_id: "p".into(),
                    issue: Some(n.to_string()),
                });
            }
            n
        })
    };
    let got = Arc::new(Mutex::new(Vec::new()));
    let to = got.clone();
    let sink = Sink::new(move |e: UiEvent| {
        if let UiEvent::BoardIssue { issue: Some(n), .. } = e {
            to.lock().push(n.parse::<u64>().unwrap());
        }
        Ok(())
    });
    std::thread::sleep(Duration::from_millis(20));
    let (state, _sub) = commands::subscribe_to(&core, sink, Subscriber::Remote);
    // Some events after the subscription, however slow the machine.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while got.lock().len() < 100 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    stop.store(true, SeqCst);
    let last = emitter.join().unwrap();
    let initial: u64 = serde_json::to_value(&state).unwrap()["boardIssues"]["p"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let got = got.lock().clone();
    // The events from the first one received to the last, each once and in order.
    let first = *got
        .first()
        .expect("no event after the subscription in 10 s");
    assert_eq!(got, (first..=last).collect::<Vec<_>>());
    // None between the state and the first event: the state applied, then the events, give the last value.
    assert!(
        first <= initial + 1,
        "lost: {} events between the state ({initial}) and the first event ({first})",
        first - initial - 1
    );
}

#[test]
fn a_slow_client_does_not_grow_the_queue_without_bound() {
    let (conn, _text, mut bytes) = Conn::new(4, 2);
    let output = conn.open(9).unwrap();
    // A terminal's reads never wait: past the bound, the client is closed.
    assert_eq!(output.send(vec![0]), Ok(()));
    assert_eq!(output.send(vec![1]), Ok(()));
    assert_eq!(output.send(vec![2]), Err(Closed));
    assert!(conn.is_closed());
    assert_eq!(bytes.len(), 2);
    let first = bytes.blocking_recv().unwrap();
    assert_eq!(&first.into_data()[..], [0, 0, 0, 9, 0]);
    assert_eq!(output.send(vec![3]), Err(Closed));
    // Events and answers never wait either: one that finds the text queue full closes the client.
    let (conn, _text, _bytes) = Conn::new(4, 2);
    let events = conn.event_sink();
    for _ in 0..4 {
        events.send(exit("x")).unwrap();
    }
    assert_eq!(events.send(exit("x")), Err(Closed));
    assert!(conn.is_closed());
    assert_eq!(conn.open(10).unwrap().send(vec![1]), Err(Closed));
}

#[test]
fn nothing_listens_without_the_variables() {
    let token = "x".repeat(32);
    assert_eq!(Config::parse(None, None).map(|c| c.is_some()), Ok(false));
    for (port, token) in [
        (Some("4000"), None),
        (None, Some(token.as_str())),
        (Some("port"), Some(token.as_str())),
        (Some("0"), Some(token.as_str())),
        (Some("70000"), Some(token.as_str())),
        (Some("4000"), Some(&token[1..])),
    ] {
        let why = Config::parse(port, token).map(|_| ()).unwrap_err();
        assert!(why.starts_with("ESCOUADE_REMOTE_"), "{why}");
        assert!(!why.contains("xxxx"), "{why}");
    }
    let config = Config::parse(Some("4000"), Some(&token)).unwrap().unwrap();
    assert_eq!(config.port, 4000);
    assert!(!format!("{config:?}").contains("xxxx"));
}

/// The only test that touches these two variables.
#[test]
fn the_variables_are_taken_out_of_the_environment() {
    std::env::set_var(PORT_VAR, "4000");
    std::env::set_var(TOKEN_VAR, "y".repeat(40));
    let config = take_from_env().unwrap().unwrap();
    assert_eq!(config.port, 4000);
    assert!(std::env::vars().all(|(k, _)| k != PORT_VAR && k != TOKEN_VAR));
    assert_eq!(take_from_env().map(|c| c.is_some()), Ok(false));
}
