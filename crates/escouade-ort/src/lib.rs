//! ONNX Runtime as the app links it, out of the voice for the local vectors to have it without the audio stack (cpal,
//! whisper.cpp; docs/REMOTE.md, « Le mode serveur »): `ort` itself, made ready once (`runtime`), where its library is
//! (`runtime_path`), and the two errors a checked model download ends with, shared by the voice and the vectors.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// ONNX Runtime, as this crate links it: the voice (`escouade-voice-local`) and the vectors run their models with it.
pub use ort;

/// ONNX Runtime is loaded from a library at run time, not linked in: Intel Macs (pyke builds none for them) and Linux
/// (pyke's needs glibc 2.38), or the `ort-dynamic` feature (to try that path on another system).
pub const DYNAMIC: bool = cfg!(any(
    feature = "ort-dynamic",
    target_os = "linux",
    all(target_os = "macos", target_arch = "x86_64")
));

/// Microsoft's library, in the app: `Contents/Frameworks` of the bundle on a Mac, `/usr/lib/escouade` of the .deb (and
/// `usr/lib/escouade` of the AppImage) beside `bin/escouade` under Linux, beside the program elsewhere;
/// `ESCOUADE_ORT_DYLIB` in its place.
pub fn runtime_path() -> PathBuf {
    if let Some(p) = std::env::var_os("ESCOUADE_ORT_DYLIB").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    if cfg!(target_os = "macos") {
        dir.join("../Frameworks/libonnxruntime.1.23.2.dylib")
    } else if cfg!(windows) {
        dir.join("onnxruntime.dll")
    } else {
        let bundled = dir.join("../lib/escouade/libonnxruntime.so.1.23.2");
        if bundled.exists() {
            bundled
        } else {
            dir.join("libonnxruntime.so.1.23.2")
        }
    }
}

/// ONNX Runtime made ready, once: loaded from its library where it is not linked in.
pub fn runtime() -> Result<()> {
    static READY: std::sync::OnceLock<std::result::Result<(), String>> = std::sync::OnceLock::new();
    READY
        .get_or_init(|| {
            #[cfg(any(
                feature = "ort-dynamic",
                target_os = "linux",
                all(target_os = "macos", target_arch = "x86_64")
            ))]
            {
                let path = runtime_path();
                let started = std::time::Instant::now();
                ort::init_from(&path)
                    .map_err(|e| format!("ONNX Runtime ({}): {e}", path.display()))?
                    .commit();
                log::info!(
                    "speech: ONNX Runtime loaded from {} in {:?}",
                    path.display(),
                    started.elapsed()
                );
            }
            Ok(())
        })
        .clone()
        .map_err(|e| anyhow::anyhow!(e))
}

/// A download called off by the user.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// What was downloaded is not the model: not kept.
#[derive(Debug)]
pub struct ChecksumMismatch;

impl std::fmt::Display for ChecksumMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("checksum mismatch")
    }
}

impl std::error::Error for ChecksumMismatch {}
