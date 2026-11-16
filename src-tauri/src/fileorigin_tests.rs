//! Tests of the paths of the files a page hands the window (`fileorigin`): what is read from a
//! message, and the command (the table and its wait: `fileorigins_tests`). What WebView2 gives the
//! host (the COM side) only runs in the real window, tried by hand.

use crate::commands::file_origin;
use crate::fileorigin::{parse_message, FileOrigins};
use serde_json::json;
use std::time::{Duration, Instant};
use tauri::Manager;

const UUID: &str = "3f2a9c1e-7b64-4d0a-8e55-1c2d3e4f5a6b";

fn message(id: &str) -> String {
    json!({ "escouadeFileOrigin": id }).to_string()
}

#[test]
fn a_message_with_an_id_asks_for_that_file() {
    assert_eq!(parse_message(&message(UUID)), Some(UUID.to_string()));
    assert_eq!(parse_message(&message("a_B-9")), Some("a_B-9".to_string()));
    assert_eq!(parse_message(&message("x")), Some("x".to_string()));
}

#[test]
fn an_id_has_from_1_to_64_characters() {
    let longest = "a".repeat(64);
    assert_eq!(parse_message(&message(&longest)), Some(longest));
    assert_eq!(parse_message(&message(&"a".repeat(65))), None);
    assert_eq!(parse_message(&message("")), None);
}

#[test]
fn an_id_is_made_of_letters_digits_dashes_and_underscores_only() {
    for id in [
        "a b", "a/b", "..\\x", "a.b", "a\"b", "a\nb", "é", "日本", "a\u{0}b", " ",
    ] {
        assert_eq!(parse_message(&message(id)), None, "{id:?}");
    }
}

#[test]
fn what_is_not_a_message_for_a_file_origin_is_not_read() {
    for text in [
        "",
        "not json",
        "{",
        "null",
        "[]",
        "\"escouadeFileOrigin\"",
        "{}",
        r#"{"other":"abc"}"#,
        r#"{"escouadeFileOrigin":42}"#,
        r#"{"escouadeFileOrigin":null}"#,
        r#"{"escouadeFileOrigin":["abc"]}"#,
        // A message of Tauri's own IPC.
        r#"{"cmd":"subscribe","callback":1,"error":2,"payload":{},"options":{},"__TAURI_INVOKE_KEY__":"k"}"#,
    ] {
        assert_eq!(parse_message(text), None, "{text:?}");
    }
}

/// The command, the way the page asks it: with the app's own table.
mod command {
    use super::*;
    use tauri::test::{mock_app, MockRuntime};

    fn app_with(origins: &FileOrigins) -> tauri::App<MockRuntime> {
        let app = mock_app();
        app.manage(origins.clone());
        app
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn the_command_gives_the_path_the_window_sent() {
        let origins = FileOrigins::default();
        let app = app_with(&origins);
        origins.deposit(UUID.into(), "C:\\Users\\Jean\\rapport.xlsx".into());
        let path = file_origin(app.handle().clone(), UUID.into()).await;
        assert_eq!(path.as_deref(), Some("C:\\Users\\Jean\\rapport.xlsx"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn the_path_the_command_gave_is_kept_for_a_file_to_be_joined_by_and_only_once() {
        let origins = FileOrigins::default();
        let app = app_with(&origins);
        origins.deposit(UUID.into(), "C:\\Users\\Jean\\rapport.xlsx".into());
        // Not asked for yet: no file is joined by it.
        assert_eq!(origins.take_handed(UUID), None);
        let path = file_origin(app.handle().clone(), UUID.into()).await;
        assert_eq!(path.as_deref(), Some("C:\\Users\\Jean\\rapport.xlsx"));
        assert_eq!(
            origins.take_handed(UUID).as_deref(),
            Some("C:\\Users\\Jean\\rapport.xlsx")
        );
        assert_eq!(origins.take_handed(UUID), None);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn the_command_waits_for_a_path_that_is_not_there_yet() {
        let origins = FileOrigins::default();
        let app = app_with(&origins);
        let sender = origins.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            sender.deposit(UUID.into(), "C:\\Users\\Jean\\rapport.xlsx".into());
        });
        let path = file_origin(app.handle().clone(), UUID.into()).await;
        assert_eq!(path.as_deref(), Some("C:\\Users\\Jean\\rapport.xlsx"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn the_command_gives_none_after_two_seconds_without_a_path() {
        let app = app_with(&FileOrigins::default());
        let started = Instant::now();
        assert_eq!(file_origin(app.handle().clone(), UUID.into()).await, None);
        assert!(started.elapsed() >= Duration::from_millis(1900));
    }

    #[tokio::test]
    async fn the_command_gives_none_at_once_for_an_id_no_page_would_send() {
        let origins = FileOrigins::default();
        let app = app_with(&origins);
        origins.deposit("a b".into(), "C:\\x".into());
        let started = Instant::now();
        assert_eq!(file_origin(app.handle().clone(), "a b".into()).await, None);
        assert_eq!(file_origin(app.handle().clone(), String::new()).await, None);
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[tokio::test]
    async fn the_command_gives_none_without_a_table() {
        let app = mock_app();
        assert_eq!(file_origin(app.handle().clone(), UUID.into()).await, None);
    }

    /// Elsewhere than on Windows, nothing is read from the window: no path, and no wait for one.
    #[cfg(not(windows))]
    #[tokio::test]
    async fn the_command_gives_none_at_once_on_a_system_with_no_bridge() {
        let origins = FileOrigins::default();
        let app = app_with(&origins);
        origins.deposit(UUID.into(), "/home/jean/rapport.xlsx".into());
        let started = Instant::now();
        assert_eq!(file_origin(app.handle().clone(), UUID.into()).await, None);
        assert!(started.elapsed() < Duration::from_millis(500));
        // Nothing was read by the host, so nothing is handed out: no file is joined by that path.
        assert_eq!(origins.take_handed(UUID), None);
    }
}
