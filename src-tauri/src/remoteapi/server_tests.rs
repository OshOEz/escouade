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
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: None,
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
fn a_foreign_origin_is_refused() {
    let loaded = load("remote-server-origin");
    let running = serve(&loaded);
    let port = running.addr.port();
    for origin in [
        format!("Origin: http://127.0.0.1:{}\r\n", port.wrapping_add(1)),
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
    let web = loaded.dir.join("web");
    std::fs::create_dir_all(web.join("assets")).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html><title>Escouade</title>",
    )
    .unwrap();
    std::fs::write(web.join("assets/app-1234.js"), "console.log(1)").unwrap();
    std::fs::write(loaded.dir.join("Cargo.toml"), "secret").unwrap();
    let config = Config {
        port: 0,
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: Some(web),
    };
    start(loaded.core.clone(), table::entries(), config).unwrap()
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
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: None,
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
        token: Some(TOKEN.into()),
        devices: Some(Arc::new(Devices::open(&loaded.dir))),
        web: None,
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
    // A terminal's reads never wait: past the budget, the client is closed.
    assert_eq!(output.send(vec![0; 1000]), Ok(()));
    assert_eq!(output.send(vec![1; 1000]), Ok(()));
    assert_eq!(output.send(vec![2]), Err(Closed));
    assert!(conn.is_closed());
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
    assert!(take_port_from_env().unwrap_err().starts_with(PORT_VAR));
}
