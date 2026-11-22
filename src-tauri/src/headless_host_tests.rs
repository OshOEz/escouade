use super::*;

#[test]
fn the_unit_serves_on_the_users_socket_by_default() {
    let serve = Serve::default();
    let text = unit(Path::new("/opt/escouade/escouade-server"), &serve, None).unwrap();
    assert_eq!(
        text,
        "# Written by `escouade-server install`, removed by `escouade-server uninstall` (docs/REMOTE.md).\n\
         [Unit]\n\
         Description=Escouade server: the core and its web interface, for the browsers paired to it\n\
         \n\
         [Service]\n\
         ExecStart=\"/opt/escouade/escouade-server\" --socket %t/escouade/server.sock\n\
         # On SIGTERM the server stops its agents itself, as the app does when it quits; what is left is then killed.\n\
         KillMode=mixed\n\
         Restart=on-failure\n\
         RestartSec=5\n\
         # Wrong arguments (2) or the data folder another server's (3): restarting would not change it.\n\
         RestartPreventExitStatus=2 3\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n"
    );
    // No token in it: the devices' are in the data folder.
    assert!(!text.contains("TOKEN"));
}

#[test]
fn the_unit_says_what_install_was_given_written_as_itself() {
    let serve = Serve {
        port: Some(8765),
        socket: None,
        web: Some(PathBuf::from("/srv/my web/100%")),
        origins: vec!["https://escouade.example.com".into()],
    };
    let text = unit(
        Path::new("/home/a\"b/$HOME/escouade-server"),
        &serve,
        Some(Path::new("/data/%h $x")),
    )
    .unwrap();
    assert!(
        text.contains(
            "ExecStart=\"/home/a\\\"b/$$HOME/escouade-server\" --port 8765 \
             --web \"/srv/my web/100%%\" --origin \"https://escouade.example.com\"\n"
        ),
        "{text}"
    );
    // Not expanded in an assignment: `$` as it is.
    assert!(
        text.contains("Environment=\"ESCOUADE_DATA_DIR=/data/%%h $x\"\n"),
        "{text}"
    );
    let socket = Serve {
        socket: Some(PathBuf::from("/run/user/1000/e/s.sock")),
        ..Serve::default()
    };
    let text = unit(Path::new("/e"), &socket, None).unwrap();
    assert!(
        text.contains("ExecStart=\"/e\" --socket \"/run/user/1000/e/s.sock\"\n"),
        "{text}"
    );
    let broken = Serve {
        web: Some(PathBuf::from("/a\nExecStartPre=/bin/evil")),
        ..Serve::default()
    };
    assert!(unit(Path::new("/e"), &broken, None).is_err());
}

#[cfg(unix)]
#[test]
fn a_path_that_is_not_utf8_is_refused_in_the_unit_not_written_wrong() {
    use std::os::unix::ffi::OsStrExt;
    let odd = PathBuf::from(std::ffi::OsStr::from_bytes(b"/srv/\xffweb"));
    let serve = Serve {
        web: Some(odd.clone()),
        ..Serve::default()
    };
    let why = unit(Path::new("/e"), &serve, None).unwrap_err();
    assert!(why.contains("UTF-8"), "{why}");
    let why = unit(&odd, &Serve::default(), None).unwrap_err();
    assert!(why.contains("UTF-8"), "{why}");
    let why = unit(Path::new("/e"), &Serve::default(), Some(&odd)).unwrap_err();
    assert!(why.contains("UTF-8"), "{why}");
}

#[cfg(unix)]
#[test]
fn the_files_sent_to_the_server_are_its_users_alone() {
    use std::os::unix::fs::PermissionsExt;
    let data = paths::DataDir::new(paths::test_dir("headless-private"));
    let dir = data.attachments();
    // Missing, then there with the umask's rights.
    keep_private(&dir);
    let mode = |d: &Path| std::fs::metadata(d).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    keep_private(&dir);
    assert_eq!(mode(&dir), 0o700);
}

#[test]
fn the_arguments_say_one_place_to_listen() {
    #[cfg(unix)]
    {
        let serve =
            parse_serve(&["--socket", "run/s.sock", "--origin", "https://E.example"]).unwrap();
        assert_eq!(
            serve.socket,
            Some(std::env::current_dir().unwrap().join("run/s.sock"))
        );
        assert_eq!(serve.origins, ["https://e.example"]);
    }
    // Refused at once where there is no Unix socket, before the core loads.
    #[cfg(not(unix))]
    assert!(parse_serve(&["--socket", "s.sock"])
        .unwrap_err()
        .contains("--port"));
    assert_eq!(parse_serve(&["--port", "8765"]).unwrap().port, Some(8765));
    for wrong in [
        &["--port", "1", "--socket", "/s"][..],
        &["--socket", "/s", "--port", "1"],
        &["--port", "0"],
        &["--port", "70000"],
        &["--port"],
        &["--socket"],
        &["--web", "/no/such/folder"],
        &["--listen", "0.0.0.0"],
    ] {
        assert!(parse_serve(wrong).is_err(), "{wrong:?}");
    }
}

/// A core on the server's host, alerts on, and what its clients are told.
fn alerting(
    name: &str,
) -> (
    Arc<Headless>,
    Arc<Core>,
    Arc<parking_lot::Mutex<Vec<serde_json::Value>>>,
) {
    let data = paths::DataDir::new(paths::test_dir(name).join("data"));
    data.ensure().unwrap();
    let settings = Settings {
        sound: true,
        os_notifications: true,
        ..Default::default()
    };
    std::fs::write(data.settings_file(), serde_json::to_vec(&settings).unwrap()).unwrap();
    let host = Arc::new(Headless::default());
    let (core, _rx) = Core::load(host.clone(), data);
    let _ = host.core.set(Arc::downgrade(&core));
    let told = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let sink = told.clone();
    std::mem::forget(
        core.hub
            .subscribe(crate::sink::Sink::new(move |e: UiEvent| {
                let e = serde_json::to_value(&e).unwrap();
                if ["chime", "notify", "focus"].contains(&e["type"].as_str().unwrap_or("")) {
                    sink.lock().push(e);
                }
                Ok(())
            })),
    );
    (host, core, told)
}

fn click(core: &Arc<Core>, id: u64) {
    let ctx = remoteapi::table::Ctx { core: core.clone() };
    let call = remoteapi::table::call(
        remoteapi::table::entries(),
        ctx,
        "notification_click",
        serde_json::json!({ "id": id }),
    );
    rt::block_on(call).unwrap();
}

#[test]
fn an_alert_chimes_and_notifies_in_the_browsers_and_its_click_comes_back_once() {
    let (_host, core, told) = alerting("headless-alert");
    let focus = UiEvent::Focus {
        project_id: "p1".into(),
        agent_id: Some("a1".into()),
    };
    core.alert(
        crate::agent::NotifyKind::Done,
        "demo".into(),
        "Tour fini".into(),
        focus,
    );
    let seen = told.lock().clone();
    assert_eq!(seen.len(), 2, "{seen:?}");
    assert_eq!(seen[0], serde_json::json!({ "type": "chime" }));
    assert_eq!(
        (
            seen[1]["type"].as_str(),
            seen[1]["title"].as_str(),
            seen[1]["body"].as_str()
        ),
        (Some("notify"), Some("demo"), Some("Tour fini"))
    );
    let id = seen[1]["id"].as_u64().unwrap();
    // The click, by the remote command: the focus the desktop's click sends.
    click(&core, id);
    assert_eq!(
        told.lock()[2],
        serde_json::json!({ "type": "focus", "projectId": "p1", "agentId": "a1" })
    );
    // Once: clicked again (another tab, another device), or an id never given, it does nothing.
    click(&core, id);
    click(&core, id + 1000);
    assert_eq!(told.lock().len(), 3);
    core.stop_for_good();
}

#[test]
fn the_clicks_kept_are_the_last_ones() {
    let (host, core, told) = alerting("headless-clicks-kept");
    let clicked = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for _ in 0..=KEPT_CLICKS {
        let clicked = clicked.clone();
        host.toast(
            "t",
            "b",
            Box::new(move || {
                clicked.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );
    }
    let ids: Vec<u64> = told
        .lock()
        .iter()
        .map(|e| e["id"].as_u64().unwrap())
        .collect();
    assert_eq!(ids.len(), KEPT_CLICKS + 1);
    host.clicked(ids[0]);
    assert_eq!(
        clicked.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "the oldest is forgotten"
    );
    host.clicked(ids[KEPT_CLICKS]);
    host.clicked(ids[1]);
    assert_eq!(clicked.load(std::sync::atomic::Ordering::SeqCst), 2);
    core.stop_for_good();
}
