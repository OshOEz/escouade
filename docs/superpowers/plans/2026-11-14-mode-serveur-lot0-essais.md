# Mode serveur, lot 0 : les essais — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Savoir, par des essais mesurés, par quelle voie le cœur d'Escouade peut tourner dans un process sans fenêtre sur un Linux sans affichage, et si Claude Code s'y connecte par `/login`.

**Architecture:** Tout tourne dans un conteneur Docker Linux sans affichage (le poste n'a pas Rust). Le code d'essai vit sur une branche jetable `spike/lot0-sans-fenetre`, jamais fusionnée ni proposée à l'amont. Le seul livrable gardé est un compte rendu, commité sur `feat/mode-serveur-web`.

**Tech Stack:** Docker 29 (hôte macOS arm64, conteneur `linux/arm64`), Rust 1.98.0 (`src-tauri/rust-toolchain.toml`), Tauri 2, `scripts/linux-deps.sh` du dépôt.

**Spec:** `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (lot 0 ; à lire avec `docs/REMOTE.md`, « L'ordre proposé », point 1).

## Global Constraints

- Aucun code de `src-tauri/`, `crates/` ou `src/` n'est modifié sur `feat/mode-serveur-web` : les modifications d'essai restent sur `spike/lot0-sans-fenetre`.
- Rien n'est poussé, aucune issue ni PR n'est ouverte chez l'amont.
- Le conteneur n'a ni serveur X, ni Wayland, ni D-Bus de session, ni périphérique audio : `DISPLAY`, `WAYLAND_DISPLAY` et `DBUS_SESSION_BUS_ADDRESS` sont absents.
- Chaque affirmation du compte rendu dit sa source, comme `docs/MOBILE-STUDY.md` : **[mesuré]** (commande lancée ici, sortie gardée), **[vérifié]** (lu dans le code), **[lu]** (documentation, non vérifié).
- Le compte rendu est en français, avec tous ses accents, dans le ton des documents de `docs/`.
- Aucun identifiant de compte (jeton, `.credentials.json`) n'entre dans le dépôt ni dans le compte rendu.
- Les sorties brutes des commandes vont dans `spikes/lot0/results/` sur la branche d'essai ; le compte rendu en cite les lignes utiles.

## Review Focus

- Un essai qui « passe » parce que les bibliothèques GTK sont installées dans l'image : la question est ce qui est **lié**, pas ce qui s'exécute. La tâche 2 le mesure dans une image sans ces bibliothèques.
- Un cœur qui démarre mais dont une tâche de fond échoue en silence (journal seulement) : la tâche 3 lit le journal entier, pas seulement le code de sortie.
- Une dépendance au bureau qui ne se montre qu'à l'usage (presse-papiers, voix, notification) et pas au démarrage : la tâche 4 la cherche dans le code, pas seulement à l'exécution.
- Un classement des usages de `tauri::` qui compte des fichiers de tests : la tâche 5 les exclut (`*_tests.rs`, `tests/`).
- Une connexion `/login` déclarée bonne parce que le terminal dit « connecté », alors qu'Escouade ne voit pas le compte : la tâche 6 vérifie le fichier que lit Escouade.

---

### Task 1: L'environnement d'essai

**Files:**
- Create (branche d'essai) : `spikes/lot0/Dockerfile`, `spikes/lot0/run.sh`, `spikes/lot0/results/.gitkeep`

**Interfaces:**
- Produces : l'image `escouade-lot0` ; `spikes/lot0/run.sh <commande…>` qui lance la commande dans le conteneur, le dépôt monté sur `/work`, avec deux volumes nommés pour le cache (`escouade-lot0-cargo` sur `/usr/local/cargo/registry`, `escouade-lot0-target` sur `/work/src-tauri/target`).

- [ ] **Step 1 :** Créer la branche `spike/lot0-sans-fenetre` depuis `feat/mode-serveur-web`, dans un worktree à part (`git worktree add ../Escouade-lot0 -b spike/lot0-sans-fenetre`).
- [ ] **Step 2 :** Écrire `spikes/lot0/Dockerfile` : base `debian:bookworm-slim`, Rust 1.98.0 par `rustup` (profil minimal), `build-essential`, `pkg-config`, `git`, `ca-certificates`, `curl`. **Aucun** paquet GTK, WebKit, X11, D-Bus ou ALSA dans cette image : c'est l'image « nue ».
- [ ] **Step 3 :** Ajouter une seconde cible `desktop-libs` dans le même Dockerfile, qui part de l'image nue et lance `scripts/linux-deps.sh` (les paquets que la CI installe, `.github/workflows/ci.yml:144`). Elle sert de témoin.
- [ ] **Step 4 :** Écrire `spikes/lot0/run.sh` (argument optionnel `--desktop-libs` pour la seconde image).
- [ ] **Step 5 :** Vérifier.

Run : `spikes/lot0/run.sh sh -c 'rustc --version; echo "DISPLAY=[$DISPLAY] DBUS=[$DBUS_SESSION_BUS_ADDRESS]"; pkg-config --exists gtk+-3.0; echo gtk=$?'`
Expected : `rustc 1.98.0`, `DISPLAY=[] DBUS=[]`, `gtk=1`.

- [ ] **Step 6 :** Commit sur la branche d'essai (`chore(spike): l'environnement des essais du lot 0`).

### Task 2: Ce que Tauri lie sous Linux

La question : `escouade-server` peut-il dépendre de la crate `tauri` sans lier WebKitGTK ni GTK ?

**Files:**
- Create (branche d'essai) : `spikes/lot0/tauri-link/Cargo.toml`, `spikes/lot0/tauri-link/src/main.rs`, `spikes/lot0/results/tauri-link.txt`

**Interfaces:**
- Consumes : `spikes/lot0/run.sh` (tâche 1).
- Produces : `results/tauri-link.txt`, lu par la tâche 7.

- [ ] **Step 1 :** Relever l'arbre actuel. Run : `spikes/lot0/run.sh sh -c 'cd src-tauri && cargo tree --target aarch64-unknown-linux-gnu -e normal -i gtk; cargo tree --target aarch64-unknown-linux-gnu -e normal -i webkit2gtk; cargo tree --target aarch64-unknown-linux-gnu -e normal -i glib'`. Garder la sortie : quelles crates tirent `gtk`, `webkit2gtk`, `glib`.
- [ ] **Step 2 :** Écrire la crate minimale `tauri-link` : dépendance `tauri = { version = "2", default-features = false, features = ["test"] }`, un `main` qui appelle `tauri::test::mock_app()` et affiche la version du paquet. Même version de Tauri que `src-tauri/Cargo.lock`.
- [ ] **Step 3 :** La construire dans l'image **nue**. Run : `spikes/lot0/run.sh sh -c 'cd spikes/lot0/tauri-link && cargo build 2>&1 | tail -40'`.
- [ ] **Step 4 :** Noter le résultat, qui est l'un de ces trois : (a) construit et s'exécute sans bibliothèque GTK ; (b) échoue à la construction, avec le nom de la crate `*-sys` et de la bibliothèque manquante ; (c) construit mais échoue à l'exécution.
- [ ] **Step 5 :** Si (b) ou (c) : refaire dans l'image `--desktop-libs`, puis `ldd target/debug/tauri-link` pour lister les bibliothèques liées. Garder la liste.
- [ ] **Step 6 :** Écrire `results/tauri-link.txt` : les commandes, leurs sorties utiles, et la réponse en une phrase à la question de la tâche.
- [ ] **Step 7 :** Commit (`chore(spike): ce que Tauri lie sous Linux sans affichage`).

### Task 3: Le cœur dans un process sans fenêtre

**Files:**
- Create (branche d'essai) : `src-tauri/src/bin/escouade-headless-spike.rs`, `spikes/lot0/results/core-headless.txt`
- Modify (branche d'essai) : `src-tauri/Cargo.toml` (la fonctionnalité `test` de `tauri` hors des dépendances de dev, pour cet essai seulement)

**Interfaces:**
- Consumes : `spikes/lot0/run.sh` ; le montage des tests, `src-tauri/src/core_tests.rs:124-125` (`mock_app()` puis `Core::load(app.handle().clone(), data)`).
- Produces : `results/core-headless.txt`, lu par les tâches 4 et 7.

- [ ] **Step 1 :** Lire comment `core_tests.rs` construit son `DataDir` et ce que `lib.rs` fait autour de `Core::load` dans `.setup` (`src-tauri/src/lib.rs:508-648`) : ce que l'app démarre après le chargement (serveur MCP, serveur des hooks, plugins, voix, mises à jour). En faire la liste dans le fichier de résultats.
- [ ] **Step 2 :** Écrire le binaire `escouade-headless-spike` : un dossier de données temporaire, `mock_app()`, `Core::load`, puis il démarre ce que la liste de l'étape 1 dit nécessaire au cœur (au moins le serveur MCP), affiche `core up` avec le nombre de projets et le port du serveur MCP, attend 20 secondes, et sort avec le code 0. Le journal (`log`) va sur la sortie d'erreur, niveau `debug`.
- [ ] **Step 3 :** Construire et lancer dans l'image `--desktop-libs` (témoin : on sait que les bibliothèques sont là). Run : `spikes/lot0/run.sh --desktop-libs sh -c 'cd src-tauri && cargo run --bin escouade-headless-spike 2> ../spikes/lot0/results/core-headless.log; echo exit=$?'`. La première construction est longue (compter plusieurs dizaines de minutes sur ce poste).
  Expected : `core up`, `exit=0`. Sinon, garder la panique ou l'erreur exacte.
- [ ] **Step 4 :** Pendant les 20 secondes, depuis un second `docker exec`, appeler le serveur MCP sur son port (`curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:<port>/`) : on attend une réponse HTTP (un refus d'authentification compte comme une réponse).
- [ ] **Step 5 :** Lire `core-headless.log` en entier et relever chaque ligne `WARN` ou `ERROR`, et tout ce qui nomme un affichage, D-Bus, GTK, ALSA, un trousseau ou une notification.
- [ ] **Step 6 :** Si la tâche 2 a répondu (a), refaire les étapes 3 à 5 dans l'image nue et noter la différence. Sinon, noter que l'essai dans l'image nue est sans objet et pourquoi.
- [ ] **Step 7 :** Écrire `results/core-headless.txt` : la liste de l'étape 1, les commandes, le résultat, les lignes relevées.
- [ ] **Step 8 :** Commit (`chore(spike): le cœur chargé dans un process sans fenêtre`).

### Task 4: Ce qui suppose un bureau

**Files:**
- Create (branche d'essai) : `spikes/lot0/results/desktop-assumptions.md`

**Interfaces:**
- Consumes : `results/core-headless.txt` (tâche 3).
- Produces : `results/desktop-assumptions.md`, lu par la tâche 7.

- [ ] **Step 1 :** Chercher dans `src-tauri/src` et `crates/` (hors tests) chaque point cité par `docs/REMOTE.md`, « Un serveur sans affichage ni son », et ceux-ci : `tauri_plugin_single_instance` (D-Bus), `cpal` (audio), le presse-papiers (`clipboard.rs`, GTK), le trousseau (`keyring` ou équivalent), `tauri_plugin_notification`, le tray et les menus, `tauri_plugin_dialog`, `tauri_plugin_opener`, `tauri_plugin_updater`, `tauri_plugin_deep_link`, `tauri_plugin_window_state`.
- [ ] **Step 2 :** Pour chacun, remplir une ligne du tableau : le fichier et la ligne, quand il est touché (au démarrage, à l'usage d'une commande, jamais sous Linux), ce qui se passe sans bureau (**[mesuré]** dans la tâche 3, ou **[vérifié]** dans le code), et ce que le mode serveur en fait d'après la spec (remplacé, caché, rendu par le navigateur).
- [ ] **Step 3 :** Ajouter ce que la tâche 3 a montré et que la liste ne prévoyait pas.
- [ ] **Step 4 :** Commit (`chore(spike): ce qui suppose un bureau dans le cœur`).

### Task 5: Les usages de `tauri::` et la voie de séparation

**Files:**
- Create (branche d'essai) : `spikes/lot0/tauri-uses.sh`, `spikes/lot0/results/tauri-uses.md`

**Interfaces:**
- Consumes : `results/tauri-link.txt` (tâche 2).
- Produces : `results/tauri-uses.md`, avec le tableau et les deux chiffrages, lu par la tâche 7.

- [ ] **Step 1 :** Écrire `tauri-uses.sh` : pour chaque fichier `.rs` de `src-tauri/src` et de `crates/` qui n'est ni `*_tests.rs` ni sous `tests/`, compter les usages de `tauri::` par famille : `async_runtime` (lancer une tâche), `ipc::Channel` et `ipc::Response`, `AppHandle`, `Manager` et `State`, `Runtime` et `Wry` (le paramètre de type), `Emitter`, fenêtres et webviews (`WebviewWindow`, `Window`, `webview`), `tray` et `menu`, `path`, `#[tauri::command]`, et « autre ».
- [ ] **Step 2 :** Le lancer et garder le tableau : une ligne par fichier, une colonne par famille, les totaux. Vérifier le total de `#[tauri::command]` contre la spec (228 dans `commands.rs`).
- [ ] **Step 3 :** Ranger les fichiers en trois groupes : ceux qui n'utilisent que `async_runtime` (remplaçable par `tokio` sans changer de sens) ; ceux du cœur qui tiennent à Tauri par `AppHandle`, `Manager`, `Runtime` ou `Channel` ; ceux qui sont l'app de bureau elle-même (fenêtres, tray, menus, `lib.rs`).
- [ ] **Step 4 :** Chiffrer les deux voies de la spec, en fichiers touchés et en lignes approximatives : **(A)** des fonctionnalités Cargo sur la crate actuelle (`tauri` facultatif, le code de bureau derrière `#[cfg(feature = "desktop")]`) ; **(B)** le cœur sorti dans une crate à lui (`crates/escouade-core`), l'app de bureau et le serveur en dépendant. Pour chacune : ce qui bouge, ce que la relecture de l'amont devra lire, le risque de conflit avec `upstream/main` (regarder `git log --since=30.days --stat` sur les fichiers les plus touchés) et la cohérence avec le découpage en crates déjà fait (`docs/CRATES.md`).
- [ ] **Step 5 :** Si la tâche 2 a répondu (a), ajouter la voie **(C)** : garder `tauri` lié avec son runtime sans fenêtre, et son coût.
- [ ] **Step 6 :** Commit (`chore(spike): les usages de tauri dans le cœur et les voies de séparation`).

### Task 6: La connexion de Claude Code sans navigateur sur place

Cette tâche demande un compte Claude réel : **l'étape 3 est faite par le demandeur**, pas par un agent.

**Files:**
- Create (branche d'essai) : `spikes/lot0/login.sh`, `spikes/lot0/results/claude-login.md`

**Interfaces:**
- Consumes : l'image nue (tâche 1).
- Produces : `results/claude-login.md`, lu par la tâche 7.

- [ ] **Step 1 :** Lire comment Escouade sait qu'un compte est connecté : `src-tauri/src/accounts.rs` (le dossier du compte donné par `CLAUDE_CONFIG_DIR`) et la lecture de la connexion dans `plugins/claude/` (`signin.rs`). Noter le fichier exact qu'il lit sous Linux et les champs qu'il attend.
- [ ] **Step 2 :** Écrire `login.sh` : il lance un conteneur de l'image nue avec Node et Claude Code installés par la voie officielle, un `CLAUDE_CONFIG_DIR` dans un volume nommé (`escouade-lot0-claude`, jamais dans le dépôt), et ouvre `claude` dans un terminal interactif (`docker run -it`).
- [ ] **Step 3 (le demandeur) :** Lancer `spikes/lot0/login.sh`, taper `/login`, ouvrir l'adresse affichée dans le navigateur du poste, coller le code dans le terminal. Dire ce qui s'est affiché à chaque pas.
- [ ] **Step 4 :** Dans le conteneur, vérifier que le fichier de l'étape 1 existe dans le dossier du compte et porte les champs attendus (afficher les **noms** des champs seulement, jamais leurs valeurs). Lancer `claude -p "réponds OK"` et noter la réponse.
- [ ] **Step 5 :** Écrire `results/claude-login.md` : le parcours pas à pas, ce qui a marché, ce qui a demandé un geste particulier (un retour par `localhost` qui échoue, un code à coller), et si Escouade verrait le compte comme connecté.
- [ ] **Step 6 :** Commit (`chore(spike): la connexion de Claude Code dans un conteneur sans navigateur`).

### Task 7: Le compte rendu

**Files:**
- Create (branche `feat/mode-serveur-web`) : `docs/superpowers/specs/2026-11-14-mode-serveur-lot0-compte-rendu.md`
- Modify (branche `feat/mode-serveur-web`) : `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (la ligne « Pas de Tauri à l'édition de liens du serveur » et le tableau des lots, d'après la voie retenue)

**Interfaces:**
- Consumes : les cinq fichiers de `spikes/lot0/results/` (tâches 2 à 6).

- [ ] **Step 1 :** Écrire le compte rendu, dans cet ordre : « En bref » (cinq lignes au plus, la voie recommandée en tête) ; les quatre réponses (ce que Tauri lie ; le cœur sans fenêtre ; ce qui suppose un bureau, en tableau ; la connexion de Claude Code) ; les voies de séparation avec leur chiffrage ; la recommandation et ce qu'elle change à la taille du lot 1 ; ce qui n'a pas été essayé (x64, les autres harness, un vrai VPS).
- [ ] **Step 2 :** Vérifier que chaque affirmation porte **[mesuré]**, **[vérifié]** ou **[lu]**, et qu'aucun identifiant n'y figure.
- [ ] **Step 3 :** Mettre la spec à jour d'après la recommandation, sans changer ce que le demandeur a décidé.
- [ ] **Step 4 :** Commit sur `feat/mode-serveur-web` (`docs(remote): le compte rendu des essais du lot 0`). La branche `spike/lot0-sans-fenetre` reste locale ; son nom et son dernier commit sont cités dans le compte rendu.
