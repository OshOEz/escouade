# Mode serveur, lot 1 : le cœur sans `AppHandle` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `Core` ne tient plus d'`AppHandle`, n'est plus générique sur `tauri::Runtime` et ne lance plus ses tâches par `tauri::async_runtime` ; l'app de bureau se comporte exactement comme avant.

**Architecture:** Trois changements indépendants, chacun proposable seul à l'amont, dans cet ordre : (1) un module de runtime à soi, que Tauri reçoit au démarrage ; (2) un trait `Host` que l'app de bureau implémente avec Tauri, à la place de `Core.app` ; (3) le paramètre `R: Runtime` retiré du cœur, devenu inutile. `hub.rs`, `commands.rs` et les `Channel` restent liés à Tauri : c'est le lot 2.

**Tech Stack:** Rust 1.98.0, Tauri 2.12, tokio 1 ; construction et tests dans le conteneur du lot 0 (`spikes/lot0/run.sh --desktop-libs`), le poste n'ayant pas Rust.

**Spec:** `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (lot 1, et « Décidé par le demandeur après le lot 0 ») ; chiffrage et sites : `docs/superpowers/specs/2026-11-14-mode-serveur-lot0-compte-rendu.md`, § 5 et § 6.

## Global Constraints

- **Aucun changement de comportement** de l'app de bureau : ni texte, ni ordre d'événements, ni notification, ni tray.
- La branche `feat/coeur-sans-apphandle` part de `upstream/main` (à récupérer juste avant), dans un worktree à part `../Escouade-lot1`. Elle ne contient **que du code** : ni `docs/superpowers/`, ni `spikes/`, ni `.superpowers/`.
- Un commit par tâche, proposable seul. Titres en français, au format de l'amont : `refactor(cœur): …`.
- Rien n'est poussé ; aucune issue ni PR chez l'amont.
- Après chaque tâche, dans le conteneur `--desktop-libs` : `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, et les tests de la tâche 0 donnent **le même résultat que la référence** (mêmes tests verts, mêmes tests rouges).
- Pas de nouvelle dépendance hors `tokio`, déjà dans le workspace.
- Les fichiers de l'app de bureau gardent leur `AppHandle<R>` pour leur propre usage : `lib.rs`, `commands.rs`, `menus.rs`, `notify.rs`, `navigation.rs`, `pagecrash.rs`, `devpage.rs`, `devpage_probe.rs`, `fileorigin.rs`, `windowthread.rs`, `clipboard.rs`, `deeplink.rs`, `updates.rs`.
- Les commentaires du code sont en anglais, dans le style du dépôt (ce qu'on ne lit pas dans le code, en une ligne).
- Hors de ce lot : la séparation par fonctionnalités Cargo, le job de CI sans GTK, `hub.rs`, les trois types de bureau que le cœur garde (`PageCrash`, `FileOrigins` comme type, `PageEvent`), `ort` hors de la voix.

## Review Focus

- Une tâche lancée **hors d'un runtime tokio** (un fil du système, un rappel de Tauri, un `Drop`) : `tauri::async_runtime::spawn` y marche, `tokio::spawn` panique. Le module de runtime doit marcher partout où l'ancien marchait (test en tâche 1).
- **Deux runtimes** : si Tauri crée le sien avant de recevoir le nôtre, les tâches se répartissent sur deux runtimes sans erreur visible. `set` doit passer avant tout usage (test en tâche 1).
- Un `JoinHandle` attendu : celui de Tauri rend `Result<T, tauri::Error>`, celui de tokio `Result<T, JoinError>`. Un `.await` qui ignorait l'erreur compile toujours ; un `?` ou un `match` change de type (tâche 1, étape 4).
- Le **clic sur une notification** : aujourd'hui il montre la fenêtre, puis émet l'événement de focus. L'ordre doit rester (test en tâche 2).
- Un test qui passait parce que `mock_app()` n'a ni tray ni fenêtre et **ne vérifiait rien** : le faux `Host` enregistre les appels, et les tests qui existaient sur les alertes et les libellés doivent toujours lire les mêmes valeurs (tâche 2, étape 6).

---

### Task 0: L'environnement et la référence

**Files:**
- Create (non suivis, exclus par `.git/info/exclude`) : `spikes/lot0/Dockerfile`, `spikes/lot0/run.sh`, copiés de la branche `spike/lot0-sans-fenetre`
- Create (hors dépôt) : `<espace du plan>/baseline.txt`

**Interfaces:**
- Produces : `spikes/lot0/run.sh --desktop-libs <commande…>` dans le worktree ; `baseline.txt`, la liste des tests et leur résultat sur `upstream/main` ; la commande de test retenue, écrite en tête de `baseline.txt`.

- [ ] **Step 1 :** `git fetch upstream`, puis `git worktree add ../Escouade-lot1 -b feat/coeur-sans-apphandle upstream/main`. Noter le commit de base.
- [ ] **Step 2 :** Copier `spikes/lot0/Dockerfile` et `run.sh` depuis `../Escouade-lot0`, et ajouter `spikes/lot0/` à `.git/info/exclude`. Vérifier que `git status` ne les montre pas.
- [ ] **Step 3 :** Lire dans `.github/workflows/ci.yml` comment la CI lance les tests Rust sous Linux (paquets, profil, variables). Retenir la commande la plus proche qui tourne dans le conteneur ; à défaut, `cargo test --locked --workspace`.
- [ ] **Step 4 :** La lancer sur la base, sortie gardée. Écrire `baseline.txt` : la commande, le nombre de tests verts, et la liste nominative des tests rouges ou ignorés avec leur cause (pas de `git` configuré, pas de harness, pas d'affichage).
- [ ] **Step 5 :** Lancer `cargo fmt --all --check` et `cargo clippy --locked --workspace --all-targets -- -D warnings` sur la base ; noter s'ils passent.

Pas de commit : rien de suivi ne change.

### Task 1: Un runtime à soi

**Files:**
- Create : `crates/escouade-base/src/rt.rs` (voir l'étape 1 pour le cas où ce crate ne convient pas)
- Modify : `crates/escouade-base/Cargo.toml`, `crates/escouade-base/src/lib.rs`, `src-tauri/src/lib.rs` (le démarrage), `crates/escouade-plugins/Cargo.toml` et `src/runtime.rs:1316`, et les 28 fichiers hors tests et 2 de tests qui écrivent `tauri::async_runtime::` (68 `spawn`, 16 `spawn_blocking`, 1 `block_on` dans `telemetry.rs:529`, 6 `JoinHandle` dans `core.rs:730, 5052, 5062, 5079, 5095` et `tui.rs:1359`)

**Interfaces:**
- Produces, dans `escouade_base::rt` :
  - `pub fn handle() -> tokio::runtime::Handle` — le runtime global, créé au premier appel (multi-fils, tout activé) ;
  - `pub fn spawn<F>(f: F) -> JoinHandle<F::Output> where F: Future + Send + 'static, F::Output: Send + 'static` ;
  - `pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T> where F: FnOnce() -> T + Send + 'static, T: Send + 'static` ;
  - `pub fn block_on<F: Future>(f: F) -> F::Output` ;
  - `pub use tokio::task::JoinHandle;`

- [ ] **Step 1 :** Vérifier qu'`escouade-base` n'est jamais construit pour `wasm32-wasip1` (`cargo tree -i escouade-base --target wasm32-wasip1 --workspace`, et `scripts/plugins-wasm.sh`). S'il l'est, créer à la place un crate `crates/escouade-rt` avec le même module et le dire dans le compte rendu de la tâche ; la suite ne change pas.
- [ ] **Step 2 : les tests, qui échouent d'abord** (dans `rt.rs`) :

```rust
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
    let there = block_on(async { spawn(async { tokio::runtime::Handle::current().id() }).await.unwrap() });
    assert_eq!(here, there);
}

#[test]
fn a_blocking_task_gives_its_value() {
    assert_eq!(block_on(async { spawn_blocking(|| 3).await.unwrap() }), 3);
}
```

- [ ] **Step 3 :** Les lancer : échec attendu, `rt` n'existe pas. Puis écrire le module (un `OnceLock<tokio::runtime::Runtime>`), et les relancer : verts.
- [ ] **Step 4 :** Dans `src-tauri/src/lib.rs`, avant de construire l'app et avant toute tâche, donner ce runtime à Tauri : `tauri::async_runtime::set(escouade_base::rt::handle());`. Ajouter dans les tests de l'app un test `tauri_runs_on_our_runtime` qui, après ce `set`, vérifie que `tauri::async_runtime::handle()` et `rt::handle()` désignent le même runtime (mêmes `id()` relevés depuis une tâche lancée par chacun).
- [ ] **Step 5 :** Remplacer chaque `tauri::async_runtime::{spawn, spawn_blocking, block_on, JoinHandle}` par celui de `rt`, fichiers de l'app de bureau compris. Pour chaque `JoinHandle` attendu, lire ce que le code faisait de l'erreur et garder le même effet. Lister dans le compte rendu les sites où le type d'erreur a compté.
- [ ] **Step 6 :** Retirer `tauri` de `crates/escouade-plugins/Cargo.toml` s'il ne sert plus qu'à ce `spawn` (ses tests compris : `plugins_reactor_tests.rs:577`) ; sinon dire ce qui le retient.
- [ ] **Step 7 :** Vérifier : `git grep -n "async_runtime::" -- src-tauri/src crates` ne rend que la ligne du `set` ; fmt, clippy, tests égaux à la référence plus les quatre nouveaux.
- [ ] **Step 8 :** Commit : `refactor(cœur): un runtime à soi à la place de celui de Tauri`.

### Task 2: Le trait `Host`

**Files:**
- Create : `src-tauri/src/host.rs`
- Modify : `src-tauri/src/core.rs` (le champ `:912`, `load` `:1866` et ses trois usages `:1999`, `:2006`, `:2014-2015`, les sites `:3483-3489`, `:3527-3528`, `:3542`, `:5359`), `src-tauri/src/commands.rs:149`, `src-tauri/src/updates.rs:1005`, `src-tauri/src/lib.rs` (l'appel de `Core::load`), les 20 fichiers de tests qui appellent `mock_app()` pour charger un cœur

**Interfaces:**
- Consumes : `escouade_base::rt` (tâche 1).
- Produces :

```rust
/// What the core asks of the app it runs in: the desktop app answers with Tauri, a server without.
pub trait Host: Send + Sync + 'static {
    fn version(&self) -> String;
    /// The folder of the installer's plugin seed, when there is one.
    fn seed_dir(&self) -> Option<std::path::PathBuf>;
    fn file_origins(&self) -> crate::fileorigin::FileOrigins { Default::default() }
    /// The main window is shown and has the focus.
    fn window_attended(&self) -> bool;
    fn window_visible(&self) -> bool;
    fn attention(&self);
    /// `on_click` runs after the host has brought its window forward.
    fn toast(&self, title: &str, body: &str, on_click: Box<dyn Fn() + Send + Sync>);
    /// The agents waiting, for the tray icon and its tooltip.
    fn waiting(&self, n: usize);
    fn relabel(&self, lang: crate::i18n::Lang, waiting: usize);
    fn stop_voice(&self);
}

pub struct Desktop<R: tauri::Runtime>(pub tauri::AppHandle<R>);   // impl Host, with the calls the core made itself
```

  - `Core.host: Arc<dyn Host>` remplace `Core.app` ; `Core::load(host: Arc<dyn Host>, data: DataDir)`.
  - Sous `#[cfg(test)]`, `host::Fake` : rend `"0.0.0-test"`, pas de graine, fenêtre ni visible ni au premier plan, et garde dans des `Mutex<Vec<…>>` publics les appels reçus (`toasts: (String, String)`, `waiting: usize`, `relabels: (Lang, usize)`, `attention: usize`, `voice_stops: usize`).

- [ ] **Step 1 : le test, qui échoue d'abord** (dans les tests du cœur, sur un cœur chargé avec `host::Fake`, notifications du système demandées et fenêtre hors du premier plan) : un agent qui passe en attente donne exactement un toast dont le titre et le corps sont ceux que le cœur garde déjà dans `alerts`, un `attention`, et `waiting` reçoit le nombre d'agents en attente ; changer la langue donne un `relabel` avec cette langue et ce nombre. Un second test : appeler la fermeture `on_click` du toast émet l'événement de focus sur le hub.
- [ ] **Step 2 :** Le lancer : échec attendu, `host` n'existe pas.
- [ ] **Step 3 :** Écrire `host.rs`. `Desktop` reprend à l'identique ce que le cœur faisait : `notify::window_attended`, `notify::flash`, `notify::toast` avec `notify::show_main` appelé avant `on_click`, le tray par `menus::TRAY_ID`, `notify::tray_icon` et `menus::tray_tooltip`, `menus::relabel`, l'arrêt de `voice::Voice` par `try_state`, `package_info().version`, `path().resource_dir()` joint à `seed`, `try_state::<FileOrigins>()`, et la fenêtre visible comme `updates.rs` la lisait.
- [ ] **Step 4 :** Brancher le cœur sur `self.host` aux sites listés ; `lib.rs` construit `Arc::new(host::Desktop(app.handle().clone()))`. `updates.rs:795` garde son propre `AppHandle` (c'est l'app de bureau).
- [ ] **Step 5 :** Relancer les tests de l'étape 1 : verts.
- [ ] **Step 6 :** Dans les tests, remplacer `mock_app()` par `host::Fake` là où il ne servait qu'à charger un cœur. Un test qui lisait vraiment la fausse app (tray, menus, fenêtre) reste sur `mock_app()` avec `host::Desktop` : les lister dans le compte rendu.
- [ ] **Step 7 :** Vérifier : `git grep -n -E "(self|core)\.app\b" -- src-tauri/src` ne rend rien hors des fichiers de l'app de bureau ; fmt, clippy, tests égaux à la référence plus les nouveaux.
- [ ] **Step 8 :** Commit : `refactor(cœur): un trait Host à la place de l'AppHandle du cœur`.

### Task 3: Le paramètre `R: Runtime` retiré du cœur

**Files:**
- Modify : `src-tauri/src/core.rs:909` (`pub struct Core<R: Runtime = Wry>` devient `pub struct Core`) et les fichiers du cœur qui portent `R` à cause de lui (75 fichiers relevés par le lot 0, dont 4 où `<R>` n'est pas celui de Tauri ; les plus chargés : `mcp/act.rs`, `mcp/docs.rs`, `mcp/read.rs`, `mcp/tools.rs`, `mcp/files.rs`, `mcp/resolve.rs`, `repos.rs`)

**Interfaces:**
- Consumes : `Core.host` (tâche 2).
- Produces : `Core` sans paramètre de type ; `CoreState` dans `commands.rs` désigne `State<Arc<Core>>` comme avant.

- [ ] **Step 1 :** Retirer le paramètre de `Core`, puis suivre le compilateur : chaque `impl<R: Runtime> … Core<R>`, chaque `fn …<R: Runtime>(… &Core<R> …)` perd `R`. Ne pas toucher aux `<R>` qui ne sont pas `tauri::Runtime`, ni à ceux des fichiers de l'app de bureau qui prennent un `AppHandle<R>` pour eux-mêmes.
- [ ] **Step 2 :** Retirer les `use tauri::{Runtime, Wry}` devenus inutiles (clippy les signale).
- [ ] **Step 3 :** Vérifier : `git grep -n -E "Core<R>|Core<MockRuntime>|Core<Wry>" -- src-tauri crates` ne rend rien ; fmt, clippy, tests égaux à la référence (aucun test nouveau : rien ne change de sens).
- [ ] **Step 4 :** Relever pour le compte rendu ce qui lie encore le cœur à Tauri après ce lot : relancer `spikes/lot0/tauri-uses.sh` (copié de la branche d'essai, non suivi) et donner le tableau avant et après.
- [ ] **Step 5 :** Commit : `refactor(cœur): Core n'est plus générique sur le runtime de Tauri`.
