use super::*;
use crate::paths::test_dir;

#[test]
fn a_second_lock_on_the_folder_is_refused_and_says_who_holds_it() {
    let dir = test_dir("lockfile-second").join("data");
    let first = Lock::take(&dir).unwrap();
    let why = Lock::take(&dir).unwrap_err();
    let pid = std::process::id().to_string();
    assert!(
        why.contains(&pid) && why.contains("already serves"),
        "{why}"
    );
    assert!(why.contains(&dir.display().to_string()), "{why}");
    drop(first);
    // Let go: the next server takes it.
    let again = Lock::take(&dir).unwrap();
    assert_eq!(std::fs::read_to_string(dir.join(LOCK)).unwrap(), pid);
    drop(again);
}

#[cfg(unix)]
#[test]
fn the_lock_file_is_the_owners_alone() {
    use std::os::unix::fs::PermissionsExt;
    let dir = test_dir("lockfile-mode");
    let _lock = Lock::take(&dir).unwrap();
    let mode = std::fs::metadata(dir.join(LOCK))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}
