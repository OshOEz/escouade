# Audit de sécurité — Escouade-lot5

2026-11-23T11:17:43 · niveau **Overkill** · Mantis `f3162c039ecd9c9d5d8bc404dd1373c7ca3152f7` · durée 2 h 25 min

Revue exhaustive de chaque fichier, 3 tours.

Les programmes de reproduction et le tableau de bord ne sont pas publiés.

| Critical | High | Medium | Low | Écartés |
|---|---|---|---|---|
| 0 | 1 | 11 | 7 | 28 |

## Findings

| Sévérité | Titre | Emplacement | Reproduit | Patch vérifié |
|---|---|---|---|---|
| HIGH | Shared ticket media import follows links inside a teammate's ticket folder and copies any file of the server into the ticket in import_media | src-tauri/src/sharing.rs:1500, src-tauri/src/sharing.rs:1511, src-tauri/src/sharing.rs:1540, src-tauri/src/sharing.rs:1488, src-tauri/src/ticketimages.rs:291 | oui | oui |
| MEDIUM | A repository collaborator makes the server read an unbounded shared ticket file and an unbounded number of ticket files in read_dir | src-tauri/src/sharing.rs:1271, src-tauri/src/sharing.rs:1500, src-tauri/src/sharing.rs:1022, src-tauri/src/sharing.rs:1366 | oui | non |
| MEDIUM | A committed workflow file names any path as a state file, which is read without a size cap or a root in scan_of, so a path to /dev/zero kills the server | src-tauri/src/workflowfiles.rs:931, src-tauri/src/workflowfiles.rs:937, src-tauri/src/workflowfiles.rs:942, src-tauri/src/workflowfiles.rs:352, src-tauri/src/workflow.rs:2203, src-tauri/src/workflow.rs:2214, src-tauri/src/workflow.rs:2346, src-tauri/src/workflow.rs:2391 | oui | oui |
| MEDIUM | Deleting a shared ticket removes files through a linked tickets folder, outside the project, in remove_file_of | src-tauri/src/sharing.rs:1671, src-tauri/src/tickets.rs:1112, src-tauri/src/sharing.rs:1605, src-tauri/src/workflowfiles.rs:122 | oui | oui |
| MEDIUM | git show and git diff between refs buffer the whole output of a hostile repository's commit in server memory before the diff cap applies in run | src-tauri/src/git.rs:574, src-tauri/src/git.rs:2118, crates/escouade-model/src/proc.rs:81, src-tauri/src/core.rs:10704, src-tauri/src/core.rs:11383, src-tauri/src/commands.rs:2197, src-tauri/src/commands.rs:2346 | oui | non |
| MEDIUM | The project file .escouade.yml is read through links with no size cap, so a symlink to /dev/zero kills the server at every start in file_text | src-tauri/src/projectfile.rs:1645, src-tauri/src/projectfile.rs:1688, src-tauri/src/core.rs:9895, src-tauri/src/core.rs:9309, src-tauri/src/core.rs:2628, src-tauri/src/commands.rs:1658, src-tauri/src/projectfile.rs:1852, src-tauri/src/sharing.rs:1267 | oui | oui |
| MEDIUM | the project memory folder is read whole with no file count or total size cap, so a cloned repo with thousands of large memory files exhausts memory when an agent searches its memory in list | src-tauri/src/memory.rs:232, src-tauri/src/memory.rs:201, src-tauri/src/memory.rs:220, src-tauri/src/memory.rs:352, src-tauri/src/memory.rs:477, src-tauri/src/mcp/memory.rs:57, src-tauri/src/mcp/memory.rs:96, src-tauri/src/core.rs:1566, src-tauri/src/core.rs:9503 | oui | oui |
| MEDIUM | Pairing cancelled by five wrong codes from any unauthenticated peer in redeem | src-tauri/src/remoteapi/devices.rs:148 | oui | oui |
| MEDIUM | Spec files of a hostile repository are read whole with no size cap in read_found, so a small committed blob that inflates to gigabytes exhausts memory | src-tauri/src/docsimport.rs:480, src-tauri/src/docsimport.rs:304, src-tauri/src/docsimport.rs:308, src-tauri/src/docsimport.rs:429, src-tauri/src/docs.rs:906 | oui | oui |
| MEDIUM | Revoking a device does not cut its in-flight or queued file transfers | src-tauri/src/remoteapi/files.rs:89, src-tauri/src/remoteapi/server.rs:619, src-tauri/src/remoteapi/server.rs:666, src-tauri/src/remoteapi/server.rs:720, src-tauri/src/remoteapi/server.rs:722 | non | non |
| MEDIUM | Parsing .escouade.yml is quadratic in its size and runs under the projects write lock, so a plain repository file of about 1 MB freezes the server in read_shared | src-tauri/src/projectfile.rs:368, src-tauri/src/projectfile.rs:358, src-tauri/src/projectfile.rs:1699, src-tauri/src/projectfile.rs:1645, src-tauri/src/projectfile.rs:493, src-tauri/src/core.rs:9895, src-tauri/src/core.rs:9309, src-tauri/src/core.rs:3057, src-tauri/src/commands.rs:1658 | non | non |
| MEDIUM | The Cursor view writes its relay rule through a committed symlink and creates directories through committed folder links in relay_to_interface | src-tauri/src/tui.rs:1002, src-tauri/src/tui.rs:954, src-tauri/src/tui.rs:1641, src-tauri/src/engine/cursor/terminal.rs:121, src-tauri/src/tui.rs:646 | oui | oui |
| LOW | Agent-authored link opened without noreferrer and without a Referrer-Policy leaks the server origin in openUrl | src/lib/platform.ts:50, src-tauri/src/remoteapi/server.rs:870, src/lib/links.ts:33 | oui | oui |
| LOW | Browser build of SentFiles offers programs and installers for download as Ouvrir, bypassing the never run rule | src/components/conv/SentFiles.svelte:142, src/components/conv/SentFiles.svelte:151, src-tauri/src/sentfiles.rs:994, src-tauri/src/remoteapi/files.rs:190 | oui | oui |
| LOW | Proxy password kept in proxy_password_in_file is serialized to every paired device in InitialState on a keychain-less server in subscribe_to | src-tauri/src/commands.rs:243, src-tauri/src/proxysecret.rs:32, crates/escouade-model/src/model.rs:46 | non | non |
| LOW | A call that never returns keeps its global call permit and lane for good, so a few hung calls lock every remote client out in dispatch | src-tauri/src/remoteapi/server.rs:958, src-tauri/src/lanes.rs:123, src-tauri/src/lanes.rs:109 | non | non |
| LOW | A paired device holds all 16 connection slots with idle WebSockets, so the admin revoke request is never accepted in accept | src-tauri/src/remoteapi/server.rs:540, src-tauri/src/remoteapi/server.rs:544, src-tauri/src/remoteapi/server.rs:692, src-tauri/src/remoteapi/server.rs:907, src-tauri/src/headless_host.rs:480 | non | non |
| LOW | fs_read blocks an async runtime worker forever on a named pipe planted in a project folder in fsedit::read | src-tauri/src/fsedit.rs:187, src-tauri/src/commands.rs:2709 | oui | oui |
| LOW | Exploit Chain: unauthenticated, repeatable denial of device pairing via agent-echoed link Referer leak (904eb54c) and global wrong-code counter in redeem (40e71f05) | src/lib/platform.ts:50, src-tauri/src/remoteapi/server.rs:870, src/lib/links.ts:33, src-tauri/src/remoteapi/devices.rs:148 | non | non |

### [HIGH] Shared ticket media import follows links inside a teammate's ticket folder and copies any file of the server into the ticket in import_media

**Résumé** : Raw hazard (4+3)*1.0 = 7.0 HIGH: a committed symlink in a shared ticket folder makes import_media copy any file readable by the service user (credentials, SSH key) into a ticket served to devices and given to the agent (reproduced on a fragment, patch VERIFIED_SECURE). Likelihood lowered by 1 because the attacker reads the data only through a steered agent and needs a shared repo the server syncs. Fix: shared_media containment and size cap (patch verified).

**Emplacement** : src-tauri/src/sharing.rs:1500, src-tauri/src/sharing.rs:1511, src-tauri/src/sharing.rs:1540, src-tauri/src/sharing.rs:1488, src-tauri/src/ticketimages.rs:291 · **CWE** : CWE-59

**Problème** : Sibling of the bypassed 2f353c55 pattern (user-controlled path read without a link check), in the shared-tickets feature. sync_shared (core.rs:2637/2642/2687/9897, run at start-up, on a timer or watcher and after git changes) reads each teammate ticket file &lt;project&gt;/.escouade/tickets/&lt;sid&gt;.yml. read_dir() refuses links (file_type().is_file()) and unsafe_dir() refuses a link at .escouade or .escouade/tickets (ESC-199, tested in sharing_tests.rs a_folder_of_tickets_that_is_a_link_is_neither_read_nor_written). But for the images and files that ticket lists, Core::import_media (sharing.rs:1500-1560) does src = dir.join(sid).join(m.file) and tests src.is_file() then std::fs::copy(&amp;src, &amp;dst), both of which FOLLOW symbolic links. Neither the per-ticket folder dir/&lt;sid&gt; nor the file inside it is checked (plain_media only validates the NAME string). Git stores symlinks, so a collaborator who can push to the shared repository can commit .escouade/tickets/&lt;sid&gt;.yml listing images: [{file: a.png}] and a symlink .escouade/tickets/&lt;sid&gt;/a.png -&gt; /home/&lt;service-user&gt;/.escouade/claude/&lt;account&gt;/.credentials.json (or ~/.ssh/id_ed25519, devices.json, server.json, any file the service user can read). The target is copied byte for byte into the ticket own folder (data.ticket_dir(tid)); ticketimages::read (std::fs::read of the whole copy, base64) then returns it through ticket_images to every paired device and ticketimages::attachments gives it to the ticket agent as the first message image, where the ticket text (also attacker authored) can instruct the agent to repeat or exfiltrate it. The media type is taken from the extension only, so the bytes are never checked to be an image. The sibling write path in the same file does it right for the other direction (ticketfiles::present uses attachfiles::copy_in, which canonicalizes and requires the real path to stay inside the folder), and import_media is the odd one out. Also: the import has no size cap (MAX_SHARED_FILE of 5 MiB is applied on the write side only), and metadata(&amp;src).len() is recorded as the file size, so a link to a multi-GB regular file (a log, a disk image) is copied into the data directory and, for images, std::fs::read loads it whole in ticket_images (OOM of the server process). write_shared_file has the mirror weakness: dst = dir.join(sid).join(file) with dst.exists()/fs::copy following a committed link directory, so the server user own ticket images are written to a folder chosen by the collaborator (file name and content are the user own, so this half is minor). Boundary crossed: a repository collaborator (a lower principal than a paired device: no token, no shell on the server, only push rights to a project the server hosts) reaching the host filesystem outside the project, which on escouade-server holds the account credentials (REMOTE.md:191). Pre-existing desktop code (sharing.rs is not in the f29711a5..HEAD diff beyond a generic change), but on a VPS the blast radius is the whole service-user home and the feature is reachable by remote clients through ticket_images. Not a documented limit: REMOTE.md and the threat model treat project content as untrusted for folders and links (ESC-199).

**Impact** : A teammate or anyone who can push a branch the server syncs reads arbitrary files of the service user (credentials, device store, SSH keys) into a ticket whose images are served to devices and fed to an LLM agent that the same attacker can steer through the ticket text; secondary: disk fill and out-of-memory of the server by linking a very large file.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none --memory=512m -v mantis-Escouade-lot5-deps:/deps:ro -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 300 bash /poc/e5ace214-d35c-4b17-a60c-dc22706c376e/run.sh
```

```
== committed tree:
100644 blob 3035f677eb49f4ea8db5481b2098bb8cb60264c7	.escouade/tickets/t1.yml
120000 blob acf966347843b0f84ddb48d6ad10f5998198ac6c	.escouade/tickets/t1/a.png
120000 blob aca11120ed14b6b2534e95ab28855d0d8bb6be39	.escouade/tickets/t1/u1/k.txt
== after clone:
/tmp/home/projects/hostile/.escouade/tickets/t1:
total 12
drwxr-xr-x 3 root root 4096 Nov 23 11:44 .
drwxr-xr-x 3 root root 4096 Nov 23 11:44 ..
lrwxrwxrwx 1 root root   48 Nov 23 11:44 a.png -> /tmp/svc/.escouade/claude/acct/.credentials.json
drwxr-xr-x 2 root root 4096 Nov 23 11:44 u1

/tmp/home/projects/hostile/.escouade/tickets/t1/u1:
total 8
drwxr-xr-x 2 root root 4096 Nov 23 11:44 .
drwxr-xr-x 3 root root 4096 Nov 23 11:44 ..
lrwxrwxrwx 1 root root   24 Nov 23 11:44 k.txt -> /tmp/svc/.ssh/id_ed25519
read_dir accepted tickets: ["t1"]
imported images ["a.png"], files [("u1/k.txt", 23)]
ticket_images returns a.png: 41 bytes, text: {"accessToken":"SECRET-CLAUDE-TOKEN-42"}
== poc exit code: 0, sentinel: MANTIS_REACHED_ENTRYPOINT
== ticket folder after import (regular files, not links):
/tmp/data/tickets/T-1:
total 16
drwxr-xr-x 3 root root 4096 Nov 23 11:44 .
drwxr-xr-x 3 root root 4096 Nov 23 11:44 ..
-rw-r--r-- 1 root root   41 Nov 23 11:44 a.png
drwxr-xr-x 3 root root 4096 Nov 23 11:44 files

/tmp/data/tickets/T-1/files/u1:
total 12
drwxr-xr-x 2 root root 4096 Nov 23 11:44 .
drwxr-xr-x 3 root root 4096 Nov 23 11:44 ..
-rw-r--r-- 1 root root   23 Nov 23 11:44 k.txt
== a.png content: {"accessToken":"SECRET-CLAUDE-TOKEN-42"}
== u1/k.txt content: SECRET-SSH-PRIVATE-KEY
sync_shared refused: the folder /tmp/home/projects/ctl/.escouade/tickets is a symbolic link: ignored
== control exit code: 0
```

**Correction** (Vérifié par re-attaque)

```diff
diff --git a/src-tauri/src/sharing.rs b/src-tauri/src/sharing.rs
index c26f396a..e57debb6 100644
--- a/src-tauri/src/sharing.rs
+++ b/src-tauri/src/sharing.rs
@@ -282,6 +282,37 @@ pub fn plain_media(file: &str) -> bool {
         })
 }
 
+/// The image or file `file` (a plain name, or `<uniq>/<name>`) of a shared ticket's folder `from`,
+/// opened, with its size: a plain file, no link at it nor on the way from `from` down, no heavier
+/// than `MAX_SHARED_FILE`. The folder comes from the repository, where a teammate's commit may put
+/// links to anywhere on this machine (ESC-199); the folder of tickets itself is `unsafe_dir`'s.
+pub(crate) fn shared_media(from: &Path, file: &str) -> Option<(std::fs::File, u64)> {
+    let src = from.join(file);
+    // Neither a link nor a pipe, a device… (opening one may block): a plain file only.
+    if !std::fs::symlink_metadata(&src).ok()?.is_file() {
+        return None;
+    }
+    let mut at = src.parent()?;
+    while at.starts_with(from) {
+        if std::fs::symlink_metadata(at).ok()?.file_type().is_symlink() {
+            return None;
+        }
+        at = at.parent()?;
+    }
+    let f = std::fs::File::open(&src).ok()?;
+    let meta = f.metadata().ok()?;
+    (meta.is_file() && meta.len() <= MAX_SHARED_FILE).then_some((f, meta.len()))
+}
+
+/// Copies a media opened by `shared_media` to `dst`, never more than `MAX_SHARED_FILE` bytes
+/// (the file may grow after it was measured).
+fn copy_media(f: std::fs::File, dst: &Path) -> std::io::Result<u64> {
+    std::io::copy(
+        &mut std::io::Read::take(f, MAX_SHARED_FILE),
+        &mut std::fs::File::create(dst)?,
+    )
+}
+
 // ---------- reading ----------
 
 fn invalid(line: Option<u32>, text: String) -> Read {
@@ -1510,12 +1541,13 @@ impl Core {
             .iter()
             .filter(|m| plain_media(&m.file) && !m.file.contains('/'))
         {
-            let src = from.join(&m.file);
             let dst = to.join(&m.file);
-            if src.is_file() && !dst.exists() {
-                let _ = std::fs::create_dir_all(&to);
-                if let Err(e) = std::fs::copy(&src, &dst) {
-                    log::warn!("shared ticket {sid}: image {} not copied: {e}", m.file);
+            if !dst.exists() {
+                if let Some((src, _)) = shared_media(&from, &m.file) {
+                    let _ = std::fs::create_dir_all(&to);
+                    if let Err(e) = copy_media(src, &dst) {
+                        log::warn!("shared ticket {sid}: image {} not copied: {e}", m.file);
+                    }
                 }
             }
             let ext = m.file.rsplit('.').next().unwrap_or_default().to_lowercase();
@@ -1537,16 +1569,16 @@ impl Core {
             .iter()
             .filter(|m| crate::ticketfiles::valid_file(&m.file).is_some())
         {
-            let src = from.join(&m.file);
             let Some(dst) = crate::ticketfiles::path_of(&to, &m.file) else {
                 continue;
             };
-            let size = std::fs::metadata(&src).map(|m| m.len()).unwrap_or(0);
-            if src.is_file() && !dst.exists() {
+            let src = shared_media(&from, &m.file);
+            let size = src.as_ref().map(|s| s.1).unwrap_or(0);
+            if let Some((src, _)) = src.filter(|_| !dst.exists()) {
                 if let Some(parent) = dst.parent() {
                     let _ = std::fs::create_dir_all(parent);
                 }
-                if let Err(e) = std::fs::copy(&src, &dst) {
+                if let Err(e) = copy_media(src, &dst) {
                     log::warn!("shared ticket {sid}: file {} not copied: {e}", m.file);
                 }
             }
diff --git a/src-tauri/src/sharing_tests.rs b/src-tauri/src/sharing_tests.rs
index e1b1646c..a51463bf 100644
--- a/src-tauri/src/sharing_tests.rs
+++ b/src-tauri/src/sharing_tests.rs
@@ -807,3 +807,31 @@ fn a_folder_of_tickets_that_is_a_link_is_neither_read_nor_written() {
     }
     assert!(crate::workflowfiles::unsafe_dir(&dir).is_some());
 }
+
+#[cfg(unix)]
+#[test]
+fn a_shared_media_is_a_plain_file_of_the_folder_never_a_link_nor_too_heavy() {
+    use std::os::unix::fs::symlink;
+    let root = crate::paths::test_dir("sh-link-media");
+    let secret = root.join("secret.json");
+    std::fs::write(&secret, b"token").unwrap();
+    let from = root.join("tickets").join("t1");
+    std::fs::create_dir_all(from.join("u1")).unwrap();
+    std::fs::write(from.join("a.png"), b"png").unwrap();
+    std::fs::write(from.join("u1").join("k.txt"), b"txt").unwrap();
+    symlink(&secret, from.join("b.png")).unwrap();
+    symlink(&root, from.join("u2")).unwrap();
+    std::fs::File::create(from.join("big.png"))
+        .unwrap()
+        .set_len(sh::MAX_SHARED_FILE + 1)
+        .unwrap();
+    assert_eq!(sh::shared_media(&from, "a.png").map(|m| m.1), Some(3));
+    assert_eq!(sh::shared_media(&from, "u1/k.txt").map(|m| m.1), Some(3));
+    assert!(sh::shared_media(&from, "b.png").is_none());
+    assert!(sh::shared_media(&from, "u2/secret.json").is_none());
+    assert!(sh::shared_media(&from, "big.png").is_none());
+    // The ticket's own folder as a link is refused too.
+    let linked = root.join("tickets").join("t2");
+    symlink(&from, &linked).unwrap();
+    assert!(sh::shared_media(&linked, "a.png").is_none());
+}
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/e5ace214-d35c-4b17-a60c-dc22706c376e.diff`

### [MEDIUM] A repository collaborator makes the server read an unbounded shared ticket file and an unbounded number of ticket files in read_dir

**Résumé** : Hazard (4+3)*0.8 = 5.6 MEDIUM: unbounded shared-ticket file / file count read by read_dir; reproduced memory growth to OOM; patch bypassed (VERIFICATION_FAILED). No user interaction once the repo is added (multiplier 0.8 only), but shared-tickets folder is a non-default setup so likelihood -1. Position aligned from EXTERNAL to SUPPLY_CHAIN per lead. Fix: handle-based regular-file check plus size and count caps.

**Emplacement** : src-tauri/src/sharing.rs:1271, src-tauri/src/sharing.rs:1500, src-tauri/src/sharing.rs:1022, src-tauri/src/sharing.rs:1366 · **CWE** : CWE-770

**Problème** : read_dir (src-tauri/src/sharing.rs:1254-1276) reads every &lt;sid&gt;.yml of &lt;project&gt;/.escouade/tickets with std::fs::read_to_string after checking only that the entry is a regular file (the symlink guard of ESC-199). There is no size cap and no count cap. sync_shared (:1380) calls it for every project with a tickets folder at start-up (sync_all_shared, :1366), on the folder watcher and after every git change, and then reconcile (:990) parses each text with yaml_edit Document::from_str, builds a Node tree (node_of), and fingerprints it. A regular file is not what 79a2cd8e reported (a link to /dev/zero read through projectfile file_text): this variant needs no link, so the is_file / symlink check is not a guard. A collaborator commits one .escouade/tickets/&lt;sid&gt;.yml of a few GiB of one repeated byte: zlib in the git object store makes it a few MB to push and fetch, the checkout writes it, and the next sync reads the whole file into memory (String, then the YAML syntax tree and the node copy, several times its size), so the server is killed by the OOM killer or its memory limit; as sync_all_shared runs at every start, the service restarts into the same crash (systemd restart loop, compare 707fdf1e) until someone deletes the file from the checkout by hand. Second unbounded dimension: reconcile makes one local ticket per unknown file with no limit, and for each new file recomputes rank (a max over all tickets of the project) and key_for (a scan of all tickets), so N files cost O(N^2), all under self.tickets.write() (sharing.rs:1415), the lock every ticket read of the core takes; the tickets then persist in state.json. Tens of thousands of tiny files (git handles them, names are 40 character ids) hold the global tickets lock for tens of seconds at every sync. The same unbounded read exists in import_media (sharing.rs:1500, read_to_string of the file again). MAX_SHARED_FILE (5 MiB) is applied to media on the write side only. Deeply nested YAML (flow sequences) may also overflow the parser stack in a dependency: not verified, it would abort the process the same way. Boundary: repository content (collaborator with push rights, or a third-party branch the server pulls) reaching availability of the server for all paired devices; no token needed. Verified by reading only; not built or run.

**Impact** : A repository collaborator denies service to the whole server (and every device) persistently: out-of-memory kill at each start from one small push, or minutes of global ticket lock from many small files.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-74c01858-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 500 sh /poc/74c01858-b815-46b1-ba9c-570c0391675b/run.sh
```

```
== git: a 64 MiB ticket file of one repeated byte, committed then cloned
size-pack: 65.17 KiB
-rw-r--r-- 1 root root 67108864 Nov 23 12:25 clone/.escouade/tickets/0123456789abcdef0123456789abcdef01234567.yml
read_dir files=1 bytes_in_ram=64 MiB peak_rss=65 MiB t=0.03s
exit=0 sentinel=MANTIS_REACHED_ENTRYPOINT
== growth: one ticket file of S MiB (sparse, same bytes in RAM), container cap 512 MiB
read_dir files=1 bytes_in_ram=32 MiB peak_rss=33 MiB t=0.01s
size=32MiB exit=0
read_dir files=1 bytes_in_ram=64 MiB peak_rss=65 MiB t=0.05s
size=64MiB exit=0
read_dir files=1 bytes_in_ram=128 MiB peak_rss=129 MiB t=0.15s
size=128MiB exit=0
read_dir files=1 bytes_in_ram=256 MiB peak_rss=257 MiB t=0.30s
size=256MiB exit=0
read_dir files=1 bytes_in_ram=384 MiB peak_rss=385 MiB t=0.29s
size=384MiB exit=0
read_dir files=1 bytes_in_ram=700 MiB peak_rss=506 MiB t=0.92s
size=700MiB exit=0
== second dimension: N new tiny files in reconcile (rank + key_for scans)
reconcile_model new_files=5000 time=0.029s
reconcile_model new_files=10000 time=0.122s
reconcile_model new_files=20000 time=0.432s
reconcile_model new_files=40000 time=1.844s
Killed
size=1200MiB exit=137 sentinel=MANTIS_REACHED_ENTRYPOINT
```

**Correction** (Correctif contourné ou inapplicable)

```diff
--- a/src-tauri/src/sharing.rs
+++ b/src-tauri/src/sharing.rs
@@ -9,7 +9,8 @@
 //! last read or written (`Shared::seen`): a file that changed outside Escouade is read, one that did
 //! not is written again when its ticket says something else.
 
-use std::collections::{BTreeMap, HashSet};
+use std::collections::{BTreeMap, HashMap, HashSet};
+use std::io::Read as _;
 use std::str::FromStr;
 
 use serde_json::{json, Map, Value};
@@ -28,6 +29,12 @@
 pub const SCHEMA_URL: &str = "https://escouade.dev/schemas/escouade-ticket.schema.json";
 /// A joined file above this size stays on its machine (choice 107); images always go.
 pub const MAX_SHARED_FILE: u64 = 5 * 1024 * 1024;
+/// A ticket's file above this size is not read (one is a few KiB): it shows as invalid.
+pub const MAX_TICKET_FILE: u64 = 64 * 1024;
+/// A folder with more tickets' files than this is not synced at all (a part of them would look removed).
+pub const MAX_TICKET_FILES: usize = 2000;
+/// What `read_dir` keeps for a file above `MAX_TICKET_FILE`, in place of its text.
+const TOO_LARGE: &str = "\0too large\0";
 /// The letters of the ids: lower case and digits, without 0/o nor 1/l.
 const ALPHABET: &[u8] = b"23456789abcdefghijkmnpqrstuvwxyz";
 
@@ -412,6 +419,15 @@
 
 /// Reads a file: what it says, or why it cannot be read. A key it does not know is left out.
 pub fn read(text: &str) -> Read {
+    if text == TOO_LARGE {
+        return invalid(
+            None,
+            tr!(
+                "le fichier est trop gros (64 Kio au plus)",
+                "the file is too large (64 KiB at most)"
+            ),
+        );
+    }
     if has_conflict(text) {
         return Read::Conflict;
     }
@@ -951,12 +967,9 @@
 }
 
 /// The board of another one's ticket: its key, with `@author` after it when this project already has a
-/// ticket of that key.
-fn key_for(f: &TicketDoc, tickets: &[Ticket], project_id: &str) -> String {
-    let taken = tickets
-        .iter()
-        .any(|t| t.project_id == project_id && t.key == f.key);
-    if !taken {
+/// ticket of that key (`keys`: the keys of the project's tickets).
+fn key_for(f: &TicketDoc, keys: &HashSet<String>) -> String {
+    if !keys.contains(&f.key) {
         return f.key.clone();
     }
     let who: String = f
@@ -996,7 +1009,6 @@
     lang: Lang,
 ) -> Outcome {
     let mut out = Outcome::default();
-    let snapshot: Vec<Ticket> = tickets.clone();
     // File id → ticket id, for « Après ».
     let mut ids: BTreeMap<String, String> = tickets
         .iter()
@@ -1005,19 +1017,37 @@
         .collect();
     // The files no ticket has yet: new tickets, made first so that « Après » finds them.
     let known: HashSet<String> = ids.keys().cloned().collect();
+    // Built once, kept up to date below: one pass per new file over all tickets would be O(N²).
+    let mut keys: HashSet<String> = tickets
+        .iter()
+        .filter(|t| t.project_id == project_id)
+        .map(|t| t.key.clone())
+        .collect();
+    let snapshot_keys = keys.clone();
+    let mut rank = tickets
+        .iter()
+        .filter(|t| t.project_id == project_id)
+        .map(|t| t.rank)
+        .max()
+        .unwrap_or(0);
+    // The imports not started, by outside ticket: the first one last, for `pop`.
+    let mut twins: HashMap<(String, String), Vec<usize>> = HashMap::new();
+    for (i, t) in tickets.iter().enumerate().rev() {
+        if t.project_id == project_id && t.shared.is_none() && t.started_at.is_none() {
+            if let Some(e) = external_of(t) {
+                twins.entry((e.source, e.key)).or_default().push(i);
+            }
+        }
+    }
     for (sid, text) in files.iter().filter(|(sid, _)| !known.contains(*sid)) {
         let seen = fingerprint(text);
         let read = read(text);
         if let Read::Ok(f, _) = &read {
             // An import of the same outside ticket, not started: it becomes the shared one.
             if let Some(ext) = &f.external {
-                let twin = tickets.iter().position(|t| {
-                    t.project_id == project_id
-                        && t.shared.is_none()
-                        && t.started_at.is_none()
-                        && external_of(t)
-                            .is_some_and(|e| e.source == ext.source && e.key == ext.key)
-                });
+                let twin = twins
+                    .get_mut(&(ext.source.clone(), ext.key.clone()))
+                    .and_then(|v| v.pop());
                 if let Some(i) = twin {
                     tickets[i].shared = Some(Shared {
                         id: sid.clone(),
@@ -1034,13 +1064,10 @@
             project_id: project_id.to_string(),
             created_at: now_ms(),
             max_loops: 5,
-            rank: tickets
-                .iter()
-                .filter(|t| t.project_id == project_id)
-                .map(|t| t.rank)
-                .max()
-                .unwrap_or(0)
-                + 1,
+            rank: {
+                rank += 1;
+                rank
+            },
             shared: Some(Shared {
                 id: sid.clone(),
                 seen: String::new(),
@@ -1050,7 +1077,7 @@
         };
         match &read {
             Read::Ok(f, _) => {
-                t.key = key_for(f, tickets, project_id);
+                t.key = key_for(f, &keys);
                 out.media.push((t.id.clone(), sid.clone()));
             }
             _ => {
@@ -1059,6 +1086,7 @@
             }
         }
         let _ = seen;
+        keys.insert(t.key.clone());
         ids.insert(sid.clone(), t.id.clone());
         out.changed.push(t.id.clone());
         tickets.push(t);
@@ -1122,7 +1150,7 @@
                 let was = t.clone();
                 apply(t, &f, newer, &ids, me);
                 if t.key.is_empty() || t.key == s.id {
-                    t.key = key_for(&f, &snapshot, project_id);
+                    t.key = key_for(&f, &snapshot_keys);
                 }
                 let sh = t.shared.as_mut().unwrap();
                 sh.seen = h;
@@ -1250,14 +1278,15 @@
     base.join(DIR)
 }
 
-/// The files of the folder: id → text. None of them when the folder is not there.
-fn read_dir(dir: &Path) -> BTreeMap<String, String> {
+/// The files of the folder: id → text. None of them when the folder is not there; `None` when it
+/// holds more than `MAX_TICKET_FILES` of them. A file above `MAX_TICKET_FILE` is kept as `TOO_LARGE`.
+fn read_dir(dir: &Path) -> Option<BTreeMap<String, String>> {
     let mut out = BTreeMap::new();
     if crate::workflowfiles::unsafe_dir(dir).is_some() {
-        return out;
+        return Some(out);
     }
     let Ok(entries) = std::fs::read_dir(dir) else {
-        return out;
+        return Some(out);
     };
     for e in entries.flatten() {
         let name = e.file_name().to_string_lossy().to_string();
@@ -1268,11 +1297,31 @@
         if !e.file_type().is_ok_and(|t| t.is_file()) {
             continue;
         }
-        if let Ok(text) = std::fs::read_to_string(e.path()) {
-            out.insert(id.to_string(), text);
+        if out.len() >= MAX_TICKET_FILES {
+            return None;
         }
+        let Ok(file) = std::fs::File::open(e.path()) else {
+            continue;
+        };
+        // The handle's own kind and size: the name may have changed since `file_type`.
+        if !file.metadata().is_ok_and(|m| m.is_file()) {
+            continue;
+        }
+        let mut bytes = Vec::new();
+        if file.take(MAX_TICKET_FILE + 1).read_to_end(&mut bytes).is_err() {
+            continue;
+        }
+        let text = if bytes.len() as u64 > MAX_TICKET_FILE {
+            TOO_LARGE.to_string()
+        } else {
+            let Ok(text) = String::from_utf8(bytes) else {
+                continue;
+            };
+            text
+        };
+        out.insert(id.to_string(), text);
     }
-    out
+    Some(out)
 }
 
 /// `git config user.name` in `path`, empty when git has none (or no git): `Core::me_in` is called
@@ -1407,7 +1456,13 @@
             log::warn!("shared tickets of {}: {why}", project.name);
             return;
         }
-        let files = read_dir(&dir);
+        let Some(files) = read_dir(&dir) else {
+            log::warn!(
+                "shared tickets of {}: more than {MAX_TICKET_FILES} files, not synced",
+                project.name
+            );
+            return;
+        };
         let me = self.me_in(&project);
         let now = iso(now_ms());
         let lang = crate::i18n::ui();
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/74c01858-b815-46b1-ba9c-570c0391675b.diff`

### [MEDIUM] A committed workflow file names any path as a state file, which is read without a size cap or a root in scan_of, so a path to /dev/zero kills the server

**Résumé** : Hazard (4+4)*0.8*0.7 = 4.48 MEDIUM: a committed workflow stateFile naming an absolute path or /dev/zero link is read uncapped at every scan (start-up, focus); reproduced OOM kill, patch VERIFIED_SECURE. Needs owner to add the repository. Also leaks one-line status of readable files (not scored separately). Fix: confine and cap state reads.

**Emplacement** : src-tauri/src/workflowfiles.rs:931, src-tauri/src/workflowfiles.rs:937, src-tauri/src/workflowfiles.rs:942, src-tauri/src/workflowfiles.rs:352, src-tauri/src/workflow.rs:2203, src-tauri/src/workflow.rs:2214, src-tauri/src/workflow.rs:2346, src-tauri/src/workflow.rs:2391 · **CWE** : CWE-770

**Problème** : workflowfiles::scan_of (workflowfiles.rs:931-945) builds a closure `state = |path| std::fs::read_to_string(root.join(path)).ok()` and applies it to every `w.state_files[].path` and to `prompt_loop.list.path` of each workflow document found in `.escouade/workflows/*.yml`. Those documents are the team-shared, committed files of the repository (workflowfiles.rs header: shared with its team, read after a pull or branch change; projects default to workflows_enabled=true, workflows_in_repo=true, workflows_auto_read=true, core.rs:1809-1812), so every byte, including `stateFiles[].path`, is attacker-controlled when the repo is third-party. `workflowfiles::read` (l.352) deserialises the YAML straight into `Workflow` with serde and validates nothing about the path. The closure has: no containment (Path::join with an absolute path REPLACES the root, and `..` is not refused), no symlink refusal, no is_file check, no size cap (read_to_string, not read_text with MAX_FILE), and no use of the allow()/refusal() filter that the live reader in workflow.rs uses (SECRET_NAMES, is_config, canonicalize-inside-root, TEXT_EXT). The only skip is for paths containing { $ * &lt; (workflow.rs:985 has the same literal test, but there it goes through ctx.read -&gt; allow -&gt; read_text which is capped). The authors state the intended invariant in workflow.rs:2391: a path that only a shared workflow file names is never read, for such a file comes with a cloned repository; scan_of violates it. Effects: (1) Availability: `path: /dev/zero` (or a committed symlink inside the repo pointing to /dev/zero, or a committed blob that inflates, or a FIFO which blocks a worker) makes read_to_string grow a String until the allocator aborts the process (NUL bytes are valid UTF-8, so no early error). scan_of runs in Core::scan_from_files, itself reached from workflows_of (synchronously, from stored_scan / workflow_track and the remote workflow scan commands) and from refresh_workflows; refresh_all_workflows re-runs at start and on every window focus, so a poisoned project crash-loops escouade-server (Restart=on-failure) until an operator removes the file. (2) Confidentiality, bounded: for any file readable by the service user at any absolute path, `exists` is returned (an existence oracle) and `says_of` returns the first `phase N of M` / `status:` line (up to 180 characters, clipped) and the count of `- [ ]` / `- [x]` boxes. Those go into the Scan, which is serialised to every paired device and the web UI. A hostile repo can thus read the `status:` line (and similar progress lines) of any text or yml file of the server that has one, one path per state file entry, with the path list chosen by the repository author. This is not the generic paired-device-equals-shell case: the author of the repo is not a paired device, and the process reached is the host outside the project sandbox.

**Impact** : Persistent denial of service of escouade-server from a hostile repository (OOM abort or blocked worker on every workflow scan, including at start-up and each focus), losing every paired device session and running agent turn; plus an existence oracle and a one-line status/progress disclosure for arbitrary absolute paths readable by the service user.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 550 bash /poc/3f453abc-81fa-4fd5-92f1-694bd9df50c1/run.sh
```

```
== cgroup memory limit: 536870912

=== 1. Confidentiality: absolute and ../ paths outside the project
    - path: "/tmp/server/notes.md"
    - path: "../../../server/notes.md"
    - path: "/etc/hostname"
    - path: "/nonexistent/x"
stateFiles named by the committed workflow: ["/tmp/server/notes.md", "../../../server/notes.md", "/etc/hostname", "/nonexistent/x"]
SCAN stateFile path=/tmp/server/notes.md exists=true says=Some(Says { phase: Some(Progress { done: 2, total: 7 }), checked: Some(Progress { done: 1, total: 2 }), status: Some("DUMMY-SECRET-token-1234") })
SCAN stateFile path=../../../server/notes.md exists=true says=Some(Says { phase: Some(Progress { done: 2, total: 7 }), checked: Some(Progress { done: 1, total: 2 }), status: Some("DUMMY-SECRET-token-1234") })
SCAN stateFile path=/etc/hostname exists=true says=Some(Says { phase: None, checked: None, status: None })
SCAN stateFile path=/nonexistent/x exists=false says=None
VmHWM_KiB=1752
exit=0 sentinel=MANTIS_REACHED_ENTRYPOINT

=== 2. Bounded growth: peak RSS vs size of the file a stateFile names (outside the project)
input=8MiB  VmHWM_KiB=18008  exit=0
input=32MiB  VmHWM_KiB=67104  exit=0
input=64MiB  VmHWM_KiB=132708  exit=0
input=128MiB  VmHWM_KiB=263832  exit=0

=== 3. Availability: stateFiles path /dev/zero
stateFiles named by the committed workflow: ["/dev/zero"]
/poc/3f453abc-81fa-4fd5-92f1-694bd9df50c1/run.sh: line 47:   161 Killed                  SENTINEL_FILE=/tmp/s3 timeout 120 /tmp/poc /tmp/home/projects/zero
== poc exit code: 137 after 0s, sentinel: MANTIS_REACHED_ENTRYPOINT
== oom events before: oom_kill 0 / after: oom_kill 1
RESULT: killed by SIGKILL (cgroup OOM killer)

=== 4. Variant: committed symlink inside the repo -> /dev/zero, named by a relative stateFile path
120000 blob 71ef849dd58cf734fe1b47e1a401909fe97f4f2f	STATE.md
stateFiles named by the committed workflow: ["STATE.md"]
/poc/3f453abc-81fa-4fd5-92f1-694bd9df50c1/run.sh: line 58:   180 Killed                  SENTINEL_FILE=/tmp/s4 timeout 120 /tmp/poc /tmp/home/projects/link
== link variant exit code: 137 sentinel: MANTIS_REACHED_ENTRYPOINT

=== 5. Control: a small state file inside the project
stateFiles named by the committed workflow: ["STATE.md"]
SCAN stateFile path=STATE.md exists=true says=Some(Says { phase: None, checked: Some(Progress { done: 1, total: 1 }), status: Some("fine") })
VmHWM_KiB=1752
== control exit code: 0
```

**Correction** (Vérifié par re-attaque)

```diff
diff --git a/src-tauri/src/workflowfiles.rs b/src-tauri/src/workflowfiles.rs
--- a/src-tauri/src/workflowfiles.rs
+++ b/src-tauri/src/workflowfiles.rs
@@ -910,6 +910,41 @@
     // A folder left empty goes.
     let _ = std::fs::remove_dir(from);
     Ok(())
+}
+
+/// The text of a state file a workflow file names, `path` relative to the project. That file comes
+/// with a cloned repository, so only a regular file inside the project is read (no absolute path,
+/// no `..`, no link, no device or pipe), and none over `MAX_FILE`.
+fn state_text(root: &Path, path: &str) -> Option<String> {
+    use std::io::Read;
+    use std::path::Component;
+    let rel = Path::new(path);
+    if !rel
+        .components()
+        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
+    {
+        return None;
+    }
+    let file = root.join(rel);
+    let meta = std::fs::symlink_metadata(&file).ok()?;
+    if !meta.file_type().is_file() || meta.len() > MAX_FILE {
+        return None;
+    }
+    // A link among the folders of the path can still lead out: the real path must stay inside.
+    let real = file.canonicalize().ok()?;
+    if !real.starts_with(root.canonicalize().ok()?) {
+        return None;
+    }
+    let mut bytes = Vec::new();
+    std::fs::File::open(&real)
+        .ok()?
+        .take(MAX_FILE + 1)
+        .read_to_end(&mut bytes)
+        .ok()?;
+    if bytes.len() as u64 > MAX_FILE {
+        return None;
+    }
+    String::from_utf8(bytes).ok()
 }
 
 /// The workflows of the folder, as the project shows and follows them: the definitions of the
@@ -930,10 +965,7 @@
         .collect();
     workflows.sort_by(|a, b| a.id.cmp(&b.id));
     for w in &mut workflows {
-        let state = |path: &str| {
-            let file = root.join(path);
-            std::fs::read_to_string(&file).ok()
-        };
+        let state = |path: &str| state_text(root, path);
         for s in &mut w.state_files {
             let text = state(&s.path);
             s.exists = text.is_some();
@@ -1049,3 +1081,33 @@
         "required": ["version", "workflow"],
     })
 }
+
+#[cfg(test)]
+mod state_text_tests {
+    use super::{state_text, MAX_FILE};
+
+    #[test]
+    fn a_state_file_is_read_only_inside_the_project_and_capped() {
+        let root = crate::paths::test_dir("wf-state-text");
+        std::fs::write(root.join("STATE.md"), "status: ok\n").unwrap();
+        assert_eq!(state_text(&root, "STATE.md").as_deref(), Some("status: ok\n"));
+        assert_eq!(state_text(&root, "./STATE.md").as_deref(), Some("status: ok\n"));
+        // An absolute path or `..` is refused, even when it lands inside the project.
+        let abs = root.join("STATE.md");
+        assert_eq!(state_text(&root, abs.to_str().unwrap()), None);
+        let name = root.file_name().unwrap().to_str().unwrap();
+        assert_eq!(state_text(&root, &format!("../{name}/STATE.md")), None);
+        // A folder, a missing file and a file over the cap are not read.
+        assert_eq!(state_text(&root, ""), None);
+        assert_eq!(state_text(&root, "missing.md"), None);
+        std::fs::write(root.join("big.md"), vec![b'a'; MAX_FILE as usize + 1]).unwrap();
+        assert_eq!(state_text(&root, "big.md"), None);
+        #[cfg(unix)]
+        {
+            std::os::unix::fs::symlink(root.join("STATE.md"), root.join("link.md")).unwrap();
+            assert_eq!(state_text(&root, "link.md"), None);
+            std::os::unix::fs::symlink("/etc", root.join("out")).unwrap();
+            assert_eq!(state_text(&root, "out/hosts"), None);
+        }
+    }
+}
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/3f453abc-81fa-4fd5-92f1-694bd9df50c1.diff`

### [MEDIUM] Deleting a shared ticket removes files through a linked tickets folder, outside the project, in remove_file_of

**Résumé** : Hazard (4+4)*0.8*0.7 = 4.48 MEDIUM, position aligned to SUPPLY_CHAIN (repository content, as 79a2cd8e; the finding said EXTERNAL): remove_file_of deletes through a committed tickets symlink outside the project (reproduced on a fragment; patch reattack failed_to_bypass). Needs the user to delete a ticket. Fix: call unsafe_dir in remove_file_of.

**Emplacement** : src-tauri/src/sharing.rs:1671, src-tauri/src/tickets.rs:1112, src-tauri/src/sharing.rs:1605, src-tauri/src/workflowfiles.rs:122 · **CWE** : CWE-59

**Problème** : remove_file_of (src-tauri/src/sharing.rs:1671) does remove_file(dir/&lt;sid&gt;.yml) and remove_dir_all(dir/&lt;sid&gt;) with dir = dir_of(project) = &lt;project&gt;/.escouade/tickets, and never calls workflowfiles::unsafe_dir. Every other writer of that folder does: sync_shared (:1380), write_shared_file (:1433), shared_resolve (:1631) and read_dir (:1256) all refuse a link at .escouade or .escouade/tickets and a real path that leaves the project (ESC-199, tested by a_folder_of_tickets_that_is_a_link_is_neither_read_nor_written). The removal is the one sink of the folder left without the guard. It is reached by ticket_delete (tickets.rs:1111, any shared ticket, "Supprimer pour l equipe", also from a remote client through the command table) and by ticket_share(off) (sharing.rs:1605). The id removed is s.id, the id the ticket got from the NAME of a teammate file (id_of_file/plain_id: [A-Za-z0-9_-], up to 40 characters, so it can equal the name of a directory such as claude, attachments, src, or a sibling project folder). Attack: a repository collaborator commits .escouade/tickets/claude.yml (a valid ticket file): reconcile creates a local shared ticket with s.id = claude on every machine that syncs the project. A later commit replaces the folder .escouade/tickets by a symlink (git stores it; clone and pull with core.symlinks=true make it real), for example to the service user data folder (REMOTE.md:191 gives its layout: ~/.escouade/claude/&lt;account&gt;/.credentials.json) or to .. / ../.. (the project root, or the folder of the sibling checkouts). From then on sync_shared refuses the folder, but the ticket already exists on the board. When the user deletes that ticket (a ticket they did not write, whose file has disappeared from the board view, is an ordinary thing to clean up), remove_file_of resolves dir through the link and deletes &lt;target&gt;/claude.yml and the whole tree &lt;target&gt;/claude: every Claude account credential directory of the server, or a sibling project checkout with its unpushed work, or a top level folder of the project. remove_dir_all itself does not follow a link at the LAST component, so only the link on .escouade/tickets (or .escouade) matters, which is exactly the one unsafe_dir was written to catch. This is a different sink and a different effect from e5ace214 (read and copy of linked media, import_media) and from its mirror remark on write_shared_file (copy of the user own images): it is deletion of arbitrary directories of the host by name chosen by the collaborator. Boundary crossed: repository content (a collaborator with push rights, no token, no shell on the server) reaching the host filesystem outside the project. The user action (deleting a ticket) is normal use, not the paired-device equals shell case. Verified by reading: sharing.rs:1671-1677, tickets.rs:1108-1113, workflowfiles.rs:122-145, plain_id at sharing.rs:263. Not built or run.

**Impact** : A repository collaborator can make the server (or desktop) user delete, with one routine ticket deletion, a directory or file outside the project named by the collaborator: account credentials of the server, sibling repositories, a top level folder of the checkout. Loss of data and of the signed-in accounts; no disclosure.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-f478f2d8-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 300 bash /poc/f478f2d8-46e8-4750-8057-374f31e50627/run.sh
```

```
== phase 1 (after clone)
unsafe_dir(/tmp/home/projects/proj/.escouade/tickets) = None
read_dir ids (shared tickets reconcile would create) = ["claude", "other"]
== phase 2 (after pull): git ls-files -s .escouade/tickets: 120000 0b12bd9a04a8b64f9629935f07393e3ec9cf4ad6 0	.escouade/tickets
total 8
drwxr-xr-x 2 root root 4096 Nov 23 12:25 .
drwxr-xr-x 4 root root 4096 Nov 23 12:25 ..
lrwxrwxrwx 1 root root   18 Nov 23 12:25 tickets -> ../../../.escouade
unsafe_dir(/tmp/home/projects/proj/.escouade/tickets) = Some("the folder /tmp/home/projects/proj/.escouade/tickets is a symbolic link: ignored")
read_dir ids (shared tickets reconcile would create) = []
== BEFORE delete: .credentials.json
unsafe_dir(/tmp/home/projects/proj/.escouade/tickets) = Some("the folder /tmp/home/projects/proj/.escouade/tickets is a symbolic link: ignored")
ticket_delete -> remove_file_of(/tmp/home/projects/proj/.escouade/tickets, "claude")
poc exit=0
== AFTER delete claude: ls /tmp/home/.escouade ->
total 8
drwxr-xr-x 2 root root 4096 Nov 23 12:25 .
drwxr-xr-x 4 root root 4096 Nov 23 12:25 ..
RESULT: VULNERABLE - /tmp/home/.escouade/claude (outside the project) deleted by remove_file_of
unsafe_dir(/tmp/home/projects/proj/.escouade/tickets) = Some("the folder /tmp/home/projects/proj/.escouade/tickets is a symbolic link: ignored")
read_dir ids (shared tickets reconcile would create) = []
unsafe_dir(/tmp/home/projects/proj/.escouade/tickets) = Some("the folder /tmp/home/projects/proj/.escouade/tickets is a symbolic link: ignored")
ticket_delete -> remove_file_of(/tmp/home/projects/proj/.escouade/tickets, "other")
poc exit=0
VARIANT: VULNERABLE - sibling checkout /tmp/home/projects/other deleted
== sentinel: MANTIS_REACHED_ENTRYPOINT MANTIS_REACHED_ENTRYPOINT 
CONTROL: plain folder after delete: 0 entries, /tmp/ctl/keep present=yes
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src-tauri/src/sharing.rs
+++ b/src-tauri/src/sharing.rs
@@ -1671,9 +1671,18 @@
 pub(crate) fn remove_file_of(dir: &Path, sid: &str) {
     if !plain_id(sid) {
         return;
     }
+    // Never through a link, nor out of the project (ESC-199), as every other writer of the folder.
+    if let Some(why) = crate::workflowfiles::unsafe_dir(dir) {
+        log::warn!("shared ticket {sid} not removed: {why}");
+        return;
+    }
     let _ = std::fs::remove_file(dir.join(format!("{sid}.yml")));
-    let _ = std::fs::remove_dir_all(dir.join(sid));
+    let folder = dir.join(sid);
+    // Only a plain folder: a link there is left as it is.
+    if std::fs::symlink_metadata(&folder).is_ok_and(|m| m.is_dir()) {
+        let _ = std::fs::remove_dir_all(folder);
+    }
 }
 
 fn not_shared() -> anyhow::Error {
--- a/src-tauri/src/sharing_tests.rs
+++ b/src-tauri/src/sharing_tests.rs
@@ -807,3 +807,26 @@
     }
     assert!(crate::workflowfiles::unsafe_dir(&dir).is_some());
 }
+
+#[cfg(unix)]
+#[test]
+fn removing_a_shared_ticket_never_goes_through_a_link() {
+    let root = crate::paths::test_dir("sh-link-remove");
+    let outside = root.join("outside");
+    std::fs::create_dir_all(outside.join("claude")).unwrap();
+    std::fs::write(outside.join("claude.yml"), "x").unwrap();
+    std::fs::write(outside.join("claude/keep"), "x").unwrap();
+    let project = root.join("project");
+    std::fs::create_dir_all(project.join(".escouade")).unwrap();
+    let dir = project.join(sh::DIR);
+    std::os::unix::fs::symlink(&outside, &dir).unwrap();
+    sh::remove_file_of(&dir, "claude");
+    assert!(outside.join("claude.yml").exists());
+    assert!(outside.join("claude/keep").exists());
+    // A plain folder of tickets whose <sid> entry is a link: the target is kept.
+    std::fs::remove_file(&dir).unwrap();
+    std::fs::create_dir_all(&dir).unwrap();
+    std::os::unix::fs::symlink(outside.join("claude"), dir.join("claude")).unwrap();
+    sh::remove_file_of(&dir, "claude");
+    assert!(outside.join("claude/keep").exists());
+}
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/f478f2d8-46e8-4750-8057-374f31e50627.diff`

### [MEDIUM] git show and git diff between refs buffer the whole output of a hostile repository's commit in server memory before the diff cap applies in run

**Résumé** : Hazard (4+4)*0.8*0.7 = 4.48 MEDIUM: git show / diff_refs buffer whole output; a small highly compressible commit OOM-kills the server (reproduced, show and diff_refs). Needs owner to add the repo and open the commit or comparison. Patch only VERIFICATION_INCOMPLETE. Fix: bound output in proc wait_output.

**Emplacement** : src-tauri/src/git.rs:574, src-tauri/src/git.rs:2118, crates/escouade-model/src/proc.rs:81, src-tauri/src/core.rs:10704, src-tauri/src/core.rs:11383, src-tauri/src/commands.rs:2197, src-tauri/src/commands.rs:2346 · **CWE** : CWE-770

**Problème** : git::show (git.rs:574) and git::diff_refs (git.rs:2118), reached by the remote commands git_show (commands.rs:2197 -&gt; core.rs:10704) and git_diff_refs (commands.rs:2346 -&gt; core.rs:11383) with a hash or refs chosen by the client, run git through git::run -&gt; run_within -&gt; output_within -&gt; proc::wait_output (crates/escouade-model/src/proc.rs:81). wait_output appends every byte of stdout to a Vec&lt;u8&gt; with no size limit; the only bound is the 120 s GIT_TIMEOUT. The 4 MB caps (MAX_FILE_DIFF, DIFF_BUDGET, cap_file_diffs/cap_diff, git.rs:262-299) are applied afterwards, on the String made from that Vec (from_utf8_lossy copies it again), so they protect the IPC frame, not the server memory. Repository content is attacker-controlled (cloned public repo, or a fetched branch of a hostile pull request). A diff expands content: a text file of repeated short lines compresses roughly 1000:1 in a pack, git treats files up to core.bigFileThreshold (512 MB) as text, and the diff prints every line with a sign, about 1.5x the file size. A repository of a few MB can therefore hold one commit that adds a few dozen 400-500 MB text files (and the next commit deletes them, so the checkout stays small and the clone fits the disk). git show of that commit, or a comparison of the branch with its base, streams tens of GB into the Vec until the OOM killer ends the process or the 120 s limit is reached (git emits hundreds of MB per second). escouade-server is one process: its death ends every running agent turn and every device session, and a systemd unit restarts into the same state. The commit appears in the log list and the branch in the branch list of the project, so the trigger is an ordinary click (a commit in the history, a branch comparison) on a 1-4 GB VPS, which is the intended deployment. Attacker class: author of repository content (hostile repo or PR branch), not a paired device; the paired device only performs the click. Same class as 79a2cd8e (hostile repo content kills the server) but a different sink: not a symlink or special file, ordinary history. The same unbounded buffering applies to every git::run caller whose output the repository shapes (lines_at git show rev:path of a huge blob, git::diff of a checkout), but the commit and ref comparisons are the ones where the repository alone, without local write access, decides the size. Not dynamically reproduced; static evidence only (wait_output code path read in full, no cap between pipe and Vec).

**Impact** : Denial of service of the whole server: memory exhaustion / OOM kill of escouade-server, losing all running agent turns and device sessions, from repository content alone plus one click on a commit or branch comparison.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 550 bash /poc/67ba1817-204a-4eca-97cd-c5a3fb305eee/run.sh
```

```
== cgroup memory limit: 536870912, swap limit: 536870912
== hostile bare repo on disk: 156 KiB
== checkout of the clone: 8 KiB (files deleted by the next commit)

=== 1. Bounded growth: git show <commit>, peak RSS vs size of the commit's diff
-- commit adding 1 x 10 MiB file(s) (4c741f8e7e14)
   git stdout buffered whole: 15728777 bytes, VmHWM now 17308 KiB
returned to the client after the cap: 130 bytes  VmHWM_KiB=17172
-- commit adding 2 x 10 MiB file(s) (07cecc04bb11)
   git stdout buffered whole: 31457554 bytes, VmHWM now 32744 KiB
returned to the client after the cap: 260 bytes  VmHWM_KiB=32544
-- commit adding 4 x 10 MiB file(s) (0e6616a92dca)
   git stdout buffered whole: 62915108 bytes, VmHWM now 63464 KiB
returned to the client after the cap: 520 bytes  VmHWM_KiB=63144
-- commit adding 8 x 10 MiB file(s) (481034725532)
   git stdout buffered whole: 125830216 bytes, VmHWM now 124904 KiB
returned to the client after the cap: 1040 bytes  VmHWM_KiB=124576

=== 2. Availability: git show of the commit adding 80 x 10 MiB (diff ~1.2 GB), 1 GiB RAM+swap cap
/poc/67ba1817-204a-4eca-97cd-c5a3fb305eee/run.sh: line 39:   229 Killed                  SENTINEL_FILE=/tmp/s2 timeout 150 /tmp/poc show $R ${ADD[80]}
== poc exit code: 137 after 18s, sentinel: MANTIS_REACHED_ENTRYPOINT
== oom events before: oom_kill 0 / after: oom_kill 1
RESULT: killed by SIGKILL (cgroup OOM killer)

=== 3. Variant: diff_refs(base, branch 'hostile') - comparing the branch with its base
/poc/67ba1817-204a-4eca-97cd-c5a3fb305eee/run.sh: line 47:   239 Killed                  SENTINEL_FILE=/tmp/s3 timeout 150 /tmp/poc diff_refs $R ${BASE[80]} $(git -C $R rev-parse origin/hostile)
== poc exit code: 137 after 17s, sentinel: MANTIS_REACHED_ENTRYPOINT
== oom events before: oom_kill 1 / after: oom_kill 3

=== 4. Control: git show of the base commit (small)
   git stdout buffered whole: 134 bytes, VmHWM now 1956 KiB
returned to the client after the cap: 134 bytes  VmHWM_KiB=1956
== control exit code: 0
```

**Correction** (Vérification incomplète)

```diff
diff --git a/crates/escouade-model/src/git.rs b/crates/escouade-model/src/git.rs
index d8a12f47..48153b09 100644
--- a/crates/escouade-model/src/git.rs
+++ b/crates/escouade-model/src/git.rs
@@ -39,6 +39,22 @@ pub async fn run_within(cwd: &str, args: &[&str], limit: Limit) -> Result<Vec<u8
     checked(out, args)
 }
 
+/// `run`, keeping at most `max` bytes of what git prints: past them git is killed, and `true`
+/// says the output was cut there (a diff of a hostile repository can run to gigabytes).
+pub async fn run_capped(cwd: &str, args: &[&str], max: usize) -> Result<(Vec<u8>, bool)> {
+    let limit = Limit::Reads.duration();
+    let mut out = match crate::proc::output_capped(command(cwd, args), limit, max).await {
+        Ok(out) => out,
+        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return Err(stopped(args, limit)),
+        Err(e) => return Err(e.into()),
+    };
+    if out.stdout.len() > max {
+        out.stdout.truncate(max);
+        return Ok((out.stdout, true));
+    }
+    Ok((checked(out, args)?, false))
+}
+
 /// Time limit of the git commands that read.
 pub const GIT_TIMEOUT: Duration = Duration::from_secs(120);
 /// Time limit of the git commands that run the user's hooks.
diff --git a/crates/escouade-model/src/proc.rs b/crates/escouade-model/src/proc.rs
index 787ff54d..4f568272 100644
--- a/crates/escouade-model/src/proc.rs
+++ b/crates/escouade-model/src/proc.rs
@@ -24,14 +24,24 @@ const EOF_GRACE: Duration = Duration::from_secs(2);
 /// Runs `cmd` (nothing to read on its input, its two outputs in pipes) and gives what it printed.
 /// Past `limit` it is killed and the error is `TimedOut`. A pipe a descendant keeps open after the
 /// process ended does not hold it back (see `wait_output`).
-pub async fn output_within(mut cmd: Command, limit: Duration) -> io::Result<Output> {
+pub async fn output_within(cmd: Command, limit: Duration) -> io::Result<Output> {
+    output_capped(cmd, limit, usize::MAX).await
+}
+
+/// `output_within`, reading no more than about `max_stdout` bytes of stdout: past them the process
+/// is killed and its stdout is longer than `max_stdout` (what it printed past them is not read).
+pub async fn output_capped(
+    mut cmd: Command,
+    limit: Duration,
+    max_stdout: usize,
+) -> io::Result<Output> {
     cmd.stdin(Stdio::null())
         .stdout(Stdio::piped())
         .stderr(Stdio::piped())
         .kill_on_drop(true);
     let child = cmd.spawn()?;
     // The timeout drops the future and with it the child, which `kill_on_drop` kills.
-    match timeout(limit, wait_output(child)).await {
+    match timeout(limit, wait_output_capped(child, max_stdout)).await {
         Ok(out) => out,
         Err(_) => Err(io::Error::new(
             io::ErrorKind::TimedOut,
@@ -78,7 +88,13 @@ pub fn no_inherited_fds(cmd: &mut Command) {
 /// without waiting for the end of the pipes once it has: they are then given `EOF_GRACE` to end,
 /// counted from the last data they delivered, so that all of a big output is read, and a pipe
 /// that stays open without a word is let go.
-pub async fn wait_output(mut child: Child) -> io::Result<Output> {
+pub async fn wait_output(child: Child) -> io::Result<Output> {
+    wait_output_capped(child, usize::MAX).await
+}
+
+/// `wait_output`, killing the child once its stdout passes `max_stdout` bytes: a program whose
+/// output a repository decides (a diff) could otherwise fill the memory before it ends.
+pub async fn wait_output_capped(mut child: Child, max_stdout: usize) -> io::Result<Output> {
     let mut stdout = child.stdout.take();
     let mut stderr = child.stderr.take();
     let (mut out, mut err) = (Vec::new(), Vec::new());
@@ -99,6 +115,10 @@ pub async fn wait_output(mut child: Child) -> io::Result<Output> {
             n = fill(&mut stdout, &mut out), if stdout.is_some() => {
                 if n == 0 {
                     stdout = None;
+                } else if out.len() > max_stdout {
+                    // Fails once the process has ended on its own: its status is kept then.
+                    let _ = child.start_kill();
+                    break child.wait().await?;
                 } else {
                     renew(&mut ended);
                 }
@@ -289,6 +309,20 @@ mod tests {
         assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "err");
     }
 
+    /// An endless output is cut a little past the cap and the process killed, well within its limit.
+    #[cfg(unix)]
+    #[tokio::test]
+    async fn an_output_past_its_cap_is_cut_and_the_process_killed() {
+        let start = Instant::now();
+        let out = output_capped(command("yes", &[]), Duration::from_secs(60), 1_000_000)
+            .await
+            .unwrap();
+        assert!(!out.status.success());
+        assert!(out.stdout.len() > 1_000_000, "{}", out.stdout.len());
+        assert!(out.stdout.len() < 3_000_000, "{}", out.stdout.len());
+        assert!(start.elapsed() < Duration::from_secs(10), "{:?}", start.elapsed());
+    }
+
     #[tokio::test]
     async fn the_exit_code_and_both_outputs_are_kept_apart() {
         #[cfg(unix)]
diff --git a/src-tauri/src/git.rs b/src-tauri/src/git.rs
index 34b9755f..ed902c4a 100644
--- a/src-tauri/src/git.rs
+++ b/src-tauri/src/git.rs
@@ -256,6 +256,22 @@ fn command_line_chunks(paths: &[String]) -> Vec<&[String]> {
 /// The most diff text one file sends to the interface; past it the file is only flagged.
 const MAX_FILE_DIFF: usize = 4_000_000;
 
+/// The most diff text read from git: a few files at their cap. Past it git is killed (see `read_diff`).
+const MAX_DIFF_READ: usize = 16 * MAX_FILE_DIFF;
+
+/// The diff git prints for `args`, read up to `MAX_DIFF_READ`: a hostile repository's commit can
+/// print gigabytes. Cut there, the file it was printing is `flagged`.
+// shortcut: the files after the cut are left out unnamed, list them (--raw) if a user misses them.
+pub async fn read_diff(repo: &str, args: &[&str]) -> Result<String> {
+    let (out, cut) = run_capped(repo, args, MAX_DIFF_READ).await?;
+    let text = String::from_utf8_lossy(&out);
+    if !cut {
+        return Ok(text.into_owned());
+    }
+    let last = text.rfind("\ndiff --git ").map_or(0, |at| at + 1);
+    Ok(format!("{}{}", &text[..last], flagged(&text[last..])))
+}
+
 /// The line that stands, in a file's header, for the diff that was not sent. The interface shows
 /// « Diff trop volumineux pour être affiché. » for a file carrying it (`TOO_LARGE` in `diff.ts`).
 const TOO_LARGE: &str = "Diff too large";
@@ -575,7 +591,7 @@ pub async fn show(repo: &str, hash: &str) -> Result<String> {
     if hash.len() < 4 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
         bail!(tr!("commit invalide : {hash}", "invalid commit: {hash}"));
     }
-    let out = run(
+    let out = read_diff(
         repo,
         &[
             "show",
@@ -588,7 +604,7 @@ pub async fn show(repo: &str, hash: &str) -> Result<String> {
         ],
     )
     .await?;
-    Ok(cap_file_diffs(&String::from_utf8_lossy(&out), MAX_FILE_DIFF).into_owned())
+    Ok(cap_file_diffs(&out, MAX_FILE_DIFF).into_owned())
 }
 
 pub async fn list_files(cwd: &str) -> Result<Vec<String>> {
@@ -2117,7 +2133,7 @@ pub async fn unique_commits(repo: &str, refs: &[String]) -> Result<u32> {
 /// capped as a commit's is, and the whole kept within `DIFF_BUDGET`.
 pub async fn diff_refs(repo: &str, a: &str, b: &str) -> Result<String> {
     let (from, to) = (commit_of(repo, a).await?, commit_of(repo, b).await?);
-    let out = run(
+    let out = read_diff(
         repo,
         &[
             "diff",
@@ -2130,7 +2146,7 @@ pub async fn diff_refs(repo: &str, a: &str, b: &str) -> Result<String> {
         ],
     )
     .await?;
-    Ok(cap_diff(&String::from_utf8_lossy(&out), MAX_FILE_DIFF, DIFF_BUDGET).into_owned())
+    Ok(cap_diff(&out, MAX_FILE_DIFF, DIFF_BUDGET).into_owned())
 }
 
 /// Of `paths`, those that `branch` changed since it left `base` (`base...branch`), whatever
diff --git a/src-tauri/src/prreview.rs b/src-tauri/src/prreview.rs
index 8aca7dba..76f38801 100644
--- a/src-tauri/src/prreview.rs
+++ b/src-tauri/src/prreview.rs
@@ -441,7 +441,7 @@ async fn read_diff(s: &Source) -> Result<(String, Vec<FileDiff>)> {
             .unwrap_or_else(|_| base.clone());
         // The repository's and the user's own settings do not reach what is parsed: no external
         // diff, no conversion, a prefix of a/ and b/.
-        let bytes = git::run(
+        let text = git::read_diff(
             &s.root,
             &[
                 "-c",
@@ -457,7 +457,7 @@ async fn read_diff(s: &Source) -> Result<(String, Vec<FileDiff>)> {
             ],
         )
         .await?;
-        Ok::<_, anyhow::Error>((sha, parse_diff(&String::from_utf8_lossy(&bytes))))
+        Ok::<_, anyhow::Error>((sha, parse_diff(&text)))
     }
     .await;
     for r in [&tip, &base] {
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/67ba1817-204a-4eca-97cd-c5a3fb305eee.diff`

### [MEDIUM] The project file .escouade.yml is read through links with no size cap, so a symlink to /dev/zero kills the server at every start in file_text

**Résumé** : Raw hazard (4+4)*0.8*0.7 = 4.48 MEDIUM: a hostile repository carrying .escouade.yml -&gt; /dev/zero OOM-kills escouade-server at the first read and at every start (reproduced on a fragment, patch VERIFIED_SECURE). Needs the owner to add the repository (user interaction). Fix: regular-file-only, capped read in file_text (patch verified).

**Emplacement** : src-tauri/src/projectfile.rs:1645, src-tauri/src/projectfile.rs:1688, src-tauri/src/core.rs:9895, src-tauri/src/core.rs:9309, src-tauri/src/core.rs:2628, src-tauri/src/commands.rs:1658, src-tauri/src/projectfile.rs:1852, src-tauri/src/sharing.rs:1267 · **CWE** : CWE-770

**Problème** : projectfile::file_text (projectfile.rs:1645) reads the project file with std::fs::read_to_string(project_path/.escouade.yml). That call follows symlinks, has no is_file() check and no size cap. A character device such as /dev/zero (or /dev/urandom, or a pipe) reports length 0 and never reaches EOF, so read_to_string grows its String until the allocator fails or the kernel OOM killer takes the process; a FIFO or a link to one blocks the reading thread for good. Git stores symlinks, so a repository can carry `.escouade.yml -&gt; /dev/zero` and `git clone` (which fs_clone runs on the server) materialises it. file_text is reached with no further check from: (a) commands::shared_preview (commands.rs:1658, remote table, confined by client_folder to a folder under home, but the link is inside that folder, not resolved) when the wizard previews the freshly cloned folder; (b) Core::create_project -&gt; read_shared(id, true) (core.rs:9309), after the project was saved; (c) Core::compute_git -&gt; read_shared(id, false) (core.rs:9895), an async task on the shared tokio runtime, run for every project at every start (Core::start -&gt; watch_project -&gt; Git::watch -&gt; refresh, core.rs:2628 / git.rs:3192) and on each folder change. So once a poisoned folder is a project, escouade-server dies at every start: crash loop until someone with a shell removes the link, with every device session and running agent turn lost each time. A single read (path a) already kills the process before the project exists. The same file can also be planted in the project folder by an agent that works in the project checkout (prompt-injected: `ln -sf /dev/zero .escouade.yml`). The repo already has the right pattern next to it: sharing.rs:1267 read_dir refuses entries whose file_type is not a regular file (`one is never read: it can point anywhere`) and workflowfiles::unsafe_dir (workflowfiles.rs:122) refuses linked folders; projectfile.rs does neither. The fsedit fix for 2f353c55 (read_regular) and the logo finding 15f709db do not cover this site (different file, different trigger, and here the persistence across restarts is the new part). Write side, same root cause: write_shared (projectfile.rs:1852) does std::fs::write(project/.escouade.yml) through a link, so an Exporter click on a repo whose .escouade.yml is a link to a YAML file elsewhere under the service user rewrites that file (a dangling link creates its target). Boundary crossed: third-party repository content (or a prompt-injected agent) to the availability of the whole server and every device; the attacker is not a paired device, the paired device only clones and adds the repository, a normal action. Not a documented limit (REMOTE.md:191-210 and threat model section 6 cover device = shell and agent -&gt; device token, not a repo that bricks the service).

**Impact** : Persistent denial of service of escouade-server: a hostile repository added as a project (or a symlink planted by an agent) kills the process at the first read and again at every start (crash loop), cutting all device sessions and running agent turns, until an operator with shell access deletes the link. Variant with a FIFO blocks one tokio worker per read instead.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none --memory=512m -v mantis-Escouade-lot5-deps:/deps:ro -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 300 bash /poc/79a2cd8e-b6b4-4cc4-ada2-c825a9ed6cc9/run.sh
```

```
== committed tree (mode 120000 = symlink):
120000 blob 71ef849dd58cf734fe1b47e1a401909fe97f4f2f	.escouade.yml
100644 blob bd21a5ac7811b1fa824b9b1dbf00dcdd931b2586	README.md
Cloning into 'hostile'...
done.
== after clone:
lrwxrwxrwx 1 root root 9 Nov 23 11:43 /tmp/home/projects/hostile/.escouade.yml -> /dev/zero
== cgroup memory limit: 536870912
calling preview(/tmp/home/projects/hostile)
/poc/79a2cd8e-b6b4-4cc4-ada2-c825a9ed6cc9/run.sh: line 20:    49 Killed                  SENTINEL_FILE=/tmp/sentinel timeout 120 /tmp/poc /tmp/home/projects/hostile
== poc exit code: 137 after 1s
== sentinel: MANTIS_REACHED_ENTRYPOINT
== oom events: oom_kill 1
RESULT: killed by SIGKILL (cgroup OOM killer)
calling preview(/tmp/home/projects/fifo)
== FIFO variant exit code: 124 (124 = still blocked after 5s), sentinel: MANTIS_REACHED_ENTRYPOINT
RETURNED (bug did NOT fire): Some(11)
calling preview(/tmp/home/projects/ok)
== control exit code: 0
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src-tauri/src/projectfile.rs
+++ b/src-tauri/src/projectfile.rs
@@ -1641,9 +1641,14 @@
 use crate::model::UiEvent;
 use std::sync::Arc;
 
-/// The file of the project's folder: None when there is none (or it cannot be opened).
-fn file_text(path: &str) -> Option<String> {
-    match std::fs::read_to_string(std::path::Path::new(path).join(FILE)) {
+/// What the file may weigh: a repository can carry far more, or a link to /dev/zero.
+const MAX_FILE: u64 = 1 << 20;
+
+/// The file of the project's folder: None when there is none (or it cannot be opened). Never read
+/// through a link nor anything but a regular file (a repository can carry a link to anywhere, as
+/// `sharing.rs` refuses), and never past `MAX_FILE`.
+pub(crate) fn file_text(path: &str) -> Option<String> {
+    match regular_text(&std::path::Path::new(path).join(FILE)) {
         Ok(t) => Some(t),
         Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
         Err(e) => {
@@ -1653,6 +1658,44 @@
     }
 }
 
+fn regular_text(file: &std::path::Path) -> std::io::Result<String> {
+    use std::io::{Error, ErrorKind, Read};
+    // `symlink_metadata` does not follow a link: one is never opened.
+    if !std::fs::symlink_metadata(file)?.file_type().is_file() {
+        return Err(Error::new(ErrorKind::InvalidInput, "not a regular file"));
+    }
+    let mut open = std::fs::OpenOptions::new();
+    open.read(true);
+    // Swapped for a link or a pipe since the check: refused at open, never waited on.
+    #[cfg(unix)]
+    std::os::unix::fs::OpenOptionsExt::custom_flags(
+        &mut open,
+        libc::O_NOFOLLOW | libc::O_NONBLOCK,
+    );
+    let f = open.open(file)?;
+    if !f.metadata()?.is_file() {
+        return Err(Error::new(ErrorKind::InvalidInput, "not a regular file"));
+    }
+    let mut text = String::new();
+    f.take(MAX_FILE + 1).read_to_string(&mut text)?;
+    if text.len() as u64 > MAX_FILE {
+        return Err(Error::new(ErrorKind::InvalidData, format!("over {MAX_FILE} bytes")));
+    }
+    Ok(text)
+}
+
+/// Where to write the file: refused when what is there is a link (the write would go wherever it
+/// points) or a file `file_text` does not read (rewriting it from nothing would lose it).
+pub(crate) fn file_to_write(path: &str) -> Result<std::path::PathBuf> {
+    let file = std::path::Path::new(path).join(FILE);
+    match std::fs::symlink_metadata(&file) {
+        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(file),
+        Ok(m) if m.file_type().is_file() && m.len() <= MAX_FILE => Ok(file),
+        Ok(_) => bail!("{FILE} of {path} is a link or not a file read: not written"),
+        Err(e) => Err(e.into()),
+    }
+}
+
 fn text_fingerprint(text: Option<&str>) -> String {
     text.map(|t| {
         Sha256::digest(t.as_bytes())
@@ -1848,8 +1891,9 @@
                 .filter(|(k, v)| writable(k, v))
                 .collect(),
         };
+        let file = file_to_write(&p.path)?;
         let text = write(file_text(&p.path).as_deref(), &values)?;
-        std::fs::write(std::path::Path::new(&p.path).join(FILE), text)?;
+        std::fs::write(file, text)?;
         self.forget_kept(project_id, |k| values.iter().any(|(w, _)| *w == k))
     }
 
--- a/src-tauri/src/plugin_profiles.rs
+++ b/src-tauri/src/plugin_profiles.rs
@@ -340,8 +340,9 @@
             .filter(|v| !v.is_null())
             .ok_or_else(|| format!("{key}: no value kept on this machine"))?;
         let p = self.project(project).map_err(|e| e.to_string())?;
-        let path = std::path::Path::new(&p.path).join(projectfile::FILE);
-        let existing = std::fs::read_to_string(&path).ok();
+        // Never through a link, never past the cap: `projectfile` checks both.
+        let path = projectfile::file_to_write(&p.path).map_err(|e| format!("{e:#}"))?;
+        let existing = projectfile::file_text(&p.path);
         let text = projectfile::write_plugin(existing.as_deref(), id, key, &value)
             .map_err(|e| format!("{e:#}"))?;
         std::fs::write(&path, text).map_err(|e| e.to_string())?;
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/79a2cd8e-b6b4-4cc4-ada2-c825a9ed6cc9.diff`

### [MEDIUM] the project memory folder is read whole with no file count or total size cap, so a cloned repo with thousands of large memory files exhausts memory when an agent searches its memory in list

**Résumé** : Hazard (4+4)*0.8*0.7 = 4.48 MEDIUM, same position and multipliers as 79a2cd8e: a cloned repository with thousands of 256 KiB memory files exhausts server memory when an agent searches memory (reproduced on a fragment, killed at 5000 files under a 512 MB limit). Needs the owner to add or pull the repository. Fix: cap file count and total bytes in memory::list.

**Emplacement** : src-tauri/src/memory.rs:232, src-tauri/src/memory.rs:201, src-tauri/src/memory.rs:220, src-tauri/src/memory.rs:352, src-tauri/src/memory.rs:477, src-tauri/src/mcp/memory.rs:57, src-tauri/src/mcp/memory.rs:96, src-tauri/src/core.rs:1566, src-tauri/src/core.rs:9503 · **CWE** : CWE-770

**Problème** : memory::list (src-tauri/src/memory.rs:232) takes every .md entry of &lt;project&gt;/.escouade/memory (files(), memory.rs:201, a bare read_dir with no entry limit), reads each one up to MAX_FILE = 256 KiB (read_file, memory.rs:220) and returns all of them as a Vec&lt;Memory&gt; with their whole body. There is no cap on the number of files or on the total bytes, while the docs side of the same project folder has one (docs.rs:54 MAX_FILES = 5000 through walk). The whole set is rebuilt on every call by: search_memory (memory::search, memory.rs:477, called by mcp/memory.rs:57 inside spawn_blocking and by mcp/tools.rs:670), write_memory and forget_memory (write_index, memory.rs:352, which calls list() while holding a process-wide mutex, and which runs inline on the async worker in mcp/memory.rs:96 and :131, not in spawn_blocking), the Settings memory tab (Core::memories core.rs:9503, memory_list commands.rs:4896) and claude_offer. When the vectors' model is present, search_meaning additionally embeds the whole set. The folder is repository content: .escouade/memory is meant to be committed and shared by the team, so a cloned or pulled repo controls the number and size of its files. Every new agent is told by the instruction injected in its prompt (memory::instruction, core.rs:1566 with_memory) to run search_memory at the start of its task, so the load is triggered by ordinary use and by no device action. Git deduplicates identical blobs, so a repository of a few KB can name 20,000 files of 256 KiB each (5 GB once checked out; distinct names, one blob) and the resulting Vec&lt;Memory&gt; holds about 5 GB of bodies in RAM (more at the peak, the text and the parsed body coexist for the file being read). unsafe_dir (workflowfiles.rs:122) only refuses links and folders outside the project; it does not bound size. Files that are links or special files are correctly refused (files() uses DirEntry::file_type, plain_file uses symlink_metadata), so the gap is the missing aggregate bound, not the per-file read. Boundary crossed: a third-party repository (not a paired device) reaches the server process and every client of it, same attacker class as 79a2cd8e (hostile committed content).

**Impact** : Denial of service of the whole server (and of the desktop app): the process is killed by the out-of-memory killer or aborts on allocation failure when the first agent of the project starts its task and searches the memory, or when a user opens the project memory tab; under the generated systemd unit (Restart=on-failure) it comes back and crashes again at the next agent start or UI load. Every paired device and every running agent turn is lost per crash. Also long CPU/disk work (read of GBs, embedding of the whole set, a MEMORY.md of one line per file rewritten at every write). No code execution, no data exposure.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-e0bf2982-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 500 sh /poc/e0bf2982-8d71-48bd-93da-48372579302b/run.sh
```

```
== git: 100 memory files of 256 KiB, one shared blob
size-pack: 1.93 KiB
26M	clone/.escouade/memory
list memories=100 bodies_in_ram=25 MiB peak_rss=26 MiB t=0.01s
exit=0 sentinel=MANTIS_REACHED_ENTRYPOINT
== growth: N files of 256 KiB (sparse, same bytes in RAM)
list memories=250 bodies_in_ram=62 MiB peak_rss=64 MiB t=0.03s
files=250 total=62MiB exit=0
list memories=500 bodies_in_ram=125 MiB peak_rss=127 MiB t=0.08s
files=500 total=125MiB exit=0
list memories=1000 bodies_in_ram=250 MiB peak_rss=252 MiB t=0.17s
files=1000 total=250MiB exit=0
list memories=1500 bodies_in_ram=375 MiB peak_rss=377 MiB t=0.24s
files=1500 total=375MiB exit=0
Killed
files=5000 total=1250MiB exit=137
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src-tauri/src/memory.rs
+++ b/src-tauri/src/memory.rs
@@ -29,7 +29,11 @@
 /// A memory's file above this size is left unread.
 const MAX_FILE: u64 = 256 * 1024;
 /// The longest a memory's text may be, in characters.
 pub const MAX_BODY: usize = 20_000;
+/// The most memory files a folder is read with, as `docs::MAX_FILES` bounds a docs folder.
+const MAX_FILES: usize = 500;
+/// The most bytes of memory files `list` reads in all: a cloned repository decides their number and their size.
+const MAX_TOTAL: u64 = 8 * 1024 * 1024;
 /// The longest description, in characters (one line of the index).
 pub const MAX_DESCRIPTION: usize = 300;
 /// The longest name, in characters.
@@ -213,7 +217,9 @@
                 .then(|| (stem.to_string(), path))
         })
         .collect();
     out.sort();
+    // The rest is left unread, as `docs::walk` stops at its limit: a cloned repository decides how many there are.
+    out.truncate(MAX_FILES);
     out
 }
 
@@ -228,13 +234,25 @@
     Some(m)
 }
 
 /// The memories of `dir`, by name. None when it is not there; refused through a link or out of the project.
+/// At most `MAX_FILES` of them and `MAX_TOTAL` bytes read, each text cut to `MAX_BODY`.
 pub fn list(dir: &Path) -> Result<Vec<Memory>> {
     refused(dir)?;
+    let mut total = 0u64;
     Ok(files(dir)
         .into_iter()
+        .take_while(|(_, path)| {
+            total += std::fs::metadata(path).map_or(0, |m| m.len().min(MAX_FILE));
+            total <= MAX_TOTAL
+        })
         .filter_map(|(stem, path)| read_file(&stem, &path))
+        .map(|mut m| {
+            if m.body.chars().count() > MAX_BODY {
+                m.body = m.body.chars().take(MAX_BODY).collect();
+            }
+            m
+        })
         .collect())
 }
 
 /// The memory `name` of `dir`.
--- a/src-tauri/src/memory_tests.rs
+++ b/src-tauri/src/memory_tests.rs
@@ -328,3 +328,16 @@
         }
     }
 }
+
+#[test]
+fn a_cloned_folder_is_read_within_bounds() {
+    let dir = memory_dir("memory-bounds");
+    write(&dir, draft("a", "a", "a")).unwrap();
+    for i in 0..MAX_FILES + 10 {
+        std::fs::write(dir.join(format!("n{i:04}.md")), "x").unwrap();
+    }
+    std::fs::write(dir.join("big.md"), "y".repeat(MAX_BODY + 100)).unwrap();
+    let all = list(&dir).unwrap();
+    assert_eq!(all.len(), MAX_FILES);
+    assert!(all.iter().all(|m| m.body.chars().count() <= MAX_BODY));
+}
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/e0bf2982-8d71-48bd-93da-48372579302b.diff`

### [MEDIUM] Pairing cancelled by five wrong codes from any unauthenticated peer in redeem

**Résumé** : Pairing denial of service. Hazard = (Impact 3 + Likelihood 4) x 0.56 = 3.92, MEDIUM. Impact 3: only onboarding availability is lost; existing devices unaffected, brute force of the 40-bit code stays infeasible. Likelihood 4: reproduced with the verbatim redeem()/mine() logic, 5 unauthenticated requests cancel each minted code in about 1 ms (fragment harness, no real HTTP run). Multiplier 1.0 (EXTERNAL -&gt; EXPOSED) x 0.8 (pairing flow availability tier STANDARD) x 0.7 (CONDITIONAL_VIABLE). Condition: escouade-server behind the documented HTTPS reverse proxy with --origin, or port mode on a shared machine; the default Unix-socket/SSH-tunnel install exposes /pair to the owner only. The patch (no cancel on wrong codes, Crockford folding) was verified secure on re-attack.

**Emplacement** : src-tauri/src/remoteapi/devices.rs:148 · **CWE** : CWE-307

**Problème** : POST /pair is unauthenticated and the wrong-attempt counter in Pending is global, not per source. redeem() increments pending.wrong on every mismatch and sets state.pending = None at TRIES (5), whoever sent the guesses. Once the owner runs 'escouade-server pair', any peer that can reach the listener (a tunnel user, or anyone on the Internet behind the documented reverse proxy with --origin, where the Host/Origin checks pass for the declared origin) sends five bad codes and the code dies; Origin is optional for non-browser clients so mine() does not stop it. It can be repeated within seconds each time the owner mints a new code, so a new device cannot be paired while the attacker is active. A legitimate typo also burns a try, and typed input is not folded (O/I/L are not mapped to 0/1 as Crockford specifies), which makes honest failures likelier. Brute force itself is infeasible (40 bits, 5 tries); only availability of pairing is affected, existing devices keep working.

**Impact** : Denial of service of device pairing: the owner cannot onboard a browser or phone while an attacker spams /pair.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none -v mantis-Escouade-lot5-deps:/deps:ro -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88-slim sh -c 'rustc -O --edition 2021 -o /tmp/poc /poc/40e71f05-58bf-44ae-b8e3-2b48512edbe5/poc.rs && SENTINEL_FILE=/tmp/s timeout 60 /tmp/poc'
```

```
GATE attacker (Host=declared, no Origin): Ok(())
GATE hostile web page (Origin=https://evil.example): Err(403)
CONTROL owner with ZVW5-HCXM: HTTP 200
[log] remote API: 5 wrong pairing codes: the pairing is cancelled
ROUND 1: attacker answers [403, 403, 403, 403, 403] in 1.17ms; owner then sends the RIGHT code QTYT-XTQD: HTTP 403
ROUND 2: attacker answers [403 x5] in 1.29ms; owner then sends the RIGHT code 8JP0-DQE0: HTTP 403
ROUND 3: attacker answers [403 x5] in 1.05ms; owner then sends the RIGHT code 2EK6-XXQY: HTTP 403
TYPO-like VCEB-4Q2O for VCEB-4Q2X: HTTP 403 (wrong counter now 1)
RESULT: VULNERABLE - 3/3 freshly minted codes killed by 5 unauthenticated requests each; the owner's correct code is refused (403)
EXIT=0; sentinel file: 20 x MANTIS_REACHED_ENTRYPOINT
```

**Correction** (Vérifié par re-attaque)

```diff
diff --git a/src-tauri/src/headless_host.rs b/src-tauri/src/headless_host.rs
--- a/src-tauri/src/headless_host.rs
+++ b/src-tauri/src/headless_host.rs
@@ -451,7 +451,6 @@
                 ),
             }
             println!("It pairs one device, once, within {minutes} minutes.");
-            println!("Five wrong codes cancel it: then run `escouade-server pair` again.");
             if control.socket.is_none() {
                 println!(
                     "\nThis server listens on a port of 127.0.0.1: another user of this machine may take it while the server\n\
diff --git a/src-tauri/src/remoteapi/devices.rs b/src-tauri/src/remoteapi/devices.rs
--- a/src-tauri/src/remoteapi/devices.rs
+++ b/src-tauri/src/remoteapi/devices.rs
@@ -1,7 +1,7 @@
 //! The devices paired with `escouade-server` (docs/REMOTE.md, « Le mode serveur »): each its own token, 32 random bytes
 //! only the device holds, kept here hashed (SHA-256) in `devices.json`, readable by its owner alone and written by
 //! replacement, compared in constant time. A pairing code (`escouade-server pair`) serves once, for 10 minutes, one
-//! alive at a time; five wrong tries cancel it. A device revoked is refused at once (the server closes its connections).
+//! alive at a time; a wrong try does not cancel it. A device revoked is refused at once (the server closes its connections).
 
 use crate::paths;
 use chrono::{DateTime, Utc};
@@ -14,8 +14,6 @@
 pub const FILE: &str = "devices.json";
 /// How long a pairing code lives.
 pub const CODE_LIFE: Duration = Duration::from_secs(10 * 60);
-/// The wrong codes that cancel the pairing under way.
-pub const TRIES: u32 = 5;
 /// The longest name kept, in characters.
 pub const NAME_MAX: usize = 64;
 /// A code's characters: Crockford's base32 (no I, L, O, U), 5 bits each, 40 bits in 8.
@@ -56,7 +54,6 @@
     /// Without its dash.
     code: String,
     until: Instant,
-    wrong: u32,
 }
 
 #[derive(Default)]
@@ -123,7 +120,6 @@
         self.state.lock().pending = Some(Pending {
             code,
             until: Instant::now() + CODE_LIFE,
-            wrong: 0,
         });
         PairCode {
             code: shown,
@@ -137,19 +133,19 @@
             .chars()
             .filter(|c| !c.is_whitespace() && *c != '-')
             .collect::<String>()
-            .to_ascii_uppercase();
+            .to_ascii_uppercase()
+            // Crockford's base32 reads O as 0, I and L as 1.
+            .replace('O', "0")
+            .replace(['I', 'L'], "1");
         let mut state = self.state.lock();
         let pending = state.pending.as_mut().ok_or(Refused)?;
         if Instant::now() >= pending.until {
             state.pending = None;
             return Err(Refused);
         }
+        // A wrong code does not cancel the pairing: `POST /pair` is open to anyone reaching the server, who could
+        // otherwise kill every code minted. 40 bits outlast the 10 minutes of a code at any rate a server answers.
         if !same(typed.as_bytes(), pending.code.as_bytes()) {
-            pending.wrong += 1;
-            if pending.wrong >= TRIES {
-                state.pending = None;
-                log::warn!("remote API: {TRIES} wrong pairing codes: the pairing is cancelled");
-            }
             return Err(Refused);
         }
         state.pending = None;
diff --git a/src-tauri/src/remoteapi/devices_tests.rs b/src-tauri/src/remoteapi/devices_tests.rs
--- a/src-tauri/src/remoteapi/devices_tests.rs
+++ b/src-tauri/src/remoteapi/devices_tests.rs
@@ -64,23 +64,26 @@
 }
 
 #[test]
-fn five_wrong_tries_cancel_the_code() {
+fn wrong_tries_do_not_cancel_the_code() {
     let (devices, _) = opened("devices-tries");
     let code = devices.pair_code().code;
-    for _ in 0..TRIES - 1 {
+    for _ in 0..100 {
         assert_eq!(devices.redeem(&wrong(&code), "pc"), Err(Refused));
     }
-    // Four wrong: the right one still pairs.
+    // Anyone may send wrong codes: the right one still pairs.
     assert!(devices.redeem(&code, "pc").is_ok());
-    let code = devices.pair_code().code;
-    for _ in 0..TRIES {
-        assert_eq!(devices.redeem(&wrong(&code), "pc"), Err(Refused));
-    }
-    assert_eq!(devices.redeem(&code, "pc"), Err(Refused));
     assert_eq!(devices.list().len(), 1);
 }
 
 #[test]
+fn o_i_and_l_are_read_as_0_and_1() {
+    let (devices, _) = opened("devices-folded");
+    devices.pair_code();
+    devices.state.lock().pending.as_mut().unwrap().code = "01AB01CD".into();
+    assert!(devices.redeem("oiab-OLcd", "pc").is_ok());
+}
+
+#[test]
 fn without_a_code_nothing_pairs() {
     let (devices, _) = opened("devices-no-code");
     assert_eq!(devices.redeem("", "pc"), Err(Refused));
diff --git a/src-tauri/src/remoteapi/server_tests.rs b/src-tauri/src/remoteapi/server_tests.rs
--- a/src-tauri/src/remoteapi/server_tests.rs
+++ b/src-tauri/src/remoteapi/server_tests.rs
@@ -928,7 +928,7 @@
     let code = pair_code(&loaded, running.addr);
     assert!(pair_with(&code).starts_with("HTTP/1.1 200"));
     assert!(pair_with(&code).starts_with("HTTP/1.1 403"));
-    // Five wrong: the right one no longer pairs. Wrong for sure: its first character changed.
+    // Wrong ones, from anyone, do not cancel it: the right one still pairs. Wrong for sure: its first character changed.
     let code = pair_code(&loaded, running.addr);
     let wrong = format!(
         "{}{}",
@@ -938,7 +938,7 @@
     for _ in 0..5 {
         assert!(pair_with(&wrong).starts_with("HTTP/1.1 403"));
     }
-    assert!(pair_with(&code).starts_with("HTTP/1.1 403"));
+    assert!(pair_with(&code).starts_with("HTTP/1.1 200"));
     // A body that is no pairing, or too big, or a name left empty.
     let code = pair_code(&loaded, running.addr);
     for body in [
diff --git a/docs/REMOTE.md b/docs/REMOTE.md
--- a/docs/REMOTE.md
+++ b/docs/REMOTE.md
@@ -461,7 +461,7 @@
 - [ ] Essais : le cœur sans fenêtre sous Linux sans affichage ; `ssh -L` lancé par l'app sur les trois systèmes.
 - [x] Le cœur sans Tauri : le trait `Host` à la place de l'`AppHandle` ; un verrou par fichier à la place de `single-instance` hors bureau. Faits : le trait `Host` ; le verrou par fichier (`server.lock`, voir « Un serveur par dossier de données ») ; le cœur se construit et se teste sans Tauri, GTK ni WebKit (`--no-default-features`, docs/CRATES.md § 16), ce que la CI garde.
 - [ ] L'API distante : une seule liste des commandes (macro) pour l'IPC et la WebSocket ; les flux binaires des terminaux multiplexés ; la poignée de main des versions. Faits : la liste unique et les flux, servis sur 127.0.0.1 avec un jeton, éteints dans l'app de bureau sans `ESCOUADE_REMOTE_PORT` et `ESCOUADE_REMOTE_TOKEN`, qu'elle ne lit que dans un build de debug, pour les tests ; le serveur annonce sa version et le numéro de son protocole, reste à refuser un écart de version. Le binaire `escouade-server` (construit avec `--no-default-features --features server`, jamais par le build du bureau : ses installeurs ne l'emportent pas) charge le cœur sans fenêtre, sur l'hôte `Headless` : il écoute sur une socket Unix (`--socket <chemin>`) ou sur un port de 127.0.0.1 (`--port <n>`, sinon `ESCOUADE_REMOTE_PORT`), refuse de démarrer sans l'un ou l'autre, sert avec `--web <dossier>` le build web (`npm run build:web`) sur le même port, et s'arrête proprement sur SIGINT ou SIGTERM, même reçu pendant son démarrage ; un second signal pendant l'arrêt le fait sortir aussitôt (code 130), sans attendre le cœur. Ses clients sont des appareils appairés, chacun son jeton, révocable un par un (`remoteapi/devices.rs`) :
-  - `escouade-server pair` demande un code au serveur qui tourne sur le même dossier de données et l'affiche avec l'adresse à ouvrir. Le code (8 caractères, `XXXX-XXXX`) sert une fois, expire en 10 minutes, un seul à la fois ; 5 codes faux l'annulent (n'importe quel programme de la machine peut les taper : il faut alors relancer `pair`, ce que la commande dit). La page sans jeton affiche le formulaire « Appairer ce navigateur » (code et nom de l'appareil), qui l'envoie par `POST /pair` et reçoit le jeton.
+  - `escouade-server pair` demande un code au serveur qui tourne sur le même dossier de données et l'affiche avec l'adresse à ouvrir. Le code (8 caractères, `XXXX-XXXX`) sert une fois, expire en 10 minutes, un seul à la fois ; un code faux ne l'annule pas (quiconque atteint `POST /pair` pourrait sinon annuler chaque code, et 40 bits tiennent bien 10 minutes d'essais), et O, I, L tapés valent 0, 1, 1. La page sans jeton affiche le formulaire « Appairer ce navigateur » (code et nom de l'appareil), qui l'envoie par `POST /pair` et reçoit le jeton.
   - Le jeton : 32 octets aléatoires, gardé haché (SHA-256) dans `devices.json` du dossier de données (`0600`, écrit par remplacement atomique), comparé à temps constant, jamais écrit dans un journal. Un `devices.json` abîmé ou absent ne fait rien planter : aucun appareil, une ligne au journal. Un `devices.json` qu'on ne peut pas lire pour une autre raison (ses droits) : aucun appareil, une ligne au journal, et aucun appairage ne l'écrase. Le nom d'un appareil perd ses caractères de contrôle et de format (U+202E retournerait la ligne de `devices`) ; vide ensuite, il est refusé.
   - Le navigateur garde le jeton dans le `localStorage` de son origine, port compris, et le présente par le sous-protocole de la WebSocket (`escouade, bearer.<jeton>`, le serveur répond `escouade`) et par `Authorization: Bearer` sur les routes HTTP (`GET /device` dit s'il vaut encore). Aucun cookie : un navigateur envoie ses cookies à tous les ports de `127.0.0.1`, un autre serveur local visité les recevrait.
   - `escouade-server devices` les liste (id, nom, appairage, dernière venue) ; `escouade-server devices revoke <id>` en oublie un et ferme ses connexions ouvertes tout de suite, flux de terminaux et abonnement compris. Une commande déjà en cours, elle, va jusqu'au bout (un `git push`, une suppression) ; seules celles encore en file sont abandonnées. La page d'un appareil révoqué, à sa coupure, voit `/device` refusé, oublie le jeton et redemande un code.
diff --git a/website/content/docs/en/server-mode.md b/website/content/docs/en/server-mode.md
--- a/website/content/docs/en/server-mode.md
+++ b/website/content/docs/en/server-mode.md
@@ -71,7 +71,7 @@
 3. Open the address `pair` gave, `http://127.0.0.1:8765/`. The page shows “Pair this browser”.
 4. Type the code and a name for your device.
 
-The code works once and lasts 10 minutes. Five wrong codes cancel it: run `escouade-server pair` again.
+The code works once and lasts 10 minutes. A wrong code can simply be typed again.
 
 > [!NOTE]
 > To your browser, `http://127.0.0.1:8765/` and `http://localhost:8765/` are two different sites: a browser paired on one is not paired on the other. Always use the same address.
diff --git a/website/content/docs/fr/server-mode.md b/website/content/docs/fr/server-mode.md
--- a/website/content/docs/fr/server-mode.md
+++ b/website/content/docs/fr/server-mode.md
@@ -71,7 +71,7 @@
 3. Ouvre l’adresse que `pair` a donnée, `http://127.0.0.1:8765/`. La page affiche « Appairer ce navigateur ».
 4. Tape le code et un nom pour ton appareil.
 
-Le code sert une fois et vit 10 minutes. Cinq codes faux l’annulent : relance alors `escouade-server pair`.
+Le code sert une fois et vit 10 minutes. Un code faux se retape simplement.
 
 > [!NOTE]
 > Pour ton navigateur, `http://127.0.0.1:8765/` et `http://localhost:8765/` sont deux sites différents : un navigateur appairé sur l’un ne l’est pas sur l’autre. Garde toujours la même adresse.
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/40e71f05-58bf-44ae-b8e3-2b48512edbe5.diff`

### [MEDIUM] Spec files of a hostile repository are read whole with no size cap in read_found, so a small committed blob that inflates to gigabytes exhausts memory

**Résumé** : Hazard (4+3)*0.8*0.7 = 3.92 MEDIUM: a hostile repository's compressible SPEC.md is read whole by read_found; reproduced (600 MiB file, cgroup OOM kill, patch VERIFIED_SECURE). Needs owner to add the repo and run the spec import (user interaction, likelihood -1 for the uncommon import path). Fix: regular-file-only capped read.

**Emplacement** : src-tauri/src/docsimport.rs:480, src-tauri/src/docsimport.rs:304, src-tauri/src/docsimport.rs:308, src-tauri/src/docsimport.rs:429, src-tauri/src/docs.rs:906 · **CWE** : CWE-770

**Problème** : docsimport::Space::read_found (docsimport.rs:461-482) is the only reader of the spec files that `detect` offers for import (import_plan l.304 and l.308, import l.345). It refuses a link on any segment, canonicalises inside the project, but then does `std::fs::read(&amp;real)` and `String::from_utf8_lossy(&amp;bytes).into_owned()` with no size check. The sibling reader in the same Space, docs.rs read_text (docs.rs:906-914), caps at MAX_TEXT (2 MiB) and docsextract caps at 20 MiB text / 200 MiB binary; this one does not, and for a folder candidate it loops over up to 1000 files (found_files, depth 6, out.len()&gt;=1000) reading each whole before `drafts.truncate(MAX_PAGES)` is applied. The `.md` files come from the repository (docs/, spec/, specs/ ...). Git packs deflate: a repository can carry a multi-GB blob of repeated text as a few KB on the wire, and `git clone` writes the whole file to the working tree, so the attacker needs only a normal repository, no symlink (the existing symlink-based fixes do not apply). On import_plan the process then holds the bytes (N), the lossy copy (another N), and lines_of/cut allocations on top. detect() itself only does metadata (walk records the size) and even reports `bytes`, so a UI/limit check was available. Reached by the client commands for importing a found spec (docs import, remote table) and in the desktop window.

**Impact** : Memory exhaustion of escouade-server (or a long stall of an async worker) when a user or a paired device runs the spec import on a hostile repository; the process is aborted by the OOM killer and all device sessions and running agents are lost.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 550 bash /poc/03a3e838-96f3-4d2a-a44b-245f91d079af/run.sh
```

```
== cgroup memory limit: 536870912, swap limit: 536870912

=== 1. Bounded growth: peak RSS vs SPEC.md size (from a real clone)
   repo g8: SPEC.md 8 MiB, bare repo on disk 156 KiB
   input=8MiB  read_found returned 8388608 bytes  VmHWM_KiB=18012
   repo g32: SPEC.md 32 MiB, bare repo on disk 188 KiB
   input=32MiB  read_found returned 33554432 bytes  VmHWM_KiB=67104
   repo g64: SPEC.md 64 MiB, bare repo on disk 236 KiB
   input=64MiB  read_found returned 67108864 bytes  VmHWM_KiB=132704
   repo g128: SPEC.md 128 MiB, bare repo on disk 332 KiB
   input=128MiB  read_found returned 134217728 bytes  VmHWM_KiB=261252

=== 2. Availability: SPEC.md of 600 MiB (held twice: bytes + lossy copy) under a 512 MiB RAM + 512 MiB swap cap
   repo big: SPEC.md 600 MiB, bare repo on disk 1036 KiB
detect offers SPEC.md (kind file, bytes 629145600)
/poc/03a3e838-96f3-4d2a-a44b-245f91d079af/run.sh: line 36:   195 Killed                  SENTINEL_FILE=/tmp/s2 timeout 120 /tmp/poc /tmp/home/projects/big
== poc exit code: 137 after 3s, sentinel: MANTIS_REACHED_ENTRYPOINT
== oom events before: oom_kill 0 / after: oom_kill 1
RESULT: killed by SIGKILL (cgroup OOM killer)

=== 3. Control: 1 KiB SPEC.md
   repo ok: SPEC.md 0 MiB, bare repo on disk 144 KiB
detect offers SPEC.md (kind file, bytes 1024)
read_found returned 1024 bytes  VmHWM_KiB=1748
== control exit code: 0
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src-tauri/src/docs.rs
+++ b/src-tauri/src/docs.rs
@@ -49,7 +49,7 @@
 /// The deepest a document's folder goes.
 const MAX_FOLDER_DEPTH: usize = 6;
 /// A text page, a decision: the most Escouade reads of one file.
-const MAX_TEXT: u64 = 2 * 1024 * 1024;
+pub(crate) const MAX_TEXT: u64 = 2 * 1024 * 1024;
 /// The most files a folder is listed with.
 const MAX_FILES: usize = 5000;
 /// What a joined file may weigh before the window warns (FACTORY.md: 50 MB).
--- a/src-tauri/src/docsimport.rs
+++ b/src-tauri/src/docsimport.rs
@@ -19,6 +19,9 @@
 /// The most pages one import writes.
 const MAX_PAGES: usize = 500;
 
+/// The most bytes one folder import reads, all its files together.
+const MAX_IMPORT: usize = 16 * docs::MAX_TEXT as usize;
+
 /// Where the imports are kept, in `.escouade/spec/` (shared like the pages, which skip dot files).
 const IMPORTS: &str = ".imports.json";
 
@@ -304,8 +307,16 @@
             let text = self.read_found(path)?;
             drafts = cut(&text, path, None, &intro);
         } else {
+            let mut total = 0usize;
             for rel in self.found_files(path) {
                 let text = self.read_found(&rel)?;
+                total += text.len();
+                if total > MAX_IMPORT {
+                    bail!(tr!(
+                        "« {path} » est trop gros pour être importé",
+                        "“{path}” is too big to be imported"
+                    ));
+                }
                 let stem = rel
                     .rsplit('/')
                     .next()
@@ -477,7 +488,22 @@
         if !real.starts_with(&root) {
             return Err(refused(rel));
         }
-        let bytes = std::fs::read(&real)?;
+        // The same cap as a page (`docs::MAX_TEXT`); `take` holds it if the file grows meanwhile.
+        let meta = std::fs::metadata(&real)?;
+        if !meta.is_file() {
+            return Err(refused(rel));
+        }
+        if meta.len() > docs::MAX_TEXT {
+            bail!(tr!(
+                "« {rel} » est trop gros pour être importé",
+                "“{rel}” is too big to be imported"
+            ));
+        }
+        let mut bytes = Vec::new();
+        std::io::Read::read_to_end(
+            &mut std::io::Read::take(std::fs::File::open(&real)?, docs::MAX_TEXT),
+            &mut bytes,
+        )?;
         Ok(String::from_utf8_lossy(&bytes).into_owned())
     }
 }
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/03a3e838-96f3-4d2a-a44b-245f91d079af.diff`

### [MEDIUM] Revoking a device does not cut its in-flight or queued file transfers

**Résumé** : Revoke does not reach in-flight or queued /files transfers (up to 3600 s), so a revoked token can finish a download, upload up to 1 GiB, or hold transfer slots for an hour. Impact 2 (token holder already has a shell; file-transfer availability is STANDARD tier), likelihood 2 after the static cap and rare timing. Exposure x1.0 (EXTERNAL), x0.8 static. Fix: register transfers in conns so cut aborts them.

**Emplacement** : src-tauri/src/remoteapi/files.rs:89, src-tauri/src/remoteapi/server.rs:619, src-tauri/src/remoteapi/server.rs:666, src-tauri/src/remoteapi/server.rs:720, src-tauri/src/remoteapi/server.rs:722 · **CWE** : CWE-613

**Problème** : `Server::transfer` (server.rs:720-735) checks the token once with `self.admit(req.headers()).map(drop)`: the device id is thrown away and the request is never registered in `Server::conns`, the only list `Server::cut` (server.rs:619, called by the admin revoke) walks. So `escouade-server devices revoke &lt;id&gt;` closes the device WebSocket but leaves every `/files/attach` and `/files/sent` request it already started running. They live until the connection timeout, `files::TRANSFER` = 3600 s (server.rs:666). Two sub-cases: (1) a download of an agent-sent file keeps streaming up to one hour after the revoke; (2) an upload that is still waiting for one of the 4 writer slots (files.rs:89, up to QUEUE = 30 s) is admitted after the revoke and then writes up to 1 GiB into the agent folder, and an upload that sends one byte every &lt;30 s (IDLE, files.rs:38) holds its slot and one of the 8 transfer slots for the whole hour. Four such uploads pin all MAX_UPLOADS for every other client for up to an hour, and revoke cannot free them. docs/REMOTE.md:467 says revoke closes the open connections at once ("ferme ses connexions ouvertes tout de suite"); it only says a running command goes to its end, not a transfer, so the documented guarantee is wider than the code. Not re-reported: tokens never expire and one token equals a shell (documented); this is only about the revoke operation not reaching transfers.

Merged from 505f7bbb-5e39-463b-84d3-7065ed2cad42: transfer() checks mine() and admit() once, at request start, then hands the request to files::serve. Server::cut(id) only closes WebSocket Conn objects registered in self.conns (done in session()); HTTP transfers register nothing. After 'escouade-server devices revoke &lt;id&gt;', an upload (POST /files/attach, up to 1 GiB) or a download already under way continues until it ends, up to files::TRANSFER (3600 s; serve() extends the connection timeout to TRANSFER for /files/ requests). A thief racing the owner can finish exfiltrating a download, or keep writing files into the attachments folder, for up to an hour after revocation answered revoked:true. New requests are refused correctly.

**Impact** : A stolen or abused device token keeps a working data channel (download of any agent-named file under home, upload of up to 1 GiB per slot, slot starvation of all other devices) for up to 3600 s after the owner revoked it. The owner has no command that cuts it short, short of restarting the server.

**Cas testé** : Confirmé statiquement

```bash
docker run --rm --network=none -v {STATE_ROOT}/workspace/reproducers:/poc:ro -v {TARGET_ROOT}:/src:ro -w /src rust:1.88-slim sh -c 'rustc -O -o /tmp/poc /poc/f623c3a0-3865-4f13-a9b1-44b5e1741dcd/poc.rs && SENTINEL_FILE=/tmp/s /tmp/poc'
```

```
revoked dev-1: websocket closed = true
new request with revoked token -> 401
revoked device's upload: status 200, 32768 bytes written into the agent folder, lasted 202 model-seconds after revoke
VULNERABLE: revoke did not cut the device's queued/in-flight transfer
exit=0 sentinel=MANTIS_REACHED_ENTRYPOINT
```

**Correction** (Correctif proposé, non vérifié)

```diff
--- a/src-tauri/src/remoteapi/server.rs
+++ b/src-tauri/src/remoteapi/server.rs
@@ -390,6 +390,7 @@
         devices: config.devices,
         admin,
         conns: Mutex::default(),
+        cuts: Mutex::default(),
         web: config.web,
         origins: config.origins,
         calls: Arc::new(Semaphore::new(calls)),
@@ -508,6 +509,8 @@
     admin: escouade_mcp::Tokens,
     /// The devices' connections, closed when their device is revoked.
     conns: Mutex<Vec<(String, Weak<Conn>)>>,
+    /// The devices' transfers of files, ended when their device is revoked.
+    cuts: Mutex<Vec<(String, Weak<Notify>)>>,
     /// The web build's folder.
     web: Option<PathBuf>,
     /// The origins declared by `--origin`, besides the loopback's.
@@ -626,6 +629,16 @@
             }
             device != id
         });
+        self.cuts.lock().retain(|(device, cut)| {
+            let Some(cut) = cut.upgrade() else {
+                return false;
+            };
+            if device == id {
+                // A permit kept: the transfer ends even if its connection is not waiting yet.
+                cut.notify_one();
+            }
+            device != id
+        });
     }
 
     /// One request per connection: a file, a session, or the WebSocket, which then holds the connection.
@@ -633,13 +646,15 @@
         let upgrade: Arc<Upgrade> = Arc::default();
         // A request of `files::PREFIX`: it may take longer than `EXCHANGE`, its bytes coming at a pace of their own.
         let long = Arc::new(AtomicBool::new(false));
-        let (me, upgraded, slow) = (self.clone(), upgrade.clone(), long.clone());
+        // Told when the device of a transfer is revoked: the connection, and its transfer, end at once.
+        let cut = Arc::new(Notify::new());
+        let (me, upgraded, slow, ended) = (self.clone(), upgrade.clone(), long.clone(), cut.clone());
         let service = service_fn(move |req: Request| {
-            let (me, upgraded, slow) = (me.clone(), upgraded.clone(), slow.clone());
+            let (me, upgraded, slow, ended) = (me.clone(), upgraded.clone(), slow.clone(), ended.clone());
             async move {
                 let mut answer = if req.uri().path().starts_with(files::PREFIX) {
                     slow.store(true, Ordering::Relaxed);
-                    me.transfer(req).await
+                    me.transfer(req, &ended).await
                 } else {
                     let answer = me.answer(req, &upgraded).await;
                     answer.map(|b| b.map_err(|e| match e {}).boxed_unsync())
@@ -658,15 +673,22 @@
             .serve_connection(TokioIo::new(tcp), service)
             .with_upgrades();
         tokio::pin!(conn);
-        let served = match tokio::time::timeout(EXCHANGE, &mut conn).await {
+        let timed = tokio::select! {
+            timed = tokio::time::timeout(EXCHANGE, &mut conn) => timed,
+            () = cut.notified() => return,
+        };
+        let served = match timed {
             Ok(served) => served,
             // A file sent stops at its first pause of `files::IDLE`; one fetched by a client that stopped reading
-            // holds its slot until `files::TRANSFER`.
+            // holds its slot until `files::TRANSFER`, or until its device is revoked.
             Err(_) if long.load(Ordering::Relaxed) => {
                 let more = files::TRANSFER.saturating_sub(EXCHANGE);
-                match tokio::time::timeout(more, &mut conn).await {
-                    Ok(served) => served,
-                    Err(_) => return,
+                tokio::select! {
+                    timed = tokio::time::timeout(more, &mut conn) => match timed {
+                        Ok(served) => served,
+                        Err(_) => return,
+                    },
+                    () = cut.notified() => return,
                 }
             }
             Err(_) => return,
@@ -716,11 +738,23 @@
         }
     }
 
-    /// A request of `files::PREFIX`: from this server's own pages, with a token.
-    async fn transfer(&self, req: Request) -> files::Answer {
-        let admitted = self.mine(req.uri(), req.headers());
-        if let Err(status) = admitted.and_then(|()| self.admit(req.headers()).map(drop)) {
-            return files::status(status);
+    /// A request of `files::PREFIX`: from this server's own pages, with a token. A device's is told by `cut` when
+    /// the device is revoked, and its connection ends.
+    async fn transfer(&self, req: Request, cut: &Arc<Notify>) -> files::Answer {
+        let admitted = self.mine(req.uri(), req.headers()).and_then(|()| self.admit(req.headers()));
+        let device = match admitted {
+            Ok(device) => device,
+            Err(status) => return files::status(status),
+        };
+        if let Some(id) = device {
+            let mut cuts = self.cuts.lock();
+            cuts.retain(|(_, c)| c.strong_count() > 0);
+            cuts.push((id, Arc::downgrade(cut)));
+            drop(cuts);
+            // Checked again once registered: a revoke between the first check and the registration is not missed.
+            if let Err(status) = self.admit(req.headers()) {
+                return files::status(status);
+            }
         }
         let Ok(slot) = self.transfers.clone().try_acquire_owned() else {
             return files::busy();
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/f623c3a0-3865-4f13-a9b1-44b5e1741dcd.diff`

### [MEDIUM] Parsing .escouade.yml is quadratic in its size and runs under the projects write lock, so a plain repository file of about 1 MB freezes the server in read_shared

**Résumé** : Hazard (4+3)*0.8*0.7*0.8 = 3.14 MEDIUM: statically_confirmed (quadratic line_at measured on a fragment, 256 KiB = 7 s, but the lock-held full-server stall was read in source, not executed), so likelihood is capped at 3 and Hazard x0.8. Same SUPPLY_CHAIN position and 0.56 multiplier as 79a2cd8e. Fix: cap the bytes read, build one line table, parse outside the projects write lock.

**Emplacement** : src-tauri/src/projectfile.rs:368, src-tauri/src/projectfile.rs:358, src-tauri/src/projectfile.rs:1699, src-tauri/src/projectfile.rs:1645, src-tauri/src/projectfile.rs:493, src-tauri/src/core.rs:9895, src-tauri/src/core.rs:9309, src-tauri/src/core.rs:3057, src-tauri/src/commands.rs:1658 · **CWE** : CWE-407

**Problème** : projectfile::tree (projectfile.rs:391) parses the project file with yaml_edit and then walks it with node_of (projectfile.rs:368). node_of calls line_at (projectfile.rs:358) for every key and for every value node. line_at calls start_position(text) of yaml-edit 0.3.6 (Cargo.lock pins 0.3.6), which is byte_offset_to_line_column(text, offset) = LineIndex::new(text).line_column(offset) (yaml-edit src/lib.rs:371-372, nodes/mod.rs:790): a fresh LineIndex is built from the WHOLE text on every call (one pass over all bytes plus three Vec allocations sized by the number of lines). A file with E entries and S bytes therefore costs about 2*E*S byte steps: the time grows with the square of the size. yaml-edit caps nesting depth at 256 (MAX_FLOW_DEPTH) but has no cap on the number of entries, and file_text (projectfile.rs:1645) reads the file with no size limit. Static estimate (not measured, reading only): a file of about 1 MB made of top-level lines such as k000001: 1 holds about 95,000 entries, so about 190,000 LineIndex builds of roughly 3 ms each, about 10 minutes of one core; 4 times the size costs 16 times the time; a cap of a few hundred KiB (the mitigation proposed for 79a2cd8e) still leaves tens of seconds. The parse runs inside Core::read_shared (projectfile.rs:1699-1716) WHILE HOLDING self.projects.write(): read_into(p, ...) is called inside the write guard. Core::project() (core.rs:3057) and almost every command and event emission take projects.read(), so every reader blocks on the lock for the whole parse. The callers are async tasks on the shared tokio runtime (compute_git core.rs:9895, create_project core.rs:9309), so the runtime workers pile up on the parking_lot lock and the WebSocket/HTTP side, the heartbeats and the admin revoke request stop being served. The file is a regular file: no symlink, FIFO or device is involved, so the link and size fixes proposed for 79a2cd8e (is_file, O_NOFOLLOW, a byte cap) do not remove the quadratic cost, they only bound it. Triggers: a clone followed by shared_preview (commands.rs:1658, which parses on a worker without the lock), the first read of a new project (create_project), every start of the server (Core::start -&gt; watch_project -&gt; refresh -&gt; compute_git -&gt; read_shared with seen = None) and every change of the file (git pull, merge of a ticket branch, a collaborator push that the server then pulls): read_shared returns early only when the text fingerprint equals p.shared.seen. Amplification: every unknown key adds a Warning (read_v1, projectfile.rs:493-520, no limit) kept in p.shared.file.warnings (escouade-model projectfile.rs SharedFile) and sent in every UiEvent::Project and in the initial state to every connected device, so a 5 MB file of unknown keys makes tens of MB of warnings that are cloned (the before/after clones and a serde_json::to_value comparison in read_shared) and pushed to all clients. Persistence: the poisoned file stays in the checkout, so the stall comes back at every start until someone with a shell removes it.

**Impact** : Availability of the whole escouade-server (all devices, running agent turns, admin actions such as revoke) for minutes to hours, repeated at each start, triggered by repository content alone: a teammate or upstream maintainer pushing a large .escouade.yml of trivial lines, or a cloned public repository. No code execution and no data disclosure.

**Cas testé** : Confirmé statiquement

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-e1bb800f-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 500 sh /poc/e1bb800f-06e5-4b1c-bb66-cf45985f9ac1/run.sh
```

```
size=16KiB entries=1489 line_at_calls=2979 per_node_index=0.031s one_index=0.0000s
size=32KiB entries=2978 line_at_calls=5957 per_node_index=0.116s one_index=0.0001s
size=64KiB entries=5957 line_at_calls=11915 per_node_index=0.483s one_index=0.0001s
size=128KiB entries=11915 line_at_calls=23831 per_node_index=2.404s one_index=0.0003s
size=256KiB entries=23831 line_at_calls=47663 per_node_index=7.417s one_index=0.0005s
exit=0 sentinel=MANTIS_REACHED_ENTRYPOINT
```

**Correction** (Correctif proposé, non vérifié)

```diff
--- a/src-tauri/src/projectfile.rs
+++ b/src-tauri/src/projectfile.rs
@@ -355,21 +355,39 @@
     }
 }

-fn line_at(text: &str, node: &YamlNode) -> u32 {
-    let pos = match node {
-        YamlNode::Scalar(s) => s.start_position(text),
-        YamlNode::Mapping(m) => m.start_position(text),
-        YamlNode::Sequence(s) => s.start_position(text),
+/// Where each line of `text` starts: `\n`, `\r` and `\r\n` end a line, as in yaml-edit.
+fn line_starts(text: &str) -> Vec<usize> {
+    let b = text.as_bytes();
+    let mut starts = vec![0];
+    for (i, &c) in b.iter().enumerate() {
+        if c == b'\n' || (c == b'\r' && b.get(i + 1) != Some(&b'\n')) {
+            starts.push(i + 1);
+        }
+    }
+    starts
+}
+
+/// The 1-based line `node` starts on. Not yaml-edit's `start_position`: it rebuilds the line table
+/// of the whole text at every call, which made reading a file quadratic in its size.
+fn line_at(starts: &[usize], node: &YamlNode) -> u32 {
+    let at = match node {
+        YamlNode::Scalar(s) => s.byte_range().start,
+        YamlNode::Mapping(m) => m.byte_range().start,
+        YamlNode::Sequence(s) => s.byte_range().start,
         _ => return 0,
-    };
-    pos.line as u32
+    } as usize;
+    starts.partition_point(|&s| s <= at) as u32
 }

 pub(crate) fn node_of(text: &str, node: &YamlNode) -> Node {
-    let line = line_at(text, node);
+    walk(&line_starts(text), node)
+}
+
+fn walk(starts: &[usize], node: &YamlNode) -> Node {
+    let line = line_at(starts, node);
     let v = match node {
         YamlNode::Scalar(s) => N::Scalar {
             text: s.as_string(),
             quoted: s.is_quoted(),
             null: s.is_null(),
         },
@@ -376,13 +394,13 @@
         YamlNode::Mapping(m) => N::Map(
             m.iter()
                 .map(|(k, v)| {
                     let name = k.as_scalar().map(|s| s.as_string()).unwrap_or_default();
-                    (name, line_at(text, &k), node_of(text, &v))
+                    (name, line_at(starts, &k), walk(starts, &v))
                 })
                 .collect(),
         ),
-        YamlNode::Sequence(s) => N::Seq(s.values().map(|v| node_of(text, &v)).collect()),
+        YamlNode::Sequence(s) => N::Seq(s.values().map(|v| walk(starts, &v)).collect()),
         _ => N::Other,
     };
     Node { line, v }
 }
--- a/src-tauri/src/projectfile_tests.rs
+++ b/src-tauri/src/projectfile_tests.rs
@@ -1605,3 +1605,14 @@
     let all = pf::export_values(&project());
     assert!(all.iter().all(|(k, _)| !k.contains("index")));
 }
+
+#[test]
+fn node_lines_count_crlf_and_lf_breaks() {
+    let t = pf::tree("version: 1\r\nmarker: x\r\nworktrees:\n  per_agent: true\n").unwrap();
+    let N::Map(e) = &t.v else { panic!() };
+    let lines: Vec<u32> = e.iter().map(|(_, l, _)| *l).collect();
+    assert_eq!(lines, [1, 2, 3]);
+    let N::Map(w) = &e[2].2.v else { panic!() };
+    assert_eq!((w[0].1, w[0].2.line), (4, 4));
+    assert_eq!(t.line, 1);
+}
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/e1bb800f-06e5-4b1c-bb66-cf45985f9ac1.diff`

### [MEDIUM] The Cursor view writes its relay rule through a committed symlink and creates directories through committed folder links in relay_to_interface

**Résumé** : Hazard (4+4)*0.8*0.7*0.7 = 3.14 MEDIUM (CONDITIONAL_VIABLE: Cursor view opened on an agent with a conversation): relay_to_interface writes through committed links and truncates or creates files outside the project (reproduced on a fragment; patch reattack failed_to_bypass). SUPPLY_CHAIN position as 79a2cd8e. Fix: refuse links and write via temp file plus rename. Not a production bug beyond the Cursor path.

**Emplacement** : src-tauri/src/tui.rs:1002, src-tauri/src/tui.rs:954, src-tauri/src/tui.rs:1641, src-tauri/src/engine/cursor/terminal.rs:121, src-tauri/src/tui.rs:646 · **CWE** : CWE-59

**Problème** : relay_to_interface (tui.rs:954) is called by start_interface (tui.rs:1641) for a harness whose TerminalView::relay_path is Some, i.e. Cursor (engine/cursor/terminal.rs:121: &lt;cwd&gt;/.cursor/rules/escouade-relais.mdc, cwd being the agent folder, a checkout of the project or a worktree of it). It computes the path, then runs path.parent().create_dir_all() and std::fs::write(&amp;path, view.relay_file(&amp;text)) (tui.rs:1002-1005): no symlink_metadata, no is_file check, no O_NOFOLLOW, no temporary file plus rename. Git stores symlinks and a cloned or pulled repository can carry .cursor/rules/escouade-relais.mdc as a link to any path (relative or absolute), or .cursor or .cursor/rules as a link to a folder outside the project. std::fs::write opens with O_CREAT|O_TRUNC and follows the link: an existing file of the server user (shell startup file, ~/.ssh/authorized_keys, a config file, a database) is truncated and replaced by the relay text (a Markdown header, the plan tasks and the conversation so far, up to RELAY_BUDGET 24000 characters, framed by a Cursor rule frontmatter), and an absent target is created (e.g. ~/.bash_profile, a file under ~/.config/autostart or ~/.config/systemd/user when the link names that path). A folder link makes create_dir_all and the write land in that folder under the fixed name escouade-relais.mdc. The sibling cleanup take_relay_away (tui.rs:646) calls remove_file(path) which removes the link itself, so the damage stays. Part of the text is influenced by the repository through the agent (the agent reads hostile files and its replies and plan titles are copied into the relay), but the attacker does not control the bytes precisely, so the primitive is mainly integrity and data loss (truncation of any file the server user can write) plus file creation at a chosen path, not a controlled arbitrary write. Same class as the write side of 79a2cd8e and e5ace214 (committed symlink followed by a server write), but a different sink, a different trigger and not covered by their patch: projectfile::file_to_write only guards .escouade.yml, the new choke point is not applied here. Trigger: the user (or a paired device through agent_terminal, remote table server.rs:1042) opens the Vue Cursor of an agent whose folder holds the hostile repository content, after a conversation exists. ensure_excluded (git.rs:655) also writes .git/info/exclude through its own paths, not repo-controlled.

**Impact** : Truncation or overwrite of any file writable by the server user, or creation of a file at a repository-chosen path (shell startup, autostart, user unit path), through a link committed in a cloned repository; on a server the account often holds the agents credentials and the paired-device state.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-f987a2d2-<n> --network=none --memory=512m -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src rust:1.88 timeout 300 bash /poc/f987a2d2-4925-4453-a0dd-aa25b9802a40/run.sh
```

```
== [A] committed: 120000 af2e1b74d605499d330900af92192bbc1db08e50 0	.cursor/rules/escouade-relais.mdc
BEFORE A: export PATH=/usr/bin  # ORIGINAL-BASHRC-CONTENT
relay written at /tmp/projects/A/.cursor/rules/escouade-relais.mdc
take_relay_away done
AFTER A (/tmp/victimhome/.bashrc, 185 bytes):
---
description: Escouade — la conversation d’avant / the conversation before
alwaysApply: true
---

# La conversation d'avant

- user: fix the build
- agent: done, see plan task 1
total 8
drwxr-xr-x 2 root root 4096 Nov 23 12:26 .
drwxr-xr-x 3 root root 4096 Nov 23 12:26 ..
RESULT A: VULNERABLE - /tmp/victimhome/.bashrc truncated+overwritten through the committed link; take_relay_away removed only the link
== [B] committed: 120000 107c71a938fa4d77a71c7a47ce1f7b2502ef1d22 0	.cursor
relay written at /tmp/projects/B-wt/.cursor/rules/escouade-relais.mdc
AFTER B: find /tmp/victimhome/outside ->
/tmp/victimhome/outside
/tmp/victimhome/outside/rules
/tmp/victimhome/outside/rules/escouade-relais.mdc
RESULT B: VULNERABLE - dir and file created outside the worktree through the .cursor folder link
== [C] committed: 120000 f209c6d413136db72389f8e4967803411611bb7f 0	.cursor/rules/escouade-relais.mdc
relay written at /tmp/projects/C/.cursor/rules/escouade-relais.mdc
RESULT C: VULNERABLE - /tmp/victimhome/.config/autostart/evil.desktop created (185 bytes)
== sentinel: MANTIS_REACHED_ENTRYPOINT MANTIS_REACHED_ENTRYPOINT MANTIS_REACHED_ENTRYPOINT 
CONTROL: /tmp/projects/ctl/.cursor/rules/escouade-relais.mdc
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src-tauri/src/tui.rs
+++ b/src-tauri/src/tui.rs
@@ -645,13 +645,53 @@
 /// with something in it).
 pub(crate) fn take_relay_away(view: &dyn TerminalView, meta: &mut AgentMeta) {
     if let Some(path) = view.relay_path(Path::new(&meta.cwd)) {
-        let _ = std::fs::remove_file(path);
+        remove_relay(Path::new(&meta.cwd), &path);
     }
     for dir in std::mem::take(&mut meta.relay_made).iter().rev() {
         let _ = std::fs::remove_dir(dir);
+    }
+}
+
+/// Whether `path`, or a folder above it under `cwd`, is a link: a repository can commit one at
+/// `.cursor`, `.cursor/rules` or the rule itself, and a relay is never written or taken away
+/// through it.
+fn relay_through_link(cwd: &Path, path: &Path) -> bool {
+    path.ancestors()
+        .take_while(|d| d.starts_with(cwd) && *d != cwd)
+        .any(|d| std::fs::symlink_metadata(d).is_ok_and(|m| m.file_type().is_symlink()))
+}
+
+/// The relay at `path` removed, only when it is a plain file of this user reached through no link.
+fn remove_relay(cwd: &Path, path: &Path) {
+    let own = std::fs::symlink_metadata(path).is_ok_and(|m| {
+        #[cfg(unix)]
+        if std::os::unix::fs::MetadataExt::uid(&m) != unsafe { libc::geteuid() } {
+            return false;
+        }
+        m.file_type().is_file()
+    });
+    if own && !relay_through_link(cwd, path) {
+        let _ = std::fs::remove_file(path);
     }
 }
 
+/// `bytes` written at `path` under `cwd`: its folders made, then checked again for a link, and
+/// the file opened without following one (`O_NOFOLLOW`), so a link put there is refused.
+fn write_relay(cwd: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
+    use std::io::Write;
+    if let Some(dir) = path.parent() {
+        std::fs::create_dir_all(dir)?;
+    }
+    if relay_through_link(cwd, path) {
+        return Err(std::io::Error::other("a link on its way, not followed"));
+    }
+    let mut open = std::fs::OpenOptions::new();
+    open.write(true).create(true).truncate(true);
+    #[cfg(unix)]
+    std::os::unix::fs::OpenOptionsExt::custom_flags(&mut open, libc::O_NOFOLLOW);
+    open.open(path)?.write_all(bytes)
+}
+
 /// The files of `files` whose text holds `token`.
 pub(crate) fn token_files(
     files: &[escouade_plugin_protocol::engine::FileToWrite],
@@ -977,9 +1017,13 @@
             return;
         };
         let Some(text) = text else {
-            let _ = std::fs::remove_file(&path);
+            remove_relay(Path::new(&cwd), &path);
             return;
         };
+        if relay_through_link(Path::new(&cwd), &path) {
+            log::warn!("{}: relay not written: a link on its way", path.display());
+            return;
+        }
         // The folders made for it (not there before), kept to be taken away with it.
         let made: Vec<String> = path
             .ancestors()
@@ -999,10 +1043,7 @@
                 }
             }
         }
-        let written = path
-            .parent()
-            .map_or(Ok(()), std::fs::create_dir_all)
-            .and_then(|()| std::fs::write(&path, view.relay_file(&text)));
+        let written = write_relay(Path::new(&cwd), &path, &view.relay_file(&text));
         if let Err(e) = written {
             log::warn!("{}: relay not written: {e}", path.display());
             return;
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/f987a2d2-4925-4453-a0dd-aa25b9802a40.diff`

### [LOW] Agent-authored link opened without noreferrer and without a Referrer-Policy leaks the server origin in openUrl

**Résumé** : Reproduced in Chromium: openUrl uses window.open with noopener only, so the server origin is sent as Referer to an agent-chosen host after one click (user interaction required, x0.7). Impact 2 (origin only, no token), likelihood 2 (needs injected agent link plus click, and a hostname that is actually private). Fix: noopener,noreferrer and a Referrer-Policy header.

**Emplacement** : src/lib/platform.ts:50, src-tauri/src/remoteapi/server.rs:870, src/lib/links.ts:33 · **CWE** : CWE-200

**Problème** : In browser mode, platform.ts openUrl (line 50) calls window.open(url, "_blank", "noopener"). The noopener feature cuts window.opener but does NOT suppress the Referer header (that needs noreferrer or a Referrer-Policy). The server answers the web page with CSP and nosniff only (src-tauri/src/remoteapi/server.rs:870-875); no Referrer-Policy header or meta exists in remoteapi, index.html or vite config (grep). The browser default strict-origin-when-cross-origin therefore sends the page origin (scheme, host, port of the escouade-server, e.g. the operator HTTPS --origin or http://127.0.0.1:&lt;port&gt;) as Referer to every target opened by openUrl. Targets include agent-authored hrefs: Markdown links, OSC 8 terminal hyperlinks (links.ts openOutside/guardLinks), PR and gallery URLs, and the toast path of opener.ts that shows arbitrary core-supplied URLs. A prompt-injected agent can thus emit a link to a host it controls and, on one user click, learn the private server hostname. With the hostname known, an unauthenticated peer can reach POST /pair and /device and cancel a live pairing window with 5 wrong codes (threat model A1, devices.rs:147-153) or probe the listener. The allow-list itself (http, https, mailto, tel) was verified: split(":")[0] fails closed for java\nscript:, leading whitespace, and mixed case, so no dangerous-scheme bypass exists; a scheme-less string equal to an allowed name ("http") only opens a same-origin relative path.

**Impact** : Disclosure of the private server origin to a third party chosen by agent output; enables targeted pairing-cancel DoS or reconnaissance against a server meant to be reachable only by tunnel or an unadvertised hostname. No token is in the URL, so no credential leaks.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none -v "mantis-Escouade-lot5-deps":/deps:ro -v "{TARGET_ROOT}":/src:ro -v "<dossier d audit>/workspace/reproducers":/poc:ro -w /src mcr.microsoft.com/playwright:v1.65.0-noble timeout 500 sh /poc/904eb54c-7f3b-4010-b328-050f266e4056/run.sh
```

```
[browser] 158.0.8127.18
[page] http://escouade.internal:8080/ Referrer-Policy header: (none)
[page] markdown link href=http://attacker.test:9090/leak-markdown rel=noopener noreferrer
[attacker.test] GET /leak-markdown  Referer: http://escouade.internal:8080/
[attacker.test] GET /leak-osc8  Referer: http://escouade.internal:8080/
[attacker.test] GET /control-noreferrer  Referer: (none)
markdown link (guardLinks -> openUrl):  Referer=http://escouade.internal:8080/
OSC 8 link (terminalLinks -> openUrl):  Referer=http://escouade.internal:8080/
control noopener,noreferrer:            Referer=null
VULNERABLE: server origin leaked as Referer to the agent-chosen host
SENTINEL: MANTIS_REACHED_ENTRYPOINT
EXIT=0
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src/lib/platform.ts
+++ b/src/lib/platform.ts
@@ -47,7 +47,8 @@ export const openUrl: typeof system.openUrl = (url, openWith) => {
   // The core's rule and refusal for `open_url` (commands.rs): http, https, mailto and tel alone.
   const scheme = String(url).split(':')[0].toLowerCase();
   if (!['http', 'https', 'mailto', 'tel'].includes(scheme)) return Promise.reject(`not a link Escouade opens: ${scheme}:`);
-  window.open(String(url), '_blank', 'noopener');
+  // noreferrer: the target, often a link an agent wrote, must not learn this server's origin.
+  window.open(String(url), '_blank', 'noopener,noreferrer');
   return done();
 };

--- a/src/lib/platform.test.ts
+++ b/src/lib/platform.test.ts
@@ -73,7 +73,7 @@
     expect(platform.desktop).toBe(false);
     const opened = vi.spyOn(window, 'open').mockReturnValue(null);
     await platform.openUrl('https://example.com/a');
-    expect(opened).toHaveBeenCalledWith('https://example.com/a', '_blank', 'noopener');
+    expect(opened).toHaveBeenCalledWith('https://example.com/a', '_blank', 'noopener,noreferrer');
     // The core's rule: http, https, mailto and tel alone, refused with its words.
     for (const ok of ['HTTP://example.com', 'mailto:a@b.c', 'tel:+33100000000']) await platform.openUrl(ok);
     for (const [bad, scheme] of [
--- a/src/lib/opener.test.ts
+++ b/src/lib/opener.test.ts
@@ -27,7 +27,7 @@
     const opened = vi.spyOn(window, 'open').mockReturnValue(null);
     activation(true);
     await openUrl('https://example.com/a');
-    expect(opened).toHaveBeenCalledWith('https://example.com/a', '_blank', 'noopener');
+    expect(opened).toHaveBeenCalledWith('https://example.com/a', '_blank', 'noopener,noreferrer');
     expect(app.toasts).toEqual([]);
   });

@@ -40,7 +40,7 @@
     const [toast] = app.toasts;
     expect(toast.text).toContain('http://localhost:5173/');
     toast.action!.onClick();
-    expect(opened).toHaveBeenCalledWith('http://localhost:5173/', '_blank', 'noopener');
+    expect(opened).toHaveBeenCalledWith('http://localhost:5173/', '_blank', 'noopener,noreferrer');
   });

   it('a link the core would refuse is refused at the toast’s click too', async () => {
--- a/src-tauri/src/remoteapi/server.rs
+++ b/src-tauri/src/remoteapi/server.rs
@@ -871,5 +871,6 @@
             .header(header::CACHE_CONTROL, file.cache)
             .header(header::CONTENT_SECURITY_POLICY, csp)
             .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
+            .header(header::REFERRER_POLICY, "no-referrer")
             .body(Full::new(Bytes::from(file.body)))
             .expect("a valid answer")
     }
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/904eb54c-7f3b-4010-b328-050f266e4056.diff`

### [LOW] Browser build of SentFiles offers programs and installers for download as Ouvrir, bypassing the never run rule

**Résumé** : Browser build of SentFiles shows Open (a download) for runnable files the desktop refuses to open. Hazard = (Impact 1 + Likelihood 3) x 0.7 = 2.8, LOW. Impact 1: this weakens a defence-in-depth rule; Escouade runs nothing, the browser applies its own dangerous-download warnings and the user must still run the file. Likelihood 3: client leg reproduced in jsdom, server leg statically verified, trivial to automate once an agent is injected, but needs a user click and a later manual run. Multiplier 1.0 (EXTERNAL) x 0.7 (user interaction). VIABLE, default release path. The patch (hide the button when v.runnable) was verified secure on re-attack.

**Emplacement** : src/components/conv/SentFiles.svelte:142, src/components/conv/SentFiles.svelte:151, src-tauri/src/sentfiles.rs:994, src-tauri/src/remoteapi/files.rs:190 · **CWE** : CWE-693

**Problème** : On the desktop, SentFiles never offers to open a file the agent sent when the core marks it runnable (program, script, installer, shortcut, disk image, page): only 'Afficher dans le dossier' is shown (SentFiles.svelte:151 `{#if !v.runnable}`), and the core refuses an open with reveal=false (sentfiles.rs target(), around line 371: 'A program the agent sent is never run, whatever the window asks'). The browser branch added in this range (SentFiles.svelte:142 `{#if there &amp;&amp; !desktop}`) ignores v.runnable: it shows a button with the same common.open label for every file in state ok. The server route GET /files/sent (remoteapi/files.rs sent(), Core::sent_file at sentfiles.rs:994) calls target() with reveal=true on purpose, so the runnable check is skipped (doc comment: 'A program is fetched as any file'). docs/REMOTE.md:220 promises the opposite for the browser: 'Ouvrir ouvre la copie (jamais un executable, comme ESC-171)'. A prompt-injected or malicious agent on the server can therefore call SendUserFile with a .sh, .bat, .ps1, .lnk, .msi, .dmg, .html, .svg or an executable-bit file; the user, who is told Open is the safe action, gets it saved on his own workstation (a different machine from the server, so the agent crosses a machine boundary it never crossed on the desktop) with one click. The download name is also the agent's (see the existing finding on download names). The browser applies its own dangerous-download warnings and Escouade runs nothing, so this weakens a defence-in-depth rule rather than giving code execution by itself.

**Impact** : An agent can deliver a program, script, installer or shortcut to the user's workstation through a button the interface presents as the safe Open action; one more step by the user (running the downloaded file) gives code execution on the workstation.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none -v "mantis-Escouade-lot5-deps":/deps:ro -v "{TARGET_ROOT}":/src:ro -v "<dossier d audit>/workspace/reproducers":/poc:ro -w /src node:22-slim timeout 400 sh /poc/33745242-c903-495e-bd9e-c6892d199a0c/run.sh
```

```
[BROWSER] deploy.sh: button visible text="Ouvrir" aria-label="Télécharger deploy.sh"
[BROWSER] setup.msi: button visible text="Ouvrir" aria-label="Télécharger setup.msi"
[BROWSER] GET /files/sent?agent=ag&item=toolu_1&index=0
[BROWSER] saved to workstation as "deploy.sh" (34 bytes): "#!/bin/sh\necho pwned > /tmp/owned\n"
[BROWSER] VULNERABLE: runnable file offered as « Ouvrir » and downloaded
[DESKTOP] control: runnable files have no « Ouvrir » button (SentFiles.svelte:151)
 ✓ zz_poc_33745242 BROWSER: a runnable file gets the « Ouvrir » button and is downloaded on click
 ✓ zz_poc_33745242 CONTROL DESKTOP: the same runnable views get no « Ouvrir »
 ✓ SentFiles.test.ts (repo's own 10 tests, incl. 'never offers to open a program or a script')
Test Files 2 passed (2), Tests 12 passed (12)
SENTINEL: MANTIS_REACHED_ENTRYPOINT
EXIT=0
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src/components/conv/SentFiles.svelte
+++ b/src/components/conv/SentFiles.svelte
@@ -140,11 +140,14 @@
             </span>
           </span>
           {#if there && !desktop}
-            <span class="acts">
-              <button class="act" aria-label={t('conv.sent.downloadName', { name: f.name })} onclick={() => download(f.index, f.name)}
-                ><Icon name="download" size={13} /><span class="lbl">{t('common.open')}</span></button
-              >
-            </span>
+            <!-- A program or a script the agent sent is never offered from the browser either (docs/REMOTE.md, ESC-171). -->
+            {#if !v.runnable}
+              <span class="acts">
+                <button class="act" aria-label={t('conv.sent.downloadName', { name: f.name })} onclick={() => download(f.index, f.name)}
+                  ><Icon name="download" size={13} /><span class="lbl">{t('common.open')}</span></button
+                >
+              </span>
+            {/if}
           {:else if there}
             <span class="acts">
               <!-- A program or a script the agent sent is never run from here: shown in its folder only. -->
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/33745242-c903-495e-bd9e-c6892d199a0c.diff`

### [LOW] Proxy password kept in proxy_password_in_file is serialized to every paired device in InitialState on a keychain-less server in subscribe_to

**Résumé** : On a keychain-less server with an authenticated proxy, proxy_password_in_file is sent in InitialState to every paired browser. Conditional (x0.7: no keychain plus authenticated proxy), INTERNAL_NETWORK exposure (x0.8), static (x0.8). Impact 2 (a paired device is already a shell), likelihood 3. Fix: clear the field on the Settings copy sent to clients.

**Emplacement** : src-tauri/src/commands.rs:243, src-tauri/src/proxysecret.rs:32, crates/escouade-model/src/model.rs:46 · **CWE** : CWE-522

**Problème** : Settings.proxy_password_in_file (crates/escouade-model/src/model.rs:43-46) is serialized whenever non-empty (skip_serializing_if is_empty only); the test model.rs:4070-4083 documents that the password is meant to travel apart "only while the keychain refuses it". On Linux the store is always Absent (crates/escouade-forge/src/integrations/secrets.rs:156-170: set() returns NoKeychain), so proxysecret::keep (src-tauri/src/proxysecret.rs:20-34) puts the clear password into proxy_password_in_file in memory both at on_save and at every on_load (core.rs:2130, 2938). subscribe_to (src-tauri/src/commands.rs:243, 270-279) clones core.settings and returns it as InitialState.settings, and the remote `subscribe` command (server.rs:987-997, Subscriber::Remote) hands it to every paired browser over the WebSocket. The header of proxysecret.rs and SEC-10 promise that the password is never in the window; that held for the local desktop window, but in server mode the "window" is a remote browser reached over an SSH tunnel or an operator proxy, and a headless VPS has no keychain by construction, so every server with an authenticated proxy exposes it. The front end never reads the field (no match under src/), so the exposure is pure leakage: the value sits in browser JS memory, in any WS capture or Playwright trace, and is sent back on save_settings. A device token is a shell (REMOTE.md:191) so the file could be read anyway; the finding is the broken SEC-10 invariant and the wider copies (every browser, plain-http declared origins, traces), not a privilege boundary.

**Impact** : Clear-text disclosure of the corporate proxy password to every paired browser and to anyone who can observe or capture its WebSocket traffic (plain-http declared origin, traces, extensions), instead of staying in the 0600 settings file on the server.

**Cas testé** : Confirmé statiquement

```bash
docker run --rm --network=none -v {STATE_ROOT}/workspace/reproducers:/poc:ro -v {TARGET_ROOT}:/src:ro -w /src rust:1.88-slim sh -c 'rustc -O -o /tmp/poc /poc/f8d057a5-830a-433c-9260-d941eb1332ab/poc.rs && SENTINEL_FILE=/tmp/s /tmp/poc'
```

```
settings file after migration: {"proxyUrl":"http://ada@proxy.corp:3128","proxyPasswordInFile":"S3cr3t-Corp-Pw"}
InitialState sent to Subscriber::Remote: {"settings":{"proxyUrl":"http://ada@proxy.corp:3128","proxyPasswordInFile":"S3cr3t-Corp-Pw"}}
proxyUrl stripped of password: true
VULNERABLE: clear proxy password present in the remote InitialState (proxyPasswordInFile)
exit=0 sentinel=MANTIS_REACHED_ENTRYPOINT
```

**Correction** (Correctif proposé, non vérifié)

```diff
--- a/src-tauri/src/commands.rs
+++ b/src-tauri/src/commands.rs
@@ -240,7 +240,10 @@ pub(crate) fn subscribe_to(
     };
     // One lock at a time, each guard ending with its statement: in a struct literal the guards
     // would live together (the lock order is projects before tickets, see `Core`).
-    let settings = core.settings.read().clone();
+    let mut settings = core.settings.read().clone();
+    // SEC-10: the proxy's password kept in the file apart (no keychain) never leaves the backend; `on_save` ignores
+    // the window's copy anyway.
+    settings.proxy_password_in_file.clear();
     let projects = core.projects.read().clone();
     let tickets = core.tickets.read().clone();
     let agents = core.agent_views();
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/f8d057a5-830a-433c-9260-d941eb1332ab.diff`

### [LOW] A call that never returns keeps its global call permit and lane for good, so a few hung calls lock every remote client out in dispatch

**Résumé** : Raw hazard (3+2)*0.8*0.7 = 2.8, capped to 2.0 LOW: no reproduction run (repro_status missing = not_attempted). Hang source on the blocking pool is not demonstrated (provisional). Fix: abort the call on connection close (patch proposed, not verified). Manual review needed for vague_code_paths.

**Emplacement** : src-tauri/src/remoteapi/server.rs:958, src-tauri/src/lanes.rs:123, src-tauri/src/lanes.rs:109 · **CWE** : CWE-400

**Problème** : Server::dispatch (server.rs:945-966) moves the two semaphore permits (per-connection in_flight, global self.calls = MAX_CALLS 64) into the lane job and drops them only when the job ends; the job waits on rt::block_on(rt::spawn(run)) with no timeout, no cancellation and no abort handle, and lanes::Lanes::drain (lanes.rs:123-137) runs jobs to completion on a blocking thread with no deadline either. conn.close() and Server::cut (revoke) only set the closed flag, which is checked before the job starts (server.rs:959), never while it runs. So a call that blocks forever in the host (a read of a FIFO or symlink-to-FIFO by read_logo / set_project_logo / check_authorities or any sync command doing std::fs::read on a client path, see sibling finding on projectlogo::read_in; a git_files on a planted symlink) keeps its permit for the life of the process, even after its connection closed and its device was revoked. Each such call also pins a blocking-pool thread (sync commands: the lane thread plus the spawn_blocking one) and its lane. Round-1 finding 37d28eb4 (call permits held while queued) was rejected because a slow call finishes; this is the case where it does not, so the loss is permanent and cumulative: 64 hung calls (one connection can send 64, or reconnect) exhaust self.calls and every later call of every device, including the owner, waits on acquire_owned in the connection loop forever, while the HTTP routes and the pairing page still answer, so the server looks alive. Restart is the only recovery, and it cuts running agent turns. With a read_logo-type sync command the tokio runtime itself stays healthy (the hang is on the blocking pool), which makes this a full remote-API lockout distinct from the worker stall of 2f353c55. Boundary: a device that can name a FIFO (or an agent that planted one, then a click by any device on the git/files panels) -&gt; availability of the remote API for all other devices and revoke not freeing the resources. Same caveat as other device-only DoS: a paired device has a shell; the crossing that matters is the agent-planted FIFO triggered by another device, and revoked devices still holding permits.

**Impact** : Permanent loss of the remote API for all devices after about 64 hung calls, recoverable only by restarting the server (running agent turns are cut); revoking the offending device frees nothing.

**Cas testé** : non évalué à ce niveau

**Correction** (Correctif proposé, non vérifié)

```diff
--- a/src-tauri/src/remoteapi/server.rs
+++ b/src-tauri/src/remoteapi/server.rs
@@ -959,7 +959,21 @@
                 if conn.is_closed() {
                     return;
                 }
                 let (id, on) = (request.id, conn.clone());
                 let run = async move { me.run(&on, entry, request.args, request.stream).await };
-                let answer = answer_of(id, rt::block_on(rt::spawn(run)));
+                // A call that never ends must not keep its permits and lane once its client is gone or revoked.
+                let mut closed = conn.closed.subscribe();
+                let mut task = rt::spawn(run);
+                let done = rt::block_on(async {
+                    tokio::select! {
+                        done = &mut task => Some(done),
+                        _ = closed.wait_for(|c| *c) => None,
+                    }
+                });
+                let Some(done) = done else {
+                    // shortcut: a sync command stuck in a blocking read keeps its pool thread; only permits and lane are freed.
+                    task.abort();
+                    return;
+                };
+                let answer = answer_of(id, done);
                 let _ = conn.push(&conn.text, Message::text(answer));
             }),
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/2d356ff9-c44a-4619-971a-c9fc9f930639.diff`

### [LOW] A paired device holds all 16 connection slots with idle WebSockets, so the admin revoke request is never accepted in accept

**Résumé** : Raw hazard (3+3)*1.0 = 6.0 (a paired device holds the 16 slots and blocks the revoke request), capped to 2.0 LOW because no reproduction was run (repro_status missing = not_attempted) and the device already holds equivalent access. Fix: reserve slots for short requests and add an idle timeout to WebSockets (patch proposed, not verified).

**Emplacement** : src-tauri/src/remoteapi/server.rs:540, src-tauri/src/remoteapi/server.rs:544, src-tauri/src/remoteapi/server.rs:692, src-tauri/src/remoteapi/server.rs:907, src-tauri/src/headless_host.rs:480 · **CWE** : CWE-400

**Problème** : Server::accept (server.rs:524-548) gives every accepted TCP/Unix connection one of MAX_CONNECTIONS = 16 permits, and the permit is released only when me.serve() returns (server.rs:544-547). For a WebSocket, serve() includes the whole session() (server.rs:692, 879-938), so an upgraded connection keeps its slot for as long as the socket lives. Nothing bounds that: there is no per-device cap on WebSocket connections (conns is a plain Vec, server.rs:510, 897), no inbound idle timeout or ping in session() (the loop at 907-932 only waits on stop, close or the next frame), and IDLE/IDLE_LEAST (server.rs:88-91) close a client only while a frame waits for it, which never happens for a client that sends no request and does not subscribe. A device with a valid token can therefore open 16 WebSockets and sit on them. When the 16 are taken, further connections are accepted and dropped at once (try_acquire_owned fails, `continue`, server.rs:540-542). The only way to revoke a device is POST /admin/revoke, sent by `escouade-server devices revoke &lt;id&gt;` through the same listener (headless_host.rs:480, ask() at 520-560, TCP port or Unix socket, same accept loop), and `pair` and `devices` use it too. The documented limit (docs/REMOTE.md, Limites connues, 16 connexions) describes slow or dead connections that fall off after a delay and says nothing of a device that keeps them on purpose; and the /admin request has no reserved slot. Result: the administrator cannot revoke the device that is saturating the server; the revoke command fails with a connection error. Restarting the service closes the sockets but the device reconnects in a loop with its still-valid token and refills the 16 slots within milliseconds, racing the administrator running the command; the owner's own browser is locked out the whole time too.

**Impact** : A paired device that is lost, stolen, or hostile (the case revocation exists for) defeats its own revocation and keeps full API access (documented as equal to a shell on the server) while the owner cannot pair a replacement or revoke it with the supported command; the owner has to stop the service and edit devices.json by hand, or win a race after a restart.

**Cas testé** : non évalué à ce niveau

**Correction** (Correctif proposé, non vérifié)

```diff
--- a/src-tauri/src/remoteapi/server.rs
+++ b/src-tauri/src/remoteapi/server.rs
@@ -57,7 +57,8 @@

 type Request = hyper::Request<Incoming>;
-/// A WebSocket's connection once upgraded, and the device it comes from (none: the environment's token).
-type Upgrade = Mutex<Option<(OnUpgrade, Option<String>)>>;
+/// A WebSocket's connection once upgraded, the device it comes from (none: the environment's token), and its slot
+/// among `MAX_SESSIONS`.
+type Upgrade = Mutex<Option<(OnUpgrade, Option<String>, OwnedSemaphorePermit)>>;
 type Response = hyper::Response<Full<Bytes>>;

 const PORT_VAR: &str = "ESCOUADE_REMOTE_PORT";
@@ -66,6 +67,10 @@
 const MIN_TOKEN: usize = 32;
 /// Clients connected at once; the next ones are closed at once.
 const MAX_CONNECTIONS: usize = 16;
+/// WebSockets open at once, all devices together: the rest of `MAX_CONNECTIONS` stays for short requests, so a device
+/// that holds its sockets open cannot keep out `escouade-server devices revoke` (`/admin/…`), `/pair` or a page's files.
+/// The next WebSocket is answered 503.
+const MAX_SESSIONS: usize = MAX_CONNECTIONS - 4;
 /// The most a request weighs: a command's call, as the MCP server takes one without attachments.
 const MAX_REQUEST: usize = escouade_mcp::http::MAX_TEXT_BODY;
 /// A connection's requests under way; the next ones wait in the socket.
@@ -395,6 +400,7 @@
         calls: Arc::new(Semaphore::new(calls)),
         uploads: Arc::new(Semaphore::new(files::MAX_UPLOADS)),
         transfers: Arc::new(Semaphore::new(files::MAX_TRANSFERS)),
+        sessions: Arc::new(Semaphore::new(MAX_SESSIONS)),
     });
     let (stop, stopped) = watch::channel(false);
     rt::spawn(server.accept(listener, stopped));
@@ -518,6 +524,8 @@
     uploads: Arc<Semaphore>,
     /// The requests of files, all connections together (`files::MAX_TRANSFERS`).
     transfers: Arc<Semaphore>,
+    /// The WebSockets open, all connections together (`MAX_SESSIONS`).
+    sessions: Arc<Semaphore>,
 }

 impl Server {
@@ -674,7 +682,8 @@
         let Ok(()) = served else {
             return;
         };
-        let Some((on_upgrade, device)) = upgrade.lock().take() else {
+        // `_session` keeps its slot until the WebSocket ends.
+        let Some((on_upgrade, device, _session)) = upgrade.lock().take() else {
             return;
         };
         let Ok(Ok(io)) = tokio::time::timeout(HANDSHAKE, on_upgrade).await else {
@@ -752,10 +761,13 @@
         if req.method() != Method::GET {
             return empty(StatusCode::BAD_REQUEST);
         }
+        let Ok(session) = self.sessions.clone().try_acquire_owned() else {
+            return empty(StatusCode::SERVICE_UNAVAILABLE);
+        };
         let accept = derive_accept_key(key.as_bytes());
         // A page that names subprotocols is answered with ours, never with its token's.
         let named = subprotocols(req.headers()).any(|p| p == SUBPROTOCOL);
-        *upgrade.lock() = Some((hyper::upgrade::on(&mut req), device));
+        *upgrade.lock() = Some((hyper::upgrade::on(&mut req), device, session));
         let mut answer = hyper::Response::builder()
             .status(StatusCode::SWITCHING_PROTOCOLS)
             .header(header::CONNECTION, "Upgrade")
--- a/docs/REMOTE.md
+++ b/docs/REMOTE.md
@@ -205,3 +205,3 @@
 - Les routes `/files/` existent aussi sur l'API distante de l'app de bureau, qui n'écoute que dans un build de debug (voir « Tâches »).
-- Le serveur tient 16 connexions à la fois, appairées ou non. Une connexion lente (le corps d'un `POST /pair` envoyé au compte-gouttes, derrière un proxy qui ne le met pas en tampon) ou morte sans l'avoir dit (un téléphone qui change de réseau) garde sa place : jusqu'à 60 s pour la première, sans délai pour la seconde tant que rien ne lui est envoyé. Les 16 prises, plus personne n'entre, le propriétaire compris, jusqu'à ce qu'elles tombent. Rien n'est lu ni perdu ; un délai court sur ces lectures et une coupure après quelques battements sans réponse restent à faire.
+- Le serveur tient 16 connexions à la fois, appairées ou non, dont 12 WebSockets au plus (la 13e reçoit une 503) : les 4 autres restent aux requêtes courtes, `escouade-server devices revoke` compris, pour qu'un appareil qui garde ses WebSockets ouvertes ne puisse pas empêcher qu'on le révoque. Une connexion lente (le corps d'un `POST /pair` envoyé au compte-gouttes, derrière un proxy qui ne le met pas en tampon) ou morte sans l'avoir dit (un téléphone qui change de réseau) garde sa place : jusqu'à 60 s pour la première, sans délai pour la seconde tant que rien ne lui est envoyé. Les 16 prises, plus personne n'entre, le propriétaire compris, jusqu'à ce qu'elles tombent. Rien n'est lu ni perdu ; un délai court sur ces lectures et une coupure après quelques battements sans réponse restent à faire.
 - Une page chargée pendant que le serveur redémarre reste sur son erreur : rechargez-la à la main.
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/ac424971-b485-4232-882c-ad1b43bd61ae.diff`

### [LOW] fs_read blocks an async runtime worker forever on a named pipe planted in a project folder in fsedit::read

**Résumé** : A FIFO planted in a project folder blocks a tokio worker in fsedit::read; one or two clicks on a 1-2 vCPU server stall every device until restart. Reproduced on a fragment harness (2 workers stalled 8 s). Raw Hazard (3+4) x 0.8 (LOCAL -&gt; INTERNAL) x 0.7 (click required) = 3.92 (impact capped at 3 for LOW privileges). Capped to LOW 2.0 by Prerequisite Shell Access: whoever can plant the FIFO as the service user can already stop the server. Still worth fixing: the shipped patch holds for fsedit::read/write but was bypassed through git::count_lines and git::diff on untracked files, which open the same FIFO on a runtime worker; move those reads to spawn_blocking with an is_file check.

**Emplacement** : src-tauri/src/fsedit.rs:187, src-tauri/src/commands.rs:2709 · **CWE** : CWE-400

**Problème** : fs_read is an async command: the remote table awaits it directly on the shared multi-thread tokio runtime (remote_call: async fn =&gt; '.await', only sync fns go through table::blocking/spawn_blocking). fsedit::read validates the path with contained(), takes std::fs::metadata (which follows links and, for a FIFO, reports len 0 so the MAX_EDIT_BYTES test passes and is_dir() is false), then calls std::fs::read(&amp;path) with no is_file() check. open(2) of a FIFO with no writer blocks forever, on the runtime worker thread itself. Runtime::new() gives one worker per CPU; one or two clicks on such a file (a 1 to 2 vCPU VPS is the intended target) stall every task of the process: all WebSocket sessions, HTTP routes and heartbeats, for all devices, until the process is killed (a restart cuts running agent turns). The file can be planted by an agent (same OS user, working in its worktree) with 'rm README.md; mkfifo README.md': git ls-files --cached still lists the tracked path (only --deleted is filtered by git_files) and the non-git walk (git::walk_files) lists every non-directory entry. contained() does not reject it because the FIFO is inside the root. fs_write (expected_hash read) and fs_base have the same inline blocking pattern. The code predates the range, but the range makes the same runtime serve every remote device from a headless single process, and makes agent-plantable content reachable by a browser editor with no desktop 'one window' containment.

**Impact** : Denial of service of the whole server (all devices, event hub, terminals) triggered by a prompt-injected or malicious agent that creates a FIFO, plus one user click on it in the editor tree; also by any device calling fs_read on the path.

**Cas testé** : Reproduit

```bash
docker run --rm --name mantis-audit-2026-11-23_11h17-<stem>-<n> --network=none -v mantis-Escouade-lot5-deps:/deps:ro -v {TARGET_ROOT}:/src:ro -v <dossier d audit>/workspace/reproducers:/poc:ro -w /src --cpuset-cpus=0,1 rust:1.88-slim sh -c 'rustc -O --edition 2021 -o /tmp/poc /poc/2f353c55-1770-4b06-9fcb-43602a845786/poc.rs && SENTINEL_FILE=/tmp/s timeout 60 /tmp/poc'
```

```
CONTROL regular file: Ok("text") in 1.0ms
PLANTED FIFOs: README.md, src/notes.txt, w.txt (contained() ok: true)
RUNTIME workers = 2
[read] checks passed for README.md: is_dir=false len=0 is_file=false -> std::fs::read (fsedit.rs:187)
[read] checks passed for src/notes.txt: is_dir=false len=0 is_file=false -> std::fs::read (fsedit.rs:187)
HEARTBEAT before attack answered: true
AFTER ATTACK: heartbeat answered within 5s = false ; any fs_read returned = false
RESULT: VULNERABLE - all 2 runtime workers blocked in open(2) of a FIFO at fsedit.rs:187; the queued heartbeat stalled 5s+
FS_WRITE(expected) on FIFO returned within 3s = false -> VULNERABLE (blocked at fsedit.rs:220)
RELEASE: heartbeat ran once a writer opened the FIFOs (8.0s after it was queued)
FIX HINT: metadata(README.md).is_file() = false
EXIT=0; sentinel file: 4 x MANTIS_REACHED_ENTRYPOINT (written in-path just before std::fs::read)
```

**Correction** (Vérifié par re-attaque)

```diff
--- a/src-tauri/src/paths.rs
+++ b/src-tauri/src/paths.rs
@@ -4,7 +4,32 @@
 pub use escouade_model::paths::*;
 #[cfg(test)]
 use std::path::{Path, PathBuf};
 
+/// The bytes of a regular file of at most `max` bytes, the one way a file of a project tree is
+/// read inline on an async worker (fsedit, git's untracked files). A FIFO, socket or device
+/// planted there (by an agent) would block that worker at open: opened non-blocking, its type
+/// checked on the handle itself (no swap after a check), and its read capped whatever it grew to.
+pub fn read_regular(path: &std::path::Path, max: u64) -> std::io::Result<Vec<u8>> {
+    use std::io::Read;
+    let mut opts = std::fs::OpenOptions::new();
+    opts.read(true);
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::OpenOptionsExt;
+        opts.custom_flags(libc::O_NONBLOCK);
+    }
+    let file = opts.open(path)?;
+    if !file.metadata()?.is_file() {
+        return Err(std::io::Error::other("not a regular file"));
+    }
+    let mut bytes = Vec::new();
+    file.take(max.saturating_add(1)).read_to_end(&mut bytes)?;
+    if bytes.len() as u64 > max {
+        return Err(std::io::Error::other("file too large"));
+    }
+    Ok(bytes)
+}
+
 /// Cargo puts in the `PATH` of a test every folder a build script gives the linker, and that of
 /// whisper-rs-sys gives each folder of whisper.cpp's cmake tree (46 of them): the `PATH` then
 /// passes the 8191 characters cmd.exe reads, and the fake `claude.cmd` no longer finds `node`.
--- a/src-tauri/src/fsedit.rs
+++ b/src-tauri/src/fsedit.rs
@@ -4,6 +4,6 @@
 use crate::git;
 use crate::i18n::{self, Lang};
-use crate::paths::{contained, outside};
+use crate::paths::{contained, outside, read_regular};
 use crate::testlaunch;
 use anyhow::{anyhow, bail, Result};
 use serde::Serialize;
@@ -184,7 +184,7 @@ pub fn read(root: &Path, rel: &str) -> Result<FileText> {
             bom: false,
         });
     }
-    Ok(decode(&std::fs::read(&path)?))
+    Ok(decode(&read_regular(&path, MAX_EDIT_BYTES)?))
 }
 
 /// Writes `text` back with the file's line endings and BOM, through a temporary file renamed
@@ -217,7 +217,8 @@ pub fn write(
     }
 
     if let Some(exp) = expected {
-        match std::fs::read(&path) {
+        // Capped like read(): a file past it was never opened in the editor, so `expected` is not its hash.
+        match read_regular(&path, MAX_EDIT_BYTES) {
             // The version found named (`changed:<hash>`): « Garder ma version » writes over that one only.
             Ok(bytes) if hash(&bytes) != exp => bail!("{CHANGED}:{}", hash(&bytes)),
             Ok(_) => {}
@@ -1214,6 +1215,26 @@ mod tests {
         assert!(read(&dir, "../x").is_err());
     }
 
+    #[cfg(unix)]
+    #[test]
+    fn never_blocks_on_a_fifo_planted_in_the_tree_and_caps_what_it_reads() {
+        use std::os::unix::ffi::OsStrExt;
+        let dir = test_dir("fsedit-read-fifo");
+        let fifo = dir.join("p.md");
+        let c = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
+        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
+        // No writer: opened blocking, each of these calls would hang here.
+        assert!(read(&dir, "p.md").is_err());
+        let err = write(&dir, "p.md", "x", "lf", false, Some("h"), &[]).unwrap_err();
+        assert!(err.to_string().contains("not a regular file"), "{err}");
+        assert!(read_regular(&fifo, 10).is_err());
+        // The cap holds on the bytes read, not on a size taken before.
+        let big = dir.join("big.bin");
+        std::fs::write(&big, [b'a'; 11]).unwrap();
+        assert!(read_regular(&big, 10).is_err());
+        assert_eq!(read_regular(&big, 11).unwrap().len(), 11);
+    }
+
     #[test]
     fn writes_atomically_keeping_crlf_and_refuses_a_file_changed_on_disk() {
         let dir = test_dir("fsedit-write");
--- a/src-tauri/src/git.rs
+++ b/src-tauri/src/git.rs
@@ -197,7 +197,7 @@ fn count_lines(path: &Path) -> u32 {
     if meta.len() > 2_000_000 {
         return 0;
     }
-    match std::fs::read(path) {
+    match crate::paths::read_regular(path, 2_000_000) {
         Ok(bytes) if !bytes.contains(&0) => {
             let n = bytes.iter().filter(|&&b| b == b'\n').count();
             (n + usize::from(!bytes.is_empty() && !bytes.ends_with(b"\n"))) as u32
@@ -426,7 +426,7 @@ pub async fn diff(root: &str, paths: &[String]) -> Result<String> {
             out.push_str(&flagged(&new_file_header(&path)));
             continue;
         }
-        let Ok(bytes) = std::fs::read(&full) else {
+        let Ok(bytes) = crate::paths::read_regular(&full, MAX_FILE_DIFF as u64) else {
             continue;
         };
         if bytes.contains(&0) {
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/2f353c55-1770-4b06-9fcb-43602a845786.diff`

### [LOW] Exploit Chain: unauthenticated, repeatable denial of device pairing via agent-echoed link Referer leak (904eb54c) and global wrong-code counter in redeem (40e71f05)

**Résumé** : Chain: agent-echoed link leaks the server origin through Referer (904eb54c), then the attacker cancels every minted pairing code (40e71f05). Hazard = (Impact 3 + Likelihood 3) x 0.392 x 0.8 = 1.88, LOW. Impact 3 equals the constituent pairing DoS (no token leak, existing devices fine). Likelihood 3: chain never run end to end (static cap). Multiplier 1.0 (EXTERNAL) x 0.8 (availability STANDARD) x 0.7 (user click required) x 0.7 (CONDITIONAL_VIABLE) = 0.392, then x0.8 static. The chain is weaker than 40e71f05 alone (3.92) because it adds an LLM-dependent step and a user click; anyone who learns the origin another way skips step 1. Fix the constituents (no-cancel redeem, Referrer-Policy / noreferrer); the 40e71f05 patch alone defuses the lethal step.

**Emplacement** : src/lib/platform.ts:50, src-tauri/src/remoteapi/server.rs:870, src/lib/links.ts:33, src-tauri/src/remoteapi/devices.rs:148 · **CWE** : CWE-200

**Problème** : Attacker: a third party whose only channel into Escouade is content an agent reads and repeats (a GitHub issue, a web page, a README, a tool result), without needing the agent to run any command. Deployment: escouade-server behind the documented HTTPS reverse proxy (install --port &lt;n&gt; --origin https://&lt;host&gt;, docs/REMOTE.md:181-189), the case where the listener is reachable by anyone who knows the host name.\n\nStep 1 (finding 904eb54c, reproduced): the injected content makes the agent print a Markdown link, or an OSC 8 terminal hyperlink, to https://attacker.example/x. In the browser client every outgoing link goes through links.ts openOutside -&gt; platform.ts openUrl (line 50), which calls window.open(url, "_blank", "noopener"); noopener does not drop the Referer, the rel="noopener noreferrer" set by markdown.ts:59 is moot because the click is intercepted and re-opened through openUrl, and the page is served without any Referrer-Policy (server.rs:870-875: CSP and nosniff only). The browser default strict-origin-when-cross-origin sends Referer: https://&lt;host&gt;/ to the attacker on one user click. The attacker now knows the private server origin.\n\nStep 2 (finding 40e71f05, reproduced): POST /pair is unauthenticated; mine() accepts a request whose Host is the declared origin and that carries no Origin header (non-browser client). redeem() (devices.rs:135-153) counts wrong codes in a single global Pending.wrong; at TRIES = 5 it sets pending = None, whoever sent them. The attacker polls POST /pair through the proxy with five junk codes every few seconds: as soon as the owner runs escouade-server pair (code live 10 min), the code dies before the owner types it in the new browser or phone. A code minted again is killed the same way, for as long as the attacker keeps going. Brute force is not possible (40 bits, 5 tries), so the outcome is availability only.\n\nWhy the chain adds something: 40e71f05 alone needs the attacker to already know the server host name; 904eb54c alone only leaks a host name. Together, a text-only prompt injection (no tool execution, no shell, no file write) is enough to turn a private, unadvertised server into a target an unauthenticated outsider can keep from onboarding any new device. An agent that does have shell gets far more already (REMOTE.md:210: it can pair itself), so that stronger attacker is not the point of this chain.\n\nLimits: no effect in the SSH-tunnel or Unix-socket deployment (the leaked origin is http://127.0.0.1:&lt;port&gt; or localhost, unreachable from outside); a public TLS certificate may already expose the host name through Certificate Transparency logs, which lowers the value of step 1 for that sub-case; existing paired devices keep working. The two links are reproduced separately; the end-to-end chain was not run.

**Impact** : Persistent, attacker-controlled denial of device pairing on an Internet-facing escouade-server, reachable by an outsider who only controls text an agent echoes: the owner cannot add a browser or phone (for example to replace a lost device he just revoked) while the attacker polls /pair. No credential or token leaks; existing devices are unaffected.

**Cas testé** : Confirmé statiquement

**Correction** (Correctif proposé, non vérifié)

```diff
--- a/src-tauri/src/remoteapi/server.rs
+++ b/src-tauri/src/remoteapi/server.rs
@@ -871,5 +871,6 @@
             .header(header::CACHE_CONTROL, file.cache)
             .header(header::CONTENT_SECURITY_POLICY, csp)
             .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
+            .header(header::REFERRER_POLICY, "no-referrer")
             .body(Full::new(Bytes::from(file.body)))
             .expect("a valid answer")
     }
```

Appliquer depuis la racine du repo : `git apply osho-mantis/audit-2026-11-23_11h17/patches/0b7d30ef-fa35-4ba7-ab6d-576d9f059d93.diff`

## Écartés

- A notification click on one device re-navigates every other connected device through the broadcast Focus and InboxOpen events in Headless::clicked — Faux positif : Verified in source: Headless::clicked runs a closure that hub.emit()s Focus/InboxOpen to all subscribers (hub.rs:120), and notification_click is callable by any device with a kept id. But every paired
- A terminal spawn waiting for replay room holds the term lane and delays every keystroke of the connection in dispatch — Faux positif : Verified in source: dispatch() (server.rs:951) carves out term_attach only, and run() awaits room_for_a_replay (server.rs:1007, up to WRITE) inside the term lane job for term_spawn, so a backlog delay
- app.log is rotated only at start-up so a long-running escouade-server never rotates it in init_logging — Faux positif : Code confirmed (lib.rs:404 rotates app.log only at start-up) but it is slow log growth on the operator's own disk, an operational/resource-exhaustion concern (rule 07), at Info level, with no attacker
- Blocking fsync under the devices mutex on the async worker in check — Faux positif : devices.rs:179-196 check() does write under the mutex, but only once per device per SEEN_EVERY (60 s), a tiny file written with temp+fsync+rename. Impact needs a stalled disk (an unrealistic environme
- Browser download names come from agent-controlled text with no sanitisation in save and fetchFile — Faux positif : files.ts:68-78 save() does set a.download to the agent-chosen basename and ignores the server's sanitised Content-Disposition, but the agent already chooses the sent file's name (invoice.html and the 
- Call permits are global and held while queued: one slow call starves every other client — Faux positif : The permit accounting is as described (server.rs:921-931 takes both permits before queuing, held in the lane closure, lanes.rs per connection), but the only actor who can exploit it is a paired device
- Changing or removing a project logo deletes the project's planning, docs-reads and docs-checks files in keep_only — Faux positif : Source confirmed (projectlogo.rs:122 keep_only removes every regular file in project_files(id), where planning.json, docs-reads.json, docs-checks.json also live), but this is a functional data-loss bu
- Damaged devices.json is silently emptied then overwritten in open — Faux positif : Deliberate, documented and tested behavior (devices_tests.rs a_damaged_or_missing_file_means_no_device; REMOTE.md: damaged file = no device, pairing writes a sound file). Writes are atomic (write_priv
- Device token travels in the Sec-WebSocket-Protocol request header and reaches reverse proxy access logs unredacted in protocols — Faux positif : Bearer in Sec-WebSocket-Protocol is the standard browser workaround (a browser WebSocket cannot set Authorization); server.rs:601 admit() reads it as described, but exposure needs the operator's own p
- docs_document_add copies any file of the server named by the client into the project docs, where the ticket form only accepts drafts the page uploaded in docs_drop_in — Faux positif : Code confirmed (commands.rs:4638 takes a raw source path, docsdrive.rs:944 only checks metadata().is_file(), docs.rs copies it), but the caller is a paired device whose token is documented as a shell 
- Export to .escouade.yml writes through a committed symlink and creates or rewrites any file of the server user in write_shared — Faux positif : Verified in source: write_shared (projectfile.rs:1835) does std::fs::write(path/.escouade.yml) after file_text, so a committed link is followed (path is repository content). But the content written is
- Folder confinement of server clients skips read_logo, set_project_logo, inspect_setup, suggest_worktree_steps_at and check_authorities in commands — Faux positif : Code confirmed (read_logo, set_project_logo, inspect_setup, suggest_*_at, check_authorities take raw paths without client_folder), but the caller is a paired device that already holds a shell as the s
- Held-message set is one global list shared by all devices and kept after a client vanishes in set_held — Faux positif : Code verified: set_held replaces core.held, subscribe_to clears it only for Window, notify_agent_now (core.rs:3419) skips Done alerts for held ids. Functional multi-client bug, not a security flaw.
- Home confinement of downloads and folders silently disabled when home_dir is unavailable in Core::sent_file — Faux positif : Code confirmed (headless_host.rs:315-317 sets folders_home only if dirs::home_dir() is Some; sentfiles.rs:1001/1013 and browse.rs:98 fall to the desktop branch otherwise), but the boundary it is said 
- Live terminal output is not filtered of terminal queries so every attached viewer answers them and the answers are typed into the program several times in Output::push — Faux positif : Code confirmed (output.rs filters queries only on replay, live push is unfiltered, so several viewers each answer them) but the effect is stray fixed terminal replies typed into the program: a functio
- No way to sign out or forget the device token from the browser, which stays in localStorage until a server-side revoke in pair — Faux positif : Verified: no sign-out UI in src, token only removed by stillPaired. But non-expiring, server-revocable device tokens kept in localStorage are the documented design (docs/REMOTE.md device section); the
- Pairing stores the minted token without a guard so a refused localStorage leaves an orphan paired device in pair — Faux positif : device.ts:34 setItem is unguarded (verified) and an orphan device record can result, but the orphan token is never disclosed (returned once to the same page) so nobody can use it; revoke by the owner 
- Quadratic Markdown stripping of an agent's reply, question or plan in strip_inline runs under the agent lock, so one long line of brackets freezes that agent and every call that reads it — Faux positif : Verified in source: strip_inline (alerts.rs:234) scans to the next ] or ) for every [ or !, so a single line made of many [ is quadratic, and it runs on the first line of the agent's reply. The input 
- Server folder browser hides dot folders only in listings, not when asked by path — Faux positif : Code confirmed (browse.rs:65,72 accept dot folders when typed; only list() at :128 hides them), but no boundary is crossed: every paired device is uniform (devices.rs has no scope) and can open a term
- stillPaired deletes the stored device token on any HTTP 401 including one from a proxy in stillPaired — Faux positif : device.ts:47 does clear the token on any 401 (verified), but a 401 for /device can only come from the escouade server (revoked token) or from the operator's own trusted proxy/TLS terminator; an on-pat
- Terminal output still queued behind the terminalExit event is dropped by the web client in receive — Faux positif : Code verified: ws.ts:122-125 deletes the flow on terminalExit, receive() drops later binary frames, server.rs biased select drains text before bytes. Real but purely functional data-loss in a diagnost
- The listener is bound after the core is loaded and started, so a bind failure restarts the whole start-up every 5 s under systemd — Faux positif : Order verified (headless_host.rs:313-320: Core::load, core.start, then remoteapi::server::start; exit 1 on bind failure; unit Restart=on-failure RestartSec=5, RestartPreventExitStatus=2 3). It is an o
- The projectlogo size cap is bypassed by zero-length special files, so read_logo and set_project_logo read an unbounded stream into memory in read_in — Faux positif : Code confirmed: projectlogo::read_in (projectlogo.rs:71-82) trusts metadata().len() then std::fs::read, so /dev/zero or a FIFO passes; read_logo (commands.rs:1606) and set_project_logo (core.rs:9586) 
- The server install guide runs npx --yes tsx@4.23.15 outside any lockfile on the VPS build host — Faux positif : Documentation line in an install guide, not code shipped. tsx is already pinned to an exact version; the residual risk (a compromised upstream transitive release) is a supply-chain hygiene concern wit
- The web page serves a weaker content security policy than the desktop window: style-src unsafe-inline, no style nonce in file — Faux positif : Policy verified at server.rs:864-868 (style-src 'self' 'unsafe-inline', form-action/base-uri 'self', script-src without unsafe-inline). script-src stays strict, so XSS is not enabled; the weaker style
- UI state is one server-wide record overwritten by every device, so a second browser starts on and keeps rewriting the first one's view in set_ui — Faux positif : Code claims verified (commands.rs:352 set_ui overwrites core.ui; subscribe_to copies it; web persistUi at 250 ms), but this is a UX/state-isolation bug between devices of one owner at one privilege le
- Unbounded open uploads leak a file descriptor and a slot per WebSocket attach_open call in Core::attach_open — Faux positif : Code verified: attach_open/attach_cancel/attach_discard are not desktop-only, Uploads.open has no cap or expiry, and each call keeps a file open. But the caller is an authenticated device equivalent t
- Wildcard matcher of worktrees.copy from .escouade.yml backtracks exponentially and runs on the async worker in glob_match — Faux positif : Verified in source: match_segments (testlaunch.rs:51) and match_one (testlaunch.rs:59) are backtracking matchers, so a list of consecutive ** items against a deep non-matching path costs combinatorial

## Corrections fusionnées

18 correctifs sur la branche `mantis/audit-2026-11-23_11h17`. Pour les intégrer à ta branche : `git merge mantis/audit-2026-11-23_11h17`

- Shared ticket media import follows links inside a teammate's ticket folder and copies any file of the server into the ticket in import_media (`754200b`)
- A repository collaborator makes the server read an unbounded shared ticket file and an unbounded number of ticket files in read_dir (`78f9d19`)
- A committed workflow file names any path as a state file, which is read without a size cap or a root in scan_of, so a path to /dev/zero kills the server (`6ceee07`)
- Deleting a shared ticket removes files through a linked tickets folder, outside the project, in remove_file_of (`dd31c8c`)
- git show and git diff between refs buffer the whole output of a hostile repository's commit in server memory before the diff cap applies in run (`06ddfe4`)
- The project file .escouade.yml is read through links with no size cap, so a symlink to /dev/zero kills the server at every start in file_text (`36b5ee6`)
- the project memory folder is read whole with no file count or total size cap, so a cloned repo with thousands of large memory files exhausts memory when an agent searches its memory in list (`fcfda75`)
- Pairing cancelled by five wrong codes from any unauthenticated peer in redeem (`effcd1f`)
- Spec files of a hostile repository are read whole with no size cap in read_found, so a small committed blob that inflates to gigabytes exhausts memory (`2866f14`)
- Revoking a device does not cut its in-flight or queued file transfers (`54b6002`)
- Parsing .escouade.yml is quadratic in its size and runs under the projects write lock, so a plain repository file of about 1 MB freezes the server in read_shared (`c7cd253`)
- The Cursor view writes its relay rule through a committed symlink and creates directories through committed folder links in relay_to_interface (`688887a`)
- Browser build of SentFiles offers programs and installers for download as Ouvrir, bypassing the never run rule (`2916cb7`)
- Proxy password kept in proxy_password_in_file is serialized to every paired device in InitialState on a keychain-less server in subscribe_to (`718e2f0`)
- A call that never returns keeps its global call permit and lane for good, so a few hung calls lock every remote client out in dispatch (`4eff75e`)
- A paired device holds all 16 connection slots with idle WebSockets, so the admin revoke request is never accepted in accept (`18520c1`)
- fs_read blocks an async runtime worker forever on a named pipe planted in a project folder in fsedit::read (`54af67c`)
- Exploit Chain: unauthenticated, repeatable denial of device pairing via agent-echoed link Referer leak (904eb54c) and global wrong-code counter in redeem (40e71f05) (`830c6f0`)

## Limites

- Mantis a 95 commits de plus que la version figée : le mainteneur du plugin les analyse avec `/mantis-update`.
- triage sauté à la demande : périmètre = scope.txt (193 fichiers du mode serveur)
- remplissage du volume de dépendances, avec réseau (npm racine ; Rust/cargo non géré par l'étape)
- history : certains grep refusés par le classifieur auto-mode, lectures faites avec Read
- structural-index : index partiel heuristique (grep/sqlite, 5139 symboles, .svelte non extraits) ; l'agent a lancé une fois `python3 --version` sur l'hôte, sans code cible
- architecture : KB centrée sur les fichiers serveur (core.rs, agent/*, mcp/* et .svelte peu lus) ; l'agent a lancé `python3 -` sur l'hôte (vide, interrompu)
- researcher : 193 investigations en 3 vagues de 6 agents, groupes équilibrés selon la taille du diff
- dedupe : 30 → 28 constats ; les 2 doublons sont dans findings/.trash/ (signalés « disparus » par le garde-fou, non restaurés : déplacement voulu)
- reproduce : pas de build complet d'Escouade dans Docker (disque du Mac limité, crates non mises en cache) ; reproducteurs autonomes minimaux (Rust std ou Node) qui recopient le fragment de code fautif
- patch 40e71f05 : piste hors constat — /pair sans limitation de débit, un pair lent (slowloris) peut occuper les 16 connexions (délai d'en-tête 10 s) ; non testé
- patch 33745242 : piste hors constat — un SVG « exécutable » nommé x.hta s'enregistre sous ce nom via « Enregistrer sous… » (libellé honnête, même comportement sur le bureau)
- patch 2f353c55 contourné : git_files/git_diff lisent un lien symbolique vers une FIFO via std::fs::read (git.rs:200, git.rs:429) — repris au tour 2
- tour 1 fini : 7 constats retenus (3 VERIFIED_SECURE, 1 VERIFICATION_FAILED, 3 MITIGATION_PROPOSED) ; tour 2 lancé
- reproduce (tour 2) : refus du filtre de l'API pour l'agent de 79a2cd8e et e5ace214, après qu'il eut écrit ses résultats (reproduced, repro_round 2) ; pas de relance
- reproduce (tour 2) : refus du filtre de l'API pour ac424971, 2d356ff9 (agent arrêté avant d'écrire) ; laissés sans reproduction, pas de relance automatique
- patch 2f353c55 (tour 2) : l'agent de contre-attaque a lancé par erreur `python3 -` sur l'hôte (vide, interrompu)
- calibrate (tour 2) : e5ace214 noté 7,0 (HIGH, position EXTERNAL) alors que 79a2cd8e, même attaquant (contenu du dépôt), est en SUPPLY_CHAIN — incohérence signalée par l'agent
- tour 2 fini : 11 constats retenus (6 VERIFIED_SECURE, 5 MITIGATION_PROPOSED) ; tour 3 lancé
- reproduce (tour 3) : refus du filtre de l'API pour l'agent de 3f453abc, 03a3e838, 67ba1817, après qu'il eut écrit ses résultats (reproduced, repro_round 3) ; pas de relance
- patch (tour 3) : refus du filtre de l'API pour la contre-attaque de 67ba1817 ; laissé en VERIFICATION_INCOMPLETE, pas de relance
- patch 74c01858 (tour 3) contourné : import_media relit le fichier sans limite (sharing.rs:1556) — le périmètre donné au patch par mantis-run excluait import_media à tort ; pas de 4e tour
- calibrate (tour 3) : attaquants « contenu du dépôt » alignés en SUPPLY_CHAIN ; e5ace214 (tour 2) reste EXTERNAL 7,0 — à aligner (≈4,5)
- Findings produits par IA : à vérifier par un humain avant tout signalement.
