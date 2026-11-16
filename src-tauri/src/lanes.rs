//! The lanes the commands run on (ESC-290): one per command or family, the calls of one command, or of one family
//! (a terminal's writes, an attachment's chunks), in order; a slow one holds back none of the others. The window's
//! commands run on them away from its thread (`windowthread`), the remote API's calls too (`remoteapi::server`).

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A command whose part on its lane lasts more than this is written in the log.
pub const SLOW: Duration = Duration::from_millis(250);
/// The same command is written as slow once in this long; the times between are counted.
const SLOW_EVERY: Duration = Duration::from_secs(30);

/// The commands that share a lane: the calls of a family keep their order between them (the
/// keystrokes and sizes of a terminal, the chunks of an attachment, the dictation's steps).
const FAMILIES: &[(&str, &str)] = &[
    ("term_", "term"),
    ("attach_", "attach"),
    ("voice_", "voice"),
    ("conversation_", "voice"),
    ("speech_", "voice"),
    ("agent_folder_", "agent_folder"),
];

/// The lane of the command `name`: its family's, else its own.
pub fn lane_of(name: &str) -> &str {
    FAMILIES
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
        .map_or(name, |(_, lane)| lane)
}

pub type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Lane {
    queue: VecDeque<Job>,
    running: bool,
}

/// What runs now on the lanes, for the watch: the name and since when.
#[derive(Default)]
pub struct Running {
    next: Mutex<u64>,
    now: Mutex<HashMap<u64, (String, Instant)>>,
}

/// One command running, until dropped.
pub struct Entered<'a> {
    running: &'a Running,
    id: u64,
}

impl Drop for Entered<'_> {
    fn drop(&mut self) {
        self.running.now.lock().remove(&self.id);
    }
}

impl Running {
    pub fn enter(&self, name: &str) -> Entered<'_> {
        let id = {
            let mut next = self.next.lock();
            *next += 1;
            *next
        };
        self.now
            .lock()
            .insert(id, (name.to_string(), Instant::now()));
        Entered { running: self, id }
    }

    /// What runs, the longest first: « get_conversation 1200 ms, stats 300 ms ».
    pub fn told(&self) -> String {
        let mut now: Vec<(String, Duration)> = self
            .now
            .lock()
            .values()
            .map(|(name, at)| (name.clone(), at.elapsed()))
            .collect();
        now.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
        now.iter()
            .take(3)
            .map(|(name, d)| format!("{name} {} ms", d.as_millis()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The lanes the commands run on, away from the window's thread: one per command or family, each
/// running its calls in order on a thread of the blocking pool while it has some.
#[derive(Default)]
pub struct Lanes {
    lanes: Mutex<HashMap<String, Lane>>,
    pub running: Arc<Running>,
    slow: Mutex<HashMap<String, (Option<Instant>, u32)>>,
}

impl Lanes {
    pub fn new(running: Arc<Running>) -> Arc<Self> {
        Arc::new(Self {
            running,
            ..Default::default()
        })
    }

    /// Runs `job` on the lane `lane`, after the jobs it already holds.
    pub fn submit(self: &Arc<Self>, lane: &str, job: Job) {
        {
            let mut lanes = self.lanes.lock();
            let l = lanes.entry(lane.to_string()).or_default();
            l.queue.push_back(job);
            if l.running {
                return;
            }
            l.running = true;
        }
        let (me, lane) = (self.clone(), lane.to_string());
        escouade_base::rt::spawn_blocking(move || me.drain(&lane));
    }

    fn drain(&self, lane: &str) {
        loop {
            let job = {
                let mut lanes = self.lanes.lock();
                let Some(l) = lanes.get_mut(lane) else { return };
                match l.queue.pop_front() {
                    Some(job) => job,
                    None => {
                        lanes.remove(lane);
                        return;
                    }
                }
            };
            // A command that panics fails its call (Tauri's resolver, dropped, says so), not its lane.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
        }
    }

    /// The line the log gets for the command `name` that took `took`, called with `ids`, if it is
    /// slow and not already written in the last `SLOW_EVERY`.
    pub fn slow_line(&self, name: &str, took: Duration, ids: &str) -> Option<String> {
        if took <= SLOW {
            return None;
        }
        let mut slow = self.slow.lock();
        let (at, skipped) = slow.entry(name.to_string()).or_insert((None, 0));
        if at.is_some_and(|at| at.elapsed() < SLOW_EVERY) {
            *skipped += 1;
            return None;
        }
        let more = match std::mem::take(skipped) {
            0 => String::new(),
            n => format!(" ({n} more slow since the last line)"),
        };
        *at = Some(Instant::now());
        Some(format!(
            "slow command: {name} took {} ms{ids}{more}",
            took.as_millis()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn a_family_shares_a_lane_and_any_other_command_has_its_own() {
        assert_eq!(lane_of("term_write"), "term");
        assert_eq!(lane_of("term_resize"), "term");
        assert_eq!(lane_of("attach_chunk"), "attach");
        assert_eq!(lane_of("conversation_listen"), "voice");
        assert_eq!(lane_of("voice_start"), "voice");
        assert_eq!(lane_of("project_logo"), "project_logo");
        assert_eq!(lane_of("get_conversation"), "get_conversation");
    }

    #[test]
    fn a_command_over_250_ms_is_written_once_in_30_s_with_the_count_between() {
        let lanes = Lanes::new(Arc::default());
        assert_eq!(lanes.slow_line("stats", SLOW, ""), None);
        assert_eq!(
            lanes
                .slow_line("stats", Duration::from_millis(812), " (project p1)")
                .as_deref(),
            Some("slow command: stats took 812 ms (project p1)")
        );
        assert_eq!(lanes.slow_line("stats", Duration::from_secs(2), ""), None);
        assert_eq!(lanes.slow_line("stats", Duration::from_secs(2), ""), None);
        // Another command is its own.
        assert!(lanes
            .slow_line("get_conversation", Duration::from_secs(1), "")
            .is_some());
    }

    #[test]
    fn what_runs_is_told_the_longest_first_and_forgotten_once_done() {
        let running = Running::default();
        let a = running.enter("project_logo");
        std::thread::sleep(Duration::from_millis(30));
        {
            let _b = running.enter("stats");
            let told = running.told();
            assert!(told.starts_with("project_logo "), "{told}");
            assert!(told.contains(", stats "), "{told}");
        }
        assert!(!running.told().contains("stats"));
        drop(a);
        assert_eq!(running.told(), "");
    }

    /// The calls of one lane keep their order: a terminal's keystrokes reach it as typed.
    #[test]
    fn the_calls_of_a_lane_run_in_order() {
        let lanes = Lanes::new(Arc::default());
        let seen: Arc<Mutex<Vec<u32>>> = Arc::default();
        let (done, finished) = mpsc::channel();
        for i in 0..200u32 {
            let (seen, done) = (seen.clone(), done.clone());
            lanes.submit(
                "term",
                Box::new(move || {
                    if i % 50 == 0 {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    seen.lock().push(i);
                    if i == 199 {
                        done.send(()).unwrap();
                    }
                }),
            );
        }
        finished.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(*seen.lock(), (0..200).collect::<Vec<_>>());
    }
}
