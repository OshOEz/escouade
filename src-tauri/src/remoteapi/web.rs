//! The files of the web build (`escouade-server --web <folder>`), served as they are: no listing of a folder, no path
//! that leaves it. They hold nothing of the core, so they are served without the token.
use std::path::Path;

/// A file of the build, and how it is served.
pub struct File {
    pub body: Vec<u8>,
    pub mime: &'static str,
    pub cache: &'static str,
}

/// The file `path` (the request's path, still percent-encoded) of the build in `root`; `/` is `index.html`. Only plain
/// names are taken (letters, digits, `.`, `_`, `-`, never a leading dot): an encoded or relative path is none.
pub fn read(root: &Path, path: &str) -> Option<File> {
    let rel = path.strip_prefix('/')?;
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let plain = |segment: &str| {
        !segment.is_empty()
            && !segment.starts_with('.')
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    };
    if !rel.split('/').all(plain) {
        return None;
    }
    // A link inside the build that points out of it is not followed.
    let root = root.canonicalize().ok()?;
    let file = root.join(rel).canonicalize().ok()?;
    if !file.starts_with(&root) || !file.is_file() {
        return None;
    }
    let body = std::fs::read(&file).ok()?;
    let ext = rel.rsplit_once('.').map_or("", |(_, ext)| ext);
    // Vite names what it builds in `assets/` after its content: kept for good; the rest asked again each time.
    let cache = if rel.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    Some(File {
        body,
        mime: mime(ext),
        cache,
    })
}

fn mime(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_paths_inside_the_build_are_read() {
        let dir = crate::paths::test_dir("remote-web");
        let root = dir.join("web");
        std::fs::create_dir_all(root.join("assets")).unwrap();
        std::fs::write(root.join("index.html"), "page").unwrap();
        std::fs::write(root.join("assets/a-1.wasm"), "wasm").unwrap();
        std::fs::write(root.join(".env"), "hidden").unwrap();
        std::fs::write(dir.join("secret.txt"), "secret").unwrap();
        assert_eq!(read(&root, "/").unwrap().body, b"page");
        let wasm = read(&root, "/assets/a-1.wasm").unwrap();
        assert_eq!(wasm.mime, "application/wasm");
        assert!(wasm.cache.contains("immutable"));
        for path in [
            "",
            "/assets",
            "/.env",
            "/../secret.txt",
            "/%2e%2e/secret.txt",
            "/assets//a-1.wasm",
            "/assets/a-1.wasm?x",
        ] {
            assert!(read(&root, path).is_none(), "{path}");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("secret.txt"), root.join("out.txt")).unwrap();
            assert!(read(&root, "/out.txt").is_none());
        }
    }
}
