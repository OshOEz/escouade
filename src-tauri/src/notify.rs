//! Visual and audible notifications: chime, system notification (Windows toast, macOS
//! Notification Center), taskbar flash / Dock bounce, tray badge.

use std::f64::consts::PI;
use std::sync::OnceLock;
use tauri::image::Image;
use tauri::{AppHandle, Manager, Runtime};

static CHIME: OnceLock<Vec<u8>> = OnceLock::new();

/// The design's two-note chime (880 Hz then 1318.5 Hz), rendered once as a 16-bit WAV.
fn chime_wav() -> &'static [u8] {
    CHIME.get_or_init(|| {
        let rate = 44_100u32;
        let total = (rate as f64 * 0.8) as usize;
        let mut samples = vec![0f64; total];
        for (i, freq) in [880.0f64, 1318.5].iter().enumerate() {
            let offset = (i as f64 * 0.13 * rate as f64) as usize;
            let len = (0.65 * rate as f64) as usize;
            for k in 0..len {
                let t = k as f64 / rate as f64;
                let env = if t < 0.02 {
                    t / 0.02 * 0.28
                } else {
                    0.28 * (0.0004f64 / 0.28).powf((t - 0.02) / 0.58)
                };
                if let Some(s) = samples.get_mut(offset + k) {
                    *s += env * (2.0 * PI * freq * t).sin();
                }
            }
        }
        let data: Vec<u8> = samples
            .iter()
            .flat_map(|s| ((s.clamp(-1.0, 1.0) * i16::MAX as f64) as i16).to_le_bytes())
            .collect();
        let mut wav = Vec::with_capacity(44 + data.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&(rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        wav
    })
}

pub fn play_chime() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
        let wav = chime_wav();
        // SAFETY: the WAV buffer is 'static, so it outlives the asynchronous playback.
        unsafe {
            PlaySoundW(
                wav.as_ptr() as *const u16,
                std::ptr::null_mut(),
                SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
            );
        }
    }
    #[cfg(unix)]
    {
        // afplay (macOS) and paplay (Linux) play files only: the WAV is written once to the
        // temporary folder.
        let file = std::env::temp_dir().join("escouade-chime.wav");
        let wav = chime_wav();
        let ready = std::fs::metadata(&file).is_ok_and(|m| m.len() == wav.len() as u64)
            || std::fs::write(&file, wav).is_ok();
        if ready {
            let player = if cfg!(target_os = "macos") {
                "/usr/bin/afplay"
            } else {
                "paplay"
            };
            let _ = std::process::Command::new(player)
                .arg(&file)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}

pub fn window_attended<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.get_webview_window("main")
        .map(|w| {
            w.is_visible().unwrap_or(false)
                && w.is_focused().unwrap_or(false)
                && !w.is_minimized().unwrap_or(false)
        })
        .unwrap_or(false)
}

pub fn flash<R: Runtime>(app: &AppHandle<R>) {
    if crate::paths::hidden_window() {
        return;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }
}

/// Brings the window forward, shown and focused. Not a window kept out of sight
/// (`paths::hidden_window`): it stays where it is, never taking the focus from the user's app.
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if crate::paths::hidden_window() {
        return;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// System notification. On Windows, `on_click` runs when the user activates the toast; macOS
/// notifications only bring the app forward (the Dock icon shows its window again).
pub fn toast<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: &str,
    on_click: impl Fn() + Send + 'static,
) {
    #[cfg(windows)]
    {
        use tauri_winrt_notification::{Duration, Toast};
        let app_id = if cfg!(debug_assertions) {
            Toast::POWERSHELL_APP_ID.to_string()
        } else {
            app.config().identifier.clone()
        };
        let res = Toast::new(&app_id)
            .title(title)
            .text1(body)
            .sound(None)
            .duration(Duration::Short)
            .on_activated(move |_| {
                on_click();
                Ok(())
            })
            .show();
        if let Err(e) = res {
            log::warn!("toast failed: {e:?}");
        }
    }
    #[cfg(target_os = "macos")]
    {
        use tauri_plugin_notification::NotificationExt;
        let _ = on_click;
        if let Err(e) = app.notification().builder().title(title).body(body).show() {
            log::warn!("notification failed: {e:?}");
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = (app, title, body, on_click);
}

/// System notification with a button, shown long. On Windows, `on_click` runs when the user
/// activates the toast, told whether it was by its button; macOS notifications have no button
/// (the window offers the same choice).
pub fn toast_with_button<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: &str,
    button: &str,
    on_click: impl Fn(bool) + Send + 'static,
) {
    #[cfg(windows)]
    {
        use tauri_winrt_notification::{Duration, Toast};
        let app_id = if cfg!(debug_assertions) {
            Toast::POWERSHELL_APP_ID.to_string()
        } else {
            app.config().identifier.clone()
        };
        let res = Toast::new(&app_id)
            .title(title)
            .text1(body)
            .sound(None)
            .duration(Duration::Long)
            .add_button(button, "button")
            .on_activated(move |action| {
                on_click(action.as_deref() == Some("button"));
                Ok(())
            })
            .show();
        if let Err(e) = res {
            log::warn!("toast failed: {e:?}");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = button;
        toast(app, title, body, move || on_click(false));
    }
}

/// Tray icon with an amber dot in the corner when agents are waiting.
pub fn tray_icon<R: Runtime>(app: &AppHandle<R>, waiting: usize) -> Option<Image<'static>> {
    let base = app.default_window_icon()?;
    let (w, h) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();
    if waiting > 0 {
        let r = (w.min(h) as f64 * 0.24).max(3.0);
        let (cx, cy) = (w as f64 - r - 0.5, r + 0.5);
        for y in 0..h {
            for x in 0..w {
                let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
                let i = ((y * w + x) * 4) as usize;
                if d <= r {
                    rgba[i..i + 4].copy_from_slice(&[245, 196, 76, 255]);
                } else if d <= r + 1.5 {
                    rgba[i..i + 4].copy_from_slice(&[27, 25, 23, 255]);
                }
            }
        }
    }
    Some(Image::new_owned(rgba, w, h))
}

/// Where a window kept out of sight goes across: right of every screen (`right_edges`, in
/// physical pixels).
fn out_of_sight_x(right_edges: &[i32]) -> i32 {
    // No screen known: farther right than any desktop reaches.
    right_edges.iter().copied().max().unwrap_or(16_000).max(0) + 200
}

/// Shows the main window out of sight (`paths::hidden_window`), at its usual size (some tests
/// depend on it): right of every screen, with no button in the taskbar, and without taking the
/// focus (`show` would take it). Windows only, as the end-to-end tests.
pub fn show_out_of_sight<R: Runtime>(w: &tauri::WebviewWindow<R>) {
    let edges: Vec<i32> = w
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| m.position().x + m.size().width as i32)
        .collect();
    let _ = w.set_skip_taskbar(true);
    let _ = w.set_position(tauri::PhysicalPosition::new(out_of_sight_x(&edges), 0));
    #[cfg(windows)]
    if let Ok(hwnd) = w.hwnd() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
        // SAFETY: the window's own handle, on the thread the app is set up on.
        unsafe { ShowWindow(hwnd.0 as _, SW_SHOWNOACTIVATE) };
    }
    // The shell gives a window its button as it shows.
    let _ = w.set_skip_taskbar(true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_window_out_of_sight_goes_right_of_every_screen() {
        assert_eq!(out_of_sight_x(&[1920, 4480, -1280]), 4680);
        assert_eq!(out_of_sight_x(&[-1920]), 200);
        assert_eq!(out_of_sight_x(&[]), 16_200);
    }

    #[test]
    fn chime_is_a_valid_wav() {
        let wav = super::chime_wav();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert!(wav.len() > 60_000);
    }
}
