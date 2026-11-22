fn main() {
    seed();
    // Without the window (`desktop` off) there is nothing of Tauri's to build.
    #[cfg(feature = "desktop")]
    tauri();
}

/// The seed of the plugins, for `escouade-server` to embed (`$OUT_DIR/seed.rs`, included by src/bin/escouade-server.rs
/// alone): the files of `seed/` when a release (`store/scripts/seed.ts fetch`) or a build of development
/// (`scripts/seed-dev.sh`) filled it, none otherwise; and a stamp of them, the version and a hash, for the server to
/// write them again when it changes.
fn seed() {
    use std::fmt::Write;
    use std::path::{Path, PathBuf};
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=seed");
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // .DS_Store and the like are no part of it.
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }
    let root =
        Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("seed");
    let mut files = Vec::new();
    // Only the server's build embeds it (feature `server`, which builds escouade-server): the desktop's reads its own
    // among its resources.
    if std::env::var_os("CARGO_FEATURE_SERVER").is_some() {
        walk(&root, &mut files);
    }
    files.retain(|f| f != &root.join("README.md"));
    files.sort();
    // FNV-1a, 64 bits, over each path and its bytes.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut list = String::new();
    for file in &files {
        let rel = file.strip_prefix(&root).expect("under seed/");
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let bytes = std::fs::read(file).expect("a file of the seed");
        for b in rel.bytes().chain([0]).chain(bytes) {
            hash = (hash ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
        writeln!(
            list,
            "    ({rel:?}, include_bytes!({:?})),",
            file.display().to_string()
        )
        .unwrap();
    }
    let stamp = format!(
        "{} {} {hash:016x}",
        std::env::var("CARGO_PKG_VERSION").unwrap(),
        files.len()
    );
    let code = format!(
        "static SEED: &[(&str, &[u8])] = &[\n{list}];\nstatic SEED_STAMP: &str = {stamp:?};\n"
    );
    let out = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("seed.rs");
    std::fs::write(out, code).expect("seed.rs written");
}

#[cfg(feature = "desktop")]
fn tauri() {
    #[allow(unused_mut)] // only changed on Windows
    let mut attributes = tauri_build::Attributes::new();
    // Embed the Common-Controls v6 manifest in every binary, test harnesses included:
    // without it, test binaries that link the dialog/tray code fail to start on Windows
    // (STATUS_ENTRYPOINT_NOT_FOUND, TaskDialogIndirect).
    #[cfg(windows)]
    {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest = std::env::current_dir()
            .expect("cwd")
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        println!("cargo:rustc-link-arg=/WX");
    }
    tauri_build::try_build(attributes).expect("tauri build script");
}
