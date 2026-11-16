//! The desktop app: Tauri's builder, the window and the tray, what `run` starts and stops, and the probe of the
//! launches' pages. Behind the feature `desktop`, as every module of the window (docs/CRATES.md, § 16).

use super::*;
use crate::core::Core;
use crate::fileorigins::FileOrigins;
use crate::host::Host;
use crate::i18n::Lang;
use crate::integrations::secrets::SecretStore;
use crate::model::Settings;
use escouade_model::agentgroups;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_window_state::StateFlags;

/// The tray icon, its menu in the interface's language (written again when it changes:
/// `menus::relabel`).
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let lang = i18n::ui();
    let menu = menus::tray_menu(app.handle(), lang)?;
    let mut builder = TrayIconBuilder::with_id(menus::TRAY_ID)
        .tooltip(menus::tray_tooltip(lang, 0))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => notify::show_main(app),
            "reload" => navigation::reload(app),
            "quit" => {
                let core = app.state::<Arc<Core>>();
                if core.ask_before_quit() {
                    notify::show_main(app);
                } else {
                    updates::quit(app);
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                notify::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// The window's theme for « Thème »: none is the system's.
fn window_theme(choice: model::ThemeChoice) -> Option<tauri::Theme> {
    match choice {
        model::ThemeChoice::System => None,
        model::ThemeChoice::Dark => Some(tauri::Theme::Dark),
        model::ThemeChoice::Light => Some(tauri::Theme::Light),
    }
}

/// The main window, made from the config's (`create: false` there: the guard of `navigation` can
/// only be put on it as it is made), hidden until `setup` (in `run`) shows it.
fn build_window(app: &tauri::App) -> tauri::Result<()> {
    let config = app.config();
    let window = config
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .ok_or(tauri::Error::WindowNotFound)?;
    // Out of sight (`paths::hidden_window`), its WebView does not take the focus as it is made:
    // that would take it from the user's app, the window still hidden.
    let mut window = window.clone();
    window.focus &= !paths::hidden_window();
    // The size an end-to-end test gives it (a CI runner's screen): the zoom is held to the window's (ESC-440).
    if let Some((width, height)) = paths::window_size() {
        window.width = width;
        window.height = height;
    }
    // Built with Tauri’s `devtools` for the launches’ pages (ESC-311): the window’s own stay a
    // debug build’s.
    window.devtools = Some(cfg!(debug_assertions));
    // Only the dev server's address is the app's besides its own.
    let dev = if tauri::is_dev() {
        config.build.dev_url.clone()
    } else {
        None
    };
    app.manage(navigation::Home {
        url: navigation::home(dev.as_ref(), window.use_https_scheme),
        dev: dev.clone(),
    });
    let main = navigation::guard(
        WebviewWindowBuilder::from_config(app.handle(), &window)?,
        dev,
    )
    .build()?;
    // The paths of the files the page hands the window (a reload keeps the WebView, and the listener).
    let origins = fileorigin::FileOrigins::default();
    app.manage(origins.clone());
    fileorigin::install(&main, &origins);
    // A page whose process ends is loaded again, without what it showed (ESC-290).
    pagecrash::install(&main);
    Ok(())
}

/// The app's context (its config and its pages), made once: the window's and the probes'.
fn context() -> tauri::Context {
    tauri::generate_context!()
}

/// `devpage_probe` (examples): a launch's page in a real webview of this system (ESC-311), what it
/// could reach of Escouade, its storage, its devtools and its widths. Gives the exit code.
pub fn devpage_probe_cli(args: Vec<String>) -> i32 {
    give_tauri_our_runtime();
    devpage_probe::cli(args, context())
}

/// Tauri's tasks on the app's runtime: before Tauri makes its own, which `set` cannot replace.
fn give_tauri_our_runtime() {
    tauri::async_runtime::set(escouade_base::rt::handle());
}

pub fn run() {
    // First, before any thread: no child process may inherit the remote API's token. Debug builds only.
    #[cfg(debug_assertions)]
    let remote = remoteapi::server::take_from_env();
    // A debug build told `--sandbox-keychain …` takes back the keychain's entries of sandboxed runs, then stops: the
    // end-to-end fixture's, at its start and its end (ESC-584). Before anything, which would touch the user's folders.
    #[cfg(debug_assertions)]
    if let Some((out, code)) = escouade_forge::integrations::sandbox_keychain::command(
        &std::env::args().skip(1).collect::<Vec<_>>(),
    ) {
        println!("{out}");
        std::process::exit(code);
    }
    // Before GTK starts: the window's app_id and WM_CLASS, which find its icon (ESC-239).
    #[cfg(target_os = "linux")]
    desktop::name_the_program();
    #[cfg(unix)]
    shellenv::adopt_login_path();
    // Started by a process whose PATH is older than the user's (ESC-186): today's, before any thread.
    #[cfg(windows)]
    syspath::refresh();
    paths::migrate_app_folders();
    // What the last stop for an update left: the window comes back as it was then.
    let after_update = updates::take_note(
        &paths::DataDir::new(paths::default_data_dir()),
        env!("CARGO_PKG_VERSION"),
    );
    let window = updates::AfterUpdate::window(&after_update);
    // The commands, away from the window's thread; what runs on their lanes, for its watch (ESC-290).
    let running = Arc::new(windowthread::Running::default());
    let lanes = windowthread::Lanes::new(running.clone());
    let watched = running.clone();
    let focused = running.clone();
    give_tauri_our_runtime();
    let mut builder = tauri::Builder::default();
    // Sandboxed runs (end-to-end tests, demos) must not hand over to an instance the user
    // already has open.
    // Nor take the size and place of the user's window, or save their own over it (the file is the
    // app's, not the data folder's): a sandboxed window opens at the size of tauri.conf.json, as
    // big as the screen allows, whatever window the user last had.
    if paths::sandbox_dir().is_none() {
        builder = builder
            // A second launch hands its link over to this one (`escouade://plugin/<id>`, STORE-11).
            .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
                notify::show_main(app);
                deeplink::deliver(app, deeplink::links_in(args));
            }))
            .plugin(
                tauri_plugin_window_state::Builder::default()
                    .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                    .build(),
            );
    }
    let builder = builder
        // `escouade://`, registered by the installers (STORE-11); macOS hands a link over by an
        // Apple event, which this plugin receives.
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_plugin_notification::init());
    // Restarted by itself behind another app, it does not take the focus from it.
    #[cfg(target_os = "macos")]
    let builder = builder.activate_ignoring_other_apps(window == updates::Window::Front);
    builder
        .setup(move |app| {
            init_logging();
            windowthread::watch_app(app.handle().clone(), watched.clone());
            // First: nothing below may find the window missing.
            build_window(app)?;
            for note in paths::take_migration_notes() {
                log::info!("migration: {note}");
            }
            log::info!("Escouade {} starting", app.package_info().version);
            if let Some(updates::AfterUpdate::Failed(version)) = &after_update {
                log::warn!("Escouade {version} did not install: the app still runs this version");
            }
            app.manage(deeplink::Inbox::default());
            // The pages of the launches (ESC-311).
            app.manage(Arc::new(devpage::Pages::default()));
            // The process groups of agents a killed launch left behind (macOS, Linux: ESC-417),
            // stopped before any agent starts; this launch's are noted from now on.
            agentgroups::start(
                paths::DataDir::new(paths::default_data_dir())
                    .root()
                    .join("agent-groups.json"),
            );
            let (core, git_rx) = Core::load(
                Arc::new(Desktop(app.handle().clone())),
                paths::DataDir::new(paths::default_data_dir()),
            );
            app.manage(core.clone());
            // The dictation: its events to the window, as the core's.
            let hub = std::sync::Arc::downgrade(&core);
            app.manage(Arc::new(voice::Voice::for_app(
                core.data.root().to_path_buf(),
                Arc::new(move |e| {
                    if let Some(core) = hub.upgrade() {
                        core.hub.emit(model::UiEvent::Voice { voice: e });
                    }
                }),
            )));
            app.manage(updates::Updates::started(after_update));
            if !paths::hidden_window() {
                build_tray(app)?;
            }
            // In place of Tauri's default, in English. Should it fail, the default stays: its Edit
            // menu keeps copy and paste working in the WebView.
            #[cfg(target_os = "macos")]
            if let Err(e) = menus::app_menu(app.handle(), i18n::ui()).and_then(|m| app.set_menu(m))
            {
                log::error!("app menu not set: {e}");
            }
            core.start(git_rx);
            // Off unless both variables are set; a remote `subscribe` or `agent_terminal` takes the window's events or that terminal's output.
            #[cfg(debug_assertions)]
            match remote {
                Ok(None) => {}
                Err(why) => log::warn!("remote API off: {why}"),
                Ok(Some(config)) => {
                    match remoteapi::server::start(
                        core.clone(),
                        remoteapi::table::entries(),
                        config,
                    ) {
                        Ok(running) => {
                            log::info!("remote API listening on {}", running.addr);
                            app.manage(running);
                        }
                        Err(e) => log::error!("remote API not started: {e}"),
                    }
                }
            }
            // The Principals of settings saved before 1.8.1 kept or dropped, once (ESC-128).
            let migrating = core.clone();
            escouade_base::rt::spawn(async move { migrating.migrate_accounts().await });
            // Finding a `claude` of WSL may start its distribution: done now and off the main
            // thread, which neither the window nor the board then waits for (`wsl::find` keeps it).
            let warm = core.clone();
            escouade_base::rt::spawn_blocking(move || warm.warm_claude());
            // What the harness of each account lists of its models, without a turn (ESC-124, ESC-186).
            core.read_all_account_models();
            // Claude Code updated by Escouade, apart from its agents (ESC-133).
            selfupdate::watch(core.clone());
            updates::watch(app.handle().clone());
            if let Some(w) = app.get_webview_window("main") {
                // « Zoom de l’interface », before the window shows: never drawn at 100 % first, nor
                // at a zoom it is too small for (the window holds it to its size from then on).
                let saved = core.settings.read().ui_zoom;
                let scale = w.scale_factor().unwrap_or(1.0);
                let zoom = match w.inner_size() {
                    Ok(s) if s.width > 0 && s.height > 0 => model::fit_zoom(
                        saved,
                        f64::from(s.width) / scale,
                        f64::from(s.height) / scale,
                    ),
                    _ => saved,
                };
                if let Err(e) = w.set_zoom(f64::from(zoom) / 100.0) {
                    log::warn!("zoom not set: {e}");
                }
                // « Thème », before the window shows: born in it (the title bar of macOS, the
                // corners and shadow of Windows 11, what shows before the page); the page follows
                // its changes (lib/appearance.ts).
                if let Err(e) = w.set_theme(window_theme(core.settings.read().theme)) {
                    log::warn!("theme not set: {e}");
                }
                if w.theme().is_ok_and(|t| t == tauri::Theme::Light) {
                    // The light frame, where the config has the dark one.
                    let _ =
                        w.set_background_color(Some(tauri::window::Color(0xf0, 0xf1, 0xf3, 0xff)));
                }
                match window {
                    _ if paths::hidden_window() => notify::show_out_of_sight(&w),
                    updates::Window::Front => {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                    updates::Window::Behind => {
                        let _ = w.show();
                    }
                    // Minimized when the app restarted for an update: minimized again, its button
                    // in the taskbar (hidden, it would only be found in the tray).
                    updates::Window::Minimized => {
                        let _ = w.show();
                        let _ = w.minimize();
                    }
                    // Closed to the tray when the app restarted for an update: it stays there.
                    updates::Window::Hidden => {}
                }
            }
            deeplink::at_start(app.handle());
            Ok(())
        })
        .on_window_event(move |window, event| {
            // Back to the window: a harness installed meanwhile (in a terminal) gets Escouade's server.
            if let WindowEvent::Focused(true) = event {
                if let Some(core) = window.try_state::<Arc<Core>>() {
                    let _running = focused.enter("window focused");
                    core.declare_on_focus();
                    // A repository added or removed in a project's folder meanwhile (ESC-169).
                    core.refresh_all_repos();
                    core.refresh_all_workflows();
                    // The pull requests of the tickets « En revue », read again at once (ESC-59).
                    core.window_front(true);
                }
            }
            if let WindowEvent::Focused(false) = event {
                if let Some(core) = window.try_state::<Arc<Core>>() {
                    core.window_front(false);
                }
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Closed to the tray: no one sees the dictation any more, it stops.
                if let Some(v) = window.try_state::<Arc<voice::Voice>>() {
                    v.cancel();
                }
                let core = window.state::<Arc<Core>>();
                if !core.quitting.load(Ordering::Acquire) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(windowthread::off_the_window(lanes, commands().0))
        .build(context())
        .expect("error while building the application")
        .run(|app, event| match event {
            tauri::RunEvent::Exit => {
                #[cfg(debug_assertions)]
                if let Some(remote) = app.try_state::<remoteapi::server::Running>() {
                    remote.stop();
                }
                if let Some(core) = app.try_state::<Arc<Core>>() {
                    if !core.quitting.load(Ordering::Acquire) {
                        // macOS: Cmd+Q, the app menu's « Quitter Escouade » (`menus::app_menu`)
                        // and the Dock's « Quitter » come here straight, with no exit request
                        // first: the update ready installs now.
                        #[cfg(target_os = "macos")]
                        {
                            if updates::install_at_exit(app) {
                                return;
                            }
                        }
                        core.shutdown();
                    }
                }
            }
            // A click on the Dock icon brings back the window closed to the menu bar.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => notify::show_main(app),
            _ => {}
        });
}

/// The host the core runs in: the desktop app, with Tauri (`host::Host`).
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

    fn webview_version(&self) -> Option<String> {
        tauri::webview_version().ok()
    }

    fn window_attended(&self) -> bool {
        notify::window_attended(&self.0)
    }

    fn attention(&self) {
        notify::flash(&self.0);
    }

    fn chime(&self) {
        notify::play_chime();
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
        // No voice managed: no dictation to cancel.
        if let Some(v) = self.0.try_state::<Arc<crate::voice::Voice>>() {
            v.cancel();
        }
    }

    fn tune_conversation(&self, settings: &Settings, secrets: &dyn SecretStore) {
        // No voice managed: no conversation to tune.
        if let Some(v) = self.0.try_state::<Arc<crate::voice::Voice>>() {
            v.conversation_tune(&crate::commands::conv_choice(settings, secrets, &v.catalog).0);
        }
    }
}
