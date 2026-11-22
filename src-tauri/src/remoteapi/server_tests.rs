// The terminal helpers serve the Unix-only tests below.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]
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
use tokio::net::TcpStream;
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
        socket: None,
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: None,
        origins: Vec::new(),
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

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
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

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
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

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
#[test]
fn a_second_client_reads_a_terminal_already_running() {
    let loaded = load("remote-server-second-reader");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut first = connect(running.addr).await;
        let term = spawn(&mut first, 1, &project, &shell, 1).await;
        write(&mut first, 2, &term, "printf 'before-%s\\n' attach\n").await;
        read_stream(&mut first, 1, |s| s.contains("before-attach")).await;
        let mut second = connect(running.addr).await;
        let args = json!({ "id": term, "cols": 80, "rows": 24 });
        send(&mut second, 1, "term_attach", args, Some(4)).await;
        assert_eq!(answer(&mut second, 1).await["ok"]["id"], term.as_str());
        // The screen kept, then what follows, for both.
        read_stream(&mut second, 4, |s| s.contains("before-attach")).await;
        write(&mut second, 2, &term, "printf 'after-%s\\n' attach\n").await;
        let got = read_stream(&mut second, 4, |s| s.contains("after-attach")).await;
        assert_eq!(got.matches("before-attach").count(), 1, "{got}");
        read_stream(&mut first, 1, |s| s.contains("after-attach")).await;
        // A launch is not a terminal, and the reverse: refused, as a terminal that ended is.
        let a = call(
            &mut second,
            3,
            "run_attach",
            json!({ "id": term, "cols": 80, "rows": 24 }),
        )
        .await;
        assert!(a.get("err").is_some(), "{a}");
        let a = call(
            &mut second,
            4,
            "term_attach",
            json!({ "id": "gone", "cols": 80, "rows": 24 }),
        )
        .await;
        assert!(a.get("err").is_some(), "{a}");
    });
}

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
#[test]
fn an_attach_without_a_size_reads_and_one_with_a_size_resizes() {
    let loaded = load("remote-server-attach-size");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect(running.addr).await;
        let term = spawn(&mut ws, 1, &project, &shell, 1).await;
        // A passive reader: the terminal keeps its 80 columns and 24 rows.
        send(&mut ws, 2, "term_attach", json!({ "id": term }), Some(2)).await;
        assert!(answer(&mut ws, 2).await.get("ok").is_some());
        write(&mut ws, 3, &term, "stty size\n").await;
        read_stream(&mut ws, 2, |s| s.contains("24 80")).await;
        // With a size: drawn again at it.
        let args = json!({ "id": term, "cols": 100, "rows": 30 });
        send(&mut ws, 4, "term_attach", args, Some(3)).await;
        assert!(answer(&mut ws, 4).await.get("ok").is_some());
        write(&mut ws, 5, &term, "stty size\n").await;
        read_stream(&mut ws, 3, |s| s.contains("30 100")).await;
    });
}

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
#[test]
fn a_client_that_comes_back_finds_its_terminals_screen() {
    let loaded = load("remote-server-comes-back");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect(running.addr).await;
        let term = spawn(&mut ws, 1, &project, &shell, 1).await;
        write(&mut ws, 2, &term, "printf 'kept-%s\\n' screen\n").await;
        read_stream(&mut ws, 1, |s| s.contains("kept-screen")).await;
        drop(ws);
        tokio::time::sleep(Duration::from_millis(300)).await;
        // Printed while no one reads it.
        loaded
            .core
            .pty
            .write(&term, b"printf 'while-%s\\n' away\n")
            .unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut again = connect(running.addr).await;
        let args = json!({ "id": term, "cols": 80, "rows": 24 });
        send(&mut again, 1, "term_attach", args, Some(1)).await;
        assert!(answer(&mut again, 1).await.get("ok").is_some());
        let got = read_stream(&mut again, 1, |s| s.contains("while-away")).await;
        assert!(got.contains("kept-screen"), "{got}");
        // The same client twice, under two numbers: each its replay.
        let args = json!({ "id": term, "cols": 80, "rows": 24 });
        send(&mut again, 2, "term_attach", args, Some(2)).await;
        assert!(answer(&mut again, 2).await.get("ok").is_some());
        read_stream(&mut again, 2, |s| s.contains("while-away")).await;
    });
}

/// The numbers of the lines `line-N` in `s`, in the order they come.
fn lines_numbered(s: &str) -> Vec<u32> {
    s.split("line-")
        .skip(1)
        .filter_map(|p| {
            let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
#[test]
fn a_client_resumes_after_a_cut() {
    const LINES: u32 = 300;
    let loaded = load("remote-server-resume");
    let running = serve(&loaded);
    // The window, subscribed all along: every event once, in order.
    let window = Arc::new(Mutex::new(Vec::new()));
    let to = window.clone();
    let sink = Sink::new(move |e: UiEvent| {
        to.lock().push(serde_json::to_value(&e).unwrap());
        Ok(())
    });
    commands::subscribe_to(&loaded.core, sink, Subscriber::Window);
    // Events all along, numbered.
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ticker = {
        let (core, stop) = (loaded.core.clone(), stop.clone());
        std::thread::spawn(move || {
            let mut n = 0u64;
            while !stop.load(SeqCst) {
                n += 1;
                core.hub
                    .emit(UiEvent::VectorsDownload { done: n, total: 0 });
                std::thread::sleep(Duration::from_millis(5));
            }
            n
        })
    };
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut first = connect(running.addr).await;
        call(&mut first, 1, "subscribe", json!({})).await;
        let a = spawn(&mut first, 2, &project, &shell, 1).await;
        let b = spawn(&mut first, 3, &project, &shell, 2).await;
        // Lines written for a few seconds, through the cut.
        let script = format!(
            "i=0; while [ $i -lt {LINES} ]; do i=$((i+1)); echo line-$i; sleep 0.01; done\n"
        );
        write(&mut first, 4, &a, &script).await;
        write(&mut first, 5, &b, &script).await;
        let seen = |ws: &Ws, n: u32| {
            lines_numbered(&String::from_utf8_lossy(
                ws.bytes.get(&n).map_or(&[][..], |b| b),
            ))
        };
        while seen(&first, 1).len() < 20 || seen(&first, 2).len() < 20 {
            next(&mut first).await;
        }
        let before = [seen(&first, 1), seen(&first, 2)];
        // Cut: the socket dropped, no closing frame sent.
        drop(first);
        tokio::time::sleep(Duration::from_millis(500)).await;
        // Back: the state, its terminals, their screens.
        let mut again = connect(running.addr).await;
        let state = call(&mut again, 1, "subscribe", json!({})).await;
        let terminals = state["ok"]["terminals"].as_array().unwrap().clone();
        for term in [&a, &b] {
            let found = terminals.iter().find(|t| t["id"] == term.as_str());
            let found = found.unwrap_or_else(|| panic!("{term} not in {terminals:?}"));
            assert_eq!(found["launch"], false);
        }
        for (n, term) in [(1u32, &a), (2, &b)] {
            send(
                &mut again,
                10 + u64::from(n),
                "term_attach",
                json!({ "id": term }),
                Some(n),
            )
            .await;
            assert!(answer(&mut again, 10 + u64::from(n))
                .await
                .get("ok")
                .is_some());
        }
        while seen(&again, 1).last() != Some(&LINES) || seen(&again, 2).last() != Some(&LINES) {
            next(&mut again).await;
        }
        for (n, before) in [(1u32, &before[0]), (2, &before[1])] {
            let after = seen(&again, n);
            // From what the ring kept to the last line: none missing, none twice.
            let from = after[0];
            assert_eq!(after, (from..=LINES).collect::<Vec<_>>());
            // It joins what the first connection saw: no line lost in the cut.
            assert!(
                from <= before.last().unwrap() + 1,
                "{from} after {before:?}"
            );
        }
    });
    stop.store(true, SeqCst);
    let last = ticker.join().unwrap();
    let got: Vec<u64> = window
        .lock()
        .iter()
        .filter(|e| e["type"] == "vectorsDownload")
        .map(|e| e["done"].as_u64().unwrap())
        .collect();
    assert_eq!(got, (1..=last).collect::<Vec<_>>());
}

#[test]
fn an_answer_given_by_one_client_is_seen_by_the_other() {
    // The core's own harness: an agent whose fake `claude` asks a permission.
    let h = crate::core_tests::harness("remote-server-answer-once");
    let loaded = Loaded {
        core: h.core.clone(),
        dir: h.dir.clone(),
    };
    let running = serve(&loaded);
    rt::block_on(async {
        let (p, _) = h.project(false).await;
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        let mut one = connect(running.addr).await;
        let mut other = connect(running.addr).await;
        call(&mut one, 1, "subscribe", json!({})).await;
        call(&mut other, 1, "subscribe", json!({})).await;
        h.core
            .send_message(&id, "permission".into(), vec![])
            .await
            .unwrap();
        h.wait("the permission asked", |h| {
            h.items(&id).iter().any(|i| i["kind"] == "permission")
        })
        .await;
        let request = h
            .items(&id)
            .into_iter()
            .find(|i| i["kind"] == "permission")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let args = json!({ "id": id, "requestId": request, "decision": "allow" });
        let a = call(&mut one, 2, "answer_permission", args.clone()).await;
        assert!(a.get("ok").is_some(), "{a}");
        // The other client sees it closed: its agent waits for nothing more.
        loop {
            let v = text(&mut other).await;
            let agent = &v["event"]["agent"];
            if v["event"]["type"] == "agent" && agent["id"] == id.as_str() {
                let pending = agent["pending"].as_array().unwrap();
                if !pending.iter().any(|r| r == request.as_str()) {
                    break;
                }
            }
        }
        // Its own answer comes too late: refused, as one from claude.ai would be.
        let late = call(&mut other, 2, "answer_permission", args).await;
        assert_eq!(late["err"]["kind"], "failed", "{late}");
        assert_eq!(
            late["err"]["message"],
            crate::agent::already_answered(),
            "{late}"
        );
    });
}

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
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
        // Another local port is a tunnel's (below), but only as a plain port.
        "Host: localhost\r\n".to_string(),
        "Host: localhost:\r\n".to_string(),
        "Host: 127.0.0.1:0\r\n".to_string(),
        "Host: 127.0.0.1:+9000\r\n".to_string(),
        "Host: 127.0.0.1:09000\r\n".to_string(),
        "Host: 127.0.0.1:99999\r\n".to_string(),
        "Host: evil.example:9000\r\n".to_string(),
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
fn a_foreign_origin_is_refused() {
    let loaded = load("remote-server-origin");
    let running = serve(&loaded);
    let port = running.addr.port();
    for origin in [
        format!("Origin: http://127.0.0.1:{}\r\n", port.wrapping_add(1)),
        format!("Origin: http://localhost:{}\r\n", port.wrapping_add(1)),
        format!("Origin: https://127.0.0.1:{port}\r\n"),
        format!("Origin: http://127.0.0.1:{port}/\r\n"),
        format!("Origin: http://evil.example:{port}\r\n"),
        "Origin: null\r\n".to_string(),
        "Origin:\r\n".to_string(),
        // Given twice, whichever comes first.
        format!("Origin: http://127.0.0.1:{port}\r\nOrigin: http://evil.example\r\n"),
    ] {
        let line = handshake(
            running.addr,
            &(host(running.addr) + &origin + &bearer(TOKEN)),
        );
        assert!(line.contains(" 403 "), "{origin:?}: {line}");
    }
}

#[test]
fn the_servers_own_origin_is_let_in() {
    let loaded = load("remote-server-own-origin");
    let running = serve(&loaded);
    let port = running.addr.port();
    for origin in [
        format!("http://127.0.0.1:{port}"),
        format!("http://localhost:{port}"),
    ] {
        let headers = host(running.addr) + &format!("Origin: {origin}\r\n") + &bearer(TOKEN);
        let line = handshake(running.addr, &headers);
        assert!(line.contains(" 101 "), "{origin}: {line}");
    }
}

#[test]
fn the_bearer_header_still_works_without_an_origin() {
    let loaded = load("remote-server-bearer");
    let running = serve(&loaded);
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        let a = call(&mut ws, 1, "data_dir", json!({})).await;
        assert!(a["ok"].is_string(), "{a}");
    });
}

/// The whole answer to the raw HTTP `request` (the server closes after it).
fn http(addr: SocketAddr, request: &str) -> String {
    let mut tcp = std::net::TcpStream::connect(addr).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    tcp.write_all(request.as_bytes()).unwrap();
    let mut got = Vec::new();
    let _ = tcp.read_to_end(&mut got);
    String::from_utf8_lossy(&got).into_owned()
}

/// `POST /pair` with `body`, from the server's own page.
fn post_pair(addr: SocketAddr, body: &str) -> String {
    let port = addr.port();
    http(
        addr,
        &format!(
            "POST /pair HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    )
}

/// The body of an answer, as JSON.
fn body_of(answer: &str) -> Value {
    serde_json::from_str(answer.split_once("\r\n\r\n").unwrap().1).unwrap()
}

/// `method path` of `escouade-server`'s own commands, with the key the server wrote beside the devices.
fn admin(loaded: &Loaded, addr: SocketAddr, method: &str, path: &str, body: &str) -> String {
    let control: Control =
        serde_json::from_slice(&std::fs::read(loaded.dir.join(CONTROL)).unwrap()).unwrap();
    assert_eq!(control.port, addr.port());
    http(
        addr,
        &format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\
             Content-Length: {}\r\n\r\n{body}",
            addr.port(),
            control.key,
            body.len()
        ),
    )
}

/// A code asked as `escouade-server pair` asks it.
fn pair_code(loaded: &Loaded, addr: SocketAddr) -> String {
    let answer = admin(loaded, addr, "POST", "/admin/pair", "");
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    let code = body_of(&answer);
    assert_eq!(code["minutes"], 10);
    code["code"].as_str().unwrap().to_string()
}

/// A device paired through the server: its token and its id.
fn pair(loaded: &Loaded, addr: SocketAddr, name: &str) -> (String, String) {
    let code = pair_code(loaded, addr);
    let answer = post_pair(addr, &json!({ "code": code, "name": name }).to_string());
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert_eq!(header_of(&answer, "cache-control").unwrap(), "no-store");
    let body = body_of(&answer);
    (
        body["token"].as_str().unwrap().to_string(),
        body["id"].as_str().unwrap().to_string(),
    )
}

/// The headers of a page's handshake that presents `token` by the subprotocol.
fn page_handshake(addr: SocketAddr, token: &str) -> String {
    host(addr)
        + &format!(
            "Origin: http://127.0.0.1:{}\r\nSec-WebSocket-Protocol: escouade, bearer.{token}\r\n",
            addr.port()
        )
}

/// A client of a device: its token by the subprotocol, as the page gives it, past the greeting.
async fn connect_device(addr: SocketAddr, token: &str) -> Ws {
    let tcp = TcpStream::connect(addr).await.unwrap();
    let mut req = format!("ws://127.0.0.1:{}/", addr.port())
        .into_client_request()
        .unwrap();
    let protocols = format!("escouade, bearer.{token}").parse().unwrap();
    req.headers_mut()
        .insert(header::SEC_WEBSOCKET_PROTOCOL, protocols);
    let (ws, answer) = tokio_tungstenite::client_async(req, tcp).await.unwrap();
    // Answered with the protocol, never with the token.
    assert_eq!(
        answer
            .headers()
            .get(header::SEC_WEBSOCKET_PROTOCOL)
            .unwrap(),
        "escouade"
    );
    let mut ws = Ws {
        ws,
        bytes: Default::default(),
    };
    let hello = text(&mut ws).await;
    assert!(hello.get("hello").is_some(), "{hello}");
    ws
}

#[test]
fn a_code_pairs_a_device_whose_token_opens_the_socket() {
    let loaded = load("remote-server-pair");
    let running = serve(&loaded);
    let (token, id) = pair(&loaded, running.addr, "téléphone");
    assert_eq!(token.len(), 43);
    let line = handshake(running.addr, &page_handshake(running.addr, &token));
    assert!(line.contains(" 101 "), "{line}");
    // By `Authorization` too, as a download asked by `fetch` will be.
    let line = handshake(running.addr, &(host(running.addr) + &bearer(&token)));
    assert!(line.contains(" 101 "), "{line}");
    let answer = http(
        running.addr,
        &format!(
            "GET /device HTTP/1.1\r\n{}{}\r\n",
            host(running.addr),
            bearer(&token)
        ),
    );
    assert!(answer.starts_with("HTTP/1.1 204"), "{answer}");
    let listed = body_of(&admin(&loaded, running.addr, "GET", "/admin/devices", ""));
    assert_eq!(listed[0]["id"], id.as_str());
    assert_eq!(listed[0]["name"], "téléphone");
    assert!(!listed.to_string().contains(&token));
    rt::block_on(async {
        let mut ws = connect_device(running.addr, &token).await;
        let a = call(&mut ws, 1, "data_dir", json!({})).await;
        assert!(a["ok"].is_string(), "{a}");
    });
}

#[test]
fn a_code_is_refused_once_used_wrong_or_guessed() {
    let loaded = load("remote-server-pair-refused");
    let running = serve(&loaded);
    let pair_with = |code: &str| {
        post_pair(
            running.addr,
            &json!({ "code": code, "name": "pc" }).to_string(),
        )
    };
    // None asked.
    assert!(pair_with("0000-0000").starts_with("HTTP/1.1 403"));
    let code = pair_code(&loaded, running.addr);
    assert!(pair_with(&code).starts_with("HTTP/1.1 200"));
    assert!(pair_with(&code).starts_with("HTTP/1.1 403"));
    // Five wrong: the right one no longer pairs. Wrong for sure: its first character changed.
    let code = pair_code(&loaded, running.addr);
    let wrong = format!(
        "{}{}",
        if code.starts_with('0') { '1' } else { '0' },
        &code[1..]
    );
    for _ in 0..5 {
        assert!(pair_with(&wrong).starts_with("HTTP/1.1 403"));
    }
    assert!(pair_with(&code).starts_with("HTTP/1.1 403"));
    // A body that is no pairing, or too big, or a name left empty.
    let code = pair_code(&loaded, running.addr);
    for body in [
        "not json".to_string(),
        json!({ "code": code }).to_string(),
        json!({ "code": code, "name": " " }).to_string(),
        // Nothing left once cleaned.
        json!({ "code": code, "name": "\u{202E}\u{7}\u{200B}" }).to_string(),
    ] {
        assert!(
            post_pair(running.addr, &body).starts_with("HTTP/1.1 400"),
            "{body}"
        );
    }
    assert!(post_pair(running.addr, &"x".repeat(1025)).starts_with("HTTP/1.1 413"));
    // From a foreign page, not even the right code pairs.
    let body = json!({ "code": code, "name": "pc" }).to_string();
    let port = running.addr.port();
    let answer = http(
        running.addr,
        &format!(
            "POST /pair HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://evil.example\r\n\
             Content-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    );
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
    assert!(pair_with(&code).starts_with("HTTP/1.1 200"));
}

#[test]
fn the_servers_own_commands_need_its_key() {
    let loaded = load("remote-server-admin");
    let running = serve(&loaded);
    let (token, _) = pair(&loaded, running.addr, "pc");
    let port = running.addr.port();
    for auth in [String::new(), bearer(&token), bearer(TOKEN), bearer("x")] {
        for (method, path) in [
            ("POST", "/admin/pair"),
            ("GET", "/admin/devices"),
            ("POST", "/admin/revoke"),
        ] {
            let answer = http(
                running.addr,
                &format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth}Content-Length: 0\r\n\r\n"),
            );
            assert!(
                answer.starts_with("HTTP/1.1 401"),
                "{auth} {path}: {answer}"
            );
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(loaded.dir.join(CONTROL))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

#[test]
fn a_revoked_device_is_refused_and_its_open_connection_closed() {
    let loaded = load("remote-server-revoke");
    let running = serve(&loaded);
    let (token, id) = pair(&loaded, running.addr, "pc");
    let (other, _) = pair(&loaded, running.addr, "téléphone");
    rt::block_on(async {
        let mut ws = connect_device(running.addr, &token).await;
        let mut kept = connect_device(running.addr, &other).await;
        let a = call(&mut ws, 1, "data_dir", json!({})).await;
        assert!(a["ok"].is_string(), "{a}");
        let answer = admin(&loaded, running.addr, "POST", "/admin/revoke", &id);
        assert_eq!(body_of(&answer)["revoked"], true);
        // Closed at once, not at its next request.
        let end = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match ws.ws.next().await {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                }
            }
        })
        .await;
        assert!(end.is_ok(), "the revoked device's connection stays open");
        // The other device goes on.
        let a = call(&mut kept, 2, "data_dir", json!({})).await;
        assert!(a["ok"].is_string(), "{a}");
    });
    let line = handshake(running.addr, &page_handshake(running.addr, &token));
    assert!(line.contains(" 401 "), "{line}");
    let answer = admin(&loaded, running.addr, "POST", "/admin/revoke", &id);
    assert_eq!(body_of(&answer)["revoked"], false);
}

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
#[test]
fn a_revoked_device_is_cut_in_the_middle_of_its_streams() {
    let loaded = load("remote-server-revoke-streams");
    let running = serve(&loaded);
    let (token, id) = pair(&loaded, running.addr, "pc");
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect_device(running.addr, &token).await;
        call(&mut ws, 1, "subscribe", json!({})).await;
        let term = spawn(&mut ws, 2, &project, &shell, 1).await;
        write(
            &mut ws,
            3,
            &term,
            "while :; do echo flowing; sleep 0.05; done\n",
        )
        .await;
        read_stream(&mut ws, 1, |s| s.matches("flowing").count() >= 3).await;
        let answer = admin(&loaded, running.addr, "POST", "/admin/revoke", &id);
        assert_eq!(body_of(&answer)["revoked"], true);
        let end = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match ws.ws.next().await {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                }
            }
        })
        .await;
        assert!(
            end.is_ok(),
            "the revoked device's streams keep its connection open"
        );
    });
}

#[test]
fn the_token_of_the_environment_is_read_by_a_debug_build_alone() {
    assert_eq!(env_token(Some(TOKEN.into()), true), Some(TOKEN.into()));
    assert_eq!(env_token(Some(TOKEN.into()), false), None);
    assert_eq!(env_token(None, false), None);
}

#[test]
fn the_servers_file_goes_with_it_unless_another_wrote_its_own() {
    let loaded = load("remote-server-control-file");
    let file = loaded.dir.join(CONTROL);
    let running = serve(&loaded);
    assert!(file.exists());
    running.stop();
    assert!(!file.exists(), "server.json stays after the server");
    // A second server on the same folder: the first one's stop leaves its file.
    let first = serve(&loaded);
    let second = serve(&loaded);
    drop(first);
    let control: Control = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(control.port, second.addr.port());
    drop(second);
    assert!(!file.exists());
}

#[test]
fn no_answer_sets_a_cookie() {
    let loaded = load("remote-server-no-cookie");
    let running = serve_web(&loaded);
    let (token, _) = pair(&loaded, running.addr, "pc");
    let port = running.addr.port();
    let answers = [
        post_pair(
            running.addr,
            &json!({ "code": "0000-0000", "name": "pc" }).to_string(),
        ),
        get(running.addr, "/"),
        get(running.addr, "/assets/app-1234.js"),
        http(
            running.addr,
            &format!(
                "GET /device HTTP/1.1\r\n{}{}\r\n",
                host(running.addr),
                bearer(&token)
            ),
        ),
    ];
    for answer in answers {
        assert!(
            !answer.to_ascii_lowercase().contains("set-cookie"),
            "{answer}"
        );
    }
    // The session of the previous version is gone.
    let answer = http(
        running.addr,
        &format!(
            "POST /session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 32\r\n\r\n{TOKEN}"
        ),
    );
    assert!(answer.starts_with("HTTP/1.1 405"), "{answer}");
    // Paired, then the handshake: no cookie either; and a cookie is never taken for a token.
    let mut tcp = std::net::TcpStream::connect(running.addr).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let request = format!(
        "GET / HTTP/1.1\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n{}\r\n",
        page_handshake(running.addr, &token)
    );
    tcp.write_all(request.as_bytes()).unwrap();
    let mut head = vec![0u8; 1024];
    let n = tcp.read(&mut head).unwrap();
    let head = String::from_utf8_lossy(&head[..n]).to_ascii_lowercase();
    assert!(
        head.contains(" 101 ") && !head.contains("set-cookie"),
        "{head}"
    );
    assert!(!head.contains(&token.to_ascii_lowercase()), "{head}");
    for cookie in [format!("escouade-{port}={token}"), format!("token={token}")] {
        let line = handshake(
            running.addr,
            &(host(running.addr) + &format!("Cookie: {cookie}\r\n")),
        );
        assert!(line.contains(" 401 "), "{line}");
    }
}

#[test]
fn a_token_offered_twice_or_beside_a_wrong_bearer_is_refused() {
    let loaded = load("remote-server-protocols");
    let running = serve(&loaded);
    let (token, _) = pair(&loaded, running.addr, "pc");
    let twice = host(running.addr)
        + &format!("Sec-WebSocket-Protocol: escouade, bearer.{token}, bearer.{token}\r\n");
    assert!(handshake(running.addr, &twice).contains(" 401 "));
    let other = if token.starts_with('x') { "y" } else { "x" };
    let wrong = page_handshake(running.addr, &token) + &bearer(&format!("{other}{}", &token[1..]));
    assert!(handshake(running.addr, &wrong).contains(" 401 "));
    let made_up = page_handshake(running.addr, &token[1..]);
    assert!(handshake(running.addr, &made_up).contains(" 401 "));
}

/// The server of `loaded` with a web build: an `index.html`, an asset, and a secret beside the build.
fn serve_web(loaded: &Loaded) -> Running {
    let config = Config {
        port: 0,
        socket: None,
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: Some(web_build(loaded)),
        origins: Vec::new(),
    };
    start(loaded.core.clone(), table::entries(), config).unwrap()
}

/// That web build, written in `loaded`'s folder.
fn web_build(loaded: &Loaded) -> PathBuf {
    let web = loaded.dir.join("web");
    std::fs::create_dir_all(web.join("assets")).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html><title>Escouade</title>",
    )
    .unwrap();
    std::fs::write(web.join("assets/app-1234.js"), "console.log(1)").unwrap();
    std::fs::write(loaded.dir.join("Cargo.toml"), "secret").unwrap();
    web
}

/// `GET target` as the server's page asks it.
fn get(addr: SocketAddr, target: &str) -> String {
    http(
        addr,
        &format!(
            "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n\r\n",
            addr.port()
        ),
    )
}

/// The value of the header `name` in `answer`.
fn header_of(answer: &str, name: &str) -> Option<String> {
    answer.lines().take_while(|l| !l.is_empty()).find_map(|l| {
        let (n, v) = l.split_once(':')?;
        n.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

#[test]
fn the_web_build_is_served() {
    let loaded = load("remote-server-web");
    let running = serve_web(&loaded);
    let port = running.addr.port();
    let page = get(running.addr, "/");
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(
        page.ends_with("<!doctype html><title>Escouade</title>"),
        "{page}"
    );
    assert_eq!(
        header_of(&page, "content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    assert_eq!(header_of(&page, "cache-control").unwrap(), "no-cache");
    let csp = header_of(&page, "content-security-policy").unwrap();
    assert!(csp.contains("default-src 'self'"), "{csp}");
    assert!(
        csp.contains(&format!("connect-src 'self' ws://127.0.0.1:{port}")),
        "{csp}"
    );
    let asset = get(running.addr, "/assets/app-1234.js");
    assert!(asset.starts_with("HTTP/1.1 200"), "{asset}");
    assert!(asset.ends_with("console.log(1)"), "{asset}");
    assert_eq!(
        header_of(&asset, "content-type").unwrap(),
        "text/javascript; charset=utf-8"
    );
    assert!(header_of(&asset, "cache-control")
        .unwrap()
        .contains("immutable"));
    // Without a token: the files hold nothing of the core.
    assert!(!page.contains(TOKEN));
}

#[test]
fn nothing_outside_the_web_build_is_served() {
    let loaded = load("remote-server-web-outside");
    let running = serve_web(&loaded);
    for target in [
        "/absent.js",
        "/assets",
        "/assets/",
        "/../Cargo.toml",
        "/assets/../../Cargo.toml",
        "/%2e%2e/Cargo.toml",
        "/%2E%2E%2FCargo.toml",
        "/..%2fCargo.toml",
        "/..%5cCargo.toml",
        "/.%2e/Cargo.toml",
        "/assets/%2e%2e/%2e%2e/Cargo.toml",
        "//etc/passwd",
        "/index.html%00",
    ] {
        let answer = get(running.addr, target);
        assert!(answer.starts_with("HTTP/1.1 404"), "{target}: {answer}");
        assert!(!answer.contains("secret"), "{target}: {answer}");
    }
}

#[test]
fn without_a_web_build_only_the_socket_is_served() {
    let loaded = load("remote-server-no-web");
    let running = serve(&loaded);
    let answer = get(running.addr, "/");
    assert!(answer.starts_with("HTTP/1.1 404"), "{answer}");
}

#[test]
fn a_file_asked_for_another_host_is_refused() {
    let loaded = load("remote-server-web-host");
    let running = serve_web(&loaded);
    let answer = http(running.addr, "GET / HTTP/1.1\r\nHost: evil.example\r\n\r\n");
    assert!(answer.starts_with("HTTP/1.1 403"), "{answer}");
}

const PROXY: &str = "https://escouade.example.com";

/// The server of `serve_web`, behind a proxy at `PROXY` and at `http://box.lan:8080`, as `--origin` declares them.
fn serve_declared(loaded: &Loaded) -> Running {
    let config = Config {
        port: 0,
        socket: None,
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: Some(web_build(loaded)),
        origins: vec![
            parse_origin("HTTPS://Escouade.Example.com:443/").unwrap(),
            parse_origin("http://box.lan:8080").unwrap(),
        ],
    };
    start(loaded.core.clone(), table::entries(), config).unwrap()
}

/// A request of `method target` with `headers`, the whole answer.
fn request(addr: SocketAddr, method: &str, target: &str, headers: &str, body: &str) -> String {
    http(
        addr,
        &format!(
            "{method} {target} HTTP/1.1\r\n{headers}Content-Type: application/json\r\n\
             Content-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    )
}

#[test]
fn an_origin_is_declared_exactly() {
    for (url, origin) in [
        (
            "https://escouade.example.com",
            "https://escouade.example.com",
        ),
        (
            "HTTPS://Escouade.Example.COM/",
            "https://escouade.example.com",
        ),
        (
            "https://escouade.example.com:443",
            "https://escouade.example.com",
        ),
        ("http://box.lan:80", "http://box.lan"),
        ("http://box.lan:8080", "http://box.lan:8080"),
        ("https://[2001:db8::1]:8443", "https://[2001:db8::1]:8443"),
    ] {
        assert_eq!(parse_origin(url).as_deref(), Ok(origin), "{url}");
    }
    for url in [
        "",
        "escouade.example.com",
        "ftp://escouade.example.com",
        "ws://escouade.example.com",
        "https://",
        "https://user@escouade.example.com",
        "https://user:pass@escouade.example.com",
        "https://escouade.example.com/escouade",
        "https://escouade.example.com/?a=1",
        "https://escouade.example.com#top",
        "https://escouade.example.com:0",
        "https://escouade.example.com:99999",
        "https://escouade.example.com\u{0}",
        "https://*.example.com",
    ] {
        assert!(parse_origin(url).is_err(), "{url:?}");
    }
    // A name that is not ASCII: the message says how to write it.
    let why = parse_origin("https://équipe.example").unwrap_err();
    assert!(why.contains("punycode"), "{why}");
}

#[test]
fn a_declared_origin_is_let_in() {
    let loaded = load("remote-server-declared");
    let running = serve_declared(&loaded);
    let port = running.addr.port();
    for (host, origin) in [
        // The proxy that passes the browser's `Host`, or rewrites it to the server's.
        ("escouade.example.com".to_string(), PROXY),
        ("Escouade.Example.com".to_string(), PROXY),
        (format!("127.0.0.1:{port}"), PROXY),
        ("box.lan:8080".to_string(), "http://box.lan:8080"),
    ] {
        let headers = format!("Host: {host}\r\nOrigin: {origin}\r\n") + &bearer(TOKEN);
        let line = handshake(running.addr, &headers);
        assert!(line.contains(" 101 "), "{host} {origin}: {line}");
    }
    // The page, whose own WebSocket goes back the way it came, and the pairing through the proxy.
    let page = request(
        running.addr,
        "GET",
        "/",
        "Host: escouade.example.com\r\n",
        "",
    );
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    let csp = header_of(&page, "content-security-policy").unwrap();
    assert!(
        csp.contains("connect-src 'self' wss://escouade.example.com ws://box.lan:8080;"),
        "{csp}"
    );
    let page = request(running.addr, "GET", "/", "Host: box.lan:8080\r\n", "");
    let csp = header_of(&page, "content-security-policy").unwrap();
    assert!(
        csp.contains("connect-src 'self' ws://box.lan:8080 wss://escouade.example.com;"),
        "{csp}"
    );
    // The proxy that rewrites `Host` to the server's: the page of the origin still opens its `wss`.
    let page = request(
        running.addr,
        "GET",
        "/",
        &format!("Host: 127.0.0.1:{port}\r\n"),
        "",
    );
    let csp = header_of(&page, "content-security-policy").unwrap();
    assert!(csp.contains(" wss://escouade.example.com"), "{csp}");
    let code = pair_code(&loaded, running.addr);
    let body = json!({ "code": code, "name": "portable" }).to_string();
    let headers = format!("Host: escouade.example.com\r\nOrigin: {PROXY}\r\n");
    let answer = request(running.addr, "POST", "/pair", &headers, &body);
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    // Its own origins are still the server's.
    let line = handshake(running.addr, &page_handshake(running.addr, TOKEN));
    assert!(line.contains(" 101 "), "{line}");
}

#[test]
fn a_neighbour_of_a_declared_origin_is_refused() {
    let loaded = load("remote-server-neighbour");
    let running = serve_declared(&loaded);
    let port = running.addr.port();
    for origin in [
        "http://escouade.example.com",
        "https://escouade.example.com:8443",
        "https://escouade.example.com:443",
        "https://escouade.example.com/",
        "https://sub.escouade.example.com",
        "https://escouade.example.com.evil.example",
        "https://evilescouade.example.com",
        "https://escouade.example",
        "https://box.lan:8080",
        "http://box.lan",
        "http://box.lan:8081",
    ] {
        let headers = host(running.addr) + &format!("Origin: {origin}\r\n") + &bearer(TOKEN);
        let line = handshake(running.addr, &headers);
        assert!(line.contains(" 403 "), "{origin}: {line}");
    }
    for host in [
        "escouade.example.com:8443",
        "escouade.example.com:443",
        "sub.escouade.example.com",
        "escouade.example.com.evil.example",
        "escouade.example",
        "box.lan",
        "box.lan:8081",
    ] {
        let line = handshake(running.addr, &format!("Host: {host}\r\n{}", bearer(TOKEN)));
        assert!(line.contains(" 403 "), "{host}: {line}");
        let page = request(running.addr, "GET", "/", &format!("Host: {host}\r\n"), "");
        assert!(page.starts_with("HTTP/1.1 403"), "{host}: {page}");
    }
    // A declared host, but a page of another origin.
    let headers = format!(
        "Host: escouade.example.com\r\nOrigin: http://127.0.0.1:{}\r\n",
        port + 1
    );
    let line = handshake(running.addr, &(headers + &bearer(TOKEN)));
    assert!(line.contains(" 403 "), "{line}");
}

#[test]
fn the_forwarded_headers_are_not_believed() {
    let loaded = load("remote-server-forwarded");
    let running = serve_declared(&loaded);
    let forwarded = |host: &str, proto: &str, from: &str| {
        format!("X-Forwarded-Host: {host}\r\nX-Forwarded-Proto: {proto}\r\nX-Forwarded-For: {from}\r\nForwarded: for={from};host={host};proto={proto}\r\n")
    };
    // They do not let in what `Host` and `Origin` refuse…
    for headers in [
        "Host: evil.example\r\n".to_string()
            + &forwarded("escouade.example.com", "https", "127.0.0.1"),
        host(running.addr)
            + "Origin: https://evil.example\r\n"
            + &forwarded("evil.example", "https", "127.0.0.1"),
        host(running.addr)
            + "Origin: http://escouade.example.com\r\n"
            + &forwarded("escouade.example.com", "https", "127.0.0.1"),
    ] {
        let line = handshake(running.addr, &(headers.clone() + &bearer(TOKEN)));
        assert!(line.contains(" 403 "), "{headers}: {line}");
    }
    // …nor shut out what they let in.
    let headers = format!("Host: escouade.example.com\r\nOrigin: {PROXY}\r\n")
        + &forwarded("evil.example", "http", "203.0.113.9");
    let line = handshake(running.addr, &(headers + &bearer(TOKEN)));
    assert!(line.contains(" 101 "), "{line}");
    let wrong = TOKEN.replace('0', "1");
    let headers = format!("Host: escouade.example.com\r\nOrigin: {PROXY}\r\n")
        + &forwarded("127.0.0.1", "http", "127.0.0.1");
    let line = handshake(running.addr, &(headers + &bearer(&wrong)));
    assert!(line.contains(" 401 "), "{line}");
}

#[test]
fn a_tunnel_on_another_local_port_is_let_in_with_a_token() {
    let loaded = load("remote-server-tunnel");
    let running = serve_web(&loaded);
    // `ssh -L 9000:127.0.0.1:<port>`: the page is http://localhost:9000.
    let tunnel = if running.addr.port() == 9000 {
        9001
    } else {
        9000
    };
    for name in ["localhost", "127.0.0.1", "LocalHost"] {
        let host = format!("Host: {name}:{tunnel}\r\n");
        let origin = format!("Origin: http://{}:{tunnel}\r\n", name.to_ascii_lowercase());
        let line = handshake(running.addr, &(host.clone() + &origin + &bearer(TOKEN)));
        assert!(line.contains(" 101 "), "{name}: {line}");
        let line = handshake(running.addr, &(host.clone() + &bearer(TOKEN)));
        assert!(line.contains(" 101 "), "{name}: {line}");
        // Its token is what lets it in.
        let line = handshake(running.addr, &(host.clone() + &origin));
        assert!(line.contains(" 401 "), "{name}: {line}");
        let line = handshake(
            running.addr,
            &(host + &origin + &bearer(&TOKEN.replace('0', "1"))),
        );
        assert!(line.contains(" 401 "), "{name}: {line}");
    }
    // Another local page, through the tunnel or not, stays out.
    for headers in [
        format!("Host: localhost:{tunnel}\r\nOrigin: http://localhost:3000\r\n"),
        format!("Host: localhost:{tunnel}\r\nOrigin: http://127.0.0.1:{tunnel}\r\n"),
        format!("Host: localhost:{tunnel}\r\nOrigin: https://localhost:{tunnel}\r\n"),
        host(running.addr) + &format!("Origin: http://localhost:{tunnel}\r\n"),
        format!("Host: localhost:{tunnel}\r\nOrigin: null\r\n"),
    ] {
        let line = handshake(running.addr, &(headers.clone() + &bearer(TOKEN)));
        assert!(line.contains(" 403 "), "{headers}: {line}");
    }
    // The page loads and pairs through the tunnel.
    let host = format!("Host: localhost:{tunnel}\r\n");
    let page = request(running.addr, "GET", "/", &host, "");
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    let csp = header_of(&page, "content-security-policy").unwrap();
    assert!(csp.contains(&format!("ws://localhost:{tunnel};")), "{csp}");
    let code = pair_code(&loaded, running.addr);
    let body = json!({ "code": code, "name": "portable" }).to_string();
    let headers = host.clone() + &format!("Origin: http://localhost:{tunnel}\r\n");
    let answer = request(running.addr, "POST", "/pair", &headers, &body);
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    // Without `Origin`, no browser's page: a program of the machine, which reached the port anyway, may pair.
    let code = pair_code(&loaded, running.addr);
    let body = json!({ "code": code, "name": "script" }).to_string();
    let answer = request(running.addr, "POST", "/pair", &host, &body);
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
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
        socket: None,
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: None,
        origins: Vec::new(),
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

// shortcut: a live terminal session is not run on Windows here (ConPTY gave no frame on the CI runner), Unix only until it is.
#[cfg(unix)]
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
fn a_client_that_resumes_six_full_terminals_at_once_is_not_cut() {
    use crate::output::{Output, OUTPUT_KEPT};
    let loaded = load("remote-server-six-replays");
    let running = serve(&loaded);
    let terms: Vec<String> = (1..=6u8)
        .map(|n| {
            let id = format!("full-{n}");
            loaded.core.pty.insert_fake(&id, Box::new(std::io::sink()));
            let output = Output::new(OUTPUT_KEPT);
            for _ in 0..OUTPUT_KEPT / (32 << 10) {
                output.push(vec![b'a' + n; 32 << 10]);
            }
            assert!(loaded.core.pty.keep_output(&id, output));
            id
        })
        .collect();
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        // Back to back, as a client that comes back asks for its terminals.
        for (n, term) in terms.iter().enumerate() {
            let n = n as u32 + 1;
            send(
                &mut ws,
                n.into(),
                "term_attach",
                json!({ "id": term }),
                Some(n),
            )
            .await;
        }
        // Busy a moment before it reads, as a client that just came back is.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut answered = 0;
        while answered < 6 || (1..=6).any(|n| ws.bytes.get(&n).map_or(0, Vec::len) < OUTPUT_KEPT) {
            if let Some(v) = next(&mut ws).await {
                assert!(v.get("ok").is_some(), "{v}");
                answered += 1;
            }
        }
        for n in 1..=6u32 {
            let got = &ws.bytes[&n];
            assert_eq!(got.len(), OUTPUT_KEPT);
            assert!(got.iter().all(|b| *b == b'a' + n as u8));
        }
    });
}

#[test]
fn a_client_cut_while_an_attach_waits_for_room_frees_it_at_once() {
    use crate::output::{Output, OUTPUT_KEPT};
    // The wait itself: more than half the budget queued, the connection ends, the wait with it.
    let (conn, _text, _bytes) = Conn::new(4, 2 * (1004 + FRAME_COST));
    let output = conn.open(1).unwrap();
    output.send(vec![0; 1000]).unwrap();
    output.send(vec![1; 1000]).unwrap();
    rt::block_on(async {
        let waiting = rt::spawn({
            let conn = conn.clone();
            async move { conn.room_for_a_replay().await }
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!waiting.is_finished(), "no room yet: it waits");
        conn.close("left");
        let waited = tokio::time::timeout(Duration::from_secs(1), waiting).await;
        let Err(CallError::Failed(why)) = waited.expect("released at once").unwrap() else {
            panic!("a closed connection gets no room");
        };
        assert_eq!(why, "the connection is closed");
    });
    // Through the server: a client that resumes more terminals than it reads, cut mid-resume, holds no call.
    let loaded = load("remote-server-cut-mid-resume");
    let config = Config {
        port: 0,
        socket: None,
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: None,
        origins: Vec::new(),
    };
    // One call at a time across the server: the attach that waits holds it.
    let running = start_with(loaded.core.clone(), table::entries(), config, 1).unwrap();
    for n in 1..=8u8 {
        let id = format!("full-{n}");
        loaded.core.pty.insert_fake(&id, Box::new(std::io::sink()));
        let output = Output::new(OUTPUT_KEPT);
        for _ in 0..OUTPUT_KEPT / (32 << 10) {
            output.push(vec![b'a' + n; 32 << 10]);
        }
        assert!(loaded.core.pty.keep_output(&id, output));
    }
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        for n in 1..=8u32 {
            let args = json!({ "id": format!("full-{n}") });
            send(&mut ws, n.into(), "term_attach", args, Some(n)).await;
        }
        // It reads nothing: its queue passes half the budget, and an attach waits for room.
        tokio::time::sleep(Duration::from_millis(500)).await;
        drop(ws);
        let cut = std::time::Instant::now();
        let mut again = connect(running.addr).await;
        let a = call(&mut again, 1, "pr_reviews", json!({ "projectId": "none" })).await;
        assert!(a.get("id").is_some(), "{a}");
        assert!(
            cut.elapsed() < Duration::from_secs(3),
            "the calls came back after {:?}",
            cut.elapsed()
        );
    });
}

#[test]
fn a_slow_client_does_not_grow_the_queue_without_bound() {
    // Room for two frames of 1000 bytes (and their 4 of header), counted in bytes.
    let (conn, _text, mut bytes) = Conn::new(4, 2 * (1004 + FRAME_COST));
    let output = conn.open(9).unwrap();
    // A terminal's reads never wait: past the budget, refused (its stream falls behind), the client left open.
    assert_eq!(output.send(vec![0; 1000]), Ok(()));
    assert_eq!(output.send(vec![1; 1000]), Ok(()));
    assert_eq!(output.send(vec![2]), Err(Closed));
    assert!(!conn.is_closed());
    assert_eq!(bytes.len(), 2);
    let first = bytes.blocking_recv().unwrap();
    assert_eq!(&first.into_data()[..5], [0, 0, 0, 9, 0]);
    assert_eq!(output.send(vec![3]), Err(Closed));
    // Events and answers never wait either: one that finds the text queue full closes the client.
    let (conn, _text, _bytes) = Conn::new(4, BYTES_BUDGET);
    let events = conn.event_sink();
    for _ in 0..4 {
        events.send(exit("x")).unwrap();
    }
    assert_eq!(events.send(exit("x")), Err(Closed));
    assert!(conn.is_closed());
    assert_eq!(conn.open(10).unwrap().send(vec![1]), Err(Closed));
}

/// What `bytes` hold after their last full reset (a stream caught up starts from one).
#[cfg(unix)]
fn since_the_last_reset(bytes: &[u8]) -> &[u8] {
    let from = bytes
        .windows(2)
        .rposition(|w| w == b"\x1bc")
        .map_or(0, |at| at + 2);
    &bytes[from..]
}

/// Where `screen` first parts from `lines` (from the `1` that starts a line), with 80 bytes each side, escaped.
#[cfg(unix)]
fn first_gap(screen: &[u8], lines: &[u8]) -> String {
    let Some(start) = screen
        .windows(6)
        .position(|w| w == &lines[..6])
        .filter(|&at| at == 0 || !screen[at - 1].is_ascii_digit())
    else {
        return "no line 1 then 2".into();
    };
    let got = &screen[start..];
    let k = got
        .iter()
        .zip(lines)
        .position(|(a, b)| a != b)
        .unwrap_or(got.len().min(lines.len()));
    let around = |b: &[u8]| {
        b[k.saturating_sub(80)..(k + 80).min(b.len())]
            .escape_ascii()
            .to_string()
    };
    format!(
        "first gap at byte {} (line 1 at {start}): got {} / wanted {}",
        start + k,
        around(got),
        around(lines)
    )
}

// Unix only: it compares `seq`'s bytes as printed, which ConPTY does not relay but paints again (cursor moves, erases).
#[cfg(unix)]
#[test]
fn twenty_terminals_printing_at_once_do_not_cut_a_client_that_reads() {
    // task-0-report.md, B2: twenty terminals printing together closed, after some 5 s, a client that read them.
    const TERMS: u32 = 20;
    const LINES: u32 = 500_000;
    let loaded = load("remote-server-twenty");
    let running = serve(&loaded);
    rt::block_on(async {
        let (project, shell) = project(&loaded).await;
        let mut ws = connect(running.addr).await;
        let mut terms = Vec::new();
        for n in 1..=TERMS {
            terms.push(spawn(&mut ws, n.into(), &project, &shell, n).await);
        }
        // All at once; `FI""NI` keeps the line typed, which the shell echoes, from counting as its end.
        for (n, term) in (1..=TERMS).zip(&terms) {
            let line = format!("seq 1 {LINES}; echo FI\"\"NI-{n}\n");
            let args = json!({ "id": term, "data": line });
            send(&mut ws, 100 + u64::from(n), "term_write", args, None).await;
        }
        // A client that reads all along, slower than twenty terminals print (some 13 MB/s): its end of each is the
        // terminal's end, after a full reset when it fell behind.
        let started = std::time::Instant::now();
        let mut ended = std::collections::HashSet::new();
        let mut unpaced = 0;
        while ended.len() < TERMS as usize {
            assert!(
                started.elapsed() < Duration::from_secs(240),
                "ended: {ended:?}"
            );
            let frame = tokio::time::timeout(Duration::from_secs(20), ws.ws.next())
                .await
                .expect("a frame in time")
                .expect("the connection open")
                .expect("the connection open");
            let Message::Binary(b) = frame else { continue };
            let n = u32::from_be_bytes(b[..4].try_into().unwrap());
            let got = ws.bytes.entry(n).or_default();
            if b[4..].starts_with(b"\x1bc") {
                ended.remove(&n);
            }
            got.extend_from_slice(&b[4..]);
            let tail = &got[got.len().saturating_sub(b.len() + 16)..];
            let end = format!("FINI-{n}");
            if tail.windows(end.len()).any(|w| w == end.as_bytes()) {
                ended.insert(n);
            }
            unpaced += b.len();
            if unpaced >= 64 << 10 {
                unpaced = 0;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
        // `seq`'s lines whole and in order: the shell may write on the line of the first (bash's `ESC [ ? 2004 l`, a
        // title…), and a tty may add carriage returns of its own (macOS wrote `841\r\r\n`), so the CRs are left out.
        let lines: Vec<u8> = (1..=LINES)
            .flat_map(|l| format!("{l}\n").into_bytes())
            .collect();
        for n in 1..=TERMS {
            let screen: Vec<u8> = since_the_last_reset(&ws.bytes[&n])
                .iter()
                .copied()
                .filter(|&b| b != b'\r')
                .collect();
            let screen = &screen[..];
            let at = screen
                .windows(lines.len())
                .position(|w| w == lines)
                .filter(|&at| at == 0 || !screen[at - 1].is_ascii_digit());
            let end = format!("FINI-{n}");
            assert!(
                at.is_some_and(|at| {
                    let after = &screen[at + lines.len()..];
                    after.windows(end.len()).any(|w| w == end.as_bytes())
                }),
                "terminal {n}: {} bytes since the last reset, from {}; {}",
                screen.len(),
                screen[..screen.len().min(300)].escape_ascii(),
                first_gap(screen, &lines)
            );
        }
        // Still connected.
        let a = call(&mut ws, 999, "data_dir", json!({})).await;
        assert!(a.get("ok").is_some(), "{a}");
    });
}

#[test]
fn a_client_that_reads_nothing_is_closed_once_its_socket_took_nothing_for_a_while() {
    use futures_util::future::pending;
    use futures_util::sink::unfold;
    // The window of `IDLE` shortened to 2 s: one socket that takes nothing, one that takes 64 KiB a second.
    let idle = Duration::from_secs(2);
    let short = |conn: &mut Arc<Conn>| {
        let c = Arc::get_mut(conn).unwrap();
        c.idle = idle;
    };
    rt::block_on(async {
        let (mut stuck, text, bytes) = Conn::new(4, BYTES_BUDGET);
        short(&mut stuck);
        let took = Arc::new(AtomicUsize::new(0));
        let out = Box::pin(unfold((), |_, _: Message| pending::<Result<(), ()>>()));
        let _writer = rt::spawn(stuck.clone().write(out, text, bytes, took));
        stuck.push_bytes(Message::binary(vec![0; 10])).unwrap();

        let (mut slow, text, bytes) = Conn::new(4, BYTES_BUDGET);
        short(&mut slow);
        let took = Arc::new(AtomicUsize::new(0));
        let counted = took.clone();
        let out = Box::pin(unfold((), move |_, _: Message| {
            let counted = counted.clone();
            async move {
                for _ in 0..5 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    counted.fetch_add(64 << 10, SeqCst);
                }
                Ok::<_, ()>(())
            }
        }));
        let _slow_writer = rt::spawn(slow.clone().write(out, text, bytes, took));
        slow.push_bytes(Message::binary(vec![0; 10])).unwrap();

        let started = std::time::Instant::now();
        let mut closed = stuck.closed.subscribe();
        tokio::time::timeout(2 * idle + Duration::from_secs(1), closed.wait_for(|c| *c))
            .await
            .expect("closed in time")
            .unwrap();
        assert!(started.elapsed() >= idle);
        // The one that reads slowly is waited for, its frame sent.
        tokio::time::sleep(Duration::from_secs(4)).await;
        assert!(!slow.is_closed());
        assert_eq!(slow.bytes_queued.load(SeqCst), 0);
    });
}

#[test]
fn a_big_attach_holds_back_no_keystroke() {
    use crate::output::{Output, OUTPUT_KEPT};
    let loaded = load("remote-server-attach-lane");
    let running = serve(&loaded);
    for n in 1..=6u8 {
        let id = format!("full-{n}");
        loaded.core.pty.insert_fake(&id, Box::new(std::io::sink()));
        let output = Output::new(OUTPUT_KEPT);
        for _ in 0..OUTPUT_KEPT / (32 << 10) {
            output.push(vec![b'a' + n; 32 << 10]);
        }
        assert!(loaded.core.pty.keep_output(&id, output));
    }
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        // Six screens to resume, more than the budget holds at once; then a key typed.
        for n in 1..=6u32 {
            let args = json!({ "id": format!("full-{n}") });
            send(&mut ws, n.into(), "term_attach", args, Some(n)).await;
        }
        let key = json!({ "id": "full-1", "data": "x" });
        send(&mut ws, 100, "term_write", key, None).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut order = Vec::new();
        while order.len() < 7 {
            if let Some(v) = next(&mut ws).await {
                assert!(v.get("ok").is_some(), "{v}");
                order.push(v["id"].as_u64().unwrap());
            }
        }
        let key = order.iter().position(|id| *id == 100).unwrap();
        let last = order.iter().position(|id| *id == 6).unwrap();
        assert!(key < last, "the key waited for the attaches: {order:?}");
    });
}

#[test]
fn a_stream_behind_is_caught_up_from_a_reset_once_half_the_budget_is_free() {
    use crate::output::{Output, RESET};
    // Room for five frames of 1000 bytes; the terminal keeps its last 2000.
    let budget = 5 * (1004 + FRAME_COST);
    let (conn, _text, mut bytes) = Conn::new(4, budget);
    let output = Output::new(2000);
    output.read_by(conn.open(1).unwrap());
    for n in 0..10u8 {
        output.push(vec![b'0' + n; 1000]);
    }
    // Five queued, five dropped: the queue stays in its budget, the client open.
    assert!(!conn.is_closed());
    assert_eq!(bytes.len(), 5);
    assert!(conn.bytes_queued.load(SeqCst) <= budget);
    // As the writer takes them: past half the budget, the stream is caught up.
    let mut got = Vec::new();
    while let Ok(m) = bytes.try_recv() {
        conn.sent(&m);
        got.extend_from_slice(&m.into_data()[4..]);
    }
    output.push(b"live".to_vec());
    while let Ok(m) = bytes.try_recv() {
        conn.sent(&m);
        got.extend_from_slice(&m.into_data()[4..]);
    }
    let mut want = Vec::new();
    for n in 0..5u8 {
        want.extend(vec![b'0' + n; 1000]);
    }
    want.extend_from_slice(RESET);
    want.extend(vec![b'8'; 1000]);
    want.extend(vec![b'9'; 1000]);
    want.extend_from_slice(b"live");
    assert_eq!(got, want);
    assert_eq!(conn.bytes_queued.load(SeqCst), 0);
}

#[test]
fn a_shell_typed_in_is_caught_up_while_another_terminal_prints() {
    use crate::output::Output;
    // An agent that prints (stream 1) faster than the client reads, and a shell where the user types, whose screen of
    // 1000 bytes changes at each key (stream 2): both fall behind, and are caught up in turn. Neither is left behind
    // while the other prints (one caught up again and again froze the other: review-7a, I1).
    let budget = 5 * (1004 + FRAME_COST);
    let (conn, _text, mut bytes) = Conn::new(4, budget);
    let (agent, shell) = (Output::new(2000), Output::new(2000));
    agent.read_by(conn.open(1).unwrap());
    shell.read_by(conn.open(2).unwrap());
    let screen = |what: String| {
        let mut b = format!("{what} ").into_bytes();
        b.resize(1000, b'.');
        b
    };
    // Each stream's last step seen by the client, and the most it lagged.
    let (mut seen, mut lag) = ([None::<u32>; 2], [0u32; 2]);
    for step in 0..200u32 {
        for _ in 0..3 {
            agent.push(screen(format!("a{step}")));
        }
        shell.push(screen(format!("k{step}")));
        // The writer takes two frames while the agent prints three and the shell one.
        for _ in 0..2 {
            let Ok(m) = bytes.try_recv() else { break };
            conn.sent(&m);
            let data = m.into_data();
            let n = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
            let mark = if n == 1 { 'a' } else { 'k' };
            let text = String::from_utf8_lossy(&data[4..]);
            for word in text.split(|c: char| !c.is_ascii_alphanumeric()) {
                if let Some(at) = word.strip_prefix(mark).and_then(|d| d.parse().ok()) {
                    seen[n - 1] = seen[n - 1].max(Some(at));
                }
            }
        }
        for n in 0..2 {
            lag[n] = lag[n].max(step - seen[n].unwrap_or(0));
        }
    }
    assert!(
        lag.iter().all(|l| *l <= 20),
        "steps behind at most: {lag:?}"
    );
    assert!(!conn.is_closed());
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
    // `escouade-server`'s: the port it needs, the token of a debug build (this one), both taken out.
    std::env::set_var(PORT_VAR, "4001");
    std::env::set_var(TOKEN_VAR, "y".repeat(40));
    let config = take_port_from_env().unwrap();
    assert_eq!((config.port, config.token.unwrap().len()), (4001, 40));
    assert!(std::env::vars().all(|(k, _)| k != PORT_VAR && k != TOKEN_VAR));
    std::env::set_var(PORT_VAR, "4001");
    let config = take_port_from_env().unwrap();
    assert_eq!((config.port, config.token), (4001, None));
    // Unset: `--port` or `--socket` says where to listen.
    assert_eq!(take_port_from_env().unwrap().port, 0);
    std::env::set_var(PORT_VAR, "0");
    assert!(take_port_from_env().unwrap_err().starts_with(PORT_VAR));
}

/// A folder of its own under /tmp, not made yet, for a test's sockets: `sun_path` holds 104 bytes on macOS, NUL
/// included, which a folder under `$TMPDIR` overflows.
#[cfg(unix)]
fn short_dir(tag: &str) -> PathBuf {
    let pid = std::process::id().to_string();
    let dir = std::fs::canonicalize("/tmp")
        .unwrap()
        .join(format!("esc-{pid}-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    // With the longest pid (Linux's pid_max, 2^22: 7 digits), room for 20 bytes below it and the NUL.
    let longest = dir.as_os_str().len() - pid.len() + 7;
    assert!(
        longest + 20 < 104,
        "{} is too long for sockets",
        dir.display()
    );
    dir
}

/// The head of the answer to the raw HTTP request `head` (its lines, up to the blank one excluded) on `socket`.
#[cfg(unix)]
fn over_socket(socket: &std::path::Path, head: &str, body: &str) -> String {
    let mut s = std::os::unix::net::UnixStream::connect(socket).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let request = format!("{head}Content-Length: {}\r\n\r\n{body}", body.len());
    s.write_all(request.as_bytes()).unwrap();
    let (mut got, mut buf) = (Vec::new(), [0u8; 1024]);
    while !got.windows(4).any(|w| w == b"\r\n\r\n") {
        match s.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
        }
    }
    let got = String::from_utf8_lossy(&got).into_owned();
    assert!(!got.contains(TOKEN));
    got.lines().next().unwrap_or_default().to_string()
}

#[cfg(unix)]
#[test]
fn a_tunnel_to_the_socket_is_let_in_with_a_token() {
    use std::os::unix::fs::PermissionsExt;
    let loaded = load("remote-server-socket");
    let socket = short_dir("tunnel").join("s.sock");
    let devices = Arc::new(Devices::open(&loaded.dir));
    let on = |socket: &std::path::Path| Config {
        port: 0,
        socket: Some(socket.to_path_buf()),
        token: Some(TOKEN.into()),
        devices: Some(devices.clone()),
        web: Some(web_build(&loaded)),
        origins: Vec::new(),
    };
    let running = start(loaded.core.clone(), table::entries(), on(&socket)).unwrap();
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(socket.parent().unwrap()), 0o700);
    assert_eq!(mode(&socket), 0o600);
    let kept: Control =
        serde_json::from_slice(&std::fs::read(loaded.dir.join(CONTROL)).unwrap()).unwrap();
    assert_eq!(kept.socket.as_deref(), Some(socket.as_path()));
    // `ssh -L 9000:<socket>`: the page is http://localhost:9000, as through a tunnel to a port.
    let ws = "GET / HTTP/1.1\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\
              Sec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n";
    for name in ["localhost", "127.0.0.1"] {
        let host = format!("Host: {name}:9000\r\n");
        let origin = format!("Origin: http://{name}:9000\r\n");
        let line = over_socket(
            &socket,
            &(ws.to_string() + &host + &origin + &bearer(TOKEN)),
            "",
        );
        assert!(line.contains(" 101 "), "{name}: {line}");
        let line = over_socket(&socket, &(ws.to_string() + &host + &origin), "");
        assert!(line.contains(" 401 "), "{name}: {line}");
    }
    // Another local page stays out; and on a socket the server has no port of its own to let in.
    for headers in [
        "Host: localhost:9000\r\nOrigin: http://localhost:3000\r\n",
        "Host: localhost:9000\r\nOrigin: http://127.0.0.1:0\r\n",
        "Host: localhost:9000\r\nOrigin: https://localhost:9000\r\n",
        "Host: evil.example:9000\r\nOrigin: http://evil.example:9000\r\n",
    ] {
        let line = over_socket(&socket, &(ws.to_string() + headers + &bearer(TOKEN)), "");
        assert!(line.contains(" 403 "), "{headers}: {line}");
    }
    // The page loads and pairs through it.
    let host = "Host: localhost:9000\r\n";
    let line = over_socket(&socket, &format!("GET / HTTP/1.1\r\n{host}"), "");
    assert!(line.contains(" 200 "), "{line}");
    let code = devices.pair_code().code;
    let body = json!({ "code": code, "name": "portable" }).to_string();
    let head = format!("POST /pair HTTP/1.1\r\n{host}Origin: http://localhost:9000\r\n");
    let line = over_socket(&socket, &head, &body);
    assert!(line.contains(" 200 "), "{line}");
    // A whole session over it.
    rt::block_on(async {
        let stream = tokio::net::UnixStream::connect(&socket).await.unwrap();
        let mut req = "ws://127.0.0.1:9000/".into_client_request().unwrap();
        let bearer = format!("Bearer {TOKEN}").parse().unwrap();
        req.headers_mut().insert(header::AUTHORIZATION, bearer);
        let (mut ws, _) = tokio_tungstenite::client_async(req, stream).await.unwrap();
        let hello = tokio::time::timeout(Duration::from_secs(30), ws.next()).await;
        let hello = hello
            .unwrap()
            .unwrap()
            .unwrap()
            .into_text()
            .unwrap()
            .to_string();
        assert!(hello.contains("hello"), "{hello}");
    });
    // One server on a socket: the next is refused while it answers.
    let second = start(loaded.core.clone(), table::entries(), on(&socket));
    let why = second.err().unwrap().to_string();
    assert!(why.contains("another server listens"), "{why}");
    // Stopped, its core still stopping: the socket stays its own until it is dropped, the next one refused.
    running.stop();
    let second = start(loaded.core.clone(), table::entries(), on(&socket));
    let why = second.err().unwrap().to_string();
    assert!(why.contains("another server listens"), "{why}");
    drop(running);
    assert!(!socket.exists(), "the socket outlives its server");
    // One left by a server that died is replaced.
    drop(std::os::unix::net::UnixListener::bind(&socket).unwrap());
    assert!(socket.exists());
    let running = start(loaded.core.clone(), table::entries(), on(&socket)).unwrap();
    let line = over_socket(&socket, &format!("GET / HTTP/1.1\r\n{host}"), "");
    assert!(line.contains(" 200 "), "{line}");
    drop(running);
    // Not in a folder others may enter: it could be taken between the bind and the chmod, or replaced.
    let open = loaded.dir.join("open");
    std::fs::create_dir(&open).unwrap();
    std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();
    let refused = start(
        loaded.core.clone(),
        table::entries(),
        on(&open.join("s.sock")),
    );
    let why = refused.err().unwrap().to_string();
    assert!(why.contains("alone (0700)"), "{why}");
    assert!(!open.join("s.sock").exists());
    let _ = std::fs::remove_dir_all(socket.parent().unwrap());
}

#[cfg(unix)]
#[test]
fn a_socket_given_by_hand_is_bound_only_where_no_other_user_can_change_its_path() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let loaded = load("remote-server-socket-path");
    let base = short_dir("path");
    let made = |p: &std::path::Path, mode: u32| {
        std::fs::create_dir_all(p).unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    made(&base, 0o700);
    let on = |socket: &std::path::Path| Config {
        port: 0,
        socket: Some(socket.to_path_buf()),
        token: Some(TOKEN.into()),
        devices: None,
        web: None,
        origins: Vec::new(),
    };
    let serve = |socket: &std::path::Path| start(loaded.core.clone(), table::entries(), on(socket));
    // Through a link: bound at the real path, which no later change of the link moves.
    let real = base.join("real");
    made(&real, 0o700);
    symlink(&real, base.join("link")).unwrap();
    let running = serve(&base.join("link/s.sock")).unwrap();
    let bound = real.join("s.sock");
    assert_eq!(running.socket.as_deref(), Some(bound.as_path()));
    std::fs::remove_file(base.join("link")).unwrap();
    symlink(&base, base.join("link")).unwrap();
    assert!(std::os::unix::net::UnixStream::connect(&bound).is_ok());
    drop(running);
    assert!(!bound.exists());
    // Below a folder others may write to, its own 0700 or not: another user could put a link in its place.
    let open = base.join("open");
    made(&open.join("me"), 0o700);
    made(&open, 0o777);
    let why = serve(&open.join("me/s.sock")).err().unwrap().to_string();
    assert!(why.contains("other users"), "{why}");
    assert!(!open.join("me/s.sock").exists());
    // The same through a link to it.
    symlink(open.join("me"), base.join("to-open")).unwrap();
    let why = serve(&base.join("to-open/s.sock"))
        .err()
        .unwrap()
        .to_string();
    assert!(why.contains("other users"), "{why}");
    // Sticky as /tmp is (where the others above live), but not root's: refused, unless this user is root.
    // SAFETY: geteuid has no precondition.
    let root = unsafe { libc::geteuid() } == 0;
    let sticky = base.join("sticky");
    made(&sticky.join("me"), 0o700);
    made(&sticky, 0o1777);
    let got = serve(&sticky.join("me/s.sock"));
    assert_eq!(got.is_ok(), root, "{:?}", got.as_ref().err());
    drop(got);
    // Another user's, even closed to others: refused (only root can give one away, to test it).
    if root {
        let theirs = base.join("theirs");
        made(&theirs.join("me"), 0o700);
        made(&theirs, 0o755);
        let c = std::ffi::CString::new(theirs.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: a NUL-terminated path, alive for the call.
        assert_eq!(unsafe { libc::chown(c.as_ptr(), 65534, 65534) }, 0);
        let why = serve(&theirs.join("me/s.sock")).err().unwrap().to_string();
        assert!(why.contains("other users"), "{why}");
    }
    let _ = std::fs::remove_dir_all(&base);
}

/// A request of the files' routes from the server's own page, with `token` if any, its `Content-Length` if given: the
/// head of the answer, and its body.
fn files_request(
    addr: SocketAddr,
    method: &str,
    target: &str,
    token: Option<&str>,
    length: Option<u64>,
    body: &[u8],
) -> (String, Vec<u8>) {
    let port = addr.port();
    let mut head = format!(
        "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\n"
    );
    if let Some(token) = token {
        head += &format!("Authorization: Bearer {token}\r\n");
    }
    if let Some(n) = length {
        head += &format!("Content-Length: {n}\r\n");
    }
    head += "\r\n";
    let mut tcp = std::net::TcpStream::connect(addr).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    tcp.write_all(head.as_bytes()).unwrap();
    // Refused before its body is read, the server may close first.
    let _ = tcp.write_all(body);
    let mut got = Vec::new();
    let _ = tcp.read_to_end(&mut got);
    let end = got
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map_or(got.len(), |i| i + 4);
    let body = got.split_off(end);
    (String::from_utf8_lossy(&got).into_owned(), body)
}

/// What a folder holds, all the way down; nothing when it is not there.
fn everything_in(dir: &std::path::Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .flat_map(|e| {
            let p = e.path();
            let mut below = everything_in(&p);
            below.insert(0, p);
            below
        })
        .collect()
}

#[test]
fn a_file_sent_by_a_browser_is_kept_whole_under_a_safe_name_and_never_without_a_token() {
    let loaded = load("remote-files-attach");
    let running = serve(&loaded);
    let drafts = loaded.core.data.drafts_files();
    let send = |target: &str, token: Option<&str>, body: &[u8]| {
        let n = body.len() as u64;
        files_request(running.addr, "POST", target, token, Some(n), body)
    };

    // Without a token, with a wrong one, from another page: nothing written.
    let target = "/files/attach?agent=draft&name=a.txt";
    let wrong = "x".repeat(43);
    for token in [None, Some(wrong.as_str())] {
        let (head, _) = send(target, token, b"hello");
        assert!(head.starts_with("HTTP/1.1 401"), "{head}");
    }
    let port = running.addr.port();
    let foreign = http(
        running.addr,
        &format!(
            "POST {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:1\r\n\
             Authorization: Bearer {TOKEN}\r\nContent-Length: 5\r\n\r\nhello"
        ),
    );
    assert!(foreign.starts_with("HTTP/1.1 403"), "{foreign}");
    assert_eq!(everything_in(&drafts), Vec::<PathBuf>::new());

    // A hostile name: a way up, a NUL, a mark that writes backwards, a separator of Windows.
    let target = "/files/attach?agent=draft&name=..%2F..%2Fx%5Cy%00%E2%80%AEfdp.sh";
    let (head, body) = send(target, Some(TOKEN), b"hello");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(head.contains("application/json"), "{head}");
    let file: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(file["size"], 5);
    assert_eq!(file["origin"], false);
    let name = file["name"].as_str().unwrap();
    assert!(!name.contains(['/', '\\', '\0', '\u{202e}']), "{name}");
    let path = PathBuf::from(file["path"].as_str().unwrap());
    // In a folder of its own under the drafts', nowhere else.
    assert_eq!(path.parent().unwrap().parent().unwrap(), drafts);
    assert_eq!(std::fs::read(&path).unwrap(), b"hello");

    // An owner that is no agent: refused, said why.
    let (head, body) = send("/files/attach?agent=nobody&name=a.txt", Some(TOKEN), b"x");
    assert!(head.starts_with("HTTP/1.1 400"), "{head}");
    assert!(!body.is_empty(), "the reason is said");
    // No size said, or no name: refused.
    let (head, _) = files_request(
        running.addr,
        "POST",
        "/files/attach?agent=draft&name=a",
        Some(TOKEN),
        None,
        b"",
    );
    assert!(head.starts_with("HTTP/1.1 411"), "{head}");
    let (head, _) = send("/files/attach?agent=draft", Some(TOKEN), b"x");
    assert!(head.starts_with("HTTP/1.1 400"), "{head}");
    // Only the one file was kept.
    assert_eq!(
        everything_in(&drafts).len(),
        2,
        "{:?}",
        everything_in(&drafts)
    );
}

#[test]
fn a_file_too_big_or_cut_half_way_leaves_nothing() {
    let loaded = load("remote-files-cut");
    let running = serve(&loaded);
    let drafts = loaded.core.data.drafts_files();
    let target = "/files/attach?agent=draft&name=big.bin";

    // Over the server's own bound, the settings having none: refused on its size, before a byte is read.
    loaded.core.settings.write().attach_max_mb = 0;
    let over = files::MAX_UPLOAD + 1;
    let (head, _) = files_request(running.addr, "POST", target, Some(TOKEN), Some(over), b"");
    assert!(head.starts_with("HTTP/1.1 413"), "{head}");
    // Over the settings' cap.
    loaded.core.settings.write().attach_max_mb = 1;
    let (head, body) = files_request(
        running.addr,
        "POST",
        target,
        Some(TOKEN),
        Some(2 << 20),
        b"",
    );
    assert!(head.starts_with("HTTP/1.1 413"), "{head}");
    assert!(String::from_utf8_lossy(&body).contains("big.bin"));
    assert_eq!(everything_in(&drafts), Vec::<PathBuf>::new());

    // Cut half way: what was written goes, its mark with it.
    let port = running.addr.port();
    let mut tcp = std::net::TcpStream::connect(running.addr).unwrap();
    let head = format!(
        "POST {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\n\
         Content-Length: 100000\r\n\r\n"
    );
    tcp.write_all(head.as_bytes()).unwrap();
    tcp.write_all(&[7; 5000]).unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(10);
    while everything_in(&drafts).is_empty() {
        assert!(std::time::Instant::now() < until, "the copy never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(tcp);
    while !everything_in(&drafts).is_empty() {
        assert!(
            std::time::Instant::now() < until,
            "left: {:?}",
            everything_in(&drafts)
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // The server goes on: a whole file after it is kept.
    let (head, _) = files_request(running.addr, "POST", target, Some(TOKEN), Some(3), b"abc");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
}

/// A file being sent whose last bytes have not come: its head and `part` of a body of `size` bytes.
fn sending(addr: SocketAddr, size: usize, part: usize) -> std::net::TcpStream {
    let port = addr.port();
    let mut tcp = std::net::TcpStream::connect(addr).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let head = format!(
        "POST /files/attach?agent=draft&name=f.bin HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\
         Origin: http://127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {size}\r\n\r\n"
    );
    tcp.write_all(head.as_bytes()).unwrap();
    tcp.write_all(&vec![1; part]).unwrap();
    tcp
}

/// The answer to a request on `tcp`, its head and body.
fn reply_of(mut tcp: std::net::TcpStream) -> (String, String) {
    let mut got = Vec::new();
    let _ = tcp.read_to_end(&mut got);
    let got = String::from_utf8_lossy(&got).into_owned();
    let (head, body) = got.split_once("\r\n\r\n").unwrap_or((&got, ""));
    (head.to_string(), body.to_string())
}

/// Waits until `drafts` holds `n` copies under way (a mark, a folder and a file each).
fn copies_under_way(drafts: &std::path::Path, n: usize) {
    let until = std::time::Instant::now() + Duration::from_secs(10);
    while everything_in(drafts).len() < 3 * n {
        assert!(
            std::time::Instant::now() < until,
            "{:?}",
            everything_in(drafts)
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Waits until `drafts` holds nothing.
fn nothing_left(drafts: &std::path::Path) {
    let until = std::time::Instant::now() + Duration::from_secs(10);
    while !everything_in(drafts).is_empty() {
        assert!(
            std::time::Instant::now() < until,
            "{:?}",
            everything_in(drafts)
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn five_files_sent_at_once_are_all_kept() {
    let loaded = load("remote-files-five");
    let running = serve(&loaded);
    // All started, none ended: the fifth comes while the first four are written.
    let mut tcps: Vec<_> = (0..5).map(|_| sending(running.addr, 10, 5)).collect();
    copies_under_way(&loaded.core.data.drafts_files(), files::MAX_UPLOADS);
    std::thread::sleep(Duration::from_millis(200));
    for tcp in &mut tcps {
        let _ = tcp.write_all(&[2; 5]);
    }
    for tcp in tcps {
        let (head, body) = reply_of(tcp);
        assert!(head.starts_with("HTTP/1.1 200"), "{head}\n{body}");
    }
}

#[test]
fn a_file_waits_its_turn_then_is_refused_with_why() {
    let loaded = load("remote-files-queue");
    let running = serve(&loaded);
    let drafts = loaded.core.data.drafts_files();
    let stalled: Vec<_> = (0..files::MAX_UPLOADS)
        .map(|_| sending(running.addr, 10, 5))
        .collect();
    copies_under_way(&drafts, files::MAX_UPLOADS);
    let start = std::time::Instant::now();
    let (head, body) = reply_of(sending(running.addr, 3, 3));
    assert!(head.starts_with("HTTP/1.1 503"), "{head}");
    assert!(!body.is_empty(), "the page says why");
    assert!(start.elapsed() >= files::QUEUE / 2, "{:?}", start.elapsed());
    // The ones stalled, gone: nothing of them left.
    drop(stalled);
    nothing_left(&drafts);
}

#[test]
fn files_under_way_leave_room_for_the_interface() {
    let loaded = load("remote-files-room");
    let running = serve(&loaded);
    let stalled: Vec<_> = (0..files::MAX_UPLOADS)
        .map(|_| sending(running.addr, 10, 5))
        .collect();
    copies_under_way(&loaded.core.data.drafts_files(), files::MAX_UPLOADS);
    // Waiting their turn; past `MAX_TRANSFERS`, a request of files is refused at once.
    let waiting: Vec<_> = (0..files::MAX_TRANSFERS - files::MAX_UPLOADS)
        .map(|_| sending(running.addr, 3, 0))
        .collect();
    std::thread::sleep(Duration::from_millis(200));
    let start = std::time::Instant::now();
    for _ in 0..MAX_CONNECTIONS {
        let (head, _) = reply_of(sending(running.addr, 3, 3));
        assert!(head.starts_with("HTTP/1.1 503"), "{head}");
    }
    assert!(start.elapsed() < files::QUEUE, "{:?}", start.elapsed());
    // The interface still connects.
    rt::block_on(async {
        let mut ws = connect(running.addr).await;
        let _ = ws.ws.close(None).await;
    });
    drop((stalled, waiting));
}

#[test]
fn a_file_said_chunked_after_its_length_is_not_kept() {
    let loaded = load("remote-files-chunked");
    let running = serve(&loaded);
    let port = running.addr.port();
    let answer = http(
        running.addr,
        &format!(
            "POST /files/attach?agent=draft&name=c.bin HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\
             Origin: http://127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: 3\r\n\
             Transfer-Encoding: chunked\r\n\r\na\r\n0123456789\r\n0\r\n\r\n"
        ),
    );
    assert!(!answer.starts_with("HTTP/1.1 200"), "{answer}");
    nothing_left(&loaded.core.data.drafts_files());
}

#[test]
fn a_file_an_agent_sent_is_fetched_as_a_download_of_its_own_agent_alone() {
    let h = crate::core_tests::harness("remote-files-sent");
    let loaded = Loaded {
        core: h.core.clone(),
        dir: h.dir.clone(),
    };
    let running = serve(&loaded);
    // A PDF that a browser shown it in line would run as a page.
    let bytes = b"%PDF-1.7\n<html><script>alert(1)</script></html>".to_vec();
    // The PDF itself serves the Unix-only symlink case below.
    #[cfg_attr(not(unix), allow(unused_variables))]
    let (id, other, pdf) = rt::block_on(async {
        let (p, repo) = h.project(false).await;
        let id = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        let other = h.core.create_agent(&p.id, None).await.unwrap().meta.id;
        h.turn(&id, "Bonjour").await;
        let pdf = repo.join("out").join("contrat.pdf");
        std::fs::create_dir_all(pdf.parent().unwrap()).unwrap();
        std::fs::write(&pdf, &bytes).unwrap();
        let file = pdf.to_string_lossy().replace('\\', "/");
        let input = json!({ "files": [file], "caption": "Le contrat", "status": "normal" });
        h.feed(
            &id,
            json!({"type":"assistant","message":{"id":"m-f1","content":[{"type":"tool_use","id":"toolu_f1","name":"SendUserFile","input":input}]},"parent_tool_use_id":null}),
        );
        let result = format!("1 files delivered to user.\n{file} → file_uuid: 0000");
        h.feed(
            &id,
            json!({"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_f1","content":result}]},"parent_tool_use_id":null}),
        );
        h.wait("the file delivered", |h| {
            h.items(&id)
                .iter()
                .any(|i| i["id"] == "toolu_f1" && i.get("sent").is_some())
        })
        .await;
        (id, other, pdf)
    });
    let get = |agent: &str, item: &str, index: &str, token: Option<&str>| {
        let target = format!("/files/sent?agent={agent}&item={item}&index={index}");
        files_request(running.addr, "GET", &target, token, None, b"")
    };

    let (head, body) = get(&id, "toolu_f1", "0", Some(TOKEN));
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    let head = head.to_ascii_lowercase();
    // A download, never a page: no type a browser shows, nothing sniffed, sandboxed if opened all the same.
    assert!(
        head.contains("content-disposition: attachment; filename=\"contrat.pdf\""),
        "{head}"
    );
    assert!(
        head.contains("content-type: application/octet-stream"),
        "{head}"
    );
    assert!(head.contains("x-content-type-options: nosniff"), "{head}");
    assert!(head.contains("sandbox"), "{head}");
    assert_eq!(body, bytes);

    // Without a token; under another agent; an unknown call or index.
    let (head, _) = get(&id, "toolu_f1", "0", None);
    assert!(head.starts_with("HTTP/1.1 401"), "{head}");
    for (agent, item, index) in [
        (other.as_str(), "toolu_f1", "0"),
        (id.as_str(), "toolu_none", "0"),
        (id.as_str(), "toolu_f1", "1"),
        ("nobody", "toolu_f1", "0"),
    ] {
        let (head, _) = get(agent, item, index, Some(TOKEN));
        assert!(
            head.starts_with("HTTP/1.1 404"),
            "{agent} {item} {index}: {head}"
        );
    }
    let (head, _) = get(&id, "toolu_f1", "x", Some(TOKEN));
    assert!(head.starts_with("HTTP/1.1 400"), "{head}");

    // The file made a link to a private project's, after it was sent: not given.
    #[cfg(unix)]
    {
        let vault = crate::core_tests::repo(&h.dir.join("vault"));
        let secret = vault.join("secret.txt");
        std::fs::write(&secret, "secret").unwrap();
        let p = rt::block_on(h.core.create_project(
            &vault.to_string_lossy(),
            "vault",
            "oklch(0.7 0.1 200)",
            false,
            None,
            true,
        ))
        .unwrap();
        let mut projects = h.core.projects.write();
        projects.iter_mut().find(|x| x.id == p.id).unwrap().private = true;
        drop(projects);
        std::fs::remove_file(&pdf).unwrap();
        std::os::unix::fs::symlink(&secret, &pdf).unwrap();
        let (head, body) = get(&id, "toolu_f1", "0", Some(TOKEN));
        assert!(head.starts_with("HTTP/1.1 404"), "{head}");
        assert!(!String::from_utf8_lossy(&body).contains("secret"));
    }

    // Under escouade-server, a file out of the home folder is not given.
    let home = h.dir.join("another-home");
    std::fs::create_dir_all(&home).unwrap();
    h.core.folders_home.set(home).unwrap();
    let (head, _) = get(&id, "toolu_f1", "0", Some(TOKEN));
    assert!(head.starts_with("HTTP/1.1 404"), "{head}");
}
