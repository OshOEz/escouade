//! Where the core's flows go (its events, a terminal's bytes): the window's channel, or a remote
//! client's socket.

use serde::Serialize;
use std::sync::Arc;
#[cfg(feature = "desktop")]
use tauri::ipc::{Channel, InvokeResponseBody};

/// Where a flow of the core goes: the window's channel, or a remote client's socket.
pub struct Sink<T>(Arc<dyn Fn(T) -> Result<(), Closed> + Send + Sync>);

/// The other end is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Closed;

impl<T> Clone for Sink<T> {
    fn clone(&self) -> Self {
        Sink(self.0.clone())
    }
}

impl<T> Sink<T> {
    pub fn new(f: impl Fn(T) -> Result<(), Closed> + Send + Sync + 'static) -> Self {
        Sink(Arc::new(f))
    }

    pub fn send(&self, value: T) -> Result<(), Closed> {
        (self.0)(value)
    }
}

/// The window's events, serialized to JSON by the channel.
#[cfg(feature = "desktop")]
impl<T: Serialize + Send + Sync + 'static> From<Channel<T>> for Sink<T> {
    fn from(channel: Channel<T>) -> Self {
        Sink::new(move |value| {
            channel.send(value).map_err(|e| {
                log::debug!("event channel send failed: {e}");
                Closed
            })
        })
    }
}

/// A terminal's bytes, sent raw as they were read.
#[cfg(feature = "desktop")]
impl From<Channel<InvokeResponseBody>> for Sink<Vec<u8>> {
    fn from(channel: Channel<InvokeResponseBody>) -> Self {
        Sink::new(move |bytes| {
            channel
                .send(InvokeResponseBody::Raw(bytes))
                .map_err(|_| Closed)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[test]
    fn a_sink_gives_what_it_is_sent() {
        let got = Arc::new(Mutex::new(Vec::new()));
        let to = got.clone();
        let sink = Sink::new(move |v: u32| {
            to.lock().push(v);
            Ok(())
        });
        for v in [1, 2, 3] {
            sink.send(v).unwrap();
        }
        assert_eq!(*got.lock(), [1, 2, 3]);
    }

    #[test]
    fn a_closed_sink_says_so() {
        let sink = Sink::new(|_: u32| Err(Closed));
        assert_eq!(sink.send(1), Err(Closed));
    }
}
