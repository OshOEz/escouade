//! The server's folders, browsed from a browser: « Nouveau projet » without the system's dialog (docs/REMOTE.md,
//! « Ce qui change de machine »). Folders only, under the home of the service's user; the private files (`fence`) and
//! the hidden folders left out; a link that leads out of the home neither shown nor followed.

use crate::fence::Fence;
use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

/// A folder and the folders in it.
#[derive(Debug, PartialEq, Serialize)]
pub struct Listing {
    /// Its real path (links followed).
    pub path: String,
    /// The folder above it; none for the home.
    pub parent: Option<String>,
    /// By name, whatever the case.
    pub entries: Vec<Folder>,
    /// More folders than `MAX_ENTRIES` in it: only the first found are listed.
    pub truncated: bool,
}

/// The most folders a listing holds.
pub const MAX_ENTRIES: usize = 2000;

/// A folder of a listing.
#[derive(Debug, PartialEq, Serialize)]
pub struct Folder {
    pub name: String,
    /// False: a folder that cannot be read, shown but not opened.
    pub dir: bool,
    /// A Git repository: `.git` in it (a folder, or a worktree's file).
    pub git: bool,
}

/// What the browser may see: the real home, and the real paths of the fence. Real paths by `dunce`: on Windows, a
/// plain `C:\…`, not the `\\?\` form a path written with `/` cannot extend.
pub(crate) struct Bounds {
    home: PathBuf,
    fenced: Vec<PathBuf>,
}

impl Bounds {
    pub(crate) fn of(home: &Path, fence: &Fence) -> Result<Self> {
        let home = dunce::canonicalize(home)?;
        // A home at the root would show the whole disk.
        if home.parent().is_none() {
            bail!(tr!(
                "le dossier personnel est la racine du disque : le serveur ne montre rien",
                "the home folder is the root of the disk: the server shows nothing"
            ));
        }
        Ok(Bounds {
            home,
            // Their real paths: a fence written through a link still holds.
            fenced: fence
                .paths()
                .iter()
                .map(|p| dunce::canonicalize(p).unwrap_or_else(|_| p.clone()))
                .collect(),
        })
    }

    /// Whether the real path `real` is a folder the browser may see.
    fn shows(&self, real: &Path) -> bool {
        real.starts_with(&self.home)
            && !self.fenced.iter().any(|f| real.starts_with(f))
            && real.is_dir()
    }

    /// The real path of `asked`, a folder it may see: absolute, without `..`, `\0` or a link that leads out.
    pub(crate) fn folder(&self, asked: &str) -> Result<PathBuf> {
        let refused = || {
            anyhow!(tr!(
                "« {asked} » n’est pas un dossier que le serveur montre",
                "“{asked}” is not a folder the server shows"
            ))
        };
        let path = Path::new(asked);
        if asked.contains('\0')
            || !path.is_absolute()
            || path.components().any(|c| c == Component::ParentDir)
        {
            return Err(refused());
        }
        let real = dunce::canonicalize(path).map_err(|_| refused())?;
        if !self.shows(&real) {
            return Err(refused());
        }
        Ok(real)
    }
}

impl crate::core::Core {
    /// `path`, a project's folder a client gave: as it is on the desktop; under escouade-server (`folders_home`), the
    /// real path of a folder its browser may see, or refused.
    pub fn client_folder(&self, path: &str) -> Result<String> {
        let Some(home) = self.folders_home.get() else {
            return Ok(path.into());
        };
        let real = Bounds::of(home, &self.project_fence(""))?.folder(path.trim())?;
        Ok(real.display().to_string())
    }

    /// `url`, a repository a client gave to clone (a plugin's): any as on the desktop; under escouade-server, one that
    /// `clone` takes (a path of the disk or `file://` in the folders its browser may see, never `x::`).
    pub fn client_repository(&self, url: &str) -> Result<()> {
        let Some(home) = self.folders_home.get() else {
            return Ok(());
        };
        source(&Bounds::of(home, &self.project_fence(""))?, url.trim()).map(drop)
    }
}

/// The folders of `path` (the home when none), which must be one the browser may see.
pub fn list(home: &Path, fence: &Fence, path: Option<&str>) -> Result<Listing> {
    let b = Bounds::of(home, fence)?;
    let real = match path {
        Some(asked) => b.folder(asked)?,
        None => b.home.clone(),
    };
    let (mut entries, mut truncated) = (Vec::new(), false);
    for entry in std::fs::read_dir(&real)?.flatten() {
        // A name that is not UTF-8 could not be asked for again.
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Ok(target) = dunce::canonicalize(entry.path()) else {
            continue;
        };
        if !b.shows(&target) {
            continue;
        }
        if entries.len() == MAX_ENTRIES {
            truncated = true;
            break;
        }
        entries.push(Folder {
            dir: std::fs::read_dir(&target).is_ok(),
            git: target.join(".git").exists(),
            name,
        });
    }
    entries.sort_by_cached_key(|f| (f.name.to_lowercase(), f.name.clone()));
    let parent = (real != b.home)
        .then(|| real.parent().map(|p| p.display().to_string()))
        .flatten();
    Ok(Listing {
        path: real.display().to_string(),
        parent,
        entries,
        truncated,
    })
}

/// The folder a clone of `url` takes: the last word of its address, without `.git` (as `git clone` names it); none
/// for a name that is no folder's.
pub fn repo_name(url: &str) -> Option<String> {
    let last = url
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\', ':'])
        .next()?;
    let name = last.strip_suffix(".git").unwrap_or(last);
    (!name.is_empty() && !name.starts_with(['.', '-']) && !name.contains('\0')).then(|| name.into())
}

/// What `git clone` is given for `url`: a remote address as it is (`https://`, `ssh://`, `git://`, `user@host:path`);
/// a path of the disk, or `file://`, only as the real path of a folder the browser may see; a remote helper (`x::…`)
/// or a relative path never.
fn source(b: &Bounds, url: &str) -> Result<String> {
    let refused = || {
        anyhow!(tr!(
            "« {url} » n’est pas un dépôt que le serveur clone",
            "“{url}” is not a repository the server clones"
        ))
    };
    if url.contains("::") || url.starts_with('-') || url.contains('\0') {
        return Err(refused());
    }
    let local = match url.split_once("://") {
        Some(("file", path)) => path,
        Some(("https" | "http" | "ssh" | "git", _)) => return Ok(url.into()),
        Some(_) => return Err(refused()),
        None if Path::new(url).is_absolute() => url,
        // `host:path`, as scp writes it: a colon before any slash.
        None if url.find(':').is_some_and(|c| !url[..c].contains('/')) => return Ok(url.into()),
        None => return Err(refused()),
    };
    let real = b.folder(local).map_err(|_| refused())?;
    Ok(real.display().to_string())
}

/// `url` cloned into `into`, a folder the browser may see, as a folder of its own (`repo_name`); its path. A clone
/// that fails leaves nothing.
pub async fn clone(home: &Path, fence: &Fence, url: &str, into: &str) -> Result<String> {
    let b = Bounds::of(home, fence)?;
    let dir = b.folder(into)?;
    let source = source(&b, url)?;
    let Some(name) = repo_name(url) else {
        bail!(tr!(
            "« {url} » ne donne pas de nom de dossier",
            "“{url}” gives no folder name"
        ));
    };
    let dest = dir.join(&name);
    // Made here, empty, or refused: what is removed after a failure is only what this clone made.
    if std::fs::create_dir(&dest).is_err() {
        bail!(tr!(
            "« {name} » existe déjà dans ce dossier",
            "“{name}” already exists in this folder"
        ));
    }
    let args = [
        "-c",
        "protocol.ext.allow=never",
        "clone",
        "--",
        &source,
        &name,
    ];
    // Nobody at the server to answer a prompt: as a background command, within the time of one the user waits for.
    let cmd = crate::git::command(&dir.to_string_lossy(), &args);
    if let Err(e) =
        crate::git::run_net_command_within(cmd, &args, true, crate::git::NET_TIMEOUT).await
    {
        // Not there before: what is there now is the clone's. Once more a moment later: git's children, killed with
        // it past the time limit, may still have been writing.
        if std::fs::remove_dir_all(&dest).is_err() || dest.exists() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let _ = std::fs::remove_dir_all(&dest);
        }
        return Err(e);
    }
    Ok(dest.display().to_string())
}
