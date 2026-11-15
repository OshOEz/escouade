//! The app the core runs in: what the core asks of it, and the desktop app's answers.

use crate::fileorigin::FileOrigins;
use crate::i18n::{self, Lang};
use crate::integrations::secrets::SecretStore;
use crate::model::Settings;
use crate::{menus, notify};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};

/// What the core asks of the app it runs in: the desktop app answers with Tauri, a server without.
pub trait Host: Send + Sync + 'static {
    fn version(&self) -> String;
    /// The folder of the installer's plugin seed, when there is one.
    fn seed_dir(&self) -> Option<PathBuf>;
    fn file_origins(&self) -> FileOrigins {
        Default::default()
    }
    /// The main window is shown and has the focus.
    fn window_attended(&self) -> bool;
    fn attention(&self);
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

pub struct Desktop<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> Host for Desktop<R> {
    fn version(&self) -> String {
        self.0.package_info().version.to_string()
    }

    fn seed_dir(&self) -> Option<PathBuf> {
        self.0.path().resource_dir().ok().map(|d| d.join("seed"))
    }

    /// The table the window's listener fills, when the app made it (it does, before the core).
    fn file_origins(&self) -> FileOrigins {
        self.0
            .try_state::<FileOrigins>()
            .map(|s| s.inner().clone())
            .unwrap_or_default()
    }

    fn window_attended(&self) -> bool {
        notify::window_attended(&self.0)
    }

    fn attention(&self) {
        notify::flash(&self.0);
    }

    fn toast(&self, title: &str, body: &str, on_click: Box<dyn Fn() + Send + Sync>) {
        let app = self.0.clone();
        notify::toast(&self.0, title, body, move || {
            notify::show_main(&app);
            on_click();
        });
    }

    fn waiting(&self, n: usize) {
        if let Some(tray) = self.0.tray_by_id(menus::TRAY_ID) {
            let _ = tray.set_icon(notify::tray_icon(&self.0, n));
            let _ = tray.set_tooltip(Some(menus::tray_tooltip(i18n::ui(), n)));
        }
    }

    fn relabel(&self, lang: Lang, waiting: usize) {
        menus::relabel(&self.0, lang, waiting);
    }

    fn stop_voice(&self) {
        if let Some(v) = self.0.try_state::<Arc<crate::voice::Voice>>() {
            v.shutdown();
        }
    }

    fn cancel_dictation(&self) {
        if let Some(v) = self.0.try_state::<Arc<crate::voice::Voice>>() {
            v.cancel();
        }
    }

    fn tune_conversation(&self, settings: &Settings, secrets: &dyn SecretStore) {
        if let Some(v) = self.0.try_state::<Arc<crate::voice::Voice>>() {
            v.conversation_tune(&crate::commands::conv_choice(settings, secrets, &v.catalog).0);
        }
    }
}

#[cfg(test)]
pub use fake::Fake;

#[cfg(test)]
mod fake {
    use super::*;

    /// A host without a window, tray or seed, that keeps what it was asked.
    #[derive(Default)]
    pub struct Fake {
        pub toasts: parking_lot::Mutex<Vec<(String, String)>>,
        /// The toasts' `on_click`, in the same order.
        pub clicks: parking_lot::Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
        pub waiting: parking_lot::Mutex<Vec<usize>>,
        pub relabels: parking_lot::Mutex<Vec<(Lang, usize)>>,
        pub attention: parking_lot::Mutex<usize>,
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

        fn window_attended(&self) -> bool {
            false
        }

        fn attention(&self) {
            *self.attention.lock() += 1;
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

        fn cancel_dictation(&self) {}

        fn tune_conversation(&self, _: &Settings, _: &dyn SecretStore) {}
    }
}
