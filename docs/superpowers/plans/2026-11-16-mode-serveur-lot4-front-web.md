# Mode serveur, lot 4 : l'interface dans un navigateur — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** L'interface d'Escouade, sans composant réécrit, s'ouvre dans Chromium et pilote un cœur lancé sans bureau : ouvrir un projet, un terminal, voir les événements, reprendre après une coupure ; l'app de bureau ne change pas.

**Architecture:** Le front parle au cœur par un module de transport à deux mises en œuvre : l'IPC de Tauri dans la fenêtre, la WebSocket de `remoteapi` dans un navigateur ; `ipc.ts` et les trois autres fichiers qui appellent `invoke` passent par lui. Ce que le front demande au système (dialogues, ouverture de liens, contrôles de fenêtre) passe par une façade qui, dans un navigateur, fait l'équivalent web ou cache la fonction. Un binaire `escouade-server` minimal charge le cœur avec un `Host` sans fenêtre, sert les fichiers du build web et la WebSocket sur `127.0.0.1` ; il rend possible un e2e dans Chromium. L'appairage, les jetons d'appareil et l'installation en service sont le lot 5.

**Tech Stack:** Svelte 5, TypeScript, Vite, Vitest, Playwright (Chromium) ; Rust 1.98.0 (`--no-default-features` pour le serveur), `hyper` 1 et `tokio-tungstenite` (déjà là).

**Spec:** `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (« Le front sur deux transports », « Sécurité », « Ce qui change de machine », lot 4) ; `docs/REMOTE.md`, « La fondation commune », point 5, et « Ce que voit le téléphone », option A.

## Global Constraints

- **Aucun composant Svelte copié ni réécrit pour le web.** Un composant ne change que pour remplacer un import de `@tauri-apps/*` par la façade, ou pour cacher ce qui n'a pas de sens hors du bureau derrière **une** condition lue dans la façade. Chaque composant touché est listé dans le compte rendu avec le nombre de lignes changées.
- **Aucun changement de comportement** de l'app de bureau : dans la fenêtre, le transport est l'IPC de Tauri, appelé comme avant (mêmes noms de commandes, mêmes arguments, mêmes `Channel`).
- La branche `feat/front-web` part de `feat/plusieurs-clients`, dans un worktree `../Escouade-lot4`. Code, tests et doc de l'amont seulement.
- Un commit par tâche, sujets en français aux portées de l'amont (`refactor(front)`, `feat(remote)`, `build`, `test`), corps de 2 à 4 lignes coupées vers 100 colonnes, traîne `Co-Authored-By` des autres commits. Rien sur les lots, plans, tâches ou relectures. Rien n'est poussé chez l'amont.
- Aucune dépendance npm ni Cargo tierce nouvelle, sauf si une tâche le dit ; `package-lock.json` et `Cargo.lock` ne bougent que pour une raison écrite.
- Sécurité du serveur, inchangée et tenue par des tests : écoute sur `127.0.0.1` seulement ; un jeton exigé à chaque connexion ; `Host` égal à l'adresse du serveur ; une `Origin` n'est acceptée que si elle est **exactement** celle du serveur lui-même (la page qu'il sert) ; toute autre est refusée. Le jeton ne s'écrit dans aucun journal ni dans une adresse.
- Après chaque tâche : côté front `npm run check`, `npm test`, `npm run format:check`, `npm run build` ; côté Rust `cargo fmt --all --check`, clippy `-D warnings` sur le défaut et sur `--no-default-features -p escouade --all-targets`, la suite de référence égale **par nom** à celle de la tâche 0 plus les tests neufs. Un test vert avant et rouge après est relancé seul 5 fois ; rouge ici et vert sur le commit d'avant, c'est une régression : on s'arrête.
- Disque : `CARGO_INCREMENTAL=0`, une commande cargo à la fois, guetteur d'espace libre pendant les longues commandes (arrêt sous 8 Go), volumes de construction sous 25 Go.
- Commentaires du code en anglais, une ligne. Le front suit Prettier et le style des fichiers voisins.
- Hors de ce lot : l'appairage et les jetons d'appareil, l'installation en service, le navigateur des dossiers du serveur, l'envoi et le téléchargement de fichiers, les notifications du navigateur, la vue Plugins et le store, la mise en page mobile (lot 5 et après).

## Review Focus

- **Une erreur de commande** : la fenêtre reçoit de Tauri une promesse rejetée avec la valeur d'erreur (une chaîne, parfois un objet). Le transport web doit rejeter avec **la même valeur** (`err.message` du fil), sinon chaque `catch` du front change de sens (test en tâche 1, sur une erreur chaîne et une erreur objet).
- **L'ordre des appels** : la fenêtre envoie les frappes d'un terminal dans l'ordre ; le transport web aussi, sur une seule connexion, sans réordonner (test en tâche 1).
- **Une commande « bureau seulement » appelée depuis le navigateur** : le front ne l'appelle pas (la fonction est cachée) ; s'il l'appelle quand même, l'erreur `desktop` se voit dans la console sans casser la page (tâche 2 : la liste des appels du front croisée avec la liste des commandes de bureau).
- **La coupure** : connexion perdue en plein flux de terminal ; les appels en attente sont rejetés, l'interface le dit, se reconnecte, se réabonne et se rattache aux terminaux (tâche 5).
- **Deux onglets** sur le même serveur : chacun reçoit les événements ; l'état d'interface partagé (`UiState`) ne fait pas sauter l'autre onglet d'une vue à l'autre (le dernier qui écrit gagne, lu au chargement seulement — décision du demandeur ; vérifier ce que fait le front d'un `set_ui` venu d'ailleurs).

---

### Task 0: L'environnement et les références

**Files:** aucun fichier suivi. Hors dépôt : `<espace du plan>/baseline.txt` et les fichiers d'état.

- [ ] **Step 1 :** `git worktree add ../Escouade-lot4 -b feat/front-web feat/plusieurs-clients`. Copier `spikes/lot0/` (non suivi). `npm ci` dans le conteneur `--desktop-libs` (Node y est) ; noter les versions.
- [ ] **Step 2 :** Les références du front : `npm run check`, `npm test` (nombre de tests), `npm run format:check`, `npm run build` (taille de `dist/`). Les références Rust : la suite par défaut et les tests sans option, une ligne par test.
- [ ] **Step 3 :** Une image pour l'e2e web : une cible de plus dans `spikes/lot0/Dockerfile` (non suivi), partie de l'image **nue**, avec Node et le Chromium de Playwright (`npx playwright install --with-deps chromium`, à la version de `@playwright/test` de `package.json`). Vérifier qu'elle n'a ni GTK ni WebKit de bureau au-delà de ce que Chromium tire, et le dire.

### Task 1: Le transport

**Files:**
- Create : `src/lib/transport/index.ts`, `src/lib/transport/tauri.ts`, `src/lib/transport/ws.ts`, `src/lib/transport/ws.test.ts`
- Modify : `src/lib/ipc.ts`, `src/lib/opener.ts`, `src/lib/plugins.ts`, `src/lib/store.ts` (leur import de `@tauri-apps/api/core`), `src/test/ipc.ts` et `src/test/setup.ts` si les mocks en dépendent

**Interfaces:**
- Produces :

```ts
// src/lib/transport/index.ts
export interface Transport {
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  /** A flow from the core: events (JSON) or a terminal's bytes (ArrayBuffer). */
  channel<T>(): Flow<T>;
}
export interface Flow<T> { onmessage: (message: T) => void }
export const transport: Transport;           // Tauri in the window, the WebSocket elsewhere
export const invoke: Transport['invoke'];     // what ipc.ts imports instead of Tauri's
export function channel<T>(): Flow<T>;
export const desktop: boolean;                // true in the window of the desktop app
```

  - `tauri.ts` : `invoke` et `Channel` de `@tauri-apps/api/core`, tels quels. Un `Flow` y **est** un `Channel` de Tauri : il se passe en argument d'une commande comme aujourd'hui.
  - `ws.ts` : le fil de `src-tauri/src/remoteapi/wire.rs` (lire son en-tête : `hello`, `{id, cmd, args}`, `{id, ok}` / `{id, err:{kind, message}}`, `{event}`, trames binaires préfixées par 4 octets du numéro de flux). Un `Flow` passé en argument d'une commande à flux devient son numéro (`stream`) dans la requête ; `subscribe` reçoit les `{event}`. Une seule connexion, ouverte au premier appel, à l'adresse de la page (`ws(s)://<hôte de la page>/…`, chemin lu dans `wire.rs` ou le serveur).
  - Le choix : la fenêtre de Tauri si `window.__TAURI_INTERNALS__` existe, sinon la WebSocket. Le module Tauri n'est pas chargé dans le navigateur (import dynamique ou alias de build, au choix du constructeur ; le dire), pour que le build web ne dépende pas d'un objet absent.

- [ ] **Step 1 : les tests, qui échouent d'abord** (`ws.test.ts`, Vitest, avec une fausse socket écrite dans le test, sans réseau) :
  - `an_answer_resolves_its_own_call` : trois appels, réponses dans le désordre, chacun reçoit la sienne.
  - `a_failed_command_rejects_with_the_value_the_window_gets` : `err.kind = "failed"` avec `message` chaîne → rejet par cette chaîne ; avec `message` objet → rejet par cet objet (égalité profonde).
  - `the_other_kinds_reject_with_an_error_that_names_them` : `unknown`, `desktop`, `args`.
  - `the_calls_leave_in_the_order_they_were_made` : 200 `term_write` numérotés.
  - `an_event_reaches_the_flow_given_to_subscribe`.
  - `a_terminals_bytes_reach_the_flow_of_their_number` : deux flux, trames binaires entremêlées, chaque `Flow` reçoit ses octets dans l'ordre, en `ArrayBuffer` sans le préfixe.
  - `the_greeting_is_read_before_any_call` et `a_protocol_the_client_does_not_know_is_refused_with_a_message` (le `protocol` du `hello` différent de 1).
  - `a_lost_connection_rejects_the_calls_in_flight`.
- [ ] **Step 2 :** Écrire les trois modules ; brancher `ipc.ts` (ses 322 `invoke` et 6 `Channel`) et les trois autres fichiers par un remplacement d'import, sans toucher aux appels.
- [ ] **Step 3 :** Vérifier : `git grep -n "@tauri-apps/api/core" -- src` ne rend que `src/lib/transport/tauri.ts` et les tests qui se moquent de Tauri ; les vérifications de fin de tâche ; `npm run build` produit le même `dist/` à la taille près du module neuf.
- [ ] **Step 4 :** Commit : `refactor(front): les appels au cœur passent par un transport, l'IPC de Tauri ou la WebSocket`.

### Task 2: La façade du système

**Files:**
- Create : `src/lib/platform.ts`, `src/lib/platform.test.ts`
- Modify : les fichiers qui importent `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-opener`, `@tauri-apps/api/window`, `@tauri-apps/api/webview`, `@tauri-apps/api/event` — relevés à la base : `components/PagesWindow.svelte`, `components/TitleBar.svelte`, `components/modals/NewProjectModal.svelte`, `components/plugins/PluginPage.svelte`, `PluginsView.svelte`, `StorePluginPage.svelte`, `components/settings/AdvancedPage.svelte`, `lib/appearance.ts`, `lib/display.svelte.ts`, `lib/plugins-view.svelte.ts`, `lib/project-logo.svelte.ts` (relever de nouveau sur la tête : l'amont a pu en ajouter)

**Interfaces:**
- Produces : `platform.ts` exporte, avec les mêmes signatures que ce que les fichiers appelaient, ce qu'ils prenaient à Tauri (ouvrir un dialogue de dossier ou de fichier, ouvrir une adresse, la fenêtre courante et ses contrôles, le zoom de la vue, l'écoute d'un événement de fenêtre), et `desktop: boolean`. Dans un navigateur : ouvrir une adresse = `window.open(url, '_blank', 'noopener')` ; un dialogue de dossier ou de fichier rend `null` (personne n'a choisi) ; les contrôles de fenêtre et le zoom natif ne font rien.
- Ce que l'interface **cache** dans un navigateur, chacun par une seule condition `platform.desktop` : les boutons de la barre de titre propres à la fenêtre, la fenêtre « Pages » et les pages détachées, la voix (dictée, conversation, réglages de la voix), la mise à jour de l'app, « Afficher dans l'Explorateur », le choix d'un dossier par dialogue dans « Nouveau projet » (cloner un dépôt reste), la vue Plugins et le store (lot 5). La liste exacte vient de l'étape 1.

- [ ] **Step 1 :** Croiser deux listes et écrire le résultat dans `<espace du plan>/front-desktop.md` : (a) chaque nom de commande appelé par le front (`git grep -o "invoke<[^>]*>('[a-z_]*'" -- src` et les formes voisines) ; (b) les commandes « bureau seulement » de la table (le test `the_commands_of_the_desktop_alone_are_these` les liste). Pour chaque commande de (b) appelée par le front : où, par quelle fonction de l'interface, et ce que le navigateur en fait (cachée ; remplacée ; laissée, l'erreur étant attendue et attrapée). `subscribe` et les commandes à flux sont servies par le serveur : hors de cette liste.
- [ ] **Step 2 : les tests, qui échouent d'abord** (`platform.test.ts`) : dans un navigateur (pas de `__TAURI_INTERNALS__`), `desktop` est faux, ouvrir une adresse appelle `window.open` avec `noopener`, un dialogue rend `null`, les contrôles de fenêtre ne lèvent pas ; dans la fenêtre (Tauri simulé comme le font les tests voisins), chaque fonction appelle celle de Tauri avec les mêmes arguments.
- [ ] **Step 3 :** Écrire la façade, remplacer les imports, poser les conditions. Un composant ne reçoit pas de branche « web » de plus d'une condition ; au-delà, s'arrêter et le dire.
- [ ] **Step 4 :** Vérifier : `git grep -n "@tauri-apps/" -- src` ne rend que `src/lib/transport/tauri.ts`, `src/lib/platform.ts` (ou son module Tauri) et les tests ; fin de tâche.
- [ ] **Step 5 :** Commit : `refactor(front): ce que l'interface demande au système passe par une façade`.

### Task 3: Le build web

**Files:**
- Create : `vite.web.config.ts` (ou un mode de `vite.config.ts`)
- Modify : `package.json` (`build:web`), `.gitignore` (`dist-web`), `docs/` de l'amont si un fichier décrit les commandes de build

**Interfaces:**
- Produces : `npm run build:web` écrit `dist-web/` : la même `index.html`, les mêmes composants ; aucun module de `@tauri-apps/*` dans le lot chargé au départ (le vérifier dans la sortie de Vite ou par une recherche dans `dist-web/assets`).

- [ ] **Step 1 :** Écrire le build ; mesurer `dist/` et `dist-web/` (taille, nombre de fichiers) ; `npm run build` (bureau) inchangé.
- [ ] **Step 2 :** Un test (Vitest ou un script de `scripts/` selon l'usage du dépôt) qui échoue si `dist-web` contient `__TAURI_INTERNALS__` ailleurs que dans le test de présence du transport.
- [ ] **Step 3 :** Fin de tâche. Commit : `build: un build web de l'interface, sans Tauri`.

### Task 4: `escouade-server`, au plus court

**Files:**
- Create : `src-tauri/src/bin/escouade-server.rs`, `src-tauri/src/headless_host.rs` (le `Host` sans fenêtre), `src-tauri/src/remoteapi/web.rs` (les fichiers statiques), leurs tests
- Modify : `src-tauri/Cargo.toml` (le binaire, sans `required-features` de bureau), `src-tauri/src/remoteapi/server.rs` (servir des fichiers, accepter l'origine du serveur, le jeton par cookie), `src-tauri/src/lib.rs`, `.github/workflows/ci.yml` (le job sans bureau construit aussi le binaire), `docs/CRATES.md`, `docs/REMOTE.md`

**Interfaces:**
- Produces :
  - `escouade-server` : se construit par `cargo build --no-default-features --bin escouade-server` ; charge le cœur (`Core::load(Arc::new(Headless::new()), DataDir::default…)`, comme l'app : lire `app.rs` et faire la même suite d'étapes pour ce qui ne tient pas à la fenêtre — `core.start`, les migrations de comptes, etc. ; la liste des étapes reprises et laissées va dans le compte rendu), démarre `remoteapi::server` et attend un signal d'arrêt (`SIGINT`, `SIGTERM`) pour arrêter proprement le cœur (`Core::shutdown`).
  - Sa configuration, pour ce lot : `ESCOUADE_REMOTE_PORT`, `ESCOUADE_REMOTE_TOKEN` (lues puis retirées de l'environnement, comme dans l'app) et `--web <dossier>` (le build web à servir ; sans lui, la WebSocket seule). Le garde-fou « debug seulement » du lot 2 vaut pour **l'app de bureau** ; le binaire serveur, lui, est fait pour servir : il démarre dans tout build, et refuse de démarrer sans jeton.
  - `Headless` : `version` du paquet ; `seed_dir` = `None` (la graine embarquée est le lot 5) ; `window_attended` faux ; `attention`, `waiting`, `relabel`, `chime`, la voix : rien ; `toast` : un événement `UiEvent` que les clients rendront au lot 5, ou rien si l'événement n'existe pas encore — dans ce cas le dire, ne pas inventer un événement ici.
  - Le serveur HTTP : `GET /` et les fichiers de `--web` (types MIME par extension, pas de liste de dossier, pas de chemin qui sort du dossier, en-têtes `Cache-Control` sobres, `Content-Security-Policy` qui n'autorise que l'origine du serveur et sa WebSocket), la WebSocket sur le chemin d'aujourd'hui.
  - L'authentification d'un navigateur : `POST /session` avec le jeton dans le corps pose un cookie `HttpOnly; SameSite=Strict; Path=/` ; la WebSocket accepte `Authorization: Bearer` (comme avant) **ou** ce cookie ; une `Origin` présente doit être exactement `http://127.0.0.1:<port>` ou `http://localhost:<port>` ; les fichiers statiques se servent sans jeton (ils ne contiennent rien du cœur), sauf si une page d'accueil minimale demande le jeton : au constructeur, à dire. Aucune adresse ne porte le jeton.

- [ ] **Step 1 : les tests, qui échouent d'abord** (Rust, sans harness, port 0) : les fichiers servis (`/`, un fichier d'`assets`, un chemin absent → 404, `/../Cargo.toml` et ses variantes encodées → 404) ; `the_session_cookie_opens_the_socket` ; `a_wrong_token_gets_no_cookie` ; `a_foreign_origin_is_refused_even_with_the_cookie` ; `the_servers_own_origin_is_let_in` ; `the_cookie_is_httponly_and_samesite_strict` ; `the_bearer_header_still_works_without_an_origin` ; les tests du lot 2 sur la poignée de main passent toujours (toute `Origin` étrangère refusée).
- [ ] **Step 2 :** Le binaire : un test qui le lance comme un process (dans le conteneur nu), attend son port, fait un appel par la WebSocket avec le jeton, lui envoie `SIGTERM` et vérifie une sortie propre (code 0, « core stopped » au journal).
- [ ] **Step 3 :** Écrire. Vérifier dans l'image **nue** : `cargo build --locked --no-default-features --bin escouade-server` ; `ldd` du binaire ne nomme ni GTK, ni WebKit, ni D-Bus, ni ALSA ; garder la sortie de `ldd` dans le compte rendu. Le garde-fou de l'arbre des dépendances passe toujours.
- [ ] **Step 4 :** Retirer le `allow(dead_code)` de crate posé « jusqu'au binaire serveur » si plus rien n'est mort sans le bureau ; sinon le réduire et dire ce qui reste.
- [ ] **Step 5 :** Fin de tâche. Commit : `feat(remote): escouade-server, le cœur sans fenêtre qui sert l'interface web sur 127.0.0.1`.

### Task 5: Le parcours dans Chromium, et la reprise

**Files:**
- Create : `playwright.web.config.ts`, `tests/web/` (le parcours, son montage)
- Modify : `src/lib/transport/ws.ts` (la reconnexion), `src/lib/ipc.ts` ou l'endroit où le front s'abonne (se réabonner, se rattacher aux terminaux par les commandes du lot 3), `package.json` (`test:web`), `.github/workflows/ci.yml` (un job, s'il peut tourner sans paquet de bureau), `docs/REMOTE.md`

**Interfaces:**
- Produces : `npm run test:web` lance `escouade-server` (build sans option) sur un dossier de données temporaire avec `--web dist-web`, pose la session, et joue le parcours dans Chromium.

- [ ] **Step 1 : le parcours, écrit d'abord** : la page s'ouvre sans erreur de console ; l'état initial s'affiche (aucun projet) ; créer un projet sur un dépôt Git préparé par le test (le chemin saisi ou le clonage d'un dépôt local, selon ce que « Nouveau projet » permet sans dialogue) ; ouvrir un terminal, y taper `echo bonjour`, lire `bonjour` à l'écran ; ouvrir le Kanban ; recharger la page et retrouver le projet et le terminal avec son écran.
- [ ] **Step 2 :** Deux onglets : un événement provoqué dans l'un (un terminal ouvert) apparaît dans l'autre ; l'autre ne change pas de vue.
- [ ] **Step 3 :** La coupure : le test ferme la connexion côté serveur (ou coupe le réseau de la page) ; l'interface dit qu'elle est déconnectée, se reconnecte, et le terminal ouvert retrouve son écran puis la suite. Le délai de reconnexion croît et se borne ; rien ne boucle sans pause.
- [ ] **Step 4 :** Un agent sur un faux harness, si le dépôt en fournit un utilisable ici (les e2e de bureau en ont : `tests/e2e/fixtures`) : le lancer, voir sa réponse, répondre à sa question. Si le faux harness demande un outil absent de l'image, le dire et laisser cette étape hors du parcours, sans la simuler.
- [ ] **Step 5 :** Fin de tâche ; le parcours passe trois fois de suite. Commit : `test(remote): le parcours de l'interface web dans Chromium, reprise comprise`.
