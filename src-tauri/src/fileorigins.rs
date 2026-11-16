//! The paths of the files the page hands the window, as the host read them (`fileorigin`, ESC-95):
//! the table the window's listener fills and the core takes from (`Core::file_origins`).

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::{timeout_at, Instant};

/// How many paths wait to be asked for, and how many that were asked for wait to be joined by: the
/// oldest leave first, in each table. A path is asked for, then joined by, within moments of its
/// arrival, so only a page that never goes on (a file dropped, its drop gone) leaves some.
const MAX_ENTRIES: usize = 256;

/// The paths the window sent, by the id the page gave each: managed by the app, and held by the
/// core (`Core::file_origins`), which takes the ones that were handed out.
#[derive(Clone, Default)]
pub struct FileOrigins(Arc<Shared>);

#[derive(Default)]
struct Shared {
    /// What the host read, not asked for yet. Oldest first.
    paths: Mutex<VecDeque<(String, String)>>,
    /// What `file_origin` answered, kept for the one file that is joined by it. Oldest first.
    handed: Mutex<VecDeque<(String, String)>>,
    /// Woken when a path arrives.
    arrived: Notify,
}

/// `path` for `id`, the newest of `table`, which holds `MAX_ENTRIES` at most: a second one for the
/// same id replaces the first.
fn put(table: &mut VecDeque<(String, String)>, id: String, path: String) {
    table.retain(|(known, _)| *known != id);
    table.push_back((id, path));
    while table.len() > MAX_ENTRIES {
        table.pop_front();
    }
}

/// The path of `id` in `table`, which forgets it.
fn pull(table: &mut VecDeque<(String, String)>, id: &str) -> Option<String> {
    let at = table.iter().position(|(known, _)| known == id)?;
    table.remove(at).map(|(_, path)| path)
}

impl FileOrigins {
    /// Keeps `path` for `id` (a second one for the same id replaces the first) and wakes whoever waits.
    /// Only the WebView2 listener (Windows) calls it; the tests call it everywhere.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn deposit(&self, id: String, path: String) {
        put(&mut self.0.paths.lock(), id, path);
        self.0.arrived.notify_waiters();
    }

    /// The path for `id`, if it arrived: it is forgotten as it is given.
    pub(crate) fn take(&self, id: &str) -> Option<String> {
        pull(&mut self.0.paths.lock(), id)
    }

    /// `take`, and the path is kept for `take_handed`: it is the page that was told it.
    pub(crate) fn hand(&self, id: &str) -> Option<String> {
        let path = self.take(id)?;
        put(&mut self.0.handed.lock(), id.to_string(), path.clone());
        Some(path)
    }

    /// The path `file_origin` answered for `id`, forgotten as it is given: a file is joined by it
    /// once. None for an id that was never answered (a path only the host read, or none at all).
    pub(crate) fn take_handed(&self, id: &str) -> Option<String> {
        pull(&mut self.0.handed.lock(), id)
    }

    /// The path for `id` (`hand`): the one already there, or the first to arrive within `delay`.
    pub(crate) async fn wait(&self, id: &str, delay: Duration) -> Option<String> {
        let deadline = Instant::now() + delay;
        loop {
            // Made before the look: a path that arrives between the two wakes it all the same.
            let arrived = self.0.arrived.notified();
            if let Some(path) = self.hand(id) {
                return Some(path);
            }
            if timeout_at(deadline, arrived).await.is_err() {
                // A path may have come as the delay ended.
                return self.hand(id);
            }
        }
    }
}

#[cfg(test)]
#[path = "fileorigins_tests.rs"]
mod tests;
