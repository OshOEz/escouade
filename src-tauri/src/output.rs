//! A terminal's output (a shell, a launch, an agent's interface): its last bytes kept, and whoever
//! reads it now, the window and the remote clients, each sent every byte once and in order.

use crate::sink::Sink;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::{Arc, Weak};

/// The most an output keeps (bytes): its last screens, for a reader that comes later.
pub(crate) const OUTPUT_KEPT: usize = 4 << 20;
/// The size of the pieces the kept output is sent to a reader in.
pub(crate) const REPLAY_PIECE: usize = 64 << 10;
/// Where the output is cut at the start when it is too long: after the first line end found in
/// this many bytes, else at the first escape, so that what is kept does not begin in the middle of
/// a sequence.
const CUT_LOOKS_IN: usize = 4096;

/// The output of a terminal: its last bytes kept, and whoever reads it now.
pub struct Output {
    max: usize,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    /// Grows as bytes come, up to `max`.
    kept: VecDeque<u8>,
    /// The window's reader: a page that reloads takes its place; kept when a send fails, as before.
    window: Option<Sink<Vec<u8>>>,
    /// The other readers, each while its `Reader` lives and its sink takes what it is sent.
    clients: Vec<(Weak<()>, Sink<Vec<u8>>)>,
}

/// A client's place among an output's readers: dropped, it is sent nothing more.
pub struct Reader {
    _alive: Arc<()>,
}

impl Reader {
    pub fn new() -> Self {
        Reader {
            _alive: Arc::new(()),
        }
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

/// Who reads an output: the window, or a client while its `Reader` lives.
pub struct Viewer {
    sink: Sink<Vec<u8>>,
    client: Option<Weak<()>>,
}

impl Viewer {
    /// The window's: it takes the place of the window's reader before it.
    pub fn window(sink: Sink<Vec<u8>>) -> Self {
        Viewer { sink, client: None }
    }

    /// A client's, as long as `reader` lives.
    pub fn client(sink: Sink<Vec<u8>>, reader: &Reader) -> Self {
        Viewer {
            sink,
            client: Some(Arc::downgrade(&reader._alive)),
        }
    }

    /// Sends `bytes` to it (tests).
    #[cfg(test)]
    pub(crate) fn send(&self, bytes: Vec<u8>) -> Result<(), crate::sink::Closed> {
        self.sink.send(bytes)
    }

    /// The window's, from a closure (tests).
    #[cfg(test)]
    pub fn window_fn(f: impl Fn(Vec<u8>) + Send + 'static) -> Self {
        let f = Mutex::new(f);
        Self::window(Sink::new(move |bytes| {
            (f.lock())(bytes);
            Ok(())
        }))
    }
}

impl Output {
    pub fn new(kept: usize) -> Arc<Self> {
        Arc::new(Output {
            max: kept,
            inner: Mutex::default(),
        })
    }

    /// Called by the terminal's reader, never waits: kept, then sent to every reader.
    pub fn push(&self, bytes: Vec<u8>) {
        let mut o = self.inner.lock();
        keep(&mut o.kept, &bytes, self.max);
        // Readers called under the lock, each byte once and in order: a Sink never waits nor calls back.
        o.clients
            .retain(|(alive, sink)| alive.strong_count() > 0 && sink.send(bytes.clone()).is_ok());
        if let Some(window) = &o.window {
            let _ = window.send(bytes);
        }
    }

    /// Replays what is kept from a safe point, then follows; the reader leaves when its handle drops (tests: the core
    /// takes a `Viewer`, whose `Reader` its caller made first).
    #[cfg(test)]
    pub fn attach(self: &Arc<Self>, sink: Sink<Vec<u8>>) -> Reader {
        let reader = Reader::new();
        self.read_by(Viewer::client(sink, &reader));
        reader
    }

    /// `viewer` reads it from now on, sent what is kept first.
    pub fn read_by(&self, viewer: Viewer) {
        self.read_by_after(Vec::new(), viewer);
    }

    /// The same, sent `first` before what is kept (an interface's modes), in the same step.
    pub(crate) fn read_by_after(&self, first: Vec<u8>, viewer: Viewer) {
        let mut o = self.inner.lock();
        let kept: Vec<u8> = o.kept.iter().copied().collect();
        let replay = without_terminal_queries(&kept);
        let pieces = (!first.is_empty())
            .then_some(first)
            .into_iter()
            .chain(replay.chunks(REPLAY_PIECE).map(<[u8]>::to_vec));
        for piece in pieces {
            // A client gone already is not added; the window's fails are not counted, as before.
            if viewer.sink.send(piece).is_err() && viewer.client.is_some() {
                return;
            }
        }
        match viewer.client {
            None => {
                let _before = o.window.replace(viewer.sink);
            }
            Some(alive) => {
                o.clients.retain(|(alive, _)| alive.strong_count() > 0);
                o.clients.push((alive, viewer.sink));
            }
        }
    }

    /// How many bytes it keeps.
    pub fn len(&self) -> usize {
        self.inner.lock().kept.len()
    }

    /// The bytes kept (tests).
    #[cfg(test)]
    pub(crate) fn kept(&self) -> Vec<u8> {
        self.inner.lock().kept.iter().copied().collect()
    }
}

/// `bytes` added to `kept`, which keeps its last `max` and, when its start was dropped, starts again
/// at a safe point: after a line end, else at an escape, else not inside a UTF-8 character.
fn keep(kept: &mut VecDeque<u8>, bytes: &[u8], max: usize) {
    let excess = (kept.len() + bytes.len()).saturating_sub(max);
    // Dropped before it is added, and grown by doubling up to `max` only: its capacity never passes `max`.
    let from_bytes = excess.saturating_sub(kept.len());
    kept.drain(..excess.min(kept.len()));
    let need = kept.len() + bytes.len() - from_bytes;
    if need > kept.capacity() {
        let grown = need.max(kept.capacity() * 2).min(max);
        kept.reserve_exact(grown - kept.len());
    }
    kept.extend(&bytes[from_bytes..]);
    if excess == 0 {
        return;
    }
    let head = kept.iter().take(CUT_LOOKS_IN);
    let safe = match head.clone().position(|b| *b == b'\n') {
        Some(line) => line + 1,
        None => match head.clone().position(|b| *b == ESC) {
            Some(esc) => esc,
            None => head
                .take(3)
                .take_while(|b| (0x80..0xc0).contains(*b))
                .count(),
        },
    };
    kept.drain(..safe);
}

pub(crate) const ESC: u8 = 0x1b;
pub(crate) const BEL: u8 = 0x07;
/// The longest OSC or DCS string looked into for a query (the longer ones are kept as they are).
pub(crate) const MAX_STRING: usize = 4096;

/// `bytes` less the requests a terminal answers (the answers go to the program as input): device
/// attributes (`CSI c`), device and cursor status reports (`CSI n`, `CSI ? n`), `CSI > q`, mode
/// requests (`CSI $ p`), the keyboard protocol's (`CSI ? u`), window reports (`CSI 14 t`, 16, 18,
/// 19), the OSC that ask a color or the clipboard (`?` for a value), and the DCS `$ q` and `+ q`.
/// All else stays byte for byte. A sequence cut short at the end (or started before what the
/// buffer begins with) is left as it is.
pub(crate) fn without_terminal_queries(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let Some(at) = bytes[i..].iter().position(|b| *b == ESC) else {
            out.extend_from_slice(&bytes[i..]);
            break;
        };
        out.extend_from_slice(&bytes[i..i + at]);
        i += at;
        match terminal_query_len(&bytes[i..]) {
            Some(n) => i += n,
            None => {
                out.push(ESC);
                i += 1;
            }
        }
    }
    out
}

/// The length of the query `s`, which begins with ESC, opens; None when it opens something else
/// or is not whole.
fn terminal_query_len(s: &[u8]) -> Option<usize> {
    match *s.get(1)? {
        b'[' => csi_query_len(s),
        b']' => {
            let (end, len) = string_end(s, true)?;
            let mut parts = s[2..end].split(|b| *b == b';');
            let colors_or_clipboard = matches!(parts.next()?, b"4" | b"10" | b"11" | b"12" | b"52");
            (colors_or_clipboard && parts.any(|p| p == b"?")).then_some(len)
        }
        b'P' => {
            let (end, len) = string_end(s, false)?;
            let payload = &s[2..end];
            (payload.starts_with(b"$q") || payload.starts_with(b"+q")).then_some(len)
        }
        _ => None,
    }
}

/// Where the OSC or DCS string `s` (ESC, then its opener) ends: the end of its payload and the
/// length of the whole, up to its terminator (BEL too, for an OSC; else ESC \). None when it has
/// none within `MAX_STRING`, or something else comes first.
fn string_end(s: &[u8], bel: bool) -> Option<(usize, usize)> {
    let limit = s.len().min(2 + MAX_STRING);
    for i in 2..limit {
        match s[i] {
            BEL if bel => return Some((i, i + 1)),
            ESC => return (s.get(i + 1) == Some(&b'\\')).then_some((i, i + 2)),
            _ => {}
        }
    }
    None
}

/// The length of the query the CSI `s` (ESC [ …) is, if it is one.
fn csi_query_len(s: &[u8]) -> Option<usize> {
    let mut i = 2;
    while s.get(i).is_some_and(|b| (0x30..=0x3f).contains(b)) {
        i += 1;
    }
    let params = &s[2..i];
    let from = i;
    while s.get(i).is_some_and(|b| (0x20..=0x2f).contains(b)) {
        i += 1;
    }
    let intermediates = &s[from..i];
    let last = *s.get(i)?;
    // `params` less the prefix (one of `prefixes`, or none), made of digits only.
    let digits_after = |prefixes: &[u8]| {
        let rest = match params.split_first() {
            Some((p, rest)) if prefixes.contains(p) => rest,
            _ => params,
        };
        rest.iter().all(u8::is_ascii_digit)
    };
    let query = match (intermediates, last) {
        // Device attributes: `CSI c`, `CSI 0 c`, `CSI > c`, `CSI > 0 c`, `CSI = c`.
        ([], b'c') => digits_after(b">="),
        // Status reports: `CSI 5 n`, `CSI 6 n`, `CSI ? 6 n`, `CSI ? 996 n`.
        ([], b'n') => !params.is_empty() && params != b"?" && digits_after(b"?"),
        // The terminal's name and version: `CSI > q`, `CSI > 0 q`.
        ([], b'q') => params.first() == Some(&b'>') && digits_after(b">"),
        // A mode's state: `CSI Ps $ p`, `CSI ? Ps $ p`.
        ([b'$'], b'p') => digits_after(b"?"),
        // The keyboard protocol's flags: `CSI ? u`.
        ([], b'u') => params == b"?",
        // Window reports: the size in pixels (14), of a cell (16), in characters (18), of the
        // screen (19).
        ([], b't') => {
            let first = params.split(|b| *b == b';').next().unwrap_or_default();
            params.iter().all(|b| b.is_ascii_digit() || *b == b';')
                && matches!(first, b"14" | b"16" | b"18" | b"19")
        }
        _ => false,
    };
    query.then_some(i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::Closed;

    /// A sink that keeps what it gets.
    fn collecting() -> (Sink<Vec<u8>>, Arc<Mutex<Vec<u8>>>) {
        let got = Arc::new(Mutex::new(Vec::new()));
        let to = got.clone();
        let sink = Sink::new(move |b: Vec<u8>| {
            to.lock().extend(b);
            Ok(())
        });
        (sink, got)
    }

    #[test]
    fn a_reader_gets_what_is_kept_then_what_follows() {
        let out = Output::new(1024);
        out.push(b"one ".to_vec());
        out.push(b"two ".to_vec());
        let (sink, got) = collecting();
        let _reader = out.attach(sink);
        assert_eq!(*got.lock(), b"one two ");
        out.push(b"three".to_vec());
        assert_eq!(*got.lock(), b"one two three");
    }

    #[test]
    fn a_reader_that_attaches_while_bytes_come_gets_each_once_in_order() {
        // Small enough that a replay is one piece (under `REPLAY_PIECE`).
        const N: u32 = 5_000;
        let out = Output::new(1 << 20);
        let writer = {
            let out = out.clone();
            std::thread::spawn(move || {
                for n in 0..N {
                    out.push(format!("<{n}>").into_bytes());
                    if n % 50 == 0 {
                        std::thread::sleep(std::time::Duration::from_micros(50));
                    }
                }
            })
        };
        // Each reader keeps its pieces: the first is its replay, the others what came live.
        let readers: Vec<_> = (0..8)
            .map(|_| {
                std::thread::sleep(std::time::Duration::from_millis(1));
                let pieces = Arc::new(Mutex::new(Vec::<String>::new()));
                let to = pieces.clone();
                let sink = Sink::new(move |b: Vec<u8>| {
                    to.lock().push(String::from_utf8(b).unwrap());
                    Ok(())
                });
                (out.attach(sink), pieces)
            })
            .collect();
        writer.join().unwrap();
        let numbers = |s: &str| -> Vec<u32> {
            s.split('<')
                .skip(1)
                .map(|p| p.trim_end_matches('>').parse().unwrap())
                .collect()
        };
        let mut mid_stream = 0;
        for (_reader, pieces) in readers {
            let pieces = pieces.lock().clone();
            // Replay then live: every number once, in order, none missing.
            assert_eq!(numbers(&pieces.concat()), (0..N).collect::<Vec<_>>());
            // It came while the writer wrote: a replay, then live pieces that start past 0.
            let replayed = numbers(&pieces[0]);
            if replayed.len() > 1 && *replayed.last().unwrap() < N - 1 {
                assert_eq!(numbers(&pieces[1])[0], replayed.last().unwrap() + 1);
                mid_stream += 1;
            }
        }
        assert!(mid_stream > 0, "no reader attached while the writer wrote");
    }

    #[test]
    fn two_readers_get_the_same_bytes() {
        let out = Output::new(1024);
        out.push(b"a".to_vec());
        let (one, first) = collecting();
        let _one = out.attach(one);
        out.push(b"b".to_vec());
        let (two, second) = collecting();
        let _two = out.attach(two);
        out.push(b"c".to_vec());
        assert_eq!(*first.lock(), b"abc");
        assert_eq!(*second.lock(), b"abc");
    }

    #[test]
    fn the_kept_bytes_stop_at_the_bound() {
        let out = Output::new(16);
        out.push(b"0123456789".to_vec());
        out.push(b"abcdefghij".to_vec());
        assert_eq!(out.kept(), b"456789abcdefghij");
        // A piece bigger than all of it: its end stays.
        out.push(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_vec());
        assert_eq!(out.kept(), b"KLMNOPQRSTUVWXYZ");
        // The buffer never grows past the bound.
        assert!(out.inner.lock().kept.capacity() < 64);
    }

    #[test]
    fn the_buffer_never_holds_room_past_its_bound() {
        let out = Output::new(1000);
        out.push(vec![b'a'; 50]);
        for _ in 0..300 {
            out.push(vec![b'b'; 7]);
            assert!(out.inner.lock().kept.capacity() <= 1000);
        }
        assert_eq!(out.len(), 1000);
        // A full-size buffer, read in 32 KiB pieces as a terminal is.
        let out = Output::new(OUTPUT_KEPT);
        out.push(vec![b'a'; 50]);
        for _ in 0..200 {
            out.push(vec![b'b'; 32 << 10]);
        }
        assert!(out.inner.lock().kept.capacity() <= OUTPUT_KEPT);
    }

    #[test]
    fn the_replay_starts_at_a_safe_point() {
        // Cut inside a CSI sequence, with a line after it: from the line.
        let out = Output::new(32);
        out.push(b"\x1b[38;5;2".to_vec());
        out.push(b"31mred\r\nline two of the screen\r\n".to_vec());
        assert!(out.kept().starts_with(b"line two"), "{:?}", out.kept());
        // No line: from the next escape.
        let out = Output::new(16);
        out.push(b"\x1b[38;5;2".to_vec());
        out.push(b"31mred\x1b[0mtext!".to_vec());
        assert_eq!(out.kept(), b"\x1b[0mtext!");
        // Neither: not inside a character of several bytes (é is c3 a9, € is e2 82 ac).
        let out = Output::new(7);
        out.push("xyzé€abc".as_bytes().to_vec());
        let kept = out.kept();
        assert!(std::str::from_utf8(&kept).is_ok(), "{kept:?}");
        assert_eq!(kept, "€abc".as_bytes());
        // Its readers get the same.
        let (sink, got) = collecting();
        let _reader = out.attach(sink);
        assert_eq!(*got.lock(), "€abc".as_bytes());
    }

    #[test]
    fn the_replay_leaves_out_the_terminals_queries() {
        let out = Output::new(1 << 20);
        // Answered already: a second answer would be typed into the program.
        out.push(b"a\x1b[6".to_vec());
        out.push(b"nb\x1b]11;?\x07c".to_vec());
        out.push(vec![b'd'; REPLAY_PIECE + 1]);
        let pieces = Arc::new(Mutex::new(Vec::new()));
        let all = Arc::new(Mutex::new(Vec::new()));
        let (sizes, bytes) = (pieces.clone(), all.clone());
        let _reader = out.attach(Sink::new(move |b: Vec<u8>| {
            sizes.lock().push(b.len());
            bytes.lock().extend(b);
            Ok(())
        }));
        let mut want = b"abc".to_vec();
        want.extend(vec![b'd'; REPLAY_PIECE + 1]);
        assert_eq!(*all.lock(), want);
        assert!(pieces.lock().iter().all(|n| *n <= REPLAY_PIECE));
        // The terminal's own copy is left as it was printed.
        assert!(out.kept().windows(4).any(|w| w == b"\x1b[6n"));
    }

    #[test]
    fn a_reader_that_leaves_or_fails_is_forgotten() {
        let out = Output::new(1024);
        let (sink, got) = collecting();
        let reader = out.attach(sink);
        let tries = Arc::new(Mutex::new(0));
        let counted = tries.clone();
        let _failing = out.attach(Sink::new(move |_| {
            *counted.lock() += 1;
            Err(Closed)
        }));
        out.push(b"a".to_vec());
        drop(reader);
        out.push(b"b".to_vec());
        assert_eq!(*got.lock(), b"a");
        assert_eq!(*tries.lock(), 1);
        assert!(out.inner.lock().clients.is_empty());
        // One that fails as it is replayed is not added.
        let _late = out.attach(Sink::new(|_| Err(Closed)));
        assert!(out.inner.lock().clients.is_empty());
    }

    #[test]
    fn a_window_takes_the_place_of_the_one_before_and_leaves_the_clients_theirs() {
        let out = Output::new(1024);
        let (client, c) = collecting();
        let _client = out.attach(client);
        let (old, o) = collecting();
        out.read_by(Viewer::window(old));
        out.push(b"a".to_vec());
        let (new, n) = collecting();
        out.read_by(Viewer::window(new));
        out.push(b"b".to_vec());
        assert_eq!(*o.lock(), b"a");
        assert_eq!(*n.lock(), b"ab");
        assert_eq!(*c.lock(), b"ab");
    }

    #[test]
    fn push_never_waits_on_a_slow_reader() {
        let out = Output::new(1024);
        // A reader whose other end reads nothing: its queue fills, and it says so.
        let (tx, _rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(2);
        let _slow = out.attach(Sink::new(move |b| tx.try_send(b).map_err(|_| Closed)));
        let (sink, got) = collecting();
        let _fast = out.attach(sink);
        let started = std::time::Instant::now();
        for _ in 0..1000 {
            out.push(b"x".to_vec());
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(got.lock().len(), 1000);
        // The slow one was dropped at its first refusal.
        assert_eq!(out.inner.lock().clients.len(), 1);
    }
}
