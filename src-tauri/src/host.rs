//! The app the core runs in: what the core asks of it, and the desktop app's answers.

use crate::fileorigins::FileOrigins;
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
use crate::model::Settings;
use std::path::PathBuf;

/// What the core asks of the app it runs in: the desktop app answers with Tauri, a server without.
pub trait Host: Send + Sync + 'static {
    fn version(&self) -> String;
    /// The folder of the installer's plugin seed, when there is one.
    fn seed_dir(&self) -> Option<PathBuf>;
    fn file_origins(&self) -> FileOrigins {
        Default::default()
    }
    /// The version of the webview the window shows, when there is one.
    fn webview_version(&self) -> Option<String>;
    /// The main window is shown and has the focus.
    fn window_attended(&self) -> bool;
    fn attention(&self);
    /// The chime of an alert, heard on the machine.
    fn chime(&self);
    /// `on_click` runs after the host has brought its window forward.
    fn toast(&self, title: &str, body: &str, on_click: Box<dyn Fn() + Send + Sync>);
    /// The agents waiting, for the tray icon and its tooltip.
    fn waiting(&self, n: usize);
    fn relabel(&self, lang: Lang, waiting: usize);
    fn stop_voice(&self);
    /// The dictation under way, if any, called off: a page that loads again has none.
    fn cancel_dictation(&self);
    /// A conversation under way takes the voice, language, pace and volume of `settings`.
    fn tune_conversation(&self, settings: &Settings, secrets: &dyn SecretStore);
}

#[cfg(test)]
pub use fake::Fake;

#[cfg(test)]
mod fake {
    use super::*;
    use std::sync::Arc;

    /// A host without a window, tray or seed, that keeps what it was asked.
    #[derive(Default)]
    pub struct Fake {
        pub toasts: parking_lot::Mutex<Vec<(String, String)>>,
        /// The toasts' `on_click`, in the same order.
        pub clicks: parking_lot::Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
        pub waiting: parking_lot::Mutex<Vec<usize>>,
        pub relabels: parking_lot::Mutex<Vec<(Lang, usize)>>,
        pub attention: parking_lot::Mutex<usize>,
        pub chimes: parking_lot::Mutex<usize>,
        pub dictations_cancelled: parking_lot::Mutex<usize>,
        /// What `webview_version` gives.
        pub webview: Option<String>,
    }

    impl Fake {
        pub fn new() -> Arc<Self> {
            Arc::default()
        }
    }

    impl Host for Fake {
        fn version(&self) -> String {
            "0.0.0-test".into()
        }

        fn seed_dir(&self) -> Option<PathBuf> {
            None
        }

        fn webview_version(&self) -> Option<String> {
            self.webview.clone()
        }

        fn window_attended(&self) -> bool {
            false
        }

        fn attention(&self) {
            *self.attention.lock() += 1;
        }

        fn chime(&self) {
            *self.chimes.lock() += 1;
        }

        fn toast(&self, title: &str, body: &str, on_click: Box<dyn Fn() + Send + Sync>) {
            self.toasts.lock().push((title.into(), body.into()));
            self.clicks.lock().push(on_click);
        }

        fn waiting(&self, n: usize) {
            self.waiting.lock().push(n);
        }

        fn relabel(&self, lang: Lang, waiting: usize) {
            self.relabels.lock().push((lang, waiting));
        }

        fn stop_voice(&self) {}

        fn cancel_dictation(&self) {
            *self.dictations_cancelled.lock() += 1;
        }

        fn tune_conversation(&self, _: &Settings, _: &dyn SecretStore) {}
    }
}
