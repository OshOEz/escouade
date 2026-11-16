//! The core's event flow, to the window and to the remote clients subscribed.

use crate::model::UiEvent;
use crate::sink::Sink;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::mem::Discriminant;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

/// Past this many events sent to the page in one second, the second is written in the log
/// (ESC-290): an avalanche keeps the page drawing, and the window answering nothing.
pub const LOUD: u32 = 200;
/// A loud second is written once in this long.
const LOUD_EVERY: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct Hub {
    subscribers: Mutex<Subscribers>,
    meter: Mutex<Meter>,
}

#[derive(Default)]
struct Subscribers {
    /// The window's: kept when a send fails, as before, until the next window takes its place.
    window: Option<Sink<UiEvent>>,
    /// The others', each while its `Subscription` lives and its sink takes what it is sent.
    others: Vec<(Weak<()>, Sink<UiEvent>)>,
}

/// A subscriber's place in the hub: dropped, it gets no more events (it leaves at the next one).
pub struct Subscription {
    _alive: Arc<()>,
}

/// The events of the current second, by kind.
#[derive(Default)]
pub struct Meter {
    since: Option<Instant>,
    count: u32,
    kinds: HashMap<Discriminant<UiEvent>, (String, u32)>,
    told: Option<Instant>,
}

impl Meter {
    /// Counts `event`, sent at `now`; the line for the log when the second before was loud.
    pub fn count(&mut self, event: &UiEvent, now: Instant) -> Option<String> {
        let mut line = None;
        let since = *self.since.get_or_insert(now);
        if now.duration_since(since) >= Duration::from_secs(1) {
            let quiet = self
                .told
                .is_some_and(|t| now.duration_since(t) < LOUD_EVERY);
            if self.count > LOUD && !quiet {
                let mut kinds: Vec<&(String, u32)> = self.kinds.values().collect();
                kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                let kinds: Vec<String> = kinds
                    .iter()
                    .take(5)
                    .map(|(name, n)| format!("{name} {n}"))
                    .collect();
                line = Some(format!(
                    "events to the page: {} in one second ({})",
                    self.count,
                    kinds.join(", ")
                ));
                self.told = Some(now);
            }
            self.since = Some(now);
            self.count = 0;
            self.kinds.clear();
        }
        self.count += 1;
        self.kinds
            .entry(std::mem::discriminant(event))
            .or_insert_with(|| (kind_of(event), 0))
            .1 += 1;
        line
    }
}

/// The name of the event's kind (`Git`, `Conv`…), read from the start of its `Debug`, which stops
/// there: never its content.
fn kind_of(event: &UiEvent) -> String {
    struct Name(String);
    impl std::fmt::Write for Name {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            for c in s.chars() {
                if !(c.is_ascii_alphanumeric() || c == '_') {
                    return Err(std::fmt::Error);
                }
                self.0.push(c);
            }
            Ok(())
        }
    }
    let mut name = Name(String::new());
    let _ = std::fmt::Write::write_fmt(&mut name, format_args!("{event:?}"));
    name.0
}

impl Hub {
    /// Adds a subscriber; it leaves when its `Subscription` is dropped or its sink says `Closed`.
    pub fn subscribe(&self, sink: Sink<UiEvent>) -> Subscription {
        let alive = Arc::new(());
        let mut subscribers = self.subscribers.lock();
        subscribers
            .others
            .retain(|(alive, _)| alive.strong_count() > 0);
        subscribers.others.push((Arc::downgrade(&alive), sink));
        Subscription { _alive: alive }
    }

    /// The window's own subscriber: a new one takes the place of the one before (a page that reloads).
    pub fn set_window(&self, sink: Sink<UiEvent>) {
        let _before = self.subscribers.lock().window.replace(sink);
    }

    /// To every subscriber, in one order.
    pub fn emit(&self, event: UiEvent) {
        if let Some(line) = self.meter.lock().count(&event, Instant::now()) {
            log::warn!("{line}");
        }
        // Sent under the lock, so all see one order: a Sink never waits, nor calls the hub back.
        let mut subscribers = self.subscribers.lock();
        subscribers
            .others
            .retain(|(alive, sink)| alive.strong_count() > 0 && sink.send(event.clone()).is_ok());
        if let Some(window) = &subscribers.window {
            // The window's sink logs why it failed (`Sink::from`).
            let _ = window.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::Closed;

    fn git() -> UiEvent {
        UiEvent::Git {
            project_id: "p1".into(),
            git: Default::default(),
        }
    }

    #[test]
    fn a_loud_second_is_written_with_its_kinds_and_no_content() {
        let mut m = Meter::default();
        let t0 = Instant::now();
        for _ in 0..250 {
            assert_eq!(m.count(&git(), t0), None);
        }
        let line = m.count(&git(), t0 + Duration::from_millis(1001)).unwrap();
        assert_eq!(line, "events to the page: 250 in one second (Git 250)");
        assert!(!line.contains("p1"));
        // Once in 30 s.
        for _ in 0..250 {
            m.count(&git(), t0 + Duration::from_millis(1500));
        }
        assert_eq!(m.count(&git(), t0 + Duration::from_millis(2600)), None);
        for _ in 0..250 {
            m.count(&git(), t0 + Duration::from_secs(32));
        }
        assert!(m.count(&git(), t0 + Duration::from_secs(34)).is_some());
    }

    #[test]
    fn the_hub_emits_to_its_sink() {
        let got = Arc::new(Mutex::new(Vec::new()));
        let to = got.clone();
        let hub = Hub::default();
        hub.set_window(Sink::new(move |e: UiEvent| {
            to.lock().push(serde_json::to_value(&e).unwrap());
            Ok(())
        }));
        hub.emit(git());
        assert_eq!(*got.lock(), [serde_json::to_value(git()).unwrap()]);
    }

    /// An event numbered `n`, which `number` reads back.
    fn numbered(n: u64) -> UiEvent {
        UiEvent::VectorsDownload { done: n, total: 0 }
    }

    fn number(e: &UiEvent) -> u64 {
        match e {
            UiEvent::VectorsDownload { done, .. } => *done,
            other => panic!("not numbered: {other:?}"),
        }
    }

    /// A sink that keeps the numbers it gets.
    fn recorder() -> (Sink<UiEvent>, Arc<Mutex<Vec<u64>>>) {
        let got = Arc::new(Mutex::new(Vec::new()));
        let to = got.clone();
        let sink = Sink::new(move |e: UiEvent| {
            to.lock().push(number(&e));
            Ok(())
        });
        (sink, got)
    }

    #[test]
    fn every_subscriber_gets_every_event_in_one_order() {
        let hub = Arc::new(Hub::default());
        let (window, w) = recorder();
        hub.set_window(window);
        let (a, ga) = recorder();
        let (b, gb) = recorder();
        let _subs = (hub.subscribe(a), hub.subscribe(b));
        let threads: Vec<_> = [0u64, 1_000]
            .into_iter()
            .map(|base| {
                let hub = hub.clone();
                std::thread::spawn(move || (base..base + 500).for_each(|n| hub.emit(numbered(n))))
            })
            .collect();
        threads.into_iter().for_each(|t| t.join().unwrap());
        let w = w.lock().clone();
        assert_eq!(w.len(), 1_000);
        // Each series in its order, and both interleaved the same way for every subscriber.
        for base in [0, 1_000] {
            let mine: Vec<u64> = w
                .iter()
                .copied()
                .filter(|n| (base..base + 500).contains(n))
                .collect();
            assert_eq!(mine, (base..base + 500).collect::<Vec<_>>());
        }
        assert_eq!(*ga.lock(), w);
        assert_eq!(*gb.lock(), w);
    }

    #[test]
    fn a_dropped_subscription_gets_nothing_more() {
        let hub = Hub::default();
        let (sink, got) = recorder();
        let sub = hub.subscribe(sink);
        hub.emit(numbered(1));
        drop(sub);
        hub.emit(numbered(2));
        assert_eq!(*got.lock(), [1]);
        assert_eq!(hub.subscribers.lock().others.len(), 0);
    }

    #[test]
    fn a_closed_sink_is_forgotten_at_its_first_failed_send() {
        let hub = Hub::default();
        let tries = Arc::new(Mutex::new(0));
        let counted = tries.clone();
        let _sub = hub.subscribe(Sink::new(move |_: UiEvent| {
            *counted.lock() += 1;
            Err(Closed)
        }));
        let (sink, got) = recorder();
        let _alive = hub.subscribe(sink);
        assert_eq!(hub.subscribers.lock().others.len(), 2);
        hub.emit(numbered(1));
        hub.emit(numbered(2));
        assert_eq!(*tries.lock(), 1);
        assert_eq!(hub.subscribers.lock().others.len(), 1);
        assert_eq!(*got.lock(), [1, 2]);
    }

    #[test]
    fn the_window_keeps_its_events_when_a_remote_client_subscribes() {
        let hub = Hub::default();
        let (window, w) = recorder();
        hub.set_window(window);
        let (remote, r) = recorder();
        let _sub = hub.subscribe(remote);
        hub.emit(numbered(1));
        assert_eq!(*w.lock(), [1]);
        assert_eq!(*r.lock(), [1]);
    }

    #[test]
    fn the_window_keeps_its_events_when_a_remote_client_leaves() {
        let hub = Hub::default();
        let (window, w) = recorder();
        hub.set_window(window);
        let (remote, r) = recorder();
        drop(hub.subscribe(remote));
        hub.emit(numbered(1));
        assert_eq!(*w.lock(), [1]);
        assert!(r.lock().is_empty());
    }

    #[test]
    fn a_window_that_reloads_takes_its_own_place_only() {
        let hub = Hub::default();
        let (old, o) = recorder();
        hub.set_window(old);
        let (remote, r) = recorder();
        let _sub = hub.subscribe(remote);
        hub.emit(numbered(1));
        let (new, n) = recorder();
        hub.set_window(new);
        hub.emit(numbered(2));
        assert_eq!(*o.lock(), [1]);
        assert_eq!(*n.lock(), [2]);
        assert_eq!(*r.lock(), [1, 2]);
    }

    #[test]
    fn a_quiet_second_is_not_written() {
        let mut m = Meter::default();
        let t0 = Instant::now();
        for _ in 0..LOUD {
            m.count(&git(), t0);
        }
        assert_eq!(m.count(&git(), t0 + Duration::from_secs(2)), None);
    }
}
