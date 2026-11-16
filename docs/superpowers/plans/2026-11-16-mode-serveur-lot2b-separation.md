# Mode serveur, lot 2b : le cœur se construit sans bureau — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `cargo check --no-default-features` construit le cœur d'Escouade dans une image Linux sans GTK, WebKit, D-Bus ni ALSA, avec la recherche par le sens ; l'app de bureau, construite par défaut, ne change pas.

**Architecture:** La voie (A) décidée après le lot 0 : la crate reste où elle est, et ce qui tient au bureau passe derrière des fonctionnalités Cargo activées par défaut (`desktop` pour Tauri et la fenêtre, `voice` pour la voix locale). Avant de poser les fonctionnalités, le cœur cesse de nommer les modules de bureau (trois types à déplacer, un appel de télémétrie), la macro des commandes n'écrit la commande Tauri que sous `desktop`, et `ort` sort de la crate de la voix pour que la recherche par le sens ne tire plus l'audio. Un job de CI Linux sans GTK garde le tout.

**Tech Stack:** Rust 1.98.0, Cargo features, Tauri 2.12 (facultatif), GitHub Actions ; construction et tests dans le conteneur (`spikes/lot0/run.sh`, image `--desktop-libs` pour le défaut, image nue pour le serveur).

**Spec:** `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (« Décidé par le demandeur après le lot 0 » : voie (A), recherche par le sens gardée, job de CI tôt) ; `docs/superpowers/specs/2026-11-14-mode-serveur-lot0-compte-rendu.md` § 3, § 5 ; `docs/REMOTE.md`, « La fondation commune », point 1, et « Le mode serveur » (« sans WebKit ni GTK »).

## Global Constraints

- **Aucun changement de comportement** de l'app de bureau : construite sans option (`default`), elle a les mêmes dépendances, le même code compilé et les mêmes commandes qu'avant. `Cargo.lock` ne gagne aucun paquet tiers.
- La branche `feat/sans-bureau` part de `feat/api-distante` (`86a6c47d`), dans un worktree `../Escouade-lot2b`. Elle ne contient que du code, la CI et la doc de l'amont que ce code rend fausse.
- Un commit par tâche. Sujets en français, portées de l'amont (`refactor(core)`, `build(core)`, `ci`), un corps de 2 à 4 lignes dans le ton de l'amont (ce que ça fait ; rien ne change pour l'utilisateur ; le point de `docs/REMOTE.md` suivi), la traîne `Co-Authored-By` des autres commits. Rien sur les lots, plans, tâches ou relectures. Rien n'est poussé chez l'amont.
- Après chaque tâche, dans le conteneur `--desktop-libs` : `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, et la suite de référence égale **par nom** à celle de la tâche 0 plus les tests neufs. Un test vert avant et rouge après est relancé seul 5 fois ; rouge ici et vert sur le commit d'avant, c'est une régression : on s'arrête.
- À partir de la tâche 4, en plus, dans l'image **nue** : `cargo check --locked --no-default-features -p escouade` et `cargo clippy --locked --no-default-features -p escouade -- -D warnings` passent, et `cargo tree --locked --no-default-features -p escouade -e normal` ne contient ni `tauri`, `wry`, `tao`, `gtk`, `webkit2gtk`, `glib`, `dbus`, `libdbus-sys`, `cpal`, `alsa`, `whisper-rs`.
- Un `#[cfg(feature = "desktop")]` se pose au plus haut niveau possible (le module dans `lib.rs`), pas ligne par ligne dans le cœur. Chaque `cfg` dans un fichier du cœur est compté dans le compte rendu de la tâche ; au-delà d'une trentaine pour toute la branche, on s'arrête et on le dit.
- Le disque : `CARGO_INCREMENTAL=0`, une seule commande cargo à la fois, et on s'arrête si le volume de construction dépasse 35 Go. L'image nue a son propre volume de construction, à créer et à surveiller de même.
- Commentaires du code en anglais, une ligne. `docs/CRATES.md` suit toute crate ajoutée et toute règle changée.
- Hors de ce lot : le binaire `escouade-server` (lot 5), plusieurs abonnés au hub (lot 3), le front (lot 4).

## Review Focus

- **Une dépendance de bureau qui revient par une crate du workspace** (`escouade-plugins`, `escouade-forge`, `escouade-updates`, `escouade-vectors`, `escouade-engines`) : la preuve est l'arbre des dépendances de la construction sans option, pas la lecture de `src-tauri/Cargo.toml` (tâches 3 et 4).
- **L'unification des fonctionnalités dans le workspace** : `cargo check --workspace --no-default-features` peut réactiver `desktop` par une autre crate. Le job de CI construit `-p escouade` seul, et le dit (tâche 5).
- **Une commande présente dans la table distante mais dont le corps appelle un module de bureau** (`open_url` → `outside`, `play_chime` → `notify`, le presse-papiers) : sans `desktop`, elle doit compiler et rendre une erreur claire, pas disparaître en silence de la table (tâche 4).
- **Les tests sans `desktop`** : beaucoup chargent un cœur par `host::Fake` et doivent tourner sans Tauri ; ceux qui lisent une fausse app Tauri sont sous `desktop`. Un test qui disparaît sans raison est une perte (tâche 4 : le compte des tests des deux constructions).
- **La recherche par le sens** après la sortie d'`ort` : mêmes modèles, même dossier, même bibliothèque chargée (`ort-dynamic`), sur les trois systèmes (tâche 3).

---

### Task 0: L'environnement et la référence

**Files:** aucun fichier suivi. Hors dépôt : `<espace du plan>/baseline.txt`, `<espace du plan>/base-status.txt`, `<espace du plan>/base-tree.txt`.

- [ ] **Step 1 :** `git worktree add ../Escouade-lot2b -b feat/sans-bureau feat/api-distante`. Copier `spikes/lot0/` (non suivi, exclu) de `../Escouade-lot2` ; vérifier `CARGO_INCREMENTAL=0` dans `run.sh` et Node dans l'image `--desktop-libs`.
- [ ] **Step 2 :** L'image nue : la cible `bare` du même Dockerfile, avec `run.sh` sans option. Lui donner un volume de construction à elle (`escouade-lot0-target-bare`, à recréer) et vérifier qu'elle n'a aucun paquet GTK, WebKit, X11, D-Bus ni ALSA (`dpkg -l`).
- [ ] **Step 3 :** La suite de référence dans `--desktop-libs` : écrire `baseline.txt` (la commande, les totaux) et `base-status.txt` (une ligne par test). Attendu : de l'ordre de 4 530 verts et 66 rouges connus.
- [ ] **Step 4 :** L'état de départ de l'image nue : `cargo check --locked -p escouade` y échoue ; garder dans `base-tree.txt` la liste des crates `*-sys` qui échouent et `cargo tree -p escouade -e normal -i tauri` (tous les chemins vers `tauri`). C'est ce que le lot doit ramener à rien.

### Task 1: Le cœur ne nomme plus les modules de bureau

**Files:**
- Modify : `src-tauri/src/core.rs` (`page_crash`, `file_origins`), `src-tauri/src/model.rs` (`PageEvent`), `src-tauri/src/pagecrash.rs`, `src-tauri/src/fileorigin.rs`, `src-tauri/src/devpage.rs`, `src-tauri/src/host.rs` (`file_origins`, une méthode de plus), `src-tauri/src/telemetry.rs` (`tauri::webview_version()`), `src-tauri/src/attachfiles.rs`, `src-tauri/src/commands.rs` (les usages), et ce que le compilateur désigne

**Interfaces:**
- Produces : les types `PageCrash`, `FileOrigins` et `PageEvent` vivent dans des modules du cœur qui ne nomment pas `tauri` (dans `model.rs` s'ils ne sont que des données ; sinon un module à eux) ; les modules de bureau `pagecrash.rs`, `fileorigin.rs`, `devpage.rs` gardent ce qui tient à la fenêtre et réexportent le type pour ne pas changer leurs appelants. `Host` gagne `fn webview_version(&self) -> Option<String>` (le bureau rend ce que `tauri::webview_version()` rendait ; `Fake` rend `None`), lue une fois au chargement comme aujourd'hui.

- [ ] **Step 1 :** Relever, avant de toucher, chaque `crate::<module de bureau>::` écrit dans un fichier du cœur : `git grep -n -E "crate::(pagecrash|fileorigin|devpage|devpage_probe|menus|notify|navigation|windowthread|clipboard|deeplink|updates|outside)::" -- src-tauri/src` hors de ces fichiers eux-mêmes, de `lib.rs`, `commands.rs` et `host.rs`. Classer chaque ligne : un type à déplacer (cette tâche), un appel à faire passer par `Host` (cette tâche s'il est simple, sinon le dire), ou un appel qui restera derrière `desktop` (tâche 4).
- [ ] **Step 2 : le test, qui échoue d'abord** : un test du cœur `the_telemetry_takes_the_webviews_version_from_its_host` — un cœur chargé avec un `Fake` dont `webview_version` rend une chaîne donnée la porte là où la télémétrie la garde ; avec `None`, la valeur est celle d'aujourd'hui quand Tauri n'en donne pas.
- [ ] **Step 3 :** Déplacer les trois types et brancher `webview_version`. Aucun changement de sérialisation : les événements et l'état initial envoyés à la fenêtre gardent leurs champs et leurs noms (vérifier par les tests existants qui comparent du JSON).
- [ ] **Step 4 :** Vérifier : la commande de l'étape 1 ne rend plus que des lignes classées « derrière `desktop` », listées dans le compte rendu ; fmt, clippy, suite égale à la référence plus le test neuf.
- [ ] **Step 5 :** Commit : `refactor(core): le cœur ne nomme plus les types des modules de la fenêtre`.

### Task 2: La macro n'écrit la commande Tauri que pour le bureau

**Files:**
- Modify : `crates/escouade-macros/src/lib.rs`, `src-tauri/src/commands.rs` (`CoreState` et son `CommandArg`), `src-tauri/src/sink.rs` (les deux `From<Channel<…>>`), `docs/CRATES.md`

**Interfaces:**
- Produces : `#[command]` écrit `#[cfg_attr(feature = "desktop", ::tauri::command)]` (ou l'équivalent : la fonction elle-même reste, sans l'attribut de Tauri, hors `desktop`) et l'entrée distante dans tous les cas ; `handlers![…]` rend `(handler, entries)` sous `desktop` et les seules entrées sinon (une seconde forme, ou un `cfg` dans l'expansion : au choix du constructeur, à dire). `CoreState` garde son `Deref` et `inner()` dans les deux cas ; son `impl tauri::ipc::CommandArg` est sous `desktop`. Les `From<tauri::ipc::Channel<…>> for Sink<…>` sont sous `desktop`.
- La fonctionnalité `desktop` est **déclarée** dans `src-tauri/Cargo.toml` dès cette tâche (`default = ["desktop"]`, `desktop = []`), vide : elle ne retire encore rien. C'est la tâche 4 qui la remplit.

- [ ] **Step 1 : les tests, qui échouent d'abord** (dans le crate de macros, sur l'expansion) : `the_tauri_command_is_written_for_the_desktop_alone` (l'expansion porte l'attribut de Tauri sous `cfg(feature = "desktop")` et l'entrée distante sans condition) ; `the_handlers_give_the_entries_without_the_desktop` (l'expansion de `handlers!` hors `desktop` ne nomme pas `tauri`).
- [ ] **Step 2 :** Écrire, relancer. Une commande marquée `desktop` garde ses paramètres de Tauri : hors `desktop`, sa fonction entière doit être sous `cfg(feature = "desktop")` et son entrée rester dans la table avec `call: None`.
- [ ] **Step 3 :** Vérifier que l'expansion par défaut est, pour Tauri, exactement celle d'avant (comparer l'expansion d'une commande avant et après avec `cargo expand` s'il est là, sinon par le test d'expansion) ; fmt, clippy, suite égale à la référence plus les tests neufs.
- [ ] **Step 4 :** Commit : `refactor(core): la commande de Tauri n'est écrite que pour le bureau`.

### Task 3: `ort` hors de la voix

**Files:**
- Create : `crates/escouade-ort/` (`Cargo.toml`, `src/lib.rs`)
- Modify : `crates/escouade-voice-local/` (ce qui prépare et charge ONNX Runtime en sort), `crates/escouade-vectors/Cargo.toml` et son code (dépend d'`escouade-ort`, plus de la voix), `src-tauri/Cargo.toml` (le membre, la fonctionnalité `ort-dynamic`), `docs/CRATES.md`, les scripts et configurations qui nomment la voix pour ONNX Runtime (`scripts/onnxruntime.sh`, `tauri.ort-*.conf.json`) s'ils nomment la crate

**Interfaces:**
- Produces : `escouade-ort` tient la dépendance `ort` (mêmes versions et fonctionnalités par cible que `crates/escouade-voice-local/Cargo.toml:46-51`), la fonctionnalité `ort-dynamic` et ce qui rend ONNX Runtime prêt (chargement, téléchargement d'un modèle s'il est partagé). `escouade-voice-local` et `escouade-vectors` en dépendent ; `escouade-vectors` ne dépend plus d'`escouade-voice-local`. Les chemins publics que le reste du code utilise restent joignables (réexports).

- [ ] **Step 1 :** Lire ce qu'`escouade-vectors` prend à `escouade-voice-local` (`git grep -n "escouade_voice_local" -- crates/escouade-vectors src-tauri/src`) et ce que la voix fait d'`ort`. Écrire la liste de ce qui déménage ; si plus que le chargement d'`ort` et le téléchargement des modèles doit bouger, s'arrêter et le dire.
- [ ] **Step 2 :** Déménager. Aucun changement de dossier de modèles, de nom de fichier, ni de bibliothèque chargée.
- [ ] **Step 3 :** Vérifier : `cargo tree --locked -p escouade-vectors -e normal -i cpal` et `-i whisper-rs` ne trouvent rien ; `Cargo.lock` ne gagne que la crate neuve ; les tests de la voix et des vecteurs passent ; fmt, clippy, suite égale à la référence.
- [ ] **Step 4 :** Commit : `refactor(crates): escouade-ort, ONNX Runtime sort de la crate de la voix`.

### Task 4: Les fonctionnalités `desktop` et `voice`

**Files:**
- Modify : `src-tauri/Cargo.toml` (`tauri`, `tauri-build`, les `tauri-plugin-*`, `gtk`, `webkit2gtk`, les dépendances de cible Windows et macOS propres à la fenêtre, `escouade-voice-local` : facultatives ; `[features]`), `src-tauri/build.rs`, `src-tauri/src/lib.rs` (les modules de bureau et `run()` sous `desktop`, la voix sous `voice`), `src-tauri/src/main.rs`, `src-tauri/src/host.rs` (`Desktop` sous `desktop`), les crates du workspace qui tirent encore `tauri` ou la voix, `docs/CRATES.md`

**Interfaces:**
- Produces : `default = ["desktop", "voice"]` ; `desktop` active `tauri`, ses plugins, `gtk`, `webkit2gtk` et les modules de la fenêtre ; `voice` active `escouade-voice-local` et les modules de la voix ; `desktop` exige `voice` seulement si le code le demande (le dire). Sans option, la crate `escouade` construit sa bibliothèque : `Core`, `Host`, `host::Fake` (tests), `sink`, `remoteapi` (table et serveur), toutes les commandes distantes.
- Une commande distante dont le corps appelle un module de bureau (relevée à l'étape 1 de la tâche 1 : `outside`, `notify::play_chime`, le presse-papiers, les dialogues) : hors `desktop`, elle compile et rend `Err("not available without a desktop")` — par un petit module de remplacement de même nom et de mêmes signatures que le module de bureau, sous `cfg(not(feature = "desktop"))`, plutôt que par des `cfg` dans chaque commande.

- [ ] **Step 1 : le test, qui échoue d'abord** : `the_table_is_the_same_with_and_without_the_desktop` — sous les deux constructions, la table a le même nombre d'entrées et les mêmes noms ; hors `desktop`, chaque commande « bureau seulement » a `call: None`. (Le test compare à un nombre et à une liste écrits dans le test, comme ceux du lot 2.)
- [ ] **Step 2 :** Poser les fonctionnalités et les `cfg` de module. Suivre le compilateur dans l'image nue : `spikes/lot0/run.sh sh -c 'cd src-tauri && cargo check --locked --no-default-features -p escouade'`, jusqu'à ce qu'il passe ; puis `--all-targets` pour les tests.
- [ ] **Step 3 :** Les tests sans option : `cargo nextest run --no-default-features -p escouade` dans l'image nue. Relever le nombre de tests des deux constructions et la liste de ceux qui n'existent que sous `desktop`, avec la raison par groupe.
- [ ] **Step 4 :** Vérifier la contrainte de l'arbre des dépendances (aucun des paquets interdits), garder la sortie ; compter les `cfg` posés dans des fichiers du cœur ; fmt et clippy des deux constructions ; la suite par défaut égale à la référence plus le test neuf.
- [ ] **Step 5 :** Commit : `build(core): le bureau et la voix derrière des fonctionnalités, le cœur se construit sans`.

### Task 5: Le job de CI sans bureau

**Files:**
- Modify : `.github/workflows/ci.yml`, `docs/CRATES.md` (la règle), `docs/REMOTE.md` (« Tâches » : cocher ce qui est fait, en quelques mots)

**Interfaces:**
- Produces : un job Linux qui n'installe **aucun** paquet de bureau (pas de `scripts/linux-deps.sh`) et lance, dans `src-tauri` : `cargo check --locked --no-default-features -p escouade`, `cargo clippy --locked --no-default-features -p escouade -- -D warnings`, les tests sans option, et un contrôle de l'arbre des dépendances qui échoue si un des paquets interdits y revient. Les actions sont épinglées par empreinte, comme celles du fichier.

- [ ] **Step 1 :** Lire `ci.yml` : comment les jobs sont nommés, mis en cache (`Swatinem/rust-cache`, `workspaces: src-tauri`), déclenchés, et ce qu'un job Linux installe. Écrire le job dans le même style, avec un commentaire d'une ligne qui dit ce qu'il garde.
- [ ] **Step 2 :** Le contrôle de l'arbre : une étape de quelques lignes de shell (`cargo tree … | grep -E` sur la liste), pas un script neuf, sauf si le dépôt range ses contrôles dans `scripts/`.
- [ ] **Step 3 :** L'essayer hors de GitHub : jouer les mêmes commandes dans l'image nue ; puis prouver que le garde-fou mord : ajouter à l'essai un `use tauri::Manager;` hors `cfg` dans un fichier du cœur, constater l'échec, le retirer.
- [ ] **Step 4 :** Valider la syntaxe du workflow (`actionlint` s'il est là ; sinon relire et le dire).
- [ ] **Step 5 :** Commit : `ci: le cœur construit et testé sous Linux sans GTK ni WebKit`.
