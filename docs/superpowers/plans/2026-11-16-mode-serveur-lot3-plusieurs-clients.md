# Mode serveur, lot 3 : plusieurs clients et la reprise — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Plusieurs clients (la fenêtre et des clients distants) reçoivent les mêmes événements et la sortie des mêmes terminaux, et un client qui se reconnecte retrouve l'état entier et l'écran de chaque terminal ; l'app de bureau seule ne change pas.

**Architecture:** Le hub garde une liste d'abonnés au lieu d'un seul, chacun avec sa poignée de désabonnement. La sortie de chaque terminal et de chaque lancement passe par un diffuseur qui garde ses derniers octets, comme le font déjà les vues d'agent (`tui.rs`, `OUTPUT_KEPT`), et sert plusieurs lecteurs : un client qui s'y attache reçoit d'abord ce qui est gardé, puis la suite. Le serveur distant s'abonne sans rien prendre à la fenêtre.

**Tech Stack:** Rust 1.98.0, tokio ; construction et tests dans le conteneur (`spikes/lot0/run.sh --desktop-libs`), le serveur de `src-tauri/src/remoteapi/`.

**Spec:** `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (« Architecture » : « Plusieurs abonnés », « La reprise », « L'état d'interface partagé ») ; `docs/REMOTE.md`, « La fondation commune », points 3 et 4.

## Global Constraints

- **Aucun changement de comportement** de l'app de bureau quand elle est seule : mêmes événements dans le même ordre, mêmes octets de terminal, même mémoire à vide près des tampons décrits ici.
- La branche `feat/plusieurs-clients` part de `feat/sans-bureau` (tête rebasée), dans un worktree `../Escouade-lot3`. Code et doc de l'amont seulement.
- Un commit par tâche. Sujets en français, portées de l'amont, corps de 2 à 4 lignes coupées vers 100 colonnes (ce que ça fait ; rien ne change pour l'utilisateur ; le point de `docs/REMOTE.md`), traîne `Co-Authored-By` des autres commits. Rien sur les lots, plans, tâches ou relectures. Rien n'est poussé chez l'amont.
- `UiState` reste unique et partagé : le dernier qui l'écrit gagne ; naviguer dans un client ne déplace pas les autres (décidé par le demandeur). Aucun état d'interface par client dans ce lot.
- Mémoire bornée : un tampon de sortie par terminal et par lancement, de taille fixe écrite dans une constante (celle des vues d'agent, `tui::OUTPUT_KEPT`, sauf raison dite) ; il part avec son terminal. Aucune liste d'abonnés ni de lecteurs ne grandit sans borne : un abonné mort est retiré à son premier envoi raté.
- Un abonné lent ou mort ne retarde ni la fenêtre ni les autres : l'envoi à un abonné ne bloque jamais (règle du lot 2 : un `Sink` n'attend pas).
- Après chaque tâche : `cargo fmt --all --check` ; clippy `-D warnings` sur le défaut (`--desktop-libs`) et sur `--no-default-features -p escouade --all-targets` (image nue) ; la suite de référence égale **par nom** à celle de la tâche 0 plus les tests neufs ; les tests sans option relancés. Un test vert avant et rouge après est relancé seul 5 fois ; rouge ici et vert sur le commit d'avant, c'est une régression : on s'arrête.
- Disque : `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`, une commande cargo à la fois, un guetteur d'espace libre pendant les longues commandes (arrêt sous 8 Go), volumes de construction sous 25 Go.
- Commentaires du code en anglais, une ligne. Aucun `cfg(feature = …)` nouveau dans un fichier du cœur.
- Hors de ce lot : le front (lot 4), le binaire serveur, l'appairage et les jetons d'appareil (lot 5), un état d'interface par client.

## Review Focus

- **L'ordre des événements** pour un abonné : deux événements émis par deux fils arrivent à chaque abonné dans le même ordre relatif (test en tâche 1).
- **Un abonné qui s'abonne pendant une rafale** : il reçoit l'état initial puis les événements d'après, sans trou ni doublon entre les deux (test en tâche 1 ; c'est la course que la fenêtre seule ne voyait pas).
- **Le rejeu d'un terminal coupé au milieu d'une séquence d'échappement** ou d'un caractère UTF-8 : le tampon se rejoue depuis un point sûr, comme le font les vues d'agent (`without_terminal_queries`, les requêtes de terminal retirées) (test en tâche 2).
- **La fenêtre qui se recharge** pendant qu'un client distant est abonné : elle retrouve ses événements, le client garde les siens (test en tâche 3).
- **Les effets de bord de `subscribe`** (`reset_unsaved`, `held` vidé, la dictée annulée) : ils appartiennent au rechargement de la fenêtre ; un client distant qui s'abonne ne doit rien défaire chez les autres (tâche 1, décision écrite).

---

### Task 0: L'environnement et la référence

**Files:** aucun fichier suivi. Hors dépôt : `<espace du plan>/baseline.txt`, `<espace du plan>/base-status.txt`, `<espace du plan>/base-headless-status.txt`.

- [ ] **Step 1 :** `git worktree add ../Escouade-lot3 -b feat/plusieurs-clients feat/sans-bureau`. Copier `spikes/lot0/` (non suivi, exclu) du worktree du lot précédent ; vérifier `CARGO_INCREMENTAL=0` et Node dans l'image `--desktop-libs`.
- [ ] **Step 2 :** La suite de référence (défaut) et les tests sans option (`--no-default-features -p escouade`, dans `--desktop-libs`) : les deux fichiers d'état, une ligne par test, et les totaux dans `baseline.txt`.

### Task 1: Le hub à plusieurs abonnés

**Files:**
- Modify : `src-tauri/src/hub.rs`, `src-tauri/src/commands.rs` (`subscribe_to`, `subscribe`), `src-tauri/src/remoteapi/server.rs` (l'abonnement d'un client), `src-tauri/src/fluxbench.rs`, les tests qui posent le sink du hub, `docs/REMOTE.md` (« Tâches »)

**Interfaces:**
- Produces :

```rust
impl Hub {
    /// Adds a subscriber; it leaves when its `Subscription` is dropped or its sink says `Closed`.
    pub fn subscribe(&self, sink: Sink<UiEvent>) -> Subscription;
    /// The window's own subscriber: a new one takes the place of the one before (a page that reloads).
    pub fn set_window(&self, sink: Sink<UiEvent>);
    pub fn emit(&self, event: UiEvent);          // to every subscriber, in one order
}
pub struct Subscription { /* removes its subscriber on drop */ }
```

  - `subscribe_to(core, sink)` devient `subscribe_to(core, sink, who: Subscriber) -> (InitialState, Option<Subscription>)` avec `enum Subscriber { Window, Remote }` : la fenêtre passe par `set_window` et fait ce qu'elle faisait (`cancel_dictation`, `reset_unsaved`, `held` vidé) ; un client distant passe par `subscribe` et ne fait **aucun** de ces trois effets. Le serveur garde la `Subscription` avec la connexion.
  - L'état initial et l'abonnement sont pris de façon qu'aucun événement ne tombe entre les deux : soit l'abonné est ajouté **avant** la lecture de l'état et le client applique les événements par-dessus (ils sont idempotents ? le vérifier, événement par événement, dans le compte rendu), soit un verrou court couvre les deux. Le choix est au constructeur, prouvé par le test de l'étape 1.

- [ ] **Step 1 : les tests, qui échouent d'abord** (`hub.rs` et les tests du serveur, sans harness) :
  - `every_subscriber_gets_every_event_in_one_order` : trois abonnés, deux fils qui émettent chacun 500 événements numérotés ; chaque abonné reçoit les 1 000, et l'ordre relatif des deux séries est le même chez les trois.
  - `a_dropped_subscription_gets_nothing_more` ; `a_closed_sink_is_forgotten_at_its_first_failed_send` (la liste revient à sa taille).
  - `the_window_keeps_its_events_when_a_remote_client_subscribes` et `…_when_it_leaves` : c'est le contraire de ce que le lot 2 documentait.
  - `a_window_that_reloads_takes_its_own_place_only` : `set_window` deux fois, un abonné distant entre les deux garde ses événements ; l'ancienne fenêtre n'en reçoit plus.
  - `nothing_is_lost_between_the_initial_state_and_the_events` : un fil émet sans arrêt des événements numérotés qui changent un compteur lu dans l'état initial ; un client s'abonne au milieu ; l'état initial plus les événements reçus donnent le compteur final, sans trou.
  - `a_remote_subscribe_undoes_nothing_of_the_others` : `held` non vide et un brouillon non enregistré posés par la fenêtre restent après l'abonnement d'un client distant ; `Fake` ne compte aucun `cancel_dictation`.
- [ ] **Step 2 :** Écrire. `emit` ne tient pas son verrou pendant les envois si un envoi peut rappeler le hub ; le compteur de « seconde bruyante » compte un événement une fois, pas une fois par abonné.
- [ ] **Step 3 :** Retirer des commentaires, du corps de `docs/REMOTE.md` et de l'en-tête du serveur ce qui disait qu'un abonnement distant prend les événements de la fenêtre et coupe sa dictée. Laisser ce qui reste vrai (`agent_terminal`, jusqu'à la tâche 2 ; `save_settings` distant règle la conversation locale).
- [ ] **Step 4 :** Vérifications de fin de tâche (contraintes globales).
- [ ] **Step 5 :** Commit : `feat(remote): le hub diffuse à plusieurs abonnés, la fenêtre garde les siens`.

### Task 2: La sortie des terminaux gardée et diffusée

**Files:**
- Create : `src-tauri/src/output.rs` (le diffuseur), ses tests
- Modify : `src-tauri/src/tui.rs` (`TuiOutput` : s'en sert ou lui cède sa place), `src-tauri/src/pty.rs` et ce qui lance terminaux et lancements (`commands.rs` : `term_spawn_to`, `run_start_to`, `test_run_start_to`, `account_login_to`, `agent_terminal_to`), `src-tauri/src/remoteapi/server.rs`, `docs/REMOTE.md`

**Interfaces:**
- Produces :

```rust
/// The output of a terminal: its last bytes kept, and whoever reads it now.
pub struct Output { /* kept: bounded; readers: a bounded list */ }
impl Output {
    pub fn new(kept: usize) -> Arc<Self>;
    pub fn push(&self, bytes: Vec<u8>);                       // called by the PTY's reader, never waits
    /// Replays what is kept from a safe point, then follows; the reader leaves when its handle drops.
    pub fn attach(self: &Arc<Self>, sink: Sink<Vec<u8>>) -> Reader;
}
```

  - Chaque terminal, chaque lancement et chaque vue d'agent a **un** `Output`, créé avec lui et lâché avec lui ; la fenêtre et les clients distants en sont des lecteurs. Attacher un second lecteur ne détache pas le premier (c'est la fin de la prise de contrôle d'`agent_terminal`).
  - Deux commandes neuves, déclarées par `#[command]` avec `desktop` (un flux) et servies par le serveur comme les six autres : `term_attach(id, …)` et `run_attach(id, …)`, pour lire un terminal ou un lancement **déjà lancé** ; la fenêtre peut s'en servir plus tard, ce lot ne change pas le front. Leurs noms et arguments suivent ceux de `agent_terminal`.
  - Le rejeu : ce que `TuiOutput` fait aujourd'hui (`OUTPUT_KEPT`, `REPLAY_PIECE`, `without_terminal_queries`) devient le comportement de tout `Output`. `tui.rs` garde ce qui est propre aux vues d'agent.

- [ ] **Step 1 :** Lire `tui.rs` (`TuiOutput`, `attach`, `push`, le rejeu) et dire dans le compte rendu ce qui se généralise tel quel et ce qui est propre aux vues d'agent. Si `TuiOutput` porte de l'état que les terminaux simples n'ont pas (taille, mode de l'écran), le laisser en surcouche plutôt que de le forcer dans `Output`.
- [ ] **Step 2 : les tests, qui échouent d'abord** (`output.rs`, sans PTY) : `a_reader_gets_what_is_kept_then_what_follows` (ordre exact, aucun octet en double) ; `two_readers_get_the_same_bytes` ; `the_kept_bytes_stop_at_the_bound` (le début tombe, la fin reste) ; `the_replay_starts_at_a_safe_point` (un tampon coupé au milieu d'une séquence CSI et d'un caractère UTF-8 de plusieurs octets ne rejoue pas le morceau cassé) ; `the_replay_leaves_out_the_terminals_queries` (comme aujourd'hui pour les vues d'agent) ; `a_reader_that_leaves_or_fails_is_forgotten` ; `push_never_waits_on_a_slow_reader`.
- [ ] **Step 3 :** Des tests avec un vrai terminal (`printf`, `cat`), par le serveur : `a_second_client_reads_a_terminal_already_running` (il reçoit l'écran gardé puis la suite) ; `a_client_that_comes_back_finds_its_terminals_screen` (connexion fermée, nouvelle connexion, `term_attach`) ; `an_agents_view_is_read_by_two_at_once` si une vue d'agent se lance sans harness, sinon le test au niveau d'`Output` et le dire.
- [ ] **Step 4 :** Écrire. Pour la fenêtre seule, les octets reçus et leur découpage sont ceux d'avant (le test du lot 2 sur les flux de la fenêtre passe toujours).
- [ ] **Step 5 :** Mesurer et écrire dans le compte rendu : la mémoire d'un terminal au repos avec son tampon plein, et le coût par bloc de sortie avant et après (une copie de plus par lecteur est attendue ; rien de plus).
- [ ] **Step 6 :** Vérifications de fin de tâche. La table distante compte deux commandes de plus : les nombres écrits dans les tests de la table suivent.
- [ ] **Step 7 :** Commit : `feat(remote): la sortie d'un terminal est gardée et lue par plusieurs, un client retrouve son écran`.

### Task 3: La reprise d'un client, de bout en bout

**Files:**
- Modify : `src-tauri/src/remoteapi/server.rs`, `src-tauri/src/remoteapi/wire.rs`, `src-tauri/src/remoteapi/server_tests.rs`, `docs/REMOTE.md`

**Interfaces:**
- Produces : rien de neuf sur le fil si les tâches 1 et 2 suffisent ; sinon ce qui manque à un client pour reprendre, ajouté au fil et décrit en tête de `wire.rs` (par exemple la liste des terminaux et lancements vivants est déjà dans l'état initial : le vérifier).

- [ ] **Step 1 : le test, qui échoue d'abord s'il manque quelque chose** : `a_client_resumes_after_a_cut` — un client s'abonne, ouvre deux terminaux, lance une commande qui écrit pendant plusieurs secondes ; sa connexion est coupée net (la socket fermée sans trame de fin) ; un nouveau client se connecte, s'abonne, lit dans l'état initial les deux terminaux, s'y attache, et retrouve pour chacun la fin de ce qui a été écrit pendant la coupure puis la suite ; pendant tout ce temps un abonné « fenêtre » posé par le test a reçu chaque événement une seule fois.
- [ ] **Step 2 :** Deux clients à la fois : `an_answer_given_by_one_client_is_seen_by_the_other` — une demande en attente (prise parmi celles que les tests du cœur créent sans harness : une question, une autorisation simulée) est répondue par un client ; l'autre reçoit l'événement qui la ferme, et une seconde réponse du second client est refusée ou sans effet, comme une réponse venue de claude.ai aujourd'hui. Si aucune demande ne se crée sans harness, le dire et tester au niveau du cœur ce qui ferme une demande pour tous.
- [ ] **Step 3 :** Combler ce que les deux tests montrent. Mettre à jour `docs/REMOTE.md` (« Tâches » : `hub.rs` à plusieurs abonnés — fait ; `UiState` par client — non fait, dit pourquoi ; une réponse fermée chez les autres ; un tampon de sortie pour chaque terminal et chaque lancement ; la reprise d'un client).
- [ ] **Step 4 :** Vérifications de fin de tâche.
- [ ] **Step 5 :** Commit : `feat(remote): un client qui revient retrouve l'état et ses terminaux`.
