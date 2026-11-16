# Mode serveur, lot 5 : le serveur qu'on installe — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `escouade-server` s'installe en service sur un Linux sans écran, on y appaire un navigateur, et tout le parcours du but 3 de la spec s'y fait : ajouter un compte par le terminal du harness, cloner un projet, lancer un agent, joindre et récupérer des fichiers, recevoir une notification, gérer ses plugins.

**Architecture:** Le binaire minimal du lot 4 gagne ses sous-commandes (`pair`, `devices`, `install`), des jetons d'appareil gardés dans un fichier du dossier de données à la place du jeton d'environnement, un verrou par fichier, et les origines permises. Le cœur gagne les commandes qui remplacent ce que le bureau faisait par le système (parcourir les dossiers du serveur, recevoir et rendre des fichiers par un chemin d'octets à part de la WebSocket), et l'interface s'en sert quand elle n'est pas dans la fenêtre.

**Tech Stack:** Rust 1.98.0 (`--no-default-features`), `hyper` 1, `tokio-tungstenite` ; Svelte 5, Vitest, Playwright (Chromium) ; systemd (unité d'utilisateur).

**Spec:** `docs/superpowers/specs/2026-11-14-mode-serveur-web-design.md` (« Sécurité », « Ce qui change de machine », « Installation », lot 5, « Décidé par le demandeur » et « après le lot 0 ») ; `docs/REMOTE.md`, « Le mode serveur ».

## Global Constraints

- **Un serveur, un utilisateur.** Plusieurs appareils du même utilisateur, chacun son jeton, révocable un par un.
- **Jamais un port ouvert sur Internet** : écoute sur `127.0.0.1` seulement, aucune option pour écouter ailleurs. On l'atteint par un tunnel (`ssh -L`) ou par un proxy de l'utilisateur déclaré par `--origin https://nom`.
- Le jeton d'un appareil : 32 octets aléatoires, gardé haché (SHA-256) dans un fichier lisible du seul propriétaire (`0600`) du dossier de données ; il ne s'écrit dans aucun journal, aucune adresse gardée, aucun message d'erreur ; comparé à temps constant. Le code d'appairage sert une fois et expire en 10 minutes.
- Les chemins que le serveur parcourt ou écrit pour un client restent sous le dossier de l'utilisateur du service ; un chemin qui en sort est refusé. Les fichiers privés (`fence.rs`) valent ici comme ailleurs. Un fichier reçu n'est jamais exécuté ni ouvert sur le serveur.
- **Aucun changement de comportement** de l'app de bureau ; aucun composant Svelte réécrit pour le web (règle du lot 4 : une condition `platform.desktop` par endroit).
- La branche `feat/serveur` part de `feat/front-web`, dans un worktree `../Escouade-lot5`. Code, tests, CI et doc de l'amont.
- Un commit par tâche, sujets en français aux portées de l'amont, corps de 2 à 4 lignes coupées vers 100 colonnes, traîne `Co-Authored-By` des autres commits. Rien sur les lots, plans, tâches ou relectures. Rien n'est poussé chez l'amont.
- Aucune dépendance tierce nouvelle sauf raison écrite dans la tâche et dans le commit.
- Après chaque tâche : côté front `npm run check`, `npm test`, `npm run format:check`, `npm run build`, `npm run build:web`, `npm run test:web` ; côté Rust fmt, clippy `-D warnings` (défaut et `--no-default-features -p escouade --all-targets`), le garde-fou de l'arbre des dépendances, la suite par défaut et les tests sans option égaux **par nom** à la référence plus les tests neufs. Un test vert avant et rouge après est relancé seul 5 fois ; rouge ici et vert sur le commit d'avant, c'est une régression : on s'arrête.
- Disque : `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=1`, une commande cargo à la fois, guetteur d'espace libre (arrêt sous 8 Go), volumes sous 40 Go.
- Hors de ce lot : le client de bureau qui se connecte à un serveur, WireGuard, l'app mobile et le push, la mise en page mobile, la voix dans le navigateur, la mise à jour du serveur depuis l'interface, les pages détachées et le relais des ports des serveurs de test, Docker, plusieurs personnes sur un serveur.

## Review Focus

- **L'appairage rejoué** : un code déjà servi, expiré, ou deviné (limiter les essais) ne donne rien ; deux appairages simultanés ne se volent pas leur jeton (tâche 1).
- **Un appareil révoqué** : sa connexion ouverte est coupée tout de suite, pas à sa prochaine requête (tâche 1).
- **La traversée de dossiers** dans le navigateur des dossiers, l'envoi et le téléchargement : `..`, liens symboliques qui sortent, chemins absolus, noms encodés, noms avec `\0` ou des séparateurs de l'autre système (tâches 4 et 5).
- **Un envoi interrompu ou énorme** : rien de partiel ne reste, une taille maximale est tenue, le disque ne se remplit pas (tâche 5).
- **Le proxy** : avec `--origin`, seule cette origine et l'hôte local sont acceptés ; un en-tête `X-Forwarded-*` n'est jamais cru pour décider de l'accès (tâche 2).
- **Deux serveurs sur le même dossier de données** : le second refuse de démarrer et dit pourquoi ; un verrou laissé par un process mort ne bloque pas (tâche 3).

---

### Task 0: L'environnement et les références

- [ ] `git worktree add ../Escouade-lot5 -b feat/serveur feat/front-web` ; copier `spikes/lot0/` ; références du front et de Rust (défaut et sans option), une ligne par test ; `npm run test:web` passe sur la base.

### Task 1: Les appareils — appairage, jetons, révocation

**Files:** `src-tauri/src/remoteapi/devices.rs` (neuf, et ses tests), `remoteapi/server.rs`, `src-tauri/src/bin/escouade-server.rs`, `docs/REMOTE.md`.

**Interfaces:**
- `devices::Devices` : `open(data_dir) -> Devices` (fichier `devices.json`, `0600`, écrit par remplacement atomique) ; `pair_code() -> PairCode` (affiché une fois, 10 minutes, un seul vivant à la fois) ; `redeem(code, name) -> Result<Token, Refused>` ; `check(token) -> Option<DeviceId>` (temps constant, sur le haché) ; `list() -> Vec<Device { id, name, paired_at, last_seen }>` ; `revoke(id) -> bool`.
- Le serveur : `POST /pair` avec le code et un nom d'appareil rend la session (le cookie du lot 4, qui porte désormais le jeton de l'appareil) ; après 5 codes faux, l'appairage en cours est annulé. Une révocation coupe les connexions de l'appareil.
- Le binaire : `escouade-server pair` (parle au serveur qui tourne par un fichier ou une socket du dossier de données, au constructeur de choisir le plus simple, et affiche l'adresse à ouvrir et le code) ; `escouade-server devices` ; `escouade-server devices revoke <id>`. `ESCOUADE_REMOTE_TOKEN` reste accepté **pour les tests seulement** (builds de debug), comme dans l'app de bureau.

- [ ] Tests d'abord : un code sert une fois ; expire ; cinq essais faux l'annulent ; le jeton n'est gardé que haché (le fichier ne le contient pas) ; droits `0600` ; un appareil révoqué est refusé et sa connexion ouverte se ferme ; deux `redeem` simultanés du même code : un seul réussit ; le fichier abîmé ou absent ne fait pas planter le serveur (aucun appareil, journal d'une ligne).
- [ ] Écrire ; fin de tâche ; commit `feat(remote): des appareils appairés par un code, un jeton chacun, révocable`.

### Task 2: Les origines permises et le tunnel

**Files:** `remoteapi/server.rs`, `bin/escouade-server.rs`, tests, `docs/REMOTE.md`.

**Interfaces:** `--origin <url>` (répétable) : en plus de `127.0.0.1:<port>` et `localhost:<port>`, ces origines exactes (schéma, hôte, port) sont acceptées pour `Host` et `Origin` ; le cookie de session porte `Secure` quand l'origine est en `https`. `Host: localhost:<autre port>` (un `ssh -L` sur un autre port local) est accepté **si** la connexion vient de la boucle locale et que le jeton est bon : l'écrire et le tester, c'était refusé au lot 2.

- [ ] Tests d'abord : origine déclarée acceptée, voisine refusée (autre port, autre schéma, sous-domaine, suffixe) ; `X-Forwarded-Host`/`-Proto`/`-For` ignorés ; `ssh -L` sur un autre port accepté ; sans `--origin`, comportement du lot 4 inchangé. `escouade-server pair` affiche l'adresse de la première origine déclarée.
- [ ] Écrire ; fin de tâche ; commit `feat(remote): les origines qu'on déclare au serveur, pour un proxy ou un tunnel`.

### Task 3: Le service et le verrou

**Files:** `bin/escouade-server.rs`, `src-tauri/src/remoteapi/lockfile.rs` (neuf), `src-tauri/linux/` (l'unité), tests, `README.md` ou la doc de l'amont où s'écrit l'installation, `docs/REMOTE.md`.

**Interfaces:** un verrou par fichier dans le dossier de données (verrou consultatif du système, lâché à la mort du process) à la place de `single-instance` ; `escouade-server install [--user]` écrit `~/.config/systemd/user/escouade-server.service` (chemin du binaire, port, `Restart=on-failure`, pas de jeton dans l'unité), dit quoi lancer ensuite (`systemctl --user enable --now …`, `loginctl enable-linger`) sans le lancer lui-même ; `escouade-server uninstall` ; arrêt propre sur `SIGTERM` (les agents s'arrêtent comme quand on quitte l'app).

- [ ] Tests d'abord : second serveur sur le même dossier refusé avec un message clair ; verrou d'un process tué repris ; l'unité écrite se relit par `systemd-analyze verify` si l'outil est dans l'image, sinon comparée à un texte attendu ; `install` deux fois ne casse rien ; `SIGTERM` pendant un terminal ouvert sort en moins de 10 secondes, code 0.
- [ ] Écrire ; fin de tâche ; commit `feat(remote): escouade-server en service systemd de l'utilisateur, un seul par dossier de données`.

### Task 4: Les dossiers du serveur et les projets

**Files:** `src-tauri/src/commands.rs` (commandes neuves par `#[command]`), un module du cœur pour le parcours, `src/lib/platform.ts`, `src/components/modals/NewProjectModal.svelte` et ce qui choisit un dossier, tests des deux côtés, `tests/web/`.

**Interfaces:** `fs_browse(path: Option<String>) -> Listing { path, parent, entries: [{ name, dir, git }] }` (dossiers seulement, sous le dossier de l'utilisateur, fichiers privés exclus, liens qui sortent non suivis) ; dans le navigateur, « Nouveau projet » montre ce parcours à la place du dialogue du système (un seul composant neuf, petit, utilisé derrière `platform.desktop`), et cloner un dépôt reste le cas courant.

- [ ] Tests d'abord (Rust) : la traversée refusée sous toutes ses formes ; un dossier illisible rendu comme tel sans erreur globale ; l'ordre et les marques `git`. (Vitest) : le composant liste, descend, remonte, choisit. (Chromium) : créer un projet en choisissant un dossier, puis en clonant un dépôt local.
- [ ] Écrire ; fin de tâche ; commit `feat(remote): choisir un dossier du serveur depuis le navigateur`.

### Task 5: Les fichiers — joindre, coller, récupérer

**Files:** `remoteapi/files.rs` (neuf : les routes HTTP d'octets), `remoteapi/server.rs`, `src-tauri/src/attachfiles.rs` et `sentfiles.rs` (les entrées sans la fenêtre), `src/lib/platform.ts` et les endroits du front qui joignent, collent, ouvrent ou montrent un fichier, tests, `tests/web/`.

**Interfaces:** `POST /files/attach?agent=…&name=…` (corps = les octets, taille maximale écrite dans une constante, écrit dans un fichier temporaire puis déplacé, rend l'identifiant que les commandes d'aujourd'hui attendent) ; de même pour le logo d'un projet et une image collée ; `GET /files/sent/<id>` rend un fichier envoyé par un agent en téléchargement (`Content-Disposition: attachment`, jamais affiché en ligne, type sûr) ; toutes exigent la session. Dans le navigateur : joindre = envoyer ; « Ouvrir » = télécharger ; « Afficher dans l'Explorateur » caché ; coller une image = l'envoyer.

- [ ] Tests d'abord : taille maximale tenue (rejet sans rien laisser) ; envoi coupé au milieu : aucun fichier partiel ; nom de fichier hostile neutralisé ; téléchargement d'un identifiant inconnu ou d'un autre agent refusé ; sans session : 401 ; un fichier téléchargé ne s'affiche jamais en ligne (en-têtes). (Chromium) : joindre un fichier à un message, le voir compté ; récupérer un fichier envoyé par un agent (faux harness) si le parcours du lot 4 en lançait un, sinon au niveau des routes.
- [ ] Écrire ; fin de tâche ; commit `feat(remote): joindre et récupérer des fichiers depuis le navigateur`.

### Task 6: Les notifications, la graine des plugins, la vue Plugins

**Files:** `src-tauri/src/headless_host.rs`, `src/lib/platform.ts` et l'endroit où le front reçoit `UiEvent`, `src-tauri/build.rs` ou le binaire pour la graine, `src/lib/plugins.ts` et les vues des plugins, tests.

**Interfaces:**
- Notifications : le `Host` sans fenêtre émet l'événement de toast (`UiEvent::Toast` existe : lire ce qu'il porte ; s'il ne convient pas, un événement neuf et le dire) ; le front, hors de la fenêtre, le rend par l'API de notification du navigateur après avoir demandé la permission au premier geste de l'utilisateur, et par un son ; un clic ramène l'onglet et émet le focus, comme le bureau.
- La graine : décidé par le demandeur, elle est **embarquée dans `escouade-server`**. Lire comment la release de l'amont la fabrique (`src-tauri/seed/README.md`, les scripts) ; le binaire l'embarque quand elle est construite et s'en passe sinon (comme l'app sans dossier `seed`) ; `Host::seed_dir` du serveur rend l'endroit où il l'a déposée (un dossier du dossier de données, mis à jour quand la version change). Mesurer ce que la graine ajoute au binaire et l'écrire.
- La vue Plugins et le store dans le navigateur (décidé par le demandeur) : les appels qui passaient hors de `ipc.ts` passent par le transport ; ce qui ouvrait un dialogue ou le système passe par la façade ; ce qui reste propre au bureau est caché par une condition. La liste des fonctions qui marchent et de celles qui restent au bureau va dans le compte rendu.

- [ ] Tests d'abord, écrire, fin de tâche ; un commit par sujet si les trois se relisent mieux à part : `feat(remote): les notifications rendues par le navigateur`, `feat(remote): la graine des plugins embarquée dans le serveur`, `feat(remote): la vue Plugins et le store dans le navigateur`.

### Task 7: Le parcours complet, la doc, la CI

**Files:** `tests/web/`, `.github/workflows/ci.yml`, `README.md` et le site ou la doc de l'amont où il le faut, `docs/REMOTE.md`.

- [ ] Le parcours du but 3 de la spec dans Chromium, sur `escouade-server` lancé dans l'image nue : s'appairer par le code ; ajouter un compte par le terminal du harness (un faux `claude` qui joue `/login` : adresse affichée, code collé) ; cloner un projet ; lancer un agent sur le faux harness ; converser ; répondre à une autorisation ; ouvrir un terminal ; utiliser le Kanban ; joindre un fichier ; fermer le navigateur, le rouvrir, tout retrouver ; révoquer l'appareil et constater la coupure. Trois passages de suite.
- [ ] La CI : le job sans bureau construit `escouade-server`, vérifie par `ldd` qu'il ne lie ni GTK ni WebKit, et joue le parcours si Chromium s'installe sans paquet de bureau ; sinon le dire sans faire semblant.
- [ ] La doc : l'installation sur un VPS (binaire, service, tunnel SSH, appairage), ce qui vit sur le serveur (les identifiants des comptes), ce qui manque encore (voix, mise à jour, mobile) ; `docs/REMOTE.md` « Tâches » mis à jour sans surpromettre.
- [ ] Fin de tâche ; commits `test(remote): le parcours complet du mode serveur dans Chromium`, `docs(remote): installer escouade-server et s'y appairer`.
