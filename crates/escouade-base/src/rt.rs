//! The runtime the app's tasks run on: one, made at its first use, and handed to Tauri at start-up.

use std::future::Future;
use std::sync::OnceLock;

pub use tokio::task::JoinHandle;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// The runtime, made at the first call: on threads of its own, every driver on, as Tauri's was.
pub fn handle() -> tokio::runtime::Handle {
    RUNTIME
        .get_or_init(|| tokio::runtime::Runtime::new().expect("the async runtime"))
        .handle()
        .clone()
}

/// `f` on the runtime, from any thread.
#[track_caller]
pub fn spawn<F>(f: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    handle().spawn(f)
}

/// `f` on the runtime's threads for what blocks.
#[track_caller]
pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    handle().spawn_blocking(f)
}

/// `f` run to its end on the runtime, the calling thread waiting: never from inside a task.
#[track_caller]
pub fn block_on<F: Future>(f: F) -> F::Output {
    handle().block_on(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawns_from_a_thread_that_has_no_runtime() {
        let got = std::thread::spawn(|| block_on(async { spawn(async { 7 }).await.unwrap() }))
            .join()
            .unwrap();
        assert_eq!(got, 7);
    }

    #[test]
    fn every_task_runs_on_the_one_runtime() {
        let here = handle().id();
        let there = block_on(async {
            spawn(async { tokio::runtime::Handle::current().id() })
                .await
                .unwrap()
        });
        assert_eq!(here, there);
    }

    #[test]
    fn a_blocking_task_gives_its_value() {
        assert_eq!(block_on(async { spawn_blocking(|| 3).await.unwrap() }), 3);
    }
}
