# Mode serveur, lot 0 : le compte rendu des essais

Essais du 2026-11-14, pour la conception `2026-11-14-mode-serveur-web-design.md` et son plan `docs/superpowers/plans/2026-11-14-mode-serveur-lot0-essais.md`. **Aucun code de l'app n'est gardé** : le code d'essai et les sorties brutes sont sur la branche locale `spike/lot0-sans-fenetre` (dernier commit `1085d223`, jamais poussée ni proposée), dans `spikes/lot0/results/`. Ce document en cite les lignes utiles ; les fichiers entiers y restent.

**Comment lire les sources.** Chaque affirmation dit d'où elle vient :

- **[mesuré]** : une commande lancée pour ces essais, sortie gardée dans `spikes/lot0/results/` ;
- **[vérifié]** : lu dans le code (celui de l'app au fichier et à la ligne donnés, ou un manifeste du registre cargo), rien n'a été lancé ;
- **[lu]** : une documentation, un commentaire ou `Cargo.lock`, sans le code qui le fait.

Les chiffrages sont marqués **(estimation)**. Tout a tourné dans un conteneur Docker Linux aarch64 (Debian bookworm, Rust 1.98.0) sur un Mac ; l'« image nue » n'a ni GTK, ni WebKit, ni X11, ni D-Bus, ni ALSA (`dpkg -l` n'en trouve aucun, `bare-image-packages.txt`) **[mesuré]**.

## En bref

1. **Voie recommandée : (A)**, des fonctionnalités Cargo sur la crate actuelle (le bureau derrière une fonctionnalité `desktop`), plutôt que (B), le cœur déplacé dans une crate à lui ; à confirmer par le demandeur.
2. `escouade-server` ne peut pas dépendre de `tauri` : sous Linux, aucune combinaison des fonctionnalités de `tauri` 2.12 ne retire GTK ni WebKitGTK, même sur le runtime factice **[mesuré]** **[vérifié]**.
3. Le cœur démarre sans affichage ni bus de session dans un conteneur arm64 (vide : 0 projet, 0 compte, 20 s), mais seulement si les bibliothèques de bureau sont installées pour l'édition de liens **[mesuré]**.
4. Une image sans ces bibliothèques demande de sortir du binaire Tauri **et** la voix locale (ALSA, libclang), que la recherche par le sens tire aussi **[mesuré]** **[vérifié]**.
5. Lot 1 (`Host`, sans `R: Runtime` ni runtime de Tauri) : environ 105 fichiers, 1 000 à 1 200 lignes touchées (estimation) ; la connexion `/login` de Claude Code a marché dans un conteneur sans navigateur, et Escouade y lirait « connecté, jeton en date » **[mesuré]** (arm64, Docker sur un Mac ; parcours non décrit pas à pas ; ni les autres harness ni le renouvellement du jeton ; § 4).

## 1. Ce que Tauri lie sous Linux

Source : `results/tauri-link.txt`.

- **Les manifestes** : `tauri` 2.12.0 dépend de `gtk` et de `muda` (avec `gtk3`) sans condition sous Linux ; `webkit2gtk` n'y est activé que par la fonctionnalité `wry`. Mais `tauri-runtime` dépend sans condition de `gtk` **et** de `webkit2gtk` sous Linux, et n'a pas de fonctionnalité qui les retire **[vérifié]**. Le manifeste de `tauri-runtime` 2.12.0 a été lu dans le registre ; celui de 2.12.2 (la version que résout la crate d'essai) seulement dans le conteneur **[vérifié]**. La conclusion vaut donc **quelles que soient les fonctionnalités de `tauri` 2.12**, pas au-delà.
- **La crate minimale** (`spikes/lot0/tauri-link` : `tauri = "=2.12.0"`, `default-features = false`, `features = ["test"]`, un appel à `tauri::test::mock_app()`) : `wry` et `tauri-runtime-wry` partent bien de l'arbre, `gtk` et `webkit2gtk` restent **[mesuré]**.
- **Dans l'image nue**, sa construction échoue sur `glib-sys` (`glib-2.0` introuvable par pkg-config) **[mesuré]**.
- **Le témoin**, avec les bibliothèques de la CI : le binaire se construit, `mock_app()` tourne sans `DISPLAY` ni D-Bus (`exit=0`), et le binaire demande directement libgtk-3, libwebkit2gtk-4.1, libjavascriptcoregtk-4.1, libsoup-3.0 ; 168 entrées en fermeture transitive (X11, Wayland, GL, GStreamer, ALSA, libdbus…) **[mesuré]**. Copié dans l'image nue, il ne se charge pas (`libgobject-2.0.so.0` manquante, `exit=127`) **[mesuré]**.

**Ce que ça veut dire** : le problème n'est pas l'exécution mais l'édition de liens. Le cœur dont se sert `escouade-server` ne doit dépendre ni de `tauri`, ni de `tauri-runtime`, ni de `muda` sous Linux. Pas de voie « (C) » qui garderait `tauri` lié sans fenêtre. **Non essayé** : un `[patch]` de `tauri` / `tauri-runtime` qui rendrait `gtk` facultatif.

## 2. Le cœur dans un process sans fenêtre

Source : `results/core-headless.txt` et `results/core-headless.log`.

- **Le binaire d'essai** (`src-tauri/src/bin/escouade-headless-spike.rs` sur la branche d'essai) : un dossier de données neuf, `mcp_enabled = true`, `tauri::test::mock_app()`, `Core::load` puis `Core::start`, 20 s d'attente, `exit(0)` **[vérifié]**.
- **Dans l'image avec les bibliothèques de bureau, sans `DISPLAY`, sans Wayland, sans bus de session** : le cœur monte, le serveur MCP écoute sur `127.0.0.1` seulement et répond 401 sans jeton sur `/` et `/mcp`, le store se télécharge et s'écrit, le process sort avec 0 ; deux lancements, mêmes lignes **[mesuré]**.
- **Le journal entier** tient en cinq lignes : la langue, un seul `WARN` (le trousseau absent : le jeton MCP va dans un fichier du seul propriétaire), l'écoute MCP, les certificats, la connexion au store. Aucun `ERROR`, aucune ligne d'affichage, de D-Bus, de GTK, d'ALSA **[mesuré]**. Le `WARN` est celui de tout Linux, pas du conteneur (`NoKeychain`, `crates/escouade-forge/src/integrations/secrets.rs:99-119`) **[vérifié]**.
- **Le prix dans le code : 2 lignes** — la fonctionnalité `test` ajoutée au `tauri` des dépendances normales, et `mod core;` devenu `pub mod core;` (`src-tauri/src/lib.rs:43`) **[mesuré, `git diff`]**. **C'est un raccourci d'essai, pas la cible** : le cœur a tourné sur le runtime factice de Tauri ; une vraie version ne met pas `test` en production et ne lie plus `tauri` du tout (§ 1).
- **Dans l'image nue**, le binaire ne se construit pas : 13 scripts de construction `*-sys` échouent (`cargo build --keep-going`, `results/core-headless-bare-build.log`) **[mesuré]** :

| Famille | Crates en échec **[mesuré]** | Tirées par **[lu]** |
| --- | --- | --- |
| GTK, GLib, WebKit | `glib-sys`, `gobject-sys`, `gio-sys`, `gdk-sys`, `gdk-pixbuf-sys`, `atk-sys`, `cairo-sys-rs`, `pango-sys`, `javascriptcore-rs-sys`, `soup3-sys` | `tauri`, `tauri-runtime`, `wry` |
| D-Bus | `libdbus-sys` | `dbus` ← `tao` ← `tauri-runtime-wry` ← `tauri` |
| ALSA | `alsa-sys` | `alsa` ← `cpal` ← `escouade-voice-local` ← `escouade` |
| libclang | `whisper-rs-sys` | `whisper-rs` ← `escouade-voice-local` ← `escouade` |

Les chaînes de la troisième colonne ont été relevées avec `cargo tree -i` sans que la sortie soit gardée : elles sont **[lu]**, à refaire si on les cite ailleurs.

**Ce que l'essai ne dit pas** : un cœur avec des projets et un compte, un agent lancé, une durée au-delà de 20 s, x86_64. Les boucles de fond n'ont rien journalisé parce qu'elles n'avaient rien à faire **[mesuré]**.

## 3. Ce qui suppose un bureau

Source : `results/desktop-assumptions.md` (32 lignes, avec fichiers et lignes ; ci-dessous, regroupées). Seul le démarrage a été mesuré (tâche 3) ; ce qui n'est atteint qu'en appelant une commande est **[vérifié]** dans le code et non mesuré.

| Point | Construction sur un Linux sans bureau | Exécution sans bureau | Dans la conception |
| --- | --- | --- | --- |
| `tauri` et ses plugins (fenêtre, `window-state`, `deep-link`, `process`, `updater`, surveillance du fil de la fenêtre) | GTK, WebKitGTK, libdbus par `tao`, aucune porte **[mesuré]** | Avec `mock_app()`, rien d'affiché ; les bibliothèques doivent être là pour le chargeur **[mesuré]** | Exclus par « Pas de Tauri à l'édition de liens » ; l'`AppHandle` du cœur devient `Host` |
| `escouade-plugins` → `tauri`, pour un seul `tauri::async_runtime::spawn` (`crates/escouade-plugins/src/runtime.rs:1316`) ; `escouade-forge` et `escouade-updates` en héritent | Tout GTK/WebKit, même si l'app ne liait plus `tauri` **[vérifié]** | Rien de propre au bureau : un `spawn` sur tokio **[vérifié]** | Non prévu |
| La voix locale : `cpal`, `whisper-rs` | ALSA ; libclang et cmake **[mesuré]** ; aucune porte | Un périphérique ALSA à l'usage **[vérifié]** | Hors champ |
| `escouade-vectors` → `escouade-voice-local`, pour `ort` (`crates/escouade-vectors/Cargo.toml:15`) | ALSA et libclang suivent la recherche par le sens **[vérifié]** | Rien de bureau pour les vecteurs **[vérifié]** | Non prévu : la recherche par le sens n'est pas hors champ |
| `single-instance` | `zbus`, en Rust pur, aucune bibliothèque **[lu, `Cargo.lock`]** | Un bus de session ; sans bus, comportement **déduit, non mesuré** (source du plugin hors du dépôt) | « Un verrou par fichier » |
| Dialogues du système (`tauri-plugin-dialog` → `rfd`), presse-papiers GTK (`clipboard.rs`), pages des lancements (`devpage.rs`) | GTK, WebKit **[lu, `Cargo.lock`]** **[vérifié]** | Un affichage. Pour le presse-papiers sans affichage, l'échec est **déduit, non mesuré** **[vérifié]** | Dossier et logo par le serveur ; image collée envoyée ; `devpage` hors champ |
| Ouvrir dehors (`open_url`, `open_path`, `reveal`), son d'alerte (`play_chime`, `paplay`) | Rien de système **[lu]** | `xdg-open`, un navigateur, PulseAudio ; l'échec du son est ignoré **[vérifié]** | Fichiers téléchargés, « Afficher » caché, son par le navigateur ; `open_url` en général non prévu |
| Notifications, attention, tray, menu, version de l'app, canal des événements | Rien sous Linux en plus de `tauri` **[vérifié]** | Sans fenêtre ni tray, sans effet **[mesuré]** **[vérifié]** | Le trait `Host` ; `hub.rs` à plusieurs abonnés |
| Trousseau (jeton MCP, intégrations), connexion d'Antigravity par `secret-tool`, corbeille | Rien sous Linux **[vérifié]** **[lu]** | Fichier du seul propriétaire ; `secret-tool` absent rend « on ne sait pas » ; corbeille freedesktop sans bureau **[vérifié]** **[lu]** | Le fichier est prévu ; Antigravity et corbeille non prévus |

**Les commandes qui supposent un bureau** : `commands.rs` a 15 occurrences d'`AppHandle`, soit **14 commandes et un utilitaire** (`own_of`) : `clipboard_image`, `save_image`, `quit_app`, `update_check`, `update_restart`, `update_postpone` et les huit `devpage_*` **[vérifié]**. **`open_url`, `open_path` et `play_chime` supposent aussi un bureau sans prendre d'`AppHandle`** **[vérifié]** : une règle « les commandes à `AppHandle` » ne les attrape pas.

**Ce que l'essai a appris et que la conception ne savait pas** :

1. **La bibliothèque D-Bus du binaire vient de Tauri** (`libdbus-sys` ← `dbus` ← `tao`) **[mesuré]**, pas de `single-instance`, qui passe par `zbus` **[lu, `Cargo.lock`]**. Le verrou par fichier règle l'exécution ; la construction se règle en sortant Tauri.
2. **Rien de la liste n'est derrière une fonctionnalité Cargo** ; seules des portes de cible retirent quelque chose, et jamais sous Linux **[vérifié]**.
3. **Trois crates du cœur ont `tauri` dans leur arbre** (`escouade-plugins`, et par elle `escouade-forge`, `escouade-updates`), et **la recherche par le sens a la voix dans le sien** **[vérifié]**.

## 4. La connexion de Claude Code sans navigateur sur place

Source : `results/claude-login.md`.

**Ce qu'Escouade lit**, sous Linux **[vérifié]** :

- **Le fichier** : `<dossier du compte>/.credentials.json`, rien d'autre ; le trousseau n'est demandé que sous macOS (`src-tauri/src/usage.rs:135-139`, `crates/escouade-engines/src/signin.rs:93-103`). Le dossier est le `CLAUDE_CONFIG_DIR` du compte, sinon (Principal) celui de l'app, sinon `~/.claude` (`src-tauri/src/accounts.rs:713-721`) ; `CLAUDE_SECURESTORAGE_CONFIG_DIR`, s'il est défini, passe avant tout (`usage.rs:185-194`).
- **Les champs** (`stored_of`, `signin.rs:61-75`) : `claudeAiOauth.accessToken` absent ou vide, c'est « non connecté » ; `claudeAiOauth.expiresAt`, un entier en millisecondes ; `claudeAiOauth.refreshToken` n'entre que dans l'empreinte de la connexion.
- **Expiré** : `expiresAt` présent, entier, avant maintenant (`signin.rs:66`). Sans lui, le jeton est tenu en date. Expiré ne veut pas dire déconnecté : Claude Code en reprend un en tournant (`signin.rs:16-22, 36-37`). Fichier absent : « non connecté », sans erreur (`signin.rs:98`).
- Escouade ne garde ni n'utilise le jeton (`signin.rs:33-34`).

**Ce qui a tourné** :

- L'image `escouade-lot0-claude` (l'image nue, `jq`, Claude Code par `curl -fsSL https://claude.ai/install.sh | bash`) se construit ; `claude --version` y répond `2.1.325 (Claude Code)` sur aarch64, sans `DISPLAY`, sans `node`, sans `xdg-open`, sans navigateur **[vérifié]**. La documentation dit que Claude Code est un binaire natif, sans Node **[lu, code.claude.com/docs/en/setup, relu le 2026-11-14]**.
- Sur un volume de compte vide, `spikes/lot0/login-check.sh` répond : fichier absent, aucun champ, et `claude -p` dit `Not logged in · Please run /login` (code 1) **[mesuré]**.

### Le parcours `/login`

Lancé par le demandeur le 2026-11-15, dans son terminal : `spikes/lot0/login.sh`, puis `/login`, le navigateur étant sur son propre poste, puis `spikes/lot0/login-check.sh`, qui n'affiche que les **noms** des champs (sa sortie est dans `results/claude-login.md`) **[mesuré]** :

- la connexion a marché, dans le conteneur sans affichage ni navigateur ;
- `/claude-account/.credentials.json` existe, avec `claudeAiOauth.accessToken`, `claudeAiOauth.expiresAt` et `claudeAiOauth.refreshToken`, les champs qu'attend Escouade ;
- la règle de `stored_of`, rejouée en jq par `login-check.sh`, en tire « connecté, jeton en date » : Escouade verrait le compte connecté ;
- `claude -p "réponds OK"` répond `OK`.

Non observé : le parcours de `/login` pas à pas (le demandeur ne l'a pas décrit, et n'a signalé aucun écueil) ; une autre machine que Docker sur un Mac, en arm64 ; les autres harness ; le renouvellement du jeton dans le temps.

## 5. Les voies de séparation

Source : `results/tauri-uses.md` (le script `spikes/lot0/tauri-uses.sh`, lancé sur `a1410cd0`, la base de la conception).

**Le compte** : 79 fichiers hors tests nomment Tauri, pour 1 143 occurrences **[mesuré]**. Le lien du cœur est étroit : l'`AppHandle` de `Core` (`core.rs:912`) sert à 11 lignes dans 3 fichiers (version, attention, toast, tray, état de la voix, fenêtre visible, crochet de mise à jour) **[mesuré]**. Ce compte ne cherche que `self.app` et `core.app`, et en oublie : `Core::load(app: AppHandle<R>, …)` (`core.rs:1866`) se sert encore du handle trois fois, pour la version (`app.package_info()`, `core.rs:1999`), la table des origines de fichiers (`app.try_state::<crate::fileorigin::FileOrigins>()`, `core.rs:2006`) et la graine des plugins de l'installateur avec sa liste de révocations (`Seed::of_app(app.path().resource_dir()…)`, `core.rs:2014-2015`) ; et le cœur garde trois types de modules de bureau : `Core.page_crash: Mutex<Option<crate::pagecrash::PageCrash>>` (`core.rs:1001` ; `pagecrash.rs` utilise `tauri` et `webkit2gtk`), `Core.file_origins: crate::fileorigin::FileOrigins` (`core.rs:1087` ; `fileorigin.rs` utilise `tauri`), et `crate::devpage::PageEvent` dans `model.rs:287` **[vérifié, à `a1410cd0`]**. Le trait `Host` tel que listé ici ne les couvre pas ; le travail en plus est petit (quelques dizaines de lignes, estimation), mais le serveur n'a pas de dossier de ressources de Tauri d'où lire la graine (question 5). Le gros est mécanique : le paramètre `R: Runtime` (506 lignes brutes dans 75 fichiers, dont 4 où `<R>` n'est pas celui de Tauri) et `tauri::async_runtime` (89 lignes dans 28 fichiers) **[mesuré]**. `commands.rs` porte **347** commandes : 228 écrites `#[tauri::command]` et 119 `#[tauri::command(async)]`, à `a1410cd0` comme à `ca45bbd6` (`git grep -c -F '#[tauri::command]' <commit> -- src-tauri/src/commands.rs`, et de même avec `(async)`) **[vérifié]** ; `generate_handler!` en liste 347 **[mesuré]**.

**Ce qui est commun aux deux voies, et qui est le lot 1** : un trait `Host` à la place de `pub app: AppHandle<R>` ; le paramètre `R` retiré ; un module de runtime à soi qui garde, comme celui de Tauri, un runtime tokio global (`spawn` marche hors d'un runtime, ce que `tokio::spawn` ne fait pas) **[vérifié]**, donné à Tauri au démarrage par `tauri::async_runtime::set` ; les tests sur un faux `Host`. Sans ces quatre points, aucune voie ne sort le cœur de Tauri. Les voies ne diffèrent que par la façon dont le build du serveur laisse de côté l'app de bureau (13 fichiers triés à la main : `lib.rs`, `commands.rs`, `menus.rs`, `notify.rs`, `navigation.rs`, `pagecrash.rs`, `devpage.rs`, `devpage_probe.rs`, `fileorigin.rs`, `windowthread.rs`, `clipboard.rs`, `deeplink.rs`, `updates.rs`) **[vérifié]**.

| | (A) Fonctionnalités Cargo sur `src-tauri` | (B) Le cœur dans `crates/escouade-core` |
| --- | --- | --- |
| Ce qui bouge en plus du lot 1 | `tauri`, les plugins, `gtk`, `webkit2gtk` en `optional`, une fonctionnalité `desktop` par défaut, des `cfg` ; environ 10 à 18 fichiers, 125 à 205 lignes touchées (estimation, détail ci-dessous) | Environ 330 fichiers et 266 000 lignes déplacés (174 fichiers et 144 180 lignes hors tests, 156 et 122 550 de tests **[mesuré]**, comptés avec le groupe de 9 fichiers du script, d'où « environ » avec les 13 triés à la main) |
| Ce que l'amont relit | Un lot 1 large mais répétitif, refaisable par script à chaque rebase ; puis une PR courte | Un déplacement que Git ne voit comme renommage que pour les fichiers peu touchés ; 983 lignes `pub(crate)` dans 99 fichiers à relire **[mesuré]** |
| Conflits (`upstream/main`, 30 jours : 393 commits sur `src-tauri/src` **[mesuré]**) | Petits mais presque sûrs sur `core.rs` et `lib.rs` (estimation) | Tous les commits de l'amont tombent sur des fichiers déplacés, `release/2.3` comprise |
| Prix propre | Les fonctionnalités s'unifient dans le workspace : le serveur se construit à part, et il faut **un job de CI Linux sans GTK** qui le construit, sinon un `use tauri::` hors `cfg` revient sans bruit | Les règles de `docs/CRATES.md` § 7 à cette taille (`cfg(test)` d'une dépendance, `CARGO_PKG_VERSION`, `tr!`, fixtures, `include_str!`) |
| `docs/CRATES.md` | Neutre ; (B) reste possible ensuite | Même direction, mais dix fois la plus grosse extraction faite, et le cycle de modules déplacé entier plutôt que coupé **[lu]** |

**Le chiffrage de (A), et ce que « lignes » veut dire.** Ici, une **ligne touchée** est une ligne ajoutée, retirée ou modifiée, comptée une fois ; dans un diff unifié, une ligne modifiée compte deux (− et +). Le fichier de la tâche 5 annonçait « environ 20 fichiers, 100 à 150 lignes » ; sa propre liste fait moins, et en oublie. La fourchette ci-dessus en vient :

1. La liste de la tâche 5 : `src-tauri/Cargo.toml` et `build.rs` (25), `lib.rs` (20), `telemetry.rs` et `outside.rs` (10) : **environ 55 lignes dans 5 fichiers** (estimation). Elle compte aussi 2 lignes pour retirer `tauri` de `crates/escouade-plugins/Cargo.toml`, laissées de côté ici : le lot 1 le fait déjà.
2. Ce qu'elle oublie : un binaire `escouade-server` minimal que la CI construit (20 à 40 lignes, 1 à 2 fichiers) ; le job de CI sans GTK (30 à 50 lignes, 1 fichier) ; les `cfg` dans les fichiers du cœur qui appellent l'app de bureau (20 à 60 lignes, 3 à 10 fichiers) (estimation). `hub.rs` va au lot 2 et n'est pas compté ici.
3. Total : **125 à 205 lignes touchées dans 10 à 18 fichiers**, soit jusqu'à environ 400 lignes de diff si toutes étaient modifiées (estimation).
4. **Non chiffré** : rendre facultatives `escouade-voice-local` et, selon la réponse à la question 3 ci-dessous, la recherche par le sens. La tâche 5 ne les compte pas ; la tâche 4 montre qu'il le faut pour une image sans ALSA ni libclang.

Les numéros de ligne de `src-tauri/Cargo.toml` donnés par `tauri-uses.md` sont décalés d'une ligne par endroits ; au commit `ca45bbd6` de la branche d'essai (les commits suivants n'y touchent pas) : `tauri` ligne 29, les `tauri-plugin-*` lignes 30 à 35 et 38, `escouade-voice-local` 92, `gtk` 135, `webkit2gtk` 137, `tauri-plugin-notification` 140 **[vérifié]**.

## 6. La recommandation et la taille du lot 1

**La voie (A)**, à confirmer par le demandeur, pour trois raisons :

1. Le travail qui sort le cœur de Tauri est le même dans les deux voies ; (A) y ajoute 125 à 205 lignes (estimation, hors voix locale et recherche par le sens, non chiffré (question 3)), (B) un déplacement d'environ 266 000 lignes **[mesuré]**.
2. L'amont touche `src-tauri/src` 393 fois par mois **[mesuré]** : déplacer toute la crate est la PR que la conception veut éviter (« des PR si larges que l'amont ne peut pas les relire »).
3. (A) ne ferme pas (B) : une fois le cœur sans Tauri, le sortir dans une crate reste possible.

Son prix : le job de CI Linux sans GTK.

**La taille du lot 1** (le trait `Host` livré seul, l'app de bureau inchangée), la même sous (A) et (B) :

| Morceau | Fichiers | Lignes touchées |
| --- | --- | --- |
| Le paramètre `R` retiré | 75, dont 4 faux positifs **[mesuré]** | ≈ 500 modifiées, mécaniques **[mesuré]** |
| Le runtime à soi (dont le seul `spawn` d'`escouade-plugins`, ce qui y retire `tauri`) | 28 + 2 de tests **[mesuré]**, + 1 module neuf | 91 modifiées **[mesuré]**, ≈ 50 neuves (estimation) |
| `Host` et ses sites | 3 du cœur **[mesuré]**, + `lib.rs`, `notify.rs` (coupé en deux), `menus.rs` | 11 sites **[mesuré]**, + 3 dans `Core::load` et 3 types de bureau gardés par le cœur **[vérifié]** (§ 5), ≈ 200 neuves et quelques dizaines pour ceux-là (estimation) |
| Les tests sur un faux `Host` | 22 **[mesuré]** | 116 modifiées **[mesuré]**, ≈ 50 neuves (estimation) |
| **Total** | **≈ 105** (83 hors tests et 22 de tests, union mesurée) | **≈ 1 000 à 1 200 lignes touchées**, dont ≈ 750 mécaniques, soit ≈ 1 700 à 2 100 lignes de diff (estimation) |

L'union mesurée hors tests est de 682 lignes brutes dans 83 fichiers **[mesuré]**. Le fichier de la tâche 5 (`results/tauri-uses.md`) appelle ces 1 000 à 1 200 lignes « lignes de diff », mais ce sont des lignes touchées. La séparation par fonctionnalités et son job de CI ne sont pas dans le lot 1 : elles vont à la fin du lot 2 (plus tôt, et protège plus tôt contre le retour de Tauri dans le cœur) ou au début du lot 5 (avec le binaire). Le code ne tranche pas ; c'est la question 2.

## 7. Ce qui n'a pas été essayé

- **x86_64** : tout a tourné sur aarch64.
- **Un vrai VPS** : un conteneur Docker sur un Mac, pas une machine Linux sans bureau avec systemd ; la connexion de Claude Code comprise.
- **Le renouvellement du jeton de Claude Code dans le temps** : la connexion a été vue en date, pas expirée puis reprise (§ 4).
- **Le parcours de `/login` pas à pas** : le demandeur ne l'a pas décrit, et n'a signalé aucun écueil (§ 4).
- **Les autres harness** (Codex, Gemini, Copilot…) : seule la connexion de Claude Code a été essayée (§ 4).
- **Un cœur au travail** : avec des projets, un compte, un agent lancé, au-delà de 20 s ; et le cœur sur un runtime à soi plutôt que sur le runtime factice de Tauri.
- **Déduit, non mesuré** : `single-instance` sans bus de session, le presse-papiers GTK sans affichage ; non essayé : un `[patch]` de `tauri` rendant `gtk` facultatif.

## 8. Ce que ce compte rendu change à la conception

Corrigé dans `2026-11-14-mode-serveur-web-design.md`, sans toucher à ce que le demandeur a décidé : 347 commandes au lieu de 228 ; les commandes sans objet sur un serveur ne se reconnaissent pas à leur `AppHandle` ; « Pas de Tauri à l'édition de liens » devient un fait mesuré, la voie (A) y est donnée comme recommandée, à confirmer ; la bibliothèque D-Bus vient de `tao`, `single-instance` passe par `zbus` ; les deux chaînes `escouade-plugins` → `tauri` et `escouade-vectors` → `escouade-voice-local` ; le lot 1 décrit et chiffré, la place de la séparation laissée ouverte.

## Questions ouvertes pour le demandeur

1. **La voie** : (A), des fonctionnalités Cargo sur la crate actuelle (recommandée), ou (B), le cœur dans une crate à lui ?
2. **Où vont la séparation par fonctionnalités et le job de CI sans GTK** : à la fin du lot 2, ou au début du lot 5 ?
3. **La recherche par le sens quand la voix est exclue** : déplacer `ort` hors d'`escouade-voice-local` pour que `escouade-vectors` ne tire plus ALSA ni libclang, ou laisser la recherche par le sens hors du serveur dans cette version ?
4. **Ce que la conception ne prévoit pas** (tâche 4, colonne « non prévu ») : sortir `tauri` d'`escouade-plugins` (un `spawn`, couvert par le lot 1 ci-dessus) ; ce qui tient lieu, sur un serveur, de ce qui n'a de sens qu'avec une fenêtre (taille et place de la fenêtre, surveillance de son fil, nom du programme, schéma `escouade://`, dialogue « recharger une page figée ») ; `open_url` hors de la vue Plugins ; la connexion d'Antigravity lue par `secret-tool` ; la corbeille, qui marche a priori telle quelle.
5. **La graine des plugins sur le serveur** : d'où `escouade-server` tient-il la graine des plugins et sa liste de révocations, que l'app de bureau lit dans le dossier de ressources de Tauri (`core.rs:2014-2015`) ?
