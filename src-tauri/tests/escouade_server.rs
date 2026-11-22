//! `escouade-server` (`src/bin/escouade-server.rs`) as a process: its port and token in its environment, a data folder
//! and a home of its own, stopped by a signal.
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
fn it_will_not_start_without_a_token() {
    let dir = folder("no-token");
    let port = free_port().to_string();
    let mut child = program(&dir, &[("ESCOUADE_REMOTE_PORT", port)])
        .spawn()
        .unwrap();
    let code = exit_within(&mut child, Duration::from_secs(30));
    assert_eq!(code, Some(2));
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
