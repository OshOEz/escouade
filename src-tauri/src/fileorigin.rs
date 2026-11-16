//! Where a file the page hands the window lies on the disk (ESC-95, « chemin d'origine »).
//!
//! A `File` the WebView gives the page has no path, and Tauri's own drag and drop, which would give
//! it, is off (it would break the HTML drag and drop of the app). WebView2 has a way of its own: the
//! page sends its host a message with `File` objects (`chrome.webview.postMessageWithAdditionalObjects`)
//! and the host reads their paths (`ICoreWebView2File`). The page sends `{ "escouadeFileOrigin": id }`
//! with one file, then asks `file_origin(id)` for what the host read. Under Linux and macOS there is
//! no such bridge here: the command answers nothing at once, there is no path of origin, and the
//! page copies the file. A path the page would read from the drop's own data (`text/uri-list`) is
//! the page's word and no one else's: it is not one the host read, so it is never trusted.
//!
//! Only a path the host itself read may become a file joined by its path, since Escouade lets the
//! agent read such a file without asking (`AgentMeta::granted_files`): the page never gives a path
//! to `attach_origin`, only the id `file_origin` answered for. The command keeps what it answers
//! (`handed`) for that one taking (`take_handed`); an id the host read a path for and the page did
//! not ask the path of is of no use.
//!
//! The message goes to wry's own handler too, which hands it to Tauri's IPC. Tauri does not know it
//! for an invoke (`handle_ipc_message` does not find `cmd` in it) and drops it, with a
//! `console.error` in the page as its only reaction: nothing is run and nothing is answered.

pub use crate::fileorigins::FileOrigins;
use serde::Deserialize;
use std::time::Duration;
use tauri::{Runtime, WebviewWindow};

/// The longest an id may be (a UUID is 36).
const MAX_ID: usize = 64;

/// How long `file_origin` waits for a path: the message of the page crosses the WebView's process
/// to the host's thread, and the command may be asked first. A `File` that is no file of the disk
/// (a screenshot pasted) makes WebView2 refuse the message in the page itself (a TypeError, which
/// `file-origin.ts` turns into « no path » at once): this delay is for a message that got lost.
pub(crate) const WAIT: Duration = Duration::from_secs(2);

/// Whether `id` is one the page may give: 1 to 64 letters, digits, `-` or `_`.
pub(crate) fn valid_id(id: &str) -> bool {
    (1..=MAX_ID).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The id a page message asks for the path of a file with: `{ "escouadeFileOrigin": "<id>" }`.
/// Any other message (Tauri's own, another key, a bad id, no JSON) is none of ours.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn parse_message(message: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Asked {
        #[serde(rename = "escouadeFileOrigin")]
        id: String,
    }
    let Asked { id } = serde_json::from_str(message).ok()?;
    valid_id(&id).then_some(id)
}

/// Puts the listener on the WebView of `window`, which it keeps for good: the app only ever makes
/// its main window once (a reload keeps the WebView, and with it the listener).
#[cfg(windows)]
pub fn install<R: Runtime>(window: &WebviewWindow<R>, origins: &FileOrigins) {
    let origins = origins.clone();
    // The WebView's COM objects are only for its own thread: `with_webview` runs this on it.
    let asked = window.with_webview(move |webview| {
        // SAFETY: `controller` is the live controller of this WebView, used on its thread.
        if let Err(e) = unsafe { windows_side::listen(&webview.controller(), origins) } {
            log::error!("the paths of dropped files are not read: {e}");
        }
    });
    if let Err(e) = asked {
        log::error!("the paths of dropped files are not read: {e}");
    }
}

/// No bridge on the other systems: the command answers nothing.
#[cfg(not(windows))]
pub fn install<R: Runtime>(_window: &WebviewWindow<R>, _origins: &FileOrigins) {}

#[cfg(windows)]
mod windows_side {
    use super::{parse_message, FileOrigins};
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Controller, ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs,
        ICoreWebView2WebMessageReceivedEventArgs2,
    };
    use webview2_com::{take_pwstr, WebMessageReceivedEventHandler};
    use windows_core::{Interface, PWSTR};

    /// Adds to the WebView a handler of the messages the page sends it. wry has its own, for
    /// Tauri's IPC: this one comes with it, and does not take its place.
    ///
    /// # Safety
    /// `controller` must be the live controller of the WebView, and this called on its thread.
    pub unsafe fn listen(
        controller: &ICoreWebView2Controller,
        origins: FileOrigins,
    ) -> windows_core::Result<()> {
        let webview = controller.CoreWebView2()?;
        let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
            if let Some((id, path)) = args.as_ref().and_then(origin_of) {
                origins.deposit(id, path);
            }
            Ok(())
        }));
        // The registration lasts as long as the WebView: it is never removed.
        let mut token = 0;
        webview.add_WebMessageReceived(&handler, &mut token)
    }

    /// The id the page gave a file with the message, and the file's path, when the message is
    /// ours and has a file of the disk. Called for every message of the page, Tauri's IPC
    /// included, which has no object with it: `AdditionalObjects` tells them apart before
    /// anything is read.
    fn origin_of(args: &ICoreWebView2WebMessageReceivedEventArgs) -> Option<(String, String)> {
        // SAFETY: `args` is the event's, alive for the call. The strings the calls give are
        // allocated by COM (`CoTaskMemAlloc`) and taken, and so freed, by `take_pwstr`.
        unsafe {
            let objects = args
                .cast::<ICoreWebView2WebMessageReceivedEventArgs2>()
                .ok()?
                .AdditionalObjects()
                .ok()?;
            let mut count = 0;
            objects.Count(&mut count).ok()?;
            if count == 0 {
                return None;
            }
            let mut message = PWSTR::null();
            args.TryGetWebMessageAsString(&mut message).ok()?;
            let id = parse_message(&take_pwstr(message))?;
            let file = objects
                .GetValueAtIndex(0)
                .ok()?
                .cast::<ICoreWebView2File>()
                .ok()?;
            let mut path = PWSTR::null();
            file.Path(&mut path).ok()?;
            Some((id, take_pwstr(path)))
        }
    }
}
