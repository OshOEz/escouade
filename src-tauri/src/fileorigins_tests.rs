//! Tests of the table the paths of the files a page hands the window wait in (`fileorigins`), and of the wait
//! for one.

use crate::fileorigins::FileOrigins;
use std::time::{Duration, Instant};

#[test]
fn a_path_is_given_once() {
    let origins = FileOrigins::default();
    origins.deposit("a".into(), "C:\\x\\a.zip".into());
    assert_eq!(origins.take("a").as_deref(), Some("C:\\x\\a.zip"));
    assert_eq!(origins.take("a"), None);
    assert_eq!(origins.take("never"), None);
}

#[test]
fn each_id_has_its_own_path() {
    let origins = FileOrigins::default();
    origins.deposit("a".into(), "C:\\x\\a.zip".into());
    origins.deposit("b".into(), "C:\\x\\b.zip".into());
    assert_eq!(origins.take("b").as_deref(), Some("C:\\x\\b.zip"));
    assert_eq!(origins.take("a").as_deref(), Some("C:\\x\\a.zip"));
}

#[test]
fn a_second_path_for_an_id_replaces_the_first() {
    let origins = FileOrigins::default();
    origins.deposit("a".into(), "first".into());
    origins.deposit("a".into(), "second".into());
    assert_eq!(origins.take("a").as_deref(), Some("second"));
    assert_eq!(origins.take("a"), None);
}

#[test]
fn the_table_keeps_256_paths_and_the_oldest_leave_first() {
    let full = FileOrigins::default();
    for n in 0..256 {
        full.deposit(format!("id{n}"), format!("path{n}"));
    }
    // 256: nothing is lost yet.
    assert_eq!(full.take("id0").as_deref(), Some("path0"));
    full.deposit("id0".into(), "path0".into());

    // The 257th pushes the oldest out (id1: id0 came back last), and only it.
    full.deposit("id256".into(), "path256".into());
    assert_eq!(full.take("id1"), None);
    assert_eq!(full.take("id2").as_deref(), Some("path2"));
    assert_eq!(full.take("id256").as_deref(), Some("path256"));
    assert_eq!(full.take("id0").as_deref(), Some("path0"));
    for n in 3..256 {
        assert_eq!(
            full.take(&format!("id{n}")),
            Some(format!("path{n}")),
            "{n}"
        );
    }

    // Straight from empty, the same: the first of 257 is the one to go.
    let origins = FileOrigins::default();
    for n in 0..257 {
        origins.deposit(format!("id{n}"), format!("path{n}"));
    }
    assert_eq!(origins.take("id0"), None);
    for n in 1..257 {
        assert_eq!(
            origins.take(&format!("id{n}")),
            Some(format!("path{n}")),
            "{n}"
        );
    }
}

#[test]
fn replacing_a_path_does_not_make_the_table_grow() {
    let origins = FileOrigins::default();
    for n in 0..1000 {
        origins.deposit("same".into(), format!("path{n}"));
    }
    origins.deposit("other".into(), "p".into());
    assert_eq!(origins.take("same").as_deref(), Some("path999"));
    assert_eq!(origins.take("other").as_deref(), Some("p"));
}

#[test]
fn a_path_the_host_handed_out_is_kept_for_one_taking_and_then_forgotten() {
    let origins = FileOrigins::default();
    origins.deposit("a".into(), "C:\\x\\a.zip".into());
    // Read by the host, not yet asked for: no file may be joined by it.
    assert_eq!(origins.take_handed("a"), None);
    assert_eq!(origins.hand("a").as_deref(), Some("C:\\x\\a.zip"));
    // Handed out again by nobody (the deposit is gone), but kept for the taking.
    assert_eq!(origins.hand("a"), None);
    assert_eq!(origins.take_handed("a").as_deref(), Some("C:\\x\\a.zip"));
    assert_eq!(origins.take_handed("a"), None);
    assert_eq!(origins.take("a"), None);
}

#[test]
fn an_id_the_window_never_asked_the_path_of_is_not_a_path_that_was_handed() {
    let origins = FileOrigins::default();
    assert_eq!(origins.take_handed("never"), None);
    // Deposited by the host's listener, never asked for: still no.
    origins.deposit("only-read".into(), "C:\\x\\secret.txt".into());
    assert_eq!(origins.take_handed("only-read"), None);
    // The page makes up an id that is not even valid: no more.
    for id in ["", "a b", "../x", "C:\\x\\secret.txt"] {
        assert_eq!(origins.take_handed(id), None, "{id:?}");
    }
    // And what was only read is still there to be asked for.
    assert_eq!(
        origins.hand("only-read").as_deref(),
        Some("C:\\x\\secret.txt")
    );
}

#[test]
fn each_handed_path_is_taken_by_its_own_id() {
    let origins = FileOrigins::default();
    for (id, path) in [("a", "C:\\x\\a.zip"), ("b", "C:\\x\\b.zip")] {
        origins.deposit(id.into(), path.into());
        origins.hand(id).unwrap();
    }
    assert_eq!(origins.take_handed("b").as_deref(), Some("C:\\x\\b.zip"));
    assert_eq!(origins.take_handed("a").as_deref(), Some("C:\\x\\a.zip"));
}

#[test]
fn the_table_of_handed_paths_keeps_256_and_the_oldest_leave_first() {
    let origins = FileOrigins::default();
    for n in 0..257 {
        origins.deposit(format!("id{n}"), format!("path{n}"));
        origins.hand(&format!("id{n}")).unwrap();
    }
    assert_eq!(origins.take_handed("id0"), None);
    for n in 1..257 {
        assert_eq!(
            origins.take_handed(&format!("id{n}")),
            Some(format!("path{n}")),
            "{n}"
        );
    }
    // A path handed again for the same id replaces the first and does not make the table grow.
    for n in 0..1000 {
        origins.deposit("same".into(), format!("path{n}"));
        origins.hand("same").unwrap();
    }
    origins.deposit("other".into(), "p".into());
    origins.hand("other").unwrap();
    assert_eq!(origins.take_handed("same").as_deref(), Some("path999"));
    assert_eq!(origins.take_handed("other").as_deref(), Some("p"));
}

#[test]
fn the_two_tables_are_bounded_apart() {
    // 256 paths wait to be asked for and 256 have been: neither pushes the other out.
    let origins = FileOrigins::default();
    for n in 0..256 {
        origins.deposit(format!("h{n}"), format!("handed{n}"));
        origins.hand(&format!("h{n}")).unwrap();
    }
    for n in 0..256 {
        origins.deposit(format!("w{n}"), format!("waiting{n}"));
    }
    assert_eq!(origins.take_handed("h0").as_deref(), Some("handed0"));
    assert_eq!(origins.hand("w0").as_deref(), Some("waiting0"));
}

#[tokio::test]
async fn waiting_for_a_path_hands_it_out_too() {
    let origins = FileOrigins::default();
    let sender = origins.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        sender.deposit("a".into(), "C:\\x\\a.zip".into());
    });
    assert_eq!(
        origins.wait("a", Duration::from_secs(30)).await.as_deref(),
        Some("C:\\x\\a.zip")
    );
    assert_eq!(origins.take_handed("a").as_deref(), Some("C:\\x\\a.zip"));
    // A wait that ends with nothing hands nothing out.
    assert_eq!(origins.wait("b", Duration::from_millis(30)).await, None);
    assert_eq!(origins.take_handed("b"), None);
}

#[tokio::test]
async fn a_path_that_is_there_before_the_wait_is_given_at_once() {
    let origins = FileOrigins::default();
    origins.deposit("a".into(), "C:\\x\\a.zip".into());
    let started = Instant::now();
    assert_eq!(
        origins.wait("a", Duration::from_secs(30)).await.as_deref(),
        Some("C:\\x\\a.zip")
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    // Given: it is not there for a second ask.
    assert_eq!(origins.wait("a", Duration::from_millis(30)).await, None);
}

#[tokio::test]
async fn a_path_that_arrives_during_the_wait_is_given_as_it_does() {
    let origins = FileOrigins::default();
    let sender = origins.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        sender.deposit("a".into(), "C:\\x\\a.zip".into());
    });
    let started = Instant::now();
    assert_eq!(
        origins.wait("a", Duration::from_secs(30)).await.as_deref(),
        Some("C:\\x\\a.zip")
    );
    // Not the whole delay: it ended with the path.
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn without_a_path_the_wait_ends_with_its_delay_and_gives_none() {
    let origins = FileOrigins::default();
    let started = Instant::now();
    assert_eq!(origins.wait("a", Duration::from_millis(150)).await, None);
    assert!(started.elapsed() >= Duration::from_millis(140));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[tokio::test]
async fn the_path_of_another_id_does_not_end_the_wait() {
    let origins = FileOrigins::default();
    let sender = origins.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(30)).await;
        sender.deposit("other".into(), "C:\\x\\other.zip".into());
    });
    let started = Instant::now();
    assert_eq!(origins.wait("a", Duration::from_millis(200)).await, None);
    assert!(started.elapsed() >= Duration::from_millis(190));
    // Still there, for the id it is for.
    assert_eq!(origins.take("other").as_deref(), Some("C:\\x\\other.zip"));
}

#[tokio::test]
async fn several_waits_each_get_the_path_of_their_id() {
    let origins = FileOrigins::default();
    let (a, b) = (origins.clone(), origins.clone());
    let first = tokio::spawn(async move { a.wait("a", Duration::from_secs(30)).await });
    let second = tokio::spawn(async move { b.wait("b", Duration::from_secs(30)).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    origins.deposit("b".into(), "C:\\x\\b.zip".into());
    tokio::time::sleep(Duration::from_millis(20)).await;
    origins.deposit("a".into(), "C:\\x\\a.zip".into());
    assert_eq!(first.await.unwrap().as_deref(), Some("C:\\x\\a.zip"));
    assert_eq!(second.await.unwrap().as_deref(), Some("C:\\x\\b.zip"));
}
