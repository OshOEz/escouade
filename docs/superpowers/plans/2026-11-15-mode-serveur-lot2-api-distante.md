# Mode serveur, lot 2 : l'API distante du cœur — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Les commandes du cœur se déclarent une seule fois et se servent à la fois par l'IPC de Tauri et par une WebSocket locale, éteinte par défaut ; l'app de bureau ne change pas.

**Architecture:** (1) Les flux vers l'interface passent par un type `Sink` que le cœur connaît, au lieu du `Channel` de Tauri. (2) Un attribut `#[command]` remplace `#[tauri::command]` : il garde la commande Tauri telle quelle et écrit à côté une entrée appelable avec du JSON ; `handlers![…]` remplace `tauri::generate_handler![…]` et rend les deux listes à partir d'une seule. (3) Un serveur WebSocket sur `127.0.0.1` sert cette table, avec un jeton ; il ne s'allume que par une variable d'environnement et sert aux tests.

**Tech Stack:** Rust 1.98.0, Tauri 2.12, tokio 1, `tokio-tungstenite` 0.30 et `hyper` 1 (déjà dans le workspace), `syn`/`quote`/`proc-macro2` (déjà dans `Cargo.lock`) ; construction et tests dans le conteneur (`spikes/lot0/run.sh --desktop-libs`, avec Node).

**Spec:** `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (lot 2, « Architecture », « Sécurité », « Décidé par le demandeur après le lot 0 ») ; `docs/REMOTE.md`, « La fondation commune », points 2 et 6.

## Global Constraints

- **Aucun changement de comportement** de l'app de bureau. Le serveur WebSocket n'existe pas tant que la variable d'environnement n'est pas posée.
- La branche `feat/api-distante` part de `feat/coeur-sans-apphandle` (`adfbb516`), dans un worktree à part `../Escouade-lot2`. Elle ne contient **que du code** et la doc de l'amont que ce code rend fausse : ni `docs/superpowers/`, ni `spikes/`, ni `.superpowers/`.
- Un commit par tâche, proposable seul. Sujets en français, portée de l'amont : `refactor(core): …`, `feat(remote): …`. Un corps de 2 à 4 lignes : ce que ça fait, « rien ne change pour l'utilisateur », et le point de `docs/REMOTE.md` suivi. Rien sur les lots, plans, tâches ou relectures.
- Rien n'est poussé chez l'amont ; aucune issue ni PR.
- `Cargo.lock` ne gagne aucun paquet tiers : seul le nouveau crate du workspace y entre.
- Après chaque tâche, dans le conteneur : `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, et la suite de référence donne, **par nom**, le même résultat que la tâche 0, plus les tests neufs. Un test vert sur la référence et rouge ici est relancé seul 5 fois ; rouge ici et vert sur la base, c'est une régression : on s'arrête.
- Le serveur n'écoute que sur `127.0.0.1`. Il exige le jeton sur chaque connexion et refuse un `Host` autre que `127.0.0.1:<port>` ou `localhost:<port>`, et toute `Origin`, comme le serveur MCP (`crates/escouade-mcp`). Le jeton ne s'écrit dans aucun journal.
- Pas de nouveau binaire de test qui lie toute l'app : les tests vivent dans les tests de la crate.
- Commentaires du code en anglais, une ligne, ce que le code ne montre pas. Les crates et modules neufs suivent `docs/CRATES.md` (§ sur les règles des crates) et y sont décrits.
- Hors de ce lot : plusieurs abonnés au hub et la reprise (lot 3), le front (lot 4), les fonctionnalités Cargo, le job de CI sans GTK et les trois types de bureau tenus par le cœur (plan suivant, « fin du lot 2 »).

## Review Focus

- **Une commande oubliée ou mal classée** : servie à distance alors qu'elle suppose un bureau, ou l'inverse. La liste des commandes « bureau seulement » est écrite dans un test, nom par nom (tâche 3).
- **Les noms d'arguments** : Tauri attend les paramètres en camelCase (`agent_id` → `agentId`). L'entrée JSON doit accepter exactement les mêmes clés que la fenêtre envoie aujourd'hui (test en tâche 2, sur une vraie commande à plusieurs mots).
- **Une commande synchrone qui bloque** : hors `(async)`, Tauri les fait tourner hors du runtime ; à distance elles passent par `rt::spawn_blocking`, jamais sur un fil du runtime (test en tâche 2).
- **`subscribe` à distance vole les événements de la fenêtre** : le hub n'a qu'un abonné jusqu'au lot 3. C'est voulu, écrit dans le code et dans le corps du commit, et le serveur ne s'allume pas chez un utilisateur.
- **Un client qui se déconnecte en plein flux** de terminal : le `Sink` rend une erreur, le terminal continue, rien ne panique (test en tâche 4).

---

### Task 0: L'environnement et la référence

**Files:** aucun fichier suivi. Hors dépôt : `<espace du plan>/baseline.txt`, `<espace du plan>/base-status.txt`.

**Interfaces:**
- Produces : le worktree `../Escouade-lot2` sur `feat/api-distante` ; `spikes/lot0/` (non suivi, exclu) copié de `../Escouade-lot1` avec Node dans l'image ; `base-status.txt`, une ligne par test (`crate nom PASS|FAIL`) sur `adfbb516`, et en tête de `baseline.txt` la commande de référence exacte.

- [ ] **Step 1 :** `git worktree add ../Escouade-lot2 -b feat/api-distante feat/coeur-sans-apphandle`. Copier `spikes/lot0/` de `../Escouade-lot1` ; vérifier que `git status` ne le montre pas et que `node --version` répond dans le conteneur.
- [ ] **Step 2 :** Lancer la suite de référence du lot 1 (nextest, `--workspace --profile ci --no-fail-fast --retries 0`, peu de tâches cargo : le conteneur a 8 Go). Attendu : de l'ordre de 4 489 verts et 66 à 67 rouges, en 3 à 5 minutes.
- [ ] **Step 3 :** Écrire les deux fichiers. Relancer seuls 5 fois les tests rouges qui ne sont pas dus à l'outil `escouade-js`/Javy absent ni au conteneur lancé en root, et noter lesquels sont instables.

### Task 1: Les flux par un `Sink`

**Files:**
- Create : `src-tauri/src/sink.rs`
- Modify : `src-tauri/src/hub.rs` (`:8`, `:18`, `:89`), `src-tauri/src/commands.rs` (`subscribe` et les 5 commandes à `Channel<InvokeResponseBody>` : `account_login`, `agent_terminal`, `term_spawn`, `run_start`, `test_run_start`), ce qu'elles appellent dans le cœur et qui prend un `Channel` (`pty.rs`, `tui.rs`, les lancements), `src-tauri/src/fluxbench.rs`, les tests qui font `Channel::new(…)`

**Interfaces:**
- Produces, dans `crate::sink` :

```rust
/// Where a flow of the core goes: the window's channel, or a remote client's socket.
pub struct Sink<T>(Arc<dyn Fn(T) -> Result<(), Closed> + Send + Sync>);
/// The other end is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Closed;
impl<T> Clone for Sink<T> { /* the Arc */ }
impl<T> Sink<T> {
    pub fn new(f: impl Fn(T) -> Result<(), Closed> + Send + Sync + 'static) -> Self;
    pub fn send(&self, value: T) -> Result<(), Closed>;
}
impl<T: serde::Serialize + Clone + Send + 'static> From<tauri::ipc::Channel<T>> for Sink<T>;      // events
impl From<tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>> for Sink<Vec<u8>>;                 // terminal bytes
```

  - `Hub::set_sink(&self, sink: Sink<UiEvent>)` remplace `set_channel` ; le cœur et les terminaux ne nomment plus `tauri::ipc`.

- [ ] **Step 1 : les tests, qui échouent d'abord** (`sink.rs`) : `a_sink_gives_what_it_is_sent` (une fermeture qui pousse dans un `Vec`, trois envois, les trois valeurs dans l'ordre) ; `a_closed_sink_says_so` (la fermeture rend `Err(Closed)`, `send` rend `Err(Closed)`). Dans les tests du hub : `the_hub_emits_to_its_sink` (un événement émis arrive à la fermeture).
- [ ] **Step 2 :** Les lancer (échec : `sink` n'existe pas), écrire le module, les relancer (verts).
- [ ] **Step 3 :** Brancher le hub et les terminaux sur `Sink`. Les commandes gardent leur paramètre `Channel` et le convertissent à l'entrée (`channel.into()`). Lire d'abord comment chaque flux de terminal écrit aujourd'hui (octets bruts par `InvokeResponseBody::Raw`) et garder exactement le même corps envoyé à la fenêtre.
- [ ] **Step 4 :** Dans les tests, remplacer `Channel::new(…)` par `Sink::new(…)` là où il ne servait qu'à recevoir.
- [ ] **Step 5 :** Vérifier : `git grep -n "ipc::Channel\|InvokeResponseBody" -- src-tauri/src crates` ne rend que `sink.rs`, `commands.rs` et `fluxbench.rs` s'il mesure le canal de Tauri lui-même (le dire) ; fmt, clippy, suite égale à la référence plus les trois tests.
- [ ] **Step 6 :** Commit : `refactor(core): les flux du cœur passent par un Sink, plus par le Channel de Tauri`.

### Task 2: L'attribut `#[command]` et la table

**Files:**
- Create : `crates/escouade-macros/` (`Cargo.toml`, `src/lib.rs`, crate `proc-macro`), `src-tauri/src/remote/mod.rs`, `src-tauri/src/remote/table.rs`
- Modify : `src-tauri/Cargo.toml` (le membre du workspace et la dépendance), `docs/CRATES.md` (le crate neuf), `src-tauri/src/commands.rs` (**quatre commandes seulement**, pour la preuve : `set_ui`, `engines`, `account_status` et `quit_app`)

**Interfaces:**
- Consumes : `Core`, `CoreState`, `escouade_base::rt`.
- Produces :

```rust
// crates/escouade-macros
#[proc_macro_attribute] pub fn command(args, item) -> TokenStream;   // #[command], #[command(async)], #[command(desktop)], #[command(async, desktop)]
#[proc_macro] pub fn handlers(input) -> TokenStream;                 // handlers![commands::a, commands::b, …]

// src-tauri/src/remote/table.rs
pub struct Ctx { pub core: Arc<Core> }
pub type Call = fn(Ctx, serde_json::Value) -> futures::future::BoxFuture<'static, Result<serde_json::Value, String>>;
pub struct Entry { pub name: &'static str, pub call: Option<Call> }   // None: desktop only
```

  - `#[command]` sur `fn foo(core: CoreState, agent_id: String, n: u32) -> Res<T>` : laisse la fonction telle quelle sous `#[tauri::command]` (ou `#[tauri::command(async)]`), et écrit `pub const __REMOTE_FOO: Entry`. L'appel lit `agentId` et `n` dans l'objet JSON (camelCase, comme Tauri), donne `ctx.core` pour `CoreState`, attend la fonction (`async fn`) ou la lance par `rt::spawn_blocking` (fonction synchrone), et rend la valeur sérialisée ; une erreur `Err(e)` devient la chaîne que Tauri aurait rendue.
  - Un paramètre qui n'est ni `CoreState` ni un argument désérialisable (`AppHandle`, `State<…>`, `VoiceState`, `Channel<…>`, `Window`) est **refusé à la compilation** sans `desktop` : le message dit d'ajouter `desktop` ou de retirer le paramètre. Avec `desktop`, `call` vaut `None`.
  - `handlers![a, b]` rend un tuple `(impl Fn(tauri::ipc::Invoke<R>) -> bool, &'static [Entry])` : la première moitié est exactement `tauri::generate_handler![a, b]`.
  - `remote::table::call(entries, ctx, name, args) -> Result<Value, CallError>` avec `CallError::{Unknown, DesktopOnly, Args(String), Failed(String)}`.

- [ ] **Step 1 : les tests, qui échouent d'abord** (`remote/table.rs`, sur un cœur chargé avec `host::Fake`, sans harness) :
  - `a_command_is_called_by_its_name_with_camel_case_arguments` : `set_ui` appelé par la table avec l'objet que la fenêtre envoie (`{"ui": …}`), puis l'état lu dans le cœur est celui envoyé ; et une commande à paramètre de plusieurs mots prise parmi les quatre (ou une cinquième si aucune n'en a) reçoit bien sa clé en camelCase.
  - `a_missing_argument_is_an_error_that_names_it` : `Args` dont le texte contient le nom de la clé.
  - `an_unknown_command_is_refused` : `Unknown`.
  - `a_desktop_command_is_refused_remotely` : `quit_app` rend `DesktopOnly`.
  - `a_command_that_fails_gives_its_message` : la chaîne d'erreur de la commande, la même que par Tauri.
  - `a_blocking_command_runs_off_the_runtime` : une commande synchrone de test (sous `cfg(test)`) qui relève `tokio::runtime::Handle::try_current()` et le nom de son fil : elle ne tourne pas sur un fil de travail du runtime.
- [ ] **Step 2 :** Les lancer : échec, rien n'existe. Écrire le crate de macros et la table, convertir les quatre commandes, relancer : verts.
- [ ] **Step 3 :** Ajouter un test de compilation négatif **sans nouveau paquet** : un `compile_fail` en doctest dans `crates/escouade-macros/src/lib.rs` qui montre qu'un paramètre `AppHandle` sans `desktop` ne compile pas. Si un doctest ne peut pas dépendre de `tauri` depuis ce crate, le remplacer par un test unitaire de la fonction de classement de la macro (elle prend les types des paramètres et rend « distant », « bureau » ou l'erreur) et le dire.
- [ ] **Step 4 :** Vérifier que `cargo tree -p escouade-macros` ne tire que `syn`, `quote`, `proc-macro2`, et que `Cargo.lock` ne gagne que ce crate. fmt, clippy, suite égale à la référence plus les tests neufs.
- [ ] **Step 5 :** Commit : `feat(remote): un attribut de commande qui donne aussi une entrée appelable sans Tauri`.

### Task 3: Toutes les commandes dans la table

**Files:**
- Modify : `src-tauri/src/commands.rs` (les 346 attributs), `src-tauri/src/windowthread_tests.rs` et `src-tauri/src/fileorigin.rs` (leurs `#[tauri::command]`, s'ils sont dans la liste de `lib.rs`), `src-tauri/src/lib.rs:656-…` (`handlers![…]` à la place de `tauri::generate_handler![…]`, la table gardée dans un `static` ou donnée au cœur)

**Interfaces:**
- Consumes : `#[command]`, `handlers!`, `Entry` (tâche 2) ; `Sink` (tâche 1).
- Produces : `crate::remote::table::ENTRIES: &'static [Entry]` (ou une fonction qui la rend), la table complète ; pour les 6 commandes à flux (`subscribe`, `account_login`, `agent_terminal`, `term_spawn`, `run_start`, `test_run_start`), une fonction interne sans `Channel` qui prend un `Sink`, que la commande Tauri appelle : `pub(crate) fn subscribe_to(core: &Arc<Core>, sink: Sink<UiEvent>) -> InitialState`, et de même `…_to(…, output: Sink<Vec<u8>>)` pour les cinq autres, avec leurs arguments d'aujourd'hui.

- [ ] **Step 1 : les tests, qui échouent d'abord** :
  - `every_command_of_the_window_is_in_the_table` : les noms de la table sont exactement ceux de la liste donnée à Tauri, sans doublon (la macro `handlers!` le garantit par construction : le test compte et compare au nombre attendu, relevé avant la conversion).
  - `the_commands_of_the_desktop_alone_are_these` : la liste triée des noms dont `call` vaut `None` est égale à une liste écrite dans le test. Attendu au départ, à corriger d'après le code et à justifier nom par nom dans le compte rendu : les 14 commandes à `AppHandle` (`clipboard_image`, `save_image`, `quit_app`, `update_check`, `update_restart`, `update_postpone`, `devpage_open`, `devpage_place`, `devpage_go`, `devpage_devtools`, `devpage_detach`, `devpage_mode`, `devpage_show`, `devpage_close`), celles à `State<…>` d'un état de bureau (`update_download`, `update_presence`, `update_close`, `deep_links_take`, `devpage_list`), les 20 de la voix à `VoiceState`, et les 6 à flux (servies à distance par le serveur lui-même en tâche 4, pas par la table).
- [ ] **Step 2 :** Convertir. `save_settings` et `subscribe` prennent `VoiceState` en plus du cœur : lire ce qu'elles en font. Si l'usage de la voix peut passer par `core.host` (une méthode de plus sur `Host`, à nommer et justifier) sans changer le comportement, la commande devient distante ; sinon elle reste « bureau » et c'est dit. `open_url`, `open_path` et `play_chime` restent distantes dans la table (elles ne prennent rien du bureau) : leur sort sur un serveur est celui du lot 5.
- [ ] **Step 3 :** Sortir les six fonctions `…_to` et faire des commandes Tauri de simples appels.
- [ ] **Step 4 :** Vérifier : `git grep -c "#\[tauri::command" -- src-tauri/src` ne rend plus rien hors de ce que la macro écrit ; les deux tests verts ; fmt, clippy, suite égale à la référence plus les tests neufs.
- [ ] **Step 5 :** Commit : `refactor(core): toutes les commandes déclarées une fois, pour Tauri et pour la table distante`.

### Task 4: Le serveur WebSocket, éteint par défaut

**Files:**
- Create : `src-tauri/src/remote/server.rs`, `src-tauri/src/remote/wire.rs`, `src-tauri/src/remote/server_tests.rs`
- Modify : `src-tauri/src/remote/mod.rs`, `src-tauri/src/lib.rs` (le démarrage, après `core.start`), `docs/REMOTE.md` (cocher ce qui est fait dans « Tâches », en une ligne, sans réécrire l'étude)

**Interfaces:**
- Consumes : la table et les six fonctions `…_to` (tâche 3), `Sink` (tâche 1), `rt`.
- Produces :
  - `remote::server::start(core: Arc<Core>, entries: &'static [Entry], config: Config) -> std::io::Result<Running>` ; `Config { port: u16, token: String }` ; `Running { pub addr: SocketAddr, … }` qui arrête le serveur quand on le lâche.
  - `remote::server::from_env() -> Option<Config>` : lit `ESCOUADE_REMOTE_PORT` et `ESCOUADE_REMOTE_TOKEN` ; `None` si l'un manque, si le port n'est pas un nombre ou si le jeton fait moins de 32 caractères (une ligne de journal, sans le jeton).
  - Le fil (`wire.rs`), en JSON sur des trames texte :
    - à la connexion, le serveur envoie `{"hello":{"version":"<version de l'app>","protocol":1}}` ;
    - requête `{"id":<u64>,"cmd":"<nom>","args":{…}}` ; réponse `{"id":<u64>,"ok":<valeur>}` ou `{"id":<u64>,"err":{"kind":"unknown"|"desktop"|"args"|"failed","message":"…"}}` ;
    - événement poussé après `subscribe` : `{"event":<UiEvent>}` ;
    - flux de terminal : la requête d'une commande à flux porte `"stream":<u32>` choisi par le client ; les octets arrivent en trames **binaires**, 4 octets de `stream` en gros-boutiste puis les octets.
  - La poignée de main HTTP : jeton dans `Authorization: Bearer <jeton>` (comparaison à temps constant) ; `Host` égal à `127.0.0.1:<port>` ou `localhost:<port>` ; aucune `Origin`. Sinon 401 ou 403, sans corps qui renseigne.

- [ ] **Step 1 : les tests, qui échouent d'abord** (`server_tests.rs`, cœur avec `host::Fake`, port 0, client `tokio_tungstenite`, sans harness) :
  - `the_server_greets_with_its_version_and_protocol`.
  - `a_command_is_answered_under_its_id` : `set_ui`, puis une lecture qui rend ce qui a été écrit ; deux requêtes envoyées à la suite reçoivent chacune leur `id`.
  - `an_event_reaches_a_subscribed_client` : `subscribe` rend l'état initial ; une action du cœur qui émet un événement (prise parmi celles que les tests du lot 1 déclenchent sans harness) arrive en `{"event":…}`.
  - `a_terminal_stream_arrives_in_binary_frames_under_its_number` : `term_spawn` avec `stream: 7` sur une commande du système qui écrit une ligne connue (`printf`) ; les trames binaires commencent par `00 00 00 07` et portent la ligne.
  - `a_wrong_token_is_refused`, `a_missing_token_is_refused`, `a_foreign_host_is_refused`, `any_origin_is_refused` : la poignée de main échoue avec 401 ou 403.
  - `a_desktop_command_says_so` et `an_unknown_command_says_so` : `err.kind`.
  - `a_client_that_leaves_mid_stream_stops_nothing` : le client ferme pendant le flux ; le terminal existe toujours dans le cœur et un second client peut se connecter et appeler une commande.
  - `nothing_listens_without_the_variables` : `from_env()` rend `None` sans les variables, avec un port non numérique, avec un jeton de 31 caractères.
- [ ] **Step 2 :** Les lancer (échec), écrire le serveur, les relancer (verts).
- [ ] **Step 3 :** Le démarrage dans `lib.rs` : `if let Some(config) = remote::server::from_env() { … }`, le serveur gardé en vie avec l'app. Une ligne de journal dit le port, jamais le jeton. Un commentaire d'une ligne dit que `subscribe` à distance prend la place de la fenêtre tant que le hub n'a qu'un abonné.
- [ ] **Step 4 :** Vérifier : `git grep -n "0\.0\.0\.0\|\[::\]" -- src-tauri/src/remote` ne rend rien ; le jeton n'apparaît dans aucun `log::` ni `format!` d'erreur ; fmt, clippy, suite égale à la référence plus les tests neufs.
- [ ] **Step 5 :** Commit : `feat(remote): l'API du cœur servie en WebSocket sur 127.0.0.1, éteinte par défaut`.
