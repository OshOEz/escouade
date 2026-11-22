//! One `escouade-server` per data folder: its `server.lock` held by an advisory lock of the system (`flock`; elsewhere
//! the file opened without sharing it), let go by the system when the process ends, however it ends. Two servers on the
//! same folder would each write `devices.json` from what they keep in memory: one could bring back a device the other
//! revoked.

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// The file, in the data folder, that the server holds while it runs: its pid inside, for the next one's message.
pub const LOCK: &str = "server.lock";

/// The data folder held, until dropped or the process ends.
#[derive(Debug)]
pub struct Lock {
    _file: File,
}

impl Lock {
    /// `dir` held by this process; `Err` says who holds it already, or why it cannot be.
    pub fn take(dir: &Path) -> Result<Lock, String> {
        let path = dir.join(LOCK);
        let cannot = |e: std::io::Error| format!("cannot lock {}: {e}", path.display());
        std::fs::create_dir_all(dir).map_err(cannot)?;
        let Some(mut file) = acquire(&path).map_err(cannot)? else {
            let pid = std::fs::read_to_string(&path).unwrap_or_default();
            let pid = pid.trim();
            let pid = if pid.is_empty() { "?" } else { pid };
            return Err(format!(
                "another escouade-server (pid {pid}) already serves {}: one server per data folder",
                dir.display()
            ));
        };
        // Said to the next one; the lock holds without it.
        let _ = file
            .set_len(0)
            .and_then(|()| write!(file, "{}", std::process::id()));
        Ok(Lock { _file: file })
    }
}

/// The file, ours; `None` when another process holds it.
#[cfg(unix)]
pub(crate) fn acquire(path: &Path) -> std::io::Result<Option<File>> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    // SAFETY: flock on a descriptor the file owns for as long as the call.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(Some(file));
    }
    let e = std::io::Error::last_os_error();
    match e.raw_os_error() {
        Some(libc::EWOULDBLOCK) => Ok(None),
        _ => Err(e),
    }
}

/// Opened for writing, others allowed to read only: the next server's open fails while this one lives.
#[cfg(not(unix))]
fn acquire(path: &Path) -> std::io::Result<Option<File>> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    const ERROR_SHARING_VIOLATION: i32 = 32;
    let opened = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(FILE_SHARE_READ)
        .open(path);
    match opened {
        Ok(file) => Ok(Some(file)),
        Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
#[path = "lockfile_tests.rs"]
mod tests;
