//! `escouade-server` (`src/bin/escouade-server.rs`) as a process: its port (and a debug build's token) in its
//! environment, a data folder and a home of its own, stopped by a signal; a device paired with `pair` and revoked.
#![cfg(unix)]

use futures_util::{SinkExt, StreamExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

/// A folder of its own for this test, empty.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("escouade-server-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A socket in a short folder of its own under /tmp, not made yet: `sun_path` holds 104 bytes on macOS, NUL
/// included, which a folder under `$TMPDIR` overflows.
fn short_socket(tag: &str) -> PathBuf {
    let pid = std::process::id().to_string();
    let dir = std::fs::canonicalize("/tmp")
        .unwrap()
        .join(format!("esc-{pid}-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    let socket = dir.join("s.sock");
    // With the longest pid (Linux's pid_max, 2^22: 7 digits) all the same, NUL included.
    let longest = socket.as_os_str().len() - pid.len() + 7;
    assert!(
        longest < 104,
        "{} is too long for a socket",
        socket.display()
    );
    socket
}

/// The program, its data and home in `dir`, none of the variables it reads but `vars`.
fn program(dir: &Path, vars: &[(&str, String)]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_escouade-server"));
    c.env_remove("ESCOUADE_REMOTE_PORT")
        .env_remove("ESCOUADE_REMOTE_TOKEN")
        .env("ESCOUADE_DATA_DIR", dir.join("data"))
        .env("HOME", dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in vars {
        c.env(k, v);
    }
    c
}

/// A port free a moment ago.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// The exit code of `child`, waited for up to `limit`.
fn exit_within(child: &mut Child, limit: Duration) -> Option<i32> {
    let until = Instant::now() + limit;
    while Instant::now() < until {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code();
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    None
}

#[test]
fn it_will_not_start_without_a_port() {
    let dir = folder("no-port");
    let mut child = program(&dir, &[("ESCOUADE_REMOTE_TOKEN", TOKEN.into())])
        .spawn()
        .unwrap();
    let code = exit_within(&mut child, Duration::from_secs(30));
    assert_eq!(code, Some(2));
}

#[test]
fn it_will_not_start_with_an_origin_that_is_not_one() {
    for origin in [
        "escouade.example.com",
        "https://escouade.example.com/escouade",
        "ftp://box.lan",
    ] {
        let dir = folder("bad-origin");
        let port = free_port();
        let mut child = program(&dir, &[("ESCOUADE_REMOTE_PORT", port.to_string())])
            .args(["--origin", origin])
            .spawn()
            .unwrap();
        let code = exit_within(&mut child, Duration::from_secs(30));
        assert_eq!(code, Some(2), "{origin}");
    }
    let dir = folder("no-origin");
    let mut child = program(&dir, &[("ESCOUADE_REMOTE_PORT", free_port().to_string())])
        .arg("--origin")
        .spawn()
        .unwrap();
    assert_eq!(exit_within(&mut child, Duration::from_secs(30)), Some(2));
}

/// `escouade-server <args>` beside the server of `dir`: its exit code and what it printed.
fn command(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let out = program(dir, &[])
        .args(args)
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// The whole answer to the raw HTTP `request` on `port`.
fn http(port: u16, request: &str) -> String {
    use std::io::{Read, Write};
    let mut tcp = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    tcp.write_all(request.as_bytes()).unwrap();
    let mut got = String::new();
    let _ = tcp.read_to_string(&mut got);
    got
}

#[test]
fn a_device_paired_by_the_command_is_listed_then_revoked() {
    let dir = folder("pair");
    let (code, out) = command(&dir, &["pair"]);
    assert_eq!(code, Some(1), "no server runs yet: {out}");
    let port = free_port();
    // No token: the devices alone; behind a proxy, declared.
    let mut child = program(&dir, &[("ESCOUADE_REMOTE_PORT", port.to_string())])
        .args(["--origin", "HTTPS://Escouade.Example.com/"])
        .args(["--origin", "http://box.lan:8080"])
        .spawn()
        .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let until = Instant::now() + Duration::from_secs(120);
        while tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_err()
        {
            assert!(Instant::now() < until, "no server on {port}");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
    let (code, out) = command(&dir, &["pair"]);
    assert_eq!(code, Some(0), "{out}");
    // The address of the first origin declared.
    assert!(out.contains("https://escouade.example.com/"), "{out}");
    assert!(!out.contains("box.lan"), "{out}");
    assert!(
        out.contains("pair` again"),
        "the five tries are not told: {out}"
    );
    let pairing = out
        .split_whitespace()
        .find(|w| w.len() == 9 && w.as_bytes()[4] == b'-')
        .unwrap_or_else(|| panic!("no code in {out}"));
    let body = format!(r#"{{"code":"{pairing}","name":"portable"}}"#);
    let answer = http(
        port,
        &format!(
            "POST /pair HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        ),
    );
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    let paired: serde_json::Value =
        serde_json::from_str(answer.split_once("\r\n\r\n").unwrap().1).unwrap();
    let token = paired["token"].as_str().unwrap().to_string();
    let id = paired["id"].as_str().unwrap().to_string();
    let (code, out) = command(&dir, &["devices"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains(&id) && out.contains("portable"), "{out}");
    let closed = runtime.block_on(async {
        let tcp = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let mut req = format!("ws://127.0.0.1:{port}/")
            .into_client_request()
            .unwrap();
        let protocols = format!("escouade, bearer.{token}").parse().unwrap();
        req.headers_mut()
            .insert("sec-websocket-protocol", protocols);
        let (mut ws, _) = tokio_tungstenite::client_async(req, tcp).await.unwrap();
        // The greeting, then the revocation by the command closes it.
        ws.next().await.unwrap().unwrap();
        let revoke = tokio::task::spawn_blocking({
            let (dir, id) = (dir.clone(), id.clone());
            move || command(&dir, &["devices", "revoke", &id])
        });
        let closed = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match ws.next().await {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {}
                }
            }
        })
        .await
        .is_ok();
        let (code, out) = revoke.await.unwrap();
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains("revoked"), "{out}");
        closed
    });
    assert!(closed, "the revoked device's connection stays open");
    let (_, out) = command(&dir, &["devices"]);
    assert!(!out.contains(&id), "{out}");
    let kept = std::fs::read_to_string(dir.join("data/devices.json")).unwrap();
    assert!(!kept.contains(&token), "{kept}");
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    assert_eq!(exit_within(&mut child, Duration::from_secs(60)), Some(0));
    // Stopped: no port and key left for `pair` to send to whoever listens there next.
    assert!(!dir.join("data/server.json").exists());
    let log = std::fs::read_to_string(dir.join("data/app.log")).unwrap();
    assert!(!log.contains(&token) && !log.contains(pairing), "{log}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The program held in its start-up: its login shell (`SHELL`, run before the core loads) waits for `go` beside it,
/// 4 s at most (the program gives it 5). Its stderr comes line by line on the receiver.
fn held_at_startup(dir: &Path) -> (Child, std::sync::mpsc::Receiver<String>) {
    use std::io::BufRead;
    use std::os::unix::fs::PermissionsExt;
    let shell = dir.join("shell");
    std::fs::write(
        &shell,
        "#!/bin/sh\ntouch \"$0.started\"\ni=0\n\
         while [ ! -e \"$0.go\" ] && [ $i -lt 40 ]; do sleep 0.1; i=$((i+1)); done\n",
    )
    .unwrap();
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut child = program(
        dir,
        &[
            ("ESCOUADE_REMOTE_PORT", free_port().to_string()),
            ("ESCOUADE_REMOTE_TOKEN", TOKEN.to_string()),
            ("SHELL", shell.display().to_string()),
        ],
    )
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let stderr = std::io::BufReader::new(child.stderr.take().unwrap());
    std::thread::spawn(move || {
        for line in stderr.lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    let until = Instant::now() + Duration::from_secs(30);
    while !dir.join("shell.started").exists() {
        assert!(Instant::now() < until, "the login shell never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    (child, rx)
}

/// Waits for a line of `lines` that holds `needle`.
fn line_with(lines: &std::sync::mpsc::Receiver<String>, needle: &str) {
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        let left = until.saturating_duration_since(Instant::now());
        match lines.recv_timeout(left) {
            Ok(line) if line.contains(needle) => return,
            Ok(_) => {}
            Err(e) => panic!("no {needle:?} on stderr: {e}"),
        }
    }
}

#[test]
fn a_sigterm_during_startup_still_stops_the_core() {
    let dir = folder("early-sigterm");
    let (mut child, lines) = held_at_startup(&dir);
    // Caught, told once PATH is written (no thread runs while it is).
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    std::fs::write(dir.join("shell.go"), "").unwrap();
    line_with(&lines, "stopping on SIGTERM");
    let code = exit_within(&mut child, Duration::from_secs(120));
    assert_eq!(code, Some(0));
    let log = std::fs::read_to_string(dir.join("data/app.log")).unwrap();
    assert!(log.contains("core stopped"), "{log}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_second_signal_while_stopping_ends_it_at_once() {
    let dir = folder("second-signal");
    let (mut child, lines) = held_at_startup(&dir);
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    // Still held: the core has not loaded, let alone stopped. Both told once PATH is written, in either order.
    unsafe { libc::kill(child.id() as i32, libc::SIGINT) };
    std::fs::write(dir.join("shell.go"), "").unwrap();
    line_with(&lines, "again, leaving without stopping the core");
    let code = exit_within(&mut child, Duration::from_secs(10));
    assert_eq!(code, Some(130));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn it_serves_the_core_and_stops_cleanly_on_sigterm() {
    let dir = folder("sigterm");
    let port = free_port();
    let mut child = program(
        &dir,
        &[
            ("ESCOUADE_REMOTE_PORT", port.to_string()),
            ("ESCOUADE_REMOTE_TOKEN", TOKEN.to_string()),
        ],
    )
    .spawn()
    .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let answer = runtime.block_on(async {
        // Its port, once the core has loaded.
        let until = Instant::now() + Duration::from_secs(120);
        let tcp = loop {
            match tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                Ok(tcp) => break tcp,
                Err(_) if Instant::now() < until => {
                    tokio::time::sleep(Duration::from_millis(100)).await
                }
                Err(e) => panic!("no server on {port}: {e}"),
            }
        };
        let mut req = format!("ws://127.0.0.1:{port}/")
            .into_client_request()
            .unwrap();
        let bearer = format!("Bearer {TOKEN}").parse().unwrap();
        req.headers_mut().insert("authorization", bearer);
        let (mut ws, _) = tokio_tungstenite::client_async(req, tcp).await.unwrap();
        let call = r#"{"id":1,"cmd":"data_dir","args":{}}"#;
        ws.send(Message::text(call)).await.unwrap();
        loop {
            let frame = tokio::time::timeout(Duration::from_secs(30), ws.next())
                .await
                .expect("an answer in time")
                .expect("the connection open")
                .unwrap();
            let Message::Text(text) = frame else { continue };
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            if value.get("id").is_some() {
                break value;
            }
        }
    });
    assert_eq!(
        answer["ok"].as_str().map(PathBuf::from),
        Some(dir.join("data")),
        "{answer}"
    );
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    let code = exit_within(&mut child, Duration::from_secs(60));
    assert_eq!(code, Some(0));
    let log = std::fs::read_to_string(dir.join("data/app.log")).unwrap();
    assert!(log.contains("core stopped"), "{log}");
    assert!(!log.contains(TOKEN), "{log}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Waits until `socket` answers, `child` still running.
fn wait_for_socket(child: &mut Child, socket: &Path) {
    let until = Instant::now() + Duration::from_secs(120);
    while std::os::unix::net::UnixStream::connect(socket).is_err() {
        assert!(child.try_wait().unwrap().is_none(), "the server ended");
        assert!(Instant::now() < until, "no server on {}", socket.display());
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// `program` with its stderr piped, waited for to its end: its code and what it said.
fn refused(mut c: Command) -> (Option<i32>, String) {
    let mut child = c.stderr(Stdio::piped()).spawn().unwrap();
    let code = exit_within(&mut child, Duration::from_secs(60));
    let mut said = String::new();
    std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut said).unwrap();
    (code, said)
}

#[test]
fn a_second_server_on_the_same_data_folder_is_refused() {
    let dir = folder("second");
    let socket = short_socket("second");
    let mut first = program(&dir, &[])
        .arg("--socket")
        .arg(&socket)
        .spawn()
        .unwrap();
    wait_for_socket(&mut first, &socket);
    // On another socket or port all the same: the data folder is what is held.
    let other = socket.with_file_name("other.sock");
    let control = std::fs::read(dir.join("data/server.json")).unwrap();
    let mut on_socket = program(&dir, &[]);
    on_socket.arg("--socket").arg(&other);
    let on_port = program(&dir, &[("ESCOUADE_REMOTE_PORT", free_port().to_string())]);
    for second in [on_socket, on_port] {
        let (code, said) = refused(second);
        // Its own code: the unit does not restart it in a loop.
        assert_eq!(code, Some(3), "{said}");
        let pid = first.id().to_string();
        assert!(
            said.contains("already serves") && said.contains(&pid),
            "{said}"
        );
    }
    // Refused before anything: no socket, `server.json` the first's.
    assert!(!other.exists());
    assert_eq!(
        std::fs::read(dir.join("data/server.json")).unwrap(),
        control
    );
    // The first serves on, and is the only one to write `devices.json`: a revocation stays one.
    let (code, out) = command(&dir, &["pair"]);
    assert_eq!(code, Some(0), "{out}");
    unsafe { libc::kill(first.id() as i32, libc::SIGTERM) };
    assert_eq!(exit_within(&mut first, Duration::from_secs(60)), Some(0));
    assert!(!socket.exists(), "the socket outlives its server");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(socket.parent().unwrap());
}

#[test]
fn a_killed_server_leaves_nothing_that_stops_the_next() {
    use std::os::unix::fs::PermissionsExt;
    let dir = folder("killed");
    let socket = short_socket("killed");
    let start = || {
        program(&dir, &[])
            .arg("--socket")
            .arg(&socket)
            .spawn()
            .unwrap()
    };
    let mut first = start();
    wait_for_socket(&mut first, &socket);
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(socket.parent().unwrap()), 0o700);
    assert_eq!(mode(&socket), 0o600);
    // `pair` reaches it by its socket, and says how to.
    let (code, out) = command(&dir, &["pair"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains(&format!("ssh -N -L 8765:{} ", socket.display())),
        "{out}"
    );
    assert!(out.contains("http://127.0.0.1:8765/"), "{out}");
    assert!(!out.contains("serve on --socket"), "{out}");
    first.kill().unwrap();
    first.wait().unwrap();
    // Its lock went with it; its socket and `server.json` stayed, and are replaced.
    assert!(socket.exists() && dir.join("data/server.json").exists());
    let mut next = start();
    wait_for_socket(&mut next, &socket);
    let (code, out) = command(&dir, &["pair"]);
    assert_eq!(code, Some(0), "{out}");
    unsafe { libc::kill(next.id() as i32, libc::SIGTERM) };
    assert_eq!(exit_within(&mut next, Duration::from_secs(60)), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(socket.parent().unwrap());
}

#[test]
fn a_server_on_a_port_says_to_prefer_the_socket_on_a_shared_machine() {
    let dir = folder("port-advice");
    let port = free_port();
    let mut child = program(&dir, &[])
        .args(["--port", &port.to_string()])
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(120);
    while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < until, "no server on {port}");
        std::thread::sleep(Duration::from_millis(100));
    }
    let (code, out) = command(&dir, &["pair"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains(&format!("http://127.0.0.1:{port}/")), "{out}");
    assert!(out.contains("serve on --socket"), "{out}");
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    assert_eq!(exit_within(&mut child, Duration::from_secs(60)), Some(0));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The answer to the call `cmd` with `args` (and the stream `stream`), the frames before it skipped.
async fn ask(
    ws: &mut tokio_tungstenite::WebSocketStream<tokio::net::UnixStream>,
    id: u64,
    cmd: &str,
    args: serde_json::Value,
    stream: Option<u32>,
) -> serde_json::Value {
    let mut call = serde_json::json!({ "id": id, "cmd": cmd, "args": args });
    if let Some(n) = stream {
        call["stream"] = n.into();
    }
    ws.send(Message::text(call.to_string())).await.unwrap();
    loop {
        let frame = tokio::time::timeout(Duration::from_secs(30), ws.next())
            .await
            .expect("an answer in time")
            .expect("the connection open")
            .unwrap();
        let Message::Text(text) = frame else { continue };
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        if value["id"] == id {
            return value;
        }
    }
}

#[test]
fn a_sigterm_with_a_terminal_open_stops_it_within_ten_seconds() {
    let dir = folder("sigterm-terminal");
    let repo = dir.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    for args in [
        &["init", "-q"][..],
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    ] {
        let ok = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .status()
            .unwrap();
        assert!(ok.success());
    }
    let socket = short_socket("sigterm");
    // Unique on the machine: this test's pid.
    let marker = 900_000_000 + std::process::id();
    let mut child = program(&dir, &[("ESCOUADE_REMOTE_TOKEN", TOKEN.into())])
        .arg("--socket")
        .arg(&socket)
        .spawn()
        .unwrap();
    wait_for_socket(&mut child, &socket);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut ws = runtime.block_on(async {
        let stream = tokio::net::UnixStream::connect(&socket).await.unwrap();
        let mut req = "ws://localhost:8765/".into_client_request().unwrap();
        let bearer = format!("Bearer {TOKEN}").parse().unwrap();
        req.headers_mut().insert("authorization", bearer);
        let (mut ws, _) = tokio_tungstenite::client_async(req, stream).await.unwrap();
        // A project's folder from a client stays in the home of the server's user, as the browser of folders does.
        let outside = std::env::temp_dir().canonicalize().unwrap();
        let args = serde_json::json!({ "path": outside, "name": "out", "color": "oklch(0.72 0.12 48)", "worktreePerAgent": false });
        let refused = ask(&mut ws, 10, "create_project", args, None).await;
        assert!(refused["ok"].is_null() && !refused["err"].is_null(), "{refused}");
        for (id, cmd) in [(11, "inspect_folder"), (12, "first_commit")] {
            let refused = ask(&mut ws, id, cmd, serde_json::json!({ "path": outside }), None).await;
            assert!(!refused["err"].is_null(), "{cmd}: {refused}");
        }
        let project = ask(
            &mut ws,
            1,
            "create_project",
            serde_json::json!({ "path": repo, "name": "demo", "color": "oklch(0.72 0.12 48)", "worktreePerAgent": false }),
            None,
        )
        .await;
        let project = project["ok"]["id"].as_str().unwrap_or_else(|| panic!("{project}")).to_string();
        let args = serde_json::json!({ "projectId": project, "shell": "bash", "name": "t", "cols": 80, "rows": 24 });
        let term = ask(&mut ws, 2, "term_spawn", args, Some(7)).await;
        let id = term["ok"]["id"].as_str().unwrap_or_else(|| panic!("{term}"));
        // A process of the terminal's own, found by its command line once it runs.
        let line = format!("exec sleep {marker}\n");
        let wrote = ask(&mut ws, 3, "term_write", serde_json::json!({ "id": id, "data": line }), None).await;
        assert!(wrote["ok"].is_number(), "{wrote}");
        ws
    });
    let alive = || {
        Command::new("pgrep")
            .args(["-f", &format!("^sleep {marker}$")])
            .output()
            .unwrap()
            .status
            .success()
    };
    let until = Instant::now() + Duration::from_secs(30);
    while !alive() {
        assert!(Instant::now() < until, "the terminal never ran its command");
        std::thread::sleep(Duration::from_millis(100));
    }
    let sent = Instant::now();
    unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    let code = exit_within(&mut child, Duration::from_secs(10));
    assert_eq!(code, Some(0), "not stopped cleanly within 10 s");
    assert!(sent.elapsed() < Duration::from_secs(10));
    // Its clients are told: the connection ends.
    runtime.block_on(async {
        let ended = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(Ok(_)) = ws.next().await {}
        });
        assert!(ended.await.is_ok());
    });
    // The terminal's processes went with it.
    assert!(!alive(), "the terminal's process outlives the server");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(socket.parent().unwrap());
}

#[cfg(target_os = "linux")]
#[test]
fn install_writes_the_unit_alike_twice_and_uninstall_removes_it() {
    let dir = folder("install");
    let web = dir.join("web");
    std::fs::create_dir_all(&web).unwrap();
    let unit = dir.join(".config/systemd/user/escouade-server.service");
    let install = || {
        let out = program(&dir, &[])
            .env_remove("XDG_CONFIG_HOME")
            .args(["install", "--user", "--web"])
            .arg(&web)
            .args(["--origin", "https://escouade.example.com"])
            .stdout(Stdio::piped())
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stdout).into_owned();
        assert_eq!(out.status.code(), Some(0), "{said}");
        // What to run is said, not run.
        for step in [
            "systemctl --user daemon-reload",
            "systemctl --user enable --now escouade-server",
            "loginctl enable-linger",
        ] {
            assert!(said.contains(step), "{said}");
        }
        std::fs::read_to_string(&unit).unwrap()
    };
    let text = install();
    let exe = std::fs::canonicalize(env!("CARGO_BIN_EXE_escouade-server")).unwrap();
    assert!(
        text.contains(&format!(
            "ExecStart=\"{}\" --socket %t/escouade/server.sock --web \"{}\" --origin \"https://escouade.example.com\"\n",
            exe.display(),
            web.display()
        )),
        "{text}"
    );
    // The data folder it was installed for.
    let data = format!(
        "Environment=\"ESCOUADE_DATA_DIR={}\"\n",
        dir.join("data").display()
    );
    assert!(text.contains(&data), "{text}");
    assert!(text.contains("Restart=on-failure\n") && !text.contains(TOKEN));
    assert_eq!(install(), text, "a second install changed the unit");
    // Read by systemd itself where it is there.
    if Command::new("systemd-analyze")
        .arg("--version")
        .output()
        .is_ok()
    {
        let out = Command::new("systemd-analyze")
            .args(["--user", "verify"])
            .arg(&unit)
            .env("XDG_RUNTIME_DIR", &dir)
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success() && said.trim().is_empty(),
            "systemd-analyze verify: {said}"
        );
    }
    let uninstall = || {
        program(&dir, &[])
            .env_remove("XDG_CONFIG_HOME")
            .arg("uninstall")
            .stderr(Stdio::piped())
            .output()
            .unwrap()
    };
    // Still enabled: left in place.
    let wants = unit.with_file_name("default.target.wants");
    std::fs::create_dir_all(&wants).unwrap();
    std::os::unix::fs::symlink(&unit, wants.join("escouade-server.service")).unwrap();
    let out = uninstall();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("disable --now"));
    assert!(unit.exists());
    std::fs::remove_dir_all(&wants).unwrap();
    assert_eq!(uninstall().status.code(), Some(0));
    assert!(!unit.exists());
    assert_eq!(
        uninstall().status.code(),
        Some(0),
        "twice: nothing to remove"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
