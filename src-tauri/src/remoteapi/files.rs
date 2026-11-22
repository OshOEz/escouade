//! The bytes of files between a browser and the core, beside the WebSocket (docs/REMOTE.md, « Le mode serveur »): a
//! file joined in the page is sent (`POST /files/attach?agent=…&name=…`, its bytes the body), a file an agent sent is
//! fetched (`GET /files/sent?agent=…&item=…&index=…`). Both need the device's token (`server.rs`, `admit`).
//!
//! A file sent is written as the window's copies are (`attachfiles`): under its owner's folder, its name made safe, its
//! mark beside it while it is written; one cut, stalled, or bigger than it said is removed, nothing of it left. A file
//! fetched is always a download (`Content-Disposition: attachment`, a type no browser shows), never a page.

use crate::attachfiles::{safe_name, too_big};
use crate::core::Core;
use escouade_base::rt;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame, Incoming};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::Semaphore;
use tokio_tungstenite::tungstenite::http::{header, Method, StatusCode};

pub(super) type Body = UnsyncBoxBody<Bytes, std::io::Error>;
pub(super) type Answer = hyper::Response<Body>;

/// The routes of files.
pub(super) const PREFIX: &str = "/files/";
/// The most a file sent weighs, whatever the settings say (where 0 is no limit): the disk is not filled by one.
pub(super) const MAX_UPLOAD: u64 = 1 << 30;
/// Files written at once, all clients together; the next waits its turn, up to `QUEUE`.
pub(super) const MAX_UPLOADS: usize = 4;
/// How long a file sent waits for its turn before it is refused (503, said why).
pub(super) const QUEUE: Duration = Duration::from_secs(if cfg!(test) { 2 } else { 30 });
/// Requests of files at once (sent, waiting their turn, fetched), all clients together; the next is refused at once
/// (503): the server's other connections stay for the interface (`server::MAX_CONNECTIONS`), whatever a download
/// that nobody reads any more holds.
pub(super) const MAX_TRANSFERS: usize = 8;
/// How long a file sent may leave its next bytes waiting before it is given up.
const IDLE: Duration = Duration::from_secs(30);
/// How long a request of a file may take, answer included: a big one through a slow tunnel.
pub(super) const TRANSFER: Duration = Duration::from_secs(3600);
/// What is written at once: a chunk as the window sends its own (`attachfiles::MAX_CHUNK` at most).
const WRITE: usize = 1 << 20;
/// What a download reads at once.
const READ: u64 = 64 << 10;

/// The answer to a request of `PREFIX`, once its token is checked.
pub(super) async fn serve(
    core: Arc<Core>,
    uploads: Arc<Semaphore>,
    req: hyper::Request<Incoming>,
) -> Answer {
    let Some(q) = query(req.uri().query().unwrap_or_default()) else {
        return status(StatusCode::BAD_REQUEST);
    };
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/files/attach") => attach(core, uploads, req, q).await,
        (&Method::GET, "/files/sent") => sent(core, q).await,
        _ => status(StatusCode::NOT_FOUND),
    }
}

/// `POST /files/attach`: the body written as a copy of `owner` (`agent`, an agent's id or `draft`) named `name`; the
/// copy (`AttachedFile`) as the window's `attach_finish` gives it.
async fn attach(
    core: Arc<Core>,
    uploads: Arc<Semaphore>,
    req: hyper::Request<Incoming>,
    q: HashMap<String, String>,
) -> Answer {
    let (Some(owner), Some(name)) = (q.get("agent").cloned(), q.get("name").cloned()) else {
        return status(StatusCode::BAD_REQUEST);
    };
    // Its size said first: one over the cap is refused before a byte is read.
    let size = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    let Some(size) = size else {
        return status(StatusCode::LENGTH_REQUIRED);
    };
    let cap = core.attach_cap().map_or(MAX_UPLOAD, |c| c.min(MAX_UPLOAD));
    if size > cap {
        return refused(
            StatusCode::PAYLOAD_TOO_LARGE,
            too_big(&safe_name(&name), cap),
        );
    }
    let Ok(Ok(_slot)) = tokio::time::timeout(QUEUE, uploads.acquire_owned()).await else {
        return busy();
    };
    // Whatever ends this (an error, the client gone, this future dropped, even while it opens), what was written goes:
    // the copy is kept by a `Copy` made where it is opened, dropped with the task's result when nobody waits for it.
    let c = core.clone();
    let opened = rt::spawn_blocking(move || {
        let handle = c.attach_open(&owner, &name, size)?;
        anyhow::Ok(Copy {
            core: c,
            handle: Some(handle),
        })
    });
    let mut copy = match opened.await {
        Ok(Ok(copy)) => copy,
        Ok(Err(e)) => return refused(StatusCode::BAD_REQUEST, e),
        Err(_) => return status(StatusCode::INTERNAL_SERVER_ERROR),
    };
    let mut body = req.into_body();
    let mut buf = Vec::new();
    loop {
        let frame = match tokio::time::timeout(IDLE, body.frame()).await {
            Err(_) => return status(StatusCode::REQUEST_TIMEOUT),
            Ok(None) => break,
            // Cut before its end.
            Ok(Some(Err(_))) => return status(StatusCode::BAD_REQUEST),
            Ok(Some(Ok(frame))) => frame,
        };
        let Ok(data) = frame.into_data() else {
            continue;
        };
        buf.extend_from_slice(&data);
        if buf.len() >= WRITE {
            if let Err(e) = copy.write(std::mem::take(&mut buf)).await {
                return failed(e);
            }
        }
    }
    if let Err(e) = copy.write(buf).await {
        return failed(e);
    }
    match copy.finish().await {
        Ok(file) => json(&file),
        Err(e) => refused(StatusCode::BAD_REQUEST, e),
    }
}

/// A copy being written; dropped before its end, it is cancelled (its bytes removed).
struct Copy {
    core: Arc<Core>,
    handle: Option<String>,
}

impl Copy {
    async fn write(&mut self, bytes: Vec<u8>) -> anyhow::Result<()> {
        let (core, handle) = (self.core.clone(), self.handle.clone().unwrap_or_default());
        rt::spawn_blocking(move || core.attach_chunk(&handle, &bytes)).await?
    }

    async fn finish(&mut self) -> anyhow::Result<crate::attachfiles::AttachedFile> {
        // Finished or refused, the core has done with it: nothing to cancel.
        let handle = self.handle.take().unwrap_or_default();
        let core = self.core.clone();
        rt::spawn_blocking(move || core.attach_finish(&handle)).await?
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        // Off the runtime: the copy's lock may be held by a write to a slow disk.
        if let Some(handle) = self.handle.take() {
            let core = self.core.clone();
            rt::spawn_blocking(move || core.attach_cancel(&handle));
        }
    }
}

/// A write refused: the disk's failure (full, gone) apart (507) from the file's (400, bigger than it said).
fn failed(e: anyhow::Error) -> Answer {
    let disk = e.downcast_ref::<std::io::Error>().is_some();
    let code = if disk {
        StatusCode::INSUFFICIENT_STORAGE
    } else {
        StatusCode::BAD_REQUEST
    };
    refused(code, e)
}

/// Too many files under way: refused, said why (the page shows it).
pub(super) fn busy() -> Answer {
    refused(
        StatusCode::SERVICE_UNAVAILABLE,
        tr!(
            "Trop d’envois en cours sur le serveur : réessayez dans un moment.",
            "Too many files under way on the server: try again in a moment."
        ),
    )
}

/// `GET /files/sent`: the file `index` of the call `item` of the agent `agent` (`SendUserFile`), as a download. Any
/// refusal (no such agent, call or file, a file of another agent's, a fenced one) is a 404.
async fn sent(core: Arc<Core>, q: HashMap<String, String>) -> Answer {
    let index = q.get("index").and_then(|i| i.parse::<usize>().ok());
    let (Some(agent), Some(item), Some(index)) = (q.get("agent"), q.get("item"), index) else {
        return status(StatusCode::BAD_REQUEST);
    };
    // Opened where it was checked: a link swapped in since is not followed.
    let Ok((file, name)) = core.sent_file(agent, item, index).await else {
        return status(StatusCode::NOT_FOUND);
    };
    let file = tokio::fs::File::from_std(file);
    let Ok(len) = file.metadata().await.map(|m| m.len()) else {
        return status(StatusCode::NOT_FOUND);
    };
    // No more than it weighed when opened; an error ends it.
    let reader = Some(file.take(len));
    let frames = futures_util::stream::unfold(reader, |reader| async move {
        let mut reader = reader?;
        let mut buf = Vec::with_capacity(READ as usize);
        match (&mut reader).take(READ).read_to_end(&mut buf).await {
            Ok(0) => None,
            Ok(_) => Some((Ok(Frame::data(Bytes::from(buf))), Some(reader))),
            Err(e) => Some((Err(e), None)),
        }
    });
    hyper::Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_DISPOSITION, disposition(&name))
        .header(header::CONTENT_LENGTH, len)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::CACHE_CONTROL, "no-store")
        .header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; sandbox",
        )
        .body(BodyExt::boxed_unsync(StreamBody::new(frames)))
        .expect("a valid answer")
}

/// The header of a download named `name`, made safe: its name in ASCII, and whole in UTF-8 (RFC 6266).
fn disposition(name: &str) -> String {
    let name = safe_name(name);
    let ascii: String = name
        .chars()
        .map(|c| match c {
            ' ' | '!' | '#'..='$' | '&'..='[' | ']'..='~' => c,
            _ => '_',
        })
        .collect();
    let encoded: String = name
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

/// The pairs of a query (`a=1&b=x%20y`), as a form encodes them; None for one that is not UTF-8 once decoded, or badly
/// escaped.
fn query(q: &str) -> Option<HashMap<String, String>> {
    q.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            Some((decode(k)?, decode(v)?))
        })
        .collect()
}

fn decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' => {
                let hex = b.get(i + 1..i + 3)?;
                let hex = std::str::from_utf8(hex)
                    .ok()
                    .filter(|h| h.bytes().all(|c| c.is_ascii_hexdigit()))?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 2;
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn full(status: StatusCode, kind: &'static str, body: Vec<u8>) -> Answer {
    hyper::Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, kind)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(
            Full::new(Bytes::from(body))
                .map_err(|e| match e {})
                .boxed_unsync(),
        )
        .expect("a valid answer")
}

/// An answer with no body.
pub(super) fn status(status: StatusCode) -> Answer {
    full(status, "text/plain; charset=utf-8", Vec::new())
}

/// A refusal, with why in the language of the interface: what the page shows.
fn refused(status: StatusCode, why: impl std::fmt::Display) -> Answer {
    full(
        status,
        "text/plain; charset=utf-8",
        why.to_string().into_bytes(),
    )
}

fn json(value: &impl serde::Serialize) -> Answer {
    let body = serde_json::to_vec(value).expect("plain data");
    full(StatusCode::OK, "application/json", body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_is_read_as_a_form_writes_it() {
        let q = query("agent=draft&name=a+b%2F..%2Fc%C3%A9.txt&empty=&flag").unwrap();
        assert_eq!(q["agent"], "draft");
        assert_eq!(q["name"], "a b/../cé.txt");
        assert_eq!(q["empty"], "");
        assert_eq!(q["flag"], "");
        // Badly escaped, or not UTF-8: refused.
        for bad in ["name=%", "name=%4", "name=%zz", "name=%+1", "name=%FF"] {
            assert!(query(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn a_disk_that_fails_is_told_apart_from_a_file_refused() {
        let full = crate::attachfiles::copy_failed("a.bin", std::io::Error::other("No space left"));
        assert!(full.to_string().contains("No space left"), "{full}");
        assert_eq!(failed(full).status(), StatusCode::INSUFFICIENT_STORAGE);
        let refused = anyhow::anyhow!("a.bin is bigger than announced.");
        assert_eq!(failed(refused).status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn a_download_is_named_safely_whatever_the_name() {
        assert_eq!(
            disposition("contrat.pdf"),
            "attachment; filename=\"contrat.pdf\"; filename*=UTF-8''contrat.pdf"
        );
        // No quote, no line, no way out, no backwards text; what is not ASCII is whole in the UTF-8 form.
        let d = disposition("../a\"b\r\nSet-Cookie: x\u{202e}fdp.exe é");
        assert!(d.is_ascii(), "{d}");
        assert!(!d.contains(['\r', '\n', '/', '\\']), "{d}");
        assert_eq!(d.matches('"').count(), 2, "{d}");
        assert!(d.ends_with("%C3%A9"), "{d}");
        assert_eq!(
            disposition("con.txt"),
            "attachment; filename=\"_con.txt\"; filename*=UTF-8''_con.txt"
        );
    }
}
