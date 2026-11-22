use crate::browse::*;
use crate::fence::Fence;
use crate::paths::test_dir;
use std::fs;
use std::path::{Path, PathBuf};

/// A home with `projets/{alpha,Beta,zeta}` (`Beta` a repository, `zeta` a worktree's `.git` file), a hidden folder, a
/// file, and a folder beside the home: the outside. Its real path, as `list` gives it (on Windows the temporary folder
/// may come in its short 8.3 name, `RUNNER~1`).
fn ground(name: &str) -> (PathBuf, PathBuf) {
    let root = dunce::canonicalize(test_dir(name)).unwrap();
    let (home, outside) = (root.join("home"), root.join("outside"));
    for d in ["projets/alpha", "projets/Beta/.git", "projets/zeta", ".ssh"] {
        fs::create_dir_all(home.join(d)).unwrap();
    }
    fs::write(home.join("projets/zeta/.git"), "gitdir: /elsewhere\n").unwrap();
    fs::write(home.join("projets/notes.txt"), "").unwrap();
    fs::create_dir_all(outside.join("secret")).unwrap();
    (home, outside)
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

fn names(l: &Listing) -> Vec<&str> {
    l.entries.iter().map(|e| e.name.as_str()).collect()
}

#[test]
fn folders_only_by_name_whatever_the_case_with_their_git_mark() {
    let (home, _) = ground("browse-order");
    let none = Fence::default();
    let top = list(&home, &none, None).unwrap();
    assert_eq!(top.path, s(&home));
    assert_eq!(top.parent, None, "nothing above the home");
    // The hidden folder is not shown.
    assert_eq!(names(&top), ["projets"]);

    let l = list(&home, &none, Some(&s(&home.join("projets")))).unwrap();
    assert_eq!(l.parent, Some(s(&home)));
    assert_eq!(
        l.entries,
        [
            Folder {
                name: "alpha".into(),
                dir: true,
                git: false
            },
            Folder {
                name: "Beta".into(),
                dir: true,
                git: true
            },
            Folder {
                name: "zeta".into(),
                dir: true,
                git: true
            },
        ]
    );
    // Down then up again: the same listing.
    let down = list(&home, &none, Some(&s(&home.join("projets/alpha")))).unwrap();
    assert_eq!(down.parent, Some(l.path.clone()));
    assert_eq!(list(&home, &none, down.parent.as_deref()).unwrap(), l);
}

#[test]
fn a_path_out_of_the_home_is_refused_under_every_form() {
    let (home, outside) = ground("browse-traversal");
    let none = Fence::default();
    let projets = s(&home.join("projets"));
    let sep = std::path::MAIN_SEPARATOR;
    for asked in [
        // `..`, even when it lands back under the home.
        format!("{projets}{sep}.."),
        format!("{projets}{sep}..{sep}..{sep}outside"),
        format!("{projets}{sep}alpha{sep}..{sep}Beta"),
        // Absolute, outside.
        s(&outside),
        s(&outside.join("secret")),
        s(home.parent().unwrap()),
        "/".to_string(),
        // Relative: to what?
        "projets".to_string(),
        "..".to_string(),
        String::new(),
        // Encoded: a name, never decoded.
        format!("{projets}{sep}%2e%2e"),
        format!("{projets}{sep}..%2f..%2foutside"),
        // A NUL, and the other system's separators.
        format!("{projets}\0{sep}alpha"),
        format!("{projets}{sep}..\\..\\outside"),
        format!("{projets}{sep}../../outside"),
        // Not a folder, or not there.
        format!("{projets}{sep}notes.txt"),
        format!("{projets}{sep}missing"),
    ] {
        let r = list(&home, &none, Some(&asked));
        assert!(r.is_err(), "{asked:?}: {r:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_link_that_leads_out_is_neither_shown_nor_followed() {
    let (home, outside) = ground("browse-links");
    let none = Fence::default();
    let projets = home.join("projets");
    std::os::unix::fs::symlink(&outside, projets.join("dehors")).unwrap();
    std::os::unix::fs::symlink(&home, projets.join("boucle")).unwrap();
    std::os::unix::fs::symlink(projets.join("alpha"), projets.join("raccourci")).unwrap();
    std::os::unix::fs::symlink(projets.join("missing"), projets.join("casse")).unwrap();
    let l = list(&home, &none, Some(&s(&projets))).unwrap();
    // A link to a folder of the home is shown (the home itself too); one that leads out, or nowhere, is not.
    assert_eq!(names(&l), ["alpha", "Beta", "boucle", "raccourci", "zeta"]);
    assert!(list(&home, &none, Some(&s(&projets.join("dehors")))).is_err());
    assert!(list(&home, &none, Some(&s(&projets.join("dehors/secret")))).is_err());
    // Followed, a link inside gives its real path.
    let r = list(&home, &none, Some(&s(&projets.join("raccourci")))).unwrap();
    assert_eq!(r.path, s(&projets.join("alpha")));
}

#[test]
fn the_private_files_are_neither_shown_nor_opened() {
    let (home, _) = ground("browse-fence");
    let fence = Fence::new(vec![home.join("projets/Beta")]);
    let l = list(&home, &fence, Some(&s(&home.join("projets")))).unwrap();
    assert_eq!(names(&l), ["alpha", "zeta"]);
    for asked in ["projets/Beta", "projets/Beta/.git"] {
        assert!(
            list(&home, &fence, Some(&s(&home.join(asked)))).is_err(),
            "{asked}"
        );
    }
}

#[cfg(unix)]
#[test]
fn a_folder_that_cannot_be_read_is_shown_as_such_and_the_rest_listed() {
    use std::os::unix::fs::PermissionsExt;
    let (home, _) = ground("browse-unreadable");
    let none = Fence::default();
    let shut = home.join("projets/ferme");
    fs::create_dir(&shut).unwrap();
    fs::set_permissions(&shut, fs::Permissions::from_mode(0o000)).unwrap();
    // Root reads it anyway: then it is shown as one that opens.
    let readable = fs::read_dir(&shut).is_ok();
    let l = list(&home, &none, Some(&s(&home.join("projets")))).unwrap();
    fs::set_permissions(&shut, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(names(&l), ["alpha", "Beta", "ferme", "zeta"]);
    let ferme = l.entries.iter().find(|e| e.name == "ferme").unwrap();
    assert_eq!(ferme.dir, readable);
    assert!(!ferme.git);
    if unsafe { libc::geteuid() } != 0 {
        assert!(!ferme.dir);
    }
}

#[test]
fn the_name_a_clone_takes_is_its_address_last_word() {
    for (url, name) in [
        ("https://github.com/o/demo-api.git", Some("demo-api")),
        ("https://github.com/o/demo-api/", Some("demo-api")),
        ("git@github.com:o/site.git", Some("site")),
        ("git@host:site", Some("site")),
        ("/srv/git/depot.git", Some("depot")),
        ("C:\\depots\\outil", Some("outil")),
        ("file:///srv/git/x", Some("x")),
        ("https://host/..", None),
        ("https://host/.git", None),
        ("/srv/.cache", None),
        ("-u", None),
        ("", None),
    ] {
        assert_eq!(repo_name(url).as_deref(), name, "{url}");
    }
}

/// A repository with one commit at `dir`.
fn repo(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q", "-b", "main"]);
    git(&[
        "-c",
        "user.email=t@t",
        "-c",
        "user.name=t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "init",
    ]);
}

#[tokio::test]
async fn a_repository_is_cloned_into_a_folder_of_the_home_and_not_elsewhere() {
    let (home, outside) = ground("browse-clone");
    let none = Fence::default();
    let origin = home.join("sources/demo-api");
    repo(&origin);
    let projets = home.join("projets");

    let made = clone(&home, &none, &s(&origin), &s(&projets))
        .await
        .unwrap();
    assert_eq!(made, s(&projets.join("demo-api")));
    let l = list(&home, &none, Some(&s(&projets))).unwrap();
    assert!(l.entries.contains(&Folder {
        name: "demo-api".into(),
        dir: true,
        git: true
    }));

    // Not twice in the same place, not out of the home, not from an address without a name.
    assert!(clone(&home, &none, &s(&origin), &s(&projets))
        .await
        .is_err());
    assert!(
        projets.join("demo-api/.git").exists(),
        "the first clone kept"
    );
    assert!(clone(&home, &none, &s(&origin), &s(&outside))
        .await
        .is_err());
    assert!(
        clone(&home, &none, "--upload-pack=touch /tmp/x", &s(&projets))
            .await
            .is_err()
    );
    // A clone that fails leaves nothing.
    let missing = s(&home.join("sources/absent"));
    assert!(clone(&home, &none, &missing, &s(&projets)).await.is_err());
    assert!(!projets.join("absent").exists());
}

#[tokio::test]
async fn a_clone_takes_nothing_from_out_of_the_home_or_the_private_files() {
    let (home, outside) = ground("browse-clone-source");
    let secret = outside.join("secret-repo");
    repo(&secret);
    let private = home.join("projets/prive");
    repo(&private);
    let fence = Fence::new(vec![private.clone()]);
    let projets = home.join("projets");
    let into = s(&projets);
    let file = |p: &Path| format!("file://{}", p.display());
    for url in [
        s(&secret),
        file(&secret),
        s(&private),
        file(&private),
        // Relative: to whatever folder git runs in.
        "../outside/secret-repo".into(),
        "secret-repo".into(),
        // A remote helper, any: `ext::` runs a command, the others what is installed.
        "ext::sh -c touch% /tmp/escouade-ext".into(),
        "hg::https://example.com/secret-repo".into(),
        "fd::3".into(),
    ] {
        let got = clone(&home, &fence, &url, &into).await;
        assert!(got.is_err(), "{url}: {got:?}");
    }
    let l = list(&home, &fence, Some(&into)).unwrap();
    assert_eq!(names(&l), ["alpha", "Beta", "zeta"], "nothing cloned");
    // By `file://` within the home, as by its path.
    let origin = home.join("sources/demo-api");
    repo(&origin);
    let made = clone(&home, &fence, &file(&origin), &into).await.unwrap();
    assert_eq!(made, s(&projets.join("demo-api")));
}

#[tokio::test]
async fn two_clones_of_one_name_at_once_leave_the_one_that_succeeded() {
    let (home, _) = ground("browse-clone-race");
    let none = Fence::default();
    let origin = home.join("sources/demo-api");
    repo(&origin);
    let (url, into) = (s(&origin), s(&home.join("projets")));
    let (a, b) = tokio::join!(
        clone(&home, &none, &url, &into),
        clone(&home, &none, &url, &into)
    );
    assert!(a.is_ok() != b.is_ok(), "{a:?} {b:?}");
    let made = a.or(b).unwrap();
    assert!(
        Path::new(&made).join(".git").is_dir(),
        "the clone that succeeded was removed"
    );
}

#[test]
fn a_big_folder_is_listed_up_to_a_bound() {
    let (home, _) = ground("browse-big");
    let big = home.join("projets/big");
    for i in 0..MAX_ENTRIES + 1 {
        fs::create_dir_all(big.join(format!("d{i:05}"))).unwrap();
    }
    let l = list(&home, &Fence::default(), Some(&s(&big))).unwrap();
    assert_eq!(l.entries.len(), MAX_ENTRIES);
    assert!(l.truncated);
    let l = list(&home, &Fence::default(), Some(&s(&home.join("projets")))).unwrap();
    assert!(!l.truncated);
}

#[cfg(unix)]
#[test]
fn a_home_at_the_root_shows_nothing() {
    assert!(list(Path::new("/"), &Fence::default(), None).is_err());
    assert!(list(Path::new("/"), &Fence::default(), Some("/etc")).is_err());
}
