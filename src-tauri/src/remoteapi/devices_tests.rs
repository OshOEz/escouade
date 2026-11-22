//! The devices' file on a throwaway folder: the pairing code, the tokens kept hashed, the revocation.

use super::*;
use crate::paths::test_dir;
use std::sync::Arc;

fn opened(name: &str) -> (Devices, PathBuf) {
    let dir = test_dir(name);
    (Devices::open(&dir), dir)
}

/// A device paired with a fresh code: its token and its id.
fn paired(devices: &Devices, name: &str) -> (String, String) {
    let code = devices.pair_code().code;
    let token = devices.redeem(&code, name).unwrap();
    let id = devices.check(&token).unwrap();
    (token, id)
}

/// A code that is surely not `code`: its first character changed.
fn wrong(code: &str) -> String {
    let first = if code.starts_with('0') { '1' } else { '0' };
    format!("{first}{}", &code[1..])
}

#[test]
fn a_code_serves_once() {
    let (devices, _) = opened("devices-once");
    let code = devices.pair_code();
    assert_eq!(code.expires_in, CODE_LIFE);
    let token = devices.redeem(&code.code, "téléphone").unwrap();
    assert_eq!(token.len(), 43);
    assert!(devices.check(&token).is_some());
    assert_eq!(devices.redeem(&code.code, "autre"), Err(Refused));
    assert_eq!(devices.list().len(), 1);
}

#[test]
fn a_code_is_read_without_its_dash_or_case() {
    let (devices, _) = opened("devices-typed");
    let code = devices.pair_code().code;
    assert_eq!(code.len(), 9);
    assert_eq!(&code[4..5], "-");
    let typed = format!(" {} ", code.replace('-', "").to_lowercase());
    assert!(devices.redeem(&typed, "pc").is_ok());
}

#[test]
fn a_code_expires() {
    let (devices, _) = opened("devices-expires");
    let code = devices.pair_code().code;
    devices.state.lock().pending.as_mut().unwrap().until = Instant::now();
    assert_eq!(devices.redeem(&code, "pc"), Err(Refused));
    assert!(devices.list().is_empty());
}

#[test]
fn only_the_last_code_is_alive() {
    let (devices, _) = opened("devices-last-code");
    let first = devices.pair_code().code;
    let second = devices.pair_code().code;
    assert_eq!(devices.redeem(&first, "pc"), Err(Refused));
    assert!(devices.redeem(&second, "pc").is_ok());
}

#[test]
fn five_wrong_tries_cancel_the_code() {
    let (devices, _) = opened("devices-tries");
    let code = devices.pair_code().code;
    for _ in 0..TRIES - 1 {
        assert_eq!(devices.redeem(&wrong(&code), "pc"), Err(Refused));
    }
    // Four wrong: the right one still pairs.
    assert!(devices.redeem(&code, "pc").is_ok());
    let code = devices.pair_code().code;
    for _ in 0..TRIES {
        assert_eq!(devices.redeem(&wrong(&code), "pc"), Err(Refused));
    }
    assert_eq!(devices.redeem(&code, "pc"), Err(Refused));
    assert_eq!(devices.list().len(), 1);
}

#[test]
fn without_a_code_nothing_pairs() {
    let (devices, _) = opened("devices-no-code");
    assert_eq!(devices.redeem("", "pc"), Err(Refused));
    assert_eq!(devices.redeem("0000-0000", "pc"), Err(Refused));
}

#[test]
fn the_token_is_kept_hashed_only_and_the_file_is_private() {
    let (devices, dir) = opened("devices-hashed");
    let (token, id) = paired(&devices, "téléphone");
    let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
    assert!(!text.contains(&token), "{text}");
    assert!(text.contains(&hash(&token)), "{text}");
    assert!(text.contains(&id) && text.contains("téléphone"), "{text}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join(FILE))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    // Read again: the same device, the same token.
    let again = Devices::open(&dir);
    assert_eq!(again.check(&token), Some(id));
    assert_eq!(again.list()[0].name, "téléphone");
}

#[test]
fn each_device_has_its_own_token() {
    let (devices, _) = opened("devices-own");
    let (a, id_a) = paired(&devices, "pc");
    let (b, id_b) = paired(&devices, "téléphone");
    assert_ne!(a, b);
    assert_ne!(id_a, id_b);
    assert_eq!(devices.check(&a), Some(id_a));
    assert_eq!(devices.check(&b), Some(id_b));
    for wrong in ["", "x", &a[1..], &format!("{a}x"), &a.to_uppercase()] {
        assert_eq!(devices.check(wrong), None, "{wrong}");
    }
}

#[test]
fn a_revoked_device_is_refused() {
    let (devices, dir) = opened("devices-revoke");
    let (a, id_a) = paired(&devices, "pc");
    let (b, id_b) = paired(&devices, "téléphone");
    assert_eq!(devices.revoke(&id_a), Ok(true));
    assert_eq!(devices.revoke(&id_a), Ok(false));
    assert!(!devices.known(&id_a));
    assert_eq!(devices.check(&a), None);
    assert_eq!(devices.check(&b), Some(id_b.clone()));
    // Kept so: read again, it is still gone.
    assert_eq!(Devices::open(&dir).check(&a), None);
    assert_eq!(Devices::open(&dir).list().len(), 1);
}

#[test]
fn a_device_is_seen_when_it_comes() {
    let (devices, _) = opened("devices-seen");
    let (token, _) = paired(&devices, "pc");
    let device = &devices.list()[0];
    assert!(device.last_seen.is_some());
    assert!(device.last_seen.unwrap() >= device.paired_at);
    assert!(devices.check(&token).is_some());
}

#[test]
fn two_redeems_of_one_code_at_once_pair_one_device() {
    for round in 0..20 {
        let (devices, _) = opened(&format!("devices-race-{round}"));
        let devices = Arc::new(devices);
        let code = devices.pair_code().code;
        let start = Arc::new(std::sync::Barrier::new(2));
        let tries: Vec<_> = (0..2)
            .map(|i| {
                let (devices, code, start) = (devices.clone(), code.clone(), start.clone());
                std::thread::spawn(move || {
                    start.wait();
                    devices.redeem(&code, &format!("pc-{i}"))
                })
            })
            .collect();
        let won: Vec<_> = tries
            .into_iter()
            .map(|t| t.join().unwrap())
            .filter_map(Result::ok)
            .collect();
        assert_eq!(won.len(), 1, "round {round}");
        assert_eq!(devices.list().len(), 1);
    }
}

#[test]
fn a_damaged_or_missing_file_means_no_device() {
    let dir = test_dir("devices-damaged");
    assert!(Devices::open(&dir).list().is_empty());
    std::fs::write(dir.join(FILE), "{ not json").unwrap();
    let devices = Devices::open(&dir);
    assert!(devices.list().is_empty());
    assert_eq!(devices.check("anything"), None);
    // Pairing still works, and writes a sound file.
    let (token, _) = paired(&devices, "pc");
    assert!(Devices::open(&dir).check(&token).is_some());
}

#[test]
fn a_name_is_cleaned() {
    let (devices, _) = opened("devices-name");
    paired(&devices, &format!("  pc\n\u{7}{}  ", "x".repeat(100)));
    let name = &devices.list()[0].name;
    assert!(name.starts_with("pc"), "{name}");
    assert_eq!(name.chars().count(), NAME_MAX);
    assert!(!name.chars().any(char::is_control), "{name:?}");
    // The format characters too: none turns the line of `devices` around.
    assert_eq!(clean("pc\u{202E}\u{200B}\u{FEFF}\u{2066}-1"), "pc-1");
    // Nothing left but them: no name.
    assert_eq!(clean(" \u{202E}\u{7}\u{200B} "), "");
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_read_is_never_written_over() {
    let dir = test_dir("devices-unreadable");
    // A link to itself: reading it fails, and not for its absence; a write by replacement would take its place.
    std::os::unix::fs::symlink(FILE, dir.join(FILE)).unwrap();
    let devices = Devices::open(&dir);
    assert!(devices.list().is_empty());
    let code = devices.pair_code().code;
    assert_eq!(devices.redeem(&code, "pc"), Err(Refused));
    assert!(devices.list().is_empty());
    let link = std::fs::symlink_metadata(dir.join(FILE)).unwrap();
    assert!(link.file_type().is_symlink(), "written over");
}

#[test]
fn a_revocation_that_is_not_written_says_so() {
    let (devices, dir) = opened("devices-revoke-unwritten");
    let (token, id) = paired(&devices, "pc");
    // A folder with a file in it where the devices' file is: nothing can be written in its place.
    std::fs::remove_file(dir.join(FILE)).unwrap();
    std::fs::create_dir(dir.join(FILE)).unwrap();
    std::fs::write(dir.join(FILE).join("x"), "").unwrap();
    assert!(devices.revoke(&id).is_err());
    // Refused all the same, as long as this server runs.
    assert_eq!(devices.check(&token), None);
}
