//! The table of the commands: each `#[command]` of `commands.rs` gives its entry, which calls it with the arguments the
//! window would send, as JSON, and answers what Tauri would have answered.

use crate::core::Core;
use escouade_base::rt;
use futures_util::future::BoxFuture;
use serde::de::{DeserializeOwned, Deserializer, Error as _, Visitor};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

/// What a command is given remotely.
pub struct Ctx {
    pub core: Arc<Core>,
}

/// The answer of an entry, under way.
pub type Pending = BoxFuture<'static, Result<Value, CallError>>;

/// An entry's call: the context and the arguments, as the JSON object the window sends.
pub type Call = fn(Ctx, Value) -> Pending;

/// A command of the table.
pub struct Entry {
    pub name: &'static str,
    /// `None`: the command is the desktop app's alone.
    pub call: Option<Call>,
}

/// Why a call has no value.
#[derive(Debug, PartialEq)]
pub enum CallError {
    /// No command has this name.
    Unknown,
    /// The command is the desktop app's alone.
    DesktopOnly,
    /// An argument is missing or does not read: Tauri's text, which names it.
    Args(String),
    /// The command failed, or panicked: the value Tauri gives the window (its error as JSON, a text for a panic).
    Failed(Value),
}

/// Every command of the window, from the list `lib.rs` gives Tauri.
#[cfg_attr(not(debug_assertions), allow(dead_code))] // Its one caller in the app is debug-only.
pub fn entries() -> &'static [Entry] {
    crate::commands().1
}

/// The command `name` of `entries`, called with `args`; on the app's runtime, as Tauri runs one.
pub async fn call(
    entries: &[Entry],
    ctx: Ctx,
    name: &str,
    args: Value,
) -> Result<Value, CallError> {
    let entry = entries
        .iter()
        .find(|e| e.name == name)
        .ok_or(CallError::Unknown)?;
    let call = entry.call.ok_or(CallError::DesktopOnly)?;
    rt::spawn(call(ctx, args))
        .await
        .map_err(|e| CallError::Failed(e.to_string().into()))?
}

/// A synchronous command, on the runtime's threads for what blocks.
pub async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> T + Send + 'static,
) -> Result<T, CallError> {
    rt::spawn_blocking(f)
        .await
        .map_err(|e| CallError::Failed(e.to_string().into()))
}

/// The argument `key` of `command`, read as Tauri reads it: an absent key is `None` for an `Option`, an error otherwise.
pub fn arg<T: DeserializeOwned>(
    args: &Value,
    command: &'static str,
    key: &'static str,
) -> Result<T, CallError> {
    T::deserialize(Item { args, command, key })
        .map_err(|e| CallError::Args(format!("invalid args `{key}` for command `{command}`: {e}")))
}

/// Tauri's `CommandItem` as a deserializer, on the JSON object alone.
struct Item<'a> {
    args: &'a Value,
    command: &'static str,
    key: &'static str,
}

impl<'a> Item<'a> {
    fn value(&self) -> serde_json::Result<&'a Value> {
        self.args.get(self.key).ok_or_else(|| {
            serde_json::Error::custom(format!(
                "command {} missing required key {}",
                self.command, self.key
            ))
        })
    }
}

macro_rules! pass {
    ($($fn:ident($($arg:ident: $ty:ty),*);)*) => {$(
        fn $fn<V: Visitor<'a>>(self, $($arg: $ty,)* visitor: V) -> Result<V::Value, Self::Error> {
            self.value()?.$fn($($arg,)* visitor)
        }
    )*};
}

impl<'a> Deserializer<'a> for Item<'a> {
    type Error = serde_json::Error;

    fn deserialize_option<V: Visitor<'a>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.args.get(self.key) {
            Some(value) => value.deserialize_option(visitor),
            None => visitor.visit_none(),
        }
    }

    pass! {
        deserialize_any(); deserialize_bool(); deserialize_i8(); deserialize_i16(); deserialize_i32();
        deserialize_i64(); deserialize_u8(); deserialize_u16(); deserialize_u32(); deserialize_u64();
        deserialize_f32(); deserialize_f64(); deserialize_char(); deserialize_str(); deserialize_string();
        deserialize_bytes(); deserialize_byte_buf(); deserialize_unit();
        deserialize_unit_struct(name: &'static str); deserialize_newtype_struct(name: &'static str);
        deserialize_seq(); deserialize_tuple(len: usize);
        deserialize_tuple_struct(name: &'static str, len: usize); deserialize_map();
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_identifier(); deserialize_ignored_any();
    }
}

/// A command's value as Tauri answers it: chosen by autoref, a `Result` before any other value (Tauri's
/// `ipc::private`).
pub struct ValueTag;
pub struct ResultTag;

pub trait ReplyValue {
    fn reply_kind(&self) -> ValueTag {
        ValueTag
    }
}

impl<T: Serialize> ReplyValue for &T {}

pub trait ReplyResult {
    fn reply_kind(&self) -> ResultTag {
        ResultTag
    }
}

impl<T: Serialize, E: Serialize> ReplyResult for Result<T, E> {}

impl ValueTag {
    pub fn reply<T: Serialize>(self, value: T) -> Result<Value, CallError> {
        serde_json::to_value(value).map_err(|e| CallError::Failed(e.to_string().into()))
    }
}

impl ResultTag {
    /// The error is what Tauri rejects with: `E` as JSON.
    pub fn reply<T: Serialize, E: Serialize>(
        self,
        result: Result<T, E>,
    ) -> Result<Value, CallError> {
        match result {
            Ok(value) => ValueTag.reply(value),
            Err(e) => Err(CallError::Failed(
                serde_json::to_value(e).unwrap_or_else(|e| e.to_string().into()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{self, CoreState};
    use crate::model::UiState;
    use crate::paths::{test_dir, DataDir};
    use escouade_base::rt;
    use escouade_macros::command;
    use serde_json::json;

    /// Where it runs: on the app's runtime, its thread's name, and whether that thread may wait on a future, which a
    /// worker of the runtime may not.
    #[command]
    fn where_it_runs() -> (bool, String, u8) {
        let runtime = tokio::runtime::Handle::try_current().is_ok();
        let thread = std::thread::current()
            .name()
            .unwrap_or_default()
            .to_string();
        (runtime, thread, rt::block_on(async { 7 }))
    }

    #[command]
    fn given(maybe_count: Option<u32>) -> Option<u32> {
        maybe_count
    }

    #[command]
    fn breaks() {
        panic!("boom");
    }

    const TABLE: &[Entry] = &[
        commands::__REMOTE_SET_UI,
        commands::__REMOTE_ENGINES,
        commands::__REMOTE_ACCOUNT_STATUS,
        commands::__REMOTE_QUIT_APP,
        commands::__REMOTE_PR_REVIEWS,
        __REMOTE_WHERE_IT_RUNS,
        __REMOTE_GIVEN,
        __REMOTE_BREAKS,
    ];

    /// A core loaded on the fake host, stopped with the test.
    struct Loaded(Arc<Core>);

    impl Drop for Loaded {
        fn drop(&mut self) {
            self.0.stop_for_good();
        }
    }

    fn load(name: &str) -> Loaded {
        let data = DataDir::new(test_dir(name).join("data"));
        Loaded(Core::load(crate::host::Fake::new(), data).0)
    }

    fn run(core: &Loaded, name: &str, args: Value) -> Result<Value, CallError> {
        let ctx = Ctx {
            core: core.0.clone(),
        };
        rt::block_on(call(TABLE, ctx, name, args))
    }

    #[test]
    fn a_command_is_called_by_its_name_with_camel_case_arguments() {
        let core = load("remote-table-call");
        let ui = UiState {
            active_project: Some("p1".into()),
            view: "board".into(),
            ..Default::default()
        };
        assert_eq!(run(&core, "set_ui", json!({ "ui": ui })), Ok(Value::Null));
        assert_eq!(core.0.ui.read().view, "board");
        assert_eq!(core.0.ui.read().active_project.as_deref(), Some("p1"));
        assert_eq!(
            run(&core, "pr_reviews", json!({ "projectId": "p1" })),
            Ok(json!([]))
        );
        assert_eq!(
            run(&core, "given", json!({ "maybeCount": 3 })),
            Ok(json!(3))
        );
    }

    #[test]
    fn a_missing_argument_is_an_error_that_names_it() {
        let core = load("remote-table-missing");
        assert_eq!(
            run(&core, "pr_reviews", json!({ "project_id": "p1" })),
            Err(CallError::Args(
                "invalid args `projectId` for command `pr_reviews`: command pr_reviews missing \
                 required key projectId"
                    .into()
            ))
        );
        // An optional one is `None`, as Tauri gives it.
        assert_eq!(run(&core, "given", json!({})), Ok(Value::Null));
    }

    #[test]
    fn an_unknown_command_is_refused() {
        let core = load("remote-table-unknown");
        assert_eq!(run(&core, "nope", json!({})), Err(CallError::Unknown));
    }

    #[test]
    fn a_desktop_command_is_refused_remotely() {
        let core = load("remote-table-desktop");
        assert_eq!(
            run(&core, "quit_app", json!({})),
            Err(CallError::DesktopOnly)
        );
    }

    #[test]
    fn a_command_that_fails_gives_its_message() {
        let core = load("remote-table-fails");
        let by_tauri = rt::block_on(commands::account_status(
            CoreState::from(&core.0),
            "nope".into(),
        ))
        .unwrap_err();
        assert!(by_tauri.contains("nope"), "{by_tauri}");
        assert_eq!(
            run(&core, "account_status", json!({ "id": "nope" })),
            Err(CallError::Failed(by_tauri.into()))
        );
    }

    #[test]
    fn a_command_that_fails_with_a_structured_error_gives_that_structure() {
        let core = load("remote-table-structured");
        let path = "/no/such/plugin/folder";
        let by_tauri = rt::block_on(commands::plugin_inspect_folder(
            CoreState::from(&core.0),
            path.into(),
        ))
        .expect_err("no plugin there");
        let by_tauri = serde_json::to_value(by_tauri).unwrap();
        assert!(by_tauri.is_object(), "{by_tauri}");
        let ctx = Ctx {
            core: core.0.clone(),
        };
        let args = json!({ "path": path });
        assert_eq!(
            rt::block_on(call(entries(), ctx, "plugin_inspect_folder", args)),
            Err(CallError::Failed(by_tauri))
        );
    }

    #[test]
    fn every_command_of_the_window_is_in_the_table() {
        let names: Vec<&str> = entries().iter().map(|e| e.name).collect();
        // The names of `lib.rs`' list before it became `handlers!`, `term_attach`, `run_attach` and
        // `notification_click`.
        assert_eq!(names.len(), 389);
        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(unique.len(), names.len(), "a command listed twice");
        assert!(names.contains(&"subscribe") && names.contains(&"file_origin"));
    }

    /// The same table with the window or without it: a command of the desktop alone keeps its entry, without a call.
    #[test]
    fn the_table_is_the_same_with_and_without_the_desktop() {
        let table = entries();
        assert_eq!(table.len(), 389);
        assert_eq!(table.iter().filter(|e| e.call.is_some()).count(), 340);
        let called = |name: &str| {
            table
                .iter()
                .find(|e| e.name == name)
                .map(|e| e.call.is_some())
        };
        assert_eq!(called("file_origin"), Some(false));
        assert_eq!(called("voice_start"), Some(false));
        assert_eq!(called("open_url"), Some(true));
        assert_eq!(called("play_chime"), Some(true));
    }

    #[test]
    fn the_commands_of_the_desktop_alone_are_these() {
        let mut want = vec![
            // An `AppHandle`: the window, the tray, the updater, the launches' pages.
            "clipboard_image",
            "devpage_close",
            "devpage_detach",
            "devpage_devtools",
            "devpage_go",
            "devpage_mode",
            "devpage_open",
            "devpage_place",
            "devpage_show",
            "file_origin",
            "inbox_save_file",
            "quit_app",
            "save_image",
            "update_check",
            "update_postpone",
            "update_restart",
            // A state of the desktop app: its updates, its deep links, its launches' pages.
            "deep_links_take",
            "devpage_list",
            "update_close",
            "update_download",
            "update_presence",
            // The voice, on this machine's microphone and speakers.
            "conversation_hold",
            "conversation_listen",
            "conversation_say",
            "conversation_start",
            "conversation_stop",
            "speech_delete",
            "speech_download",
            "speech_download_cancel",
            "speech_info",
            "speech_preview",
            "speech_preview_online",
            "voice_cancel",
            "voice_delete_model",
            "voice_download",
            "voice_download_cancel",
            "voice_info",
            "voice_online_catalog",
            "voice_start",
            "voice_stop",
            // A stream: the server serves them itself, through their `…_to` function.
            "account_login",
            "agent_terminal",
            "run_attach",
            "run_start",
            "subscribe",
            "term_attach",
            "term_spawn",
            "test_run_start",
            // Tauri's raw request: the bytes of an attachment, sent without JSON.
            "attach_chunk",
        ];
        want.sort_unstable();
        let mut got: Vec<&str> = entries()
            .iter()
            .filter(|e| e.call.is_none())
            .map(|e| e.name)
            .collect();
        got.sort_unstable();
        assert_eq!(got, want);
    }

    #[test]
    fn a_command_that_panics_gives_an_error() {
        let core = load("remote-table-panics");
        let Err(CallError::Failed(Value::String(text))) = run(&core, "breaks", json!({})) else {
            panic!("a panic is a failure, told as a text");
        };
        assert!(text.contains("boom"), "{text}");
    }

    #[test]
    fn a_blocking_command_runs_off_the_runtime() {
        let core = load("remote-table-blocking");
        let here = std::thread::current().name().map(str::to_string);
        let ran = run(&core, "where_it_runs", json!({})).unwrap();
        let (runtime, thread, waited): (bool, String, u8) = serde_json::from_value(ran).unwrap();
        assert!(runtime, "on the app's runtime");
        assert_ne!(Some(thread), here);
        assert_eq!(waited, 7, "a worker of the runtime would have panicked");
    }
}
