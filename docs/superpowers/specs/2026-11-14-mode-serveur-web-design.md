# Mode serveur avec interface web — design

Conception du 2026-11-14. Elle part de `docs/REMOTE.md` (l'étude du mainteneur) et la réduit à une première version utile, pensée pour être proposée à l'amont en petites PR. Ce qui est marqué **« mon choix »** a été tranché par le rédacteur, pas par le demandeur.

## Intention

Faire tourner Escouade sur un serveur Linux sans écran (un VPS), jour et nuit, et le piloter depuis un navigateur avec **l'interface officielle**, sans la réécrire. Aujourd'hui le cœur et l'interface vivent dans un seul process Tauri, sur le poste de l'utilisateur.

Pourquoi maintenant : le demandeur veut développer entièrement sur un VPS, et contribuer ce mode à Escouade en open source. Le résultat doit donc servir tous les utilisateurs, pas un seul déploiement.

## Décidé par le demandeur

- **Un serveur, un utilisateur, une seule vue.** Tous les navigateurs connectés voient le même Escouade.
- **L'interface officielle dans le navigateur**, et les changements d'interface de l'amont doivent arriver sans portage à la main.
- **La connexion des harness par leur terminal** (`/login` de Claude Code dans le terminal qu'ouvre déjà « Ajouter un compte »). Pas de jeton d'environnement.
- **Pas de workers** ni d'environnements d'exécution externes.
- **Contribution à l'amont** : généraliste, dans le sens de `docs/REMOTE.md`.
- Aucune issue n'est ouverte chez l'amont avant le code.

## Décidé par le demandeur après le lot 0 (2026-11-15)

Sur le compte rendu `docs/superpowers/specs/2026-11-14-mode-serveur-lot0-compte-rendu.md` :

- **La voie (A)** : des fonctionnalités Cargo sur la crate actuelle, le bureau derrière une fonctionnalité `desktop`. Le cœur n'est pas déplacé dans une crate à lui.
- **La recherche par le sens reste sur le serveur** : `ort` sort d'`escouade-voice-local` pour qu'`escouade-vectors` ne tire plus la voix (ALSA, libclang). C'est un petit lot à part, non chiffré.
- **La graine des plugins et sa liste de révocations sont embarquées dans `escouade-server`** ; la taille du binaire est à vérifier au plan du lot 5.
- **Le job de CI Linux sans GTK arrive tôt**, à la fin du lot 2, avec la séparation par fonctionnalités.
- La branche de travail est poussée sur le fork GitHub du demandeur ; rien n'est proposé à l'amont.

Après le lot 1 (2026-11-15) : le code est sur la branche `feat/coeur-sans-apphandle` du fork, en deux commits (`23028b0c`, `adfbb516`) ; la suite de tests avec Node donne le même résultat que l'amont, plus cinq tests neufs. Le demandeur a choisi de ne rien proposer à l'amont avant d'avoir essayé l'app de bureau sur une vraie machine, et de continuer par le lot 2 (`docs/superpowers/plans/2026-11-15-mode-serveur-lot2-api-distante.md`).

Là où la suite de ce document dit encore « à confirmer par le demandeur » ou présente deux options sur ces points, c'est cette section qui vaut.

## Buts

1. Sur un Linux sans affichage (x64 et arm64), un binaire `escouade-server` lance le cœur sans WebView, sans GTK, sans D-Bus.
2. Ce binaire sert l'interface web (le même code Svelte que l'app de bureau) et une API WebSocket, sur `127.0.0.1`.
3. Depuis un navigateur, on fait le parcours complet : s'appairer, ajouter un compte Claude Code par `/login`, créer un projet en clonant un dépôt, lancer un agent, converser, répondre à une autorisation, ouvrir un terminal, utiliser le Kanban.
4. Fermer le navigateur n'arrête rien. Le rouvrir rend l'état entier et l'écran des terminaux.
5. L'app de bureau ne change pas de comportement.

## Hors champ (première version)

- Le client de bureau qui se connecte à un serveur (« Se connecter à un serveur », `ssh -L` lancé par l'app).
- WireGuard intégré, Tailscale détecté, app mobile, notifications push, mise en page mobile.
- Plusieurs serveurs dans un même client (« espaces »), le serveur d'équipe, plusieurs personnes sur un serveur.
- L'état d'interface (`UiState`) par client : il reste unique et partagé (décidé : une seule vue).
- La voix, la mise à jour du serveur depuis l'interface, les pages détachées des lancements (`devpage`), le relais des ports des serveurs de test.
- Une image Docker.

## Ce qui ferait de ce travail un échec

- **Un port ouvert sans authentification.** Ce serveur lance des agents et des terminaux : il n'écoute que sur `127.0.0.1` et exige un jeton, même en boucle locale.
- **Une interface parallèle.** Aucun composant Svelte copié ou réécrit pour le web : le navigateur charge les mêmes composants.
- **Une seconde liste de commandes** à tenir à la main à côté de celle de Tauri.
- **Une régression de l'app de bureau**, ou des PR si larges que l'amont ne peut pas les relire.

## L'existant (main, a1410cd0)

- Front : tous les appels au cœur sont dans `src/lib/ipc.ts` (321 `invoke`). 16 fichiers importent `@tauri-apps/*`, dont 7 composants (dialogue de dossier, ouverture de fichier, barre de titre, fenêtre « Pages », vues des plugins).
- Cœur : 347 commandes Tauri dans `src-tauri/src/commands.rs` (228 écrites `#[tauri::command]`, 119 `#[tauri::command(async)]` ; compte rendu du lot 0, § 5). Elles prennent `State<Arc<Core>>` ; 7 prennent un `Channel` (1 pour les événements dans `subscribe`, 6 pour des sorties de terminal) ; 15 occurrences d'`AppHandle`, soit 14 commandes et un utilitaire (presse-papiers, enregistrer une image, quitter, mise à jour, `devpage`).
- `Core<R: Runtime>` garde un `AppHandle` (`core.rs:912`) pour les notifications, le tray, les menus et l'état de la voix. Les tests font déjà tourner le cœur sans WebView (`tauri::test::mock_app`) ; le lot 0 l'a démarré ainsi dans un conteneur Linux sans affichage.
- Deux crates du cœur tirent ce qu'on ne voit pas depuis `src-tauri` (compte rendu du lot 0, § 3) : `escouade-plugins` dépend de `tauri` pour un seul `tauri::async_runtime::spawn`, et `escouade-forge` et `escouade-updates` en héritent ; `escouade-vectors` (la recherche par le sens) dépend de `escouade-voice-local` pour `ort`, si bien qu'exclure la voix exclut aussi la recherche par le sens, sauf à déplacer `ort` (à trancher par le demandeur).
- `hub.rs` garde un seul `Channel<UiEvent>` : le dernier abonné reçoit tout.
- `src-tauri` est une seule crate, et environ 80 fichiers hors tests utilisent `tauri::`. Sous Linux, lier Tauri tire WebKitGTK et GTK.
- Les e2e actuels (environ 130 fichiers) pilotent la vraie app Tauri par les DevTools de WebView2 (`playwright.config.ts`).
- L'amont a une branche `release/2.3` en avance sur `main` (travail sur le serveur d'équipe, rien sur `Host`).
- Sous Linux, les secrets vont déjà dans un fichier lisible du seul propriétaire.

## Architecture

```
Navigateur ──(tunnel de l'utilisateur : ssh -L, Tailscale, son proxy)──▶ 127.0.0.1:<port>
                                                                          escouade-server
                                                                          ├─ fichiers de l'interface web (embarqués)
                                                                          ├─ API WebSocket : commandes, événements, flux des terminaux
                                                                          └─ le cœur (agents, Git, Kanban, PTY, serveur MCP)
```

- **Pas de Tauri à l'édition de liens du serveur.** `escouade-server` ne lie ni WebKitGTK ni GTK (`docs/REMOTE.md` : « sans WebKit ni GTK »). C'est mesuré : sous Linux, aucune combinaison des fonctionnalités de `tauri` 2.12 ne retire GTK ni WebKitGTK, même sur le runtime factice (compte rendu du lot 0, § 1) ; le cœur du serveur ne dépend donc ni de `tauri`, ni de `tauri-runtime`, ni de `muda`. Le compte rendu recommande la voie (A), des fonctionnalités Cargo sur la crate actuelle (le bureau derrière une fonctionnalité par défaut, le serveur construit sans, un job de CI Linux sans GTK qui le construit), plutôt que (B), le cœur sorti dans une crate à lui : **à confirmer par le demandeur**. Dans les deux voies, le cœur se passe d'abord de Tauri (lot 1). Le plan du lot 1 s'écrit après cette confirmation.
- **Le cœur sans fenêtre.** Ce que le cœur demande à l'app (notifier, attirer l'attention, tray, menus, version, redémarrer) passe par un trait `Host`. L'app de bureau l'implémente avec Tauri ; le serveur l'implémente sans rien afficher et pousse à la place un événement que le navigateur rend.
- **Une seule liste des commandes**, servie à la fois par l'IPC de Tauri et par la WebSocket. Le moyen (macro, génération, table) est au constructeur ; la contrainte est qu'ajouter une commande à un seul endroit la rende disponible des deux côtés, et qu'un test échoue sinon.
- **Les flux.** Les 7 `Channel` deviennent une sortie abstraite qui écrit soit dans un `Channel` de Tauri, soit dans la WebSocket. Les sorties de terminal restent binaires, multiplexées sur la même connexion.
- **Plusieurs abonnés.** `hub.rs` diffuse à tous les navigateurs connectés. Une réponse donnée dans l'un (autorisation, question) se ferme dans les autres.
- **L'état d'interface partagé.** `UiState` reste unique : un navigateur le lit au chargement, et le dernier qui l'écrit gagne. Naviguer dans un navigateur ne déplace pas les autres en direct (confirmé par le demandeur ; `docs/REMOTE.md` vise à terme un état par client).
- **La reprise.** À la connexion, le client reçoit `InitialState` puis les événements. Chaque terminal et chaque lancement garde un tampon de sortie, comme les vues d'agent aujourd'hui (4 Mo), pour redessiner l'écran.
- **Le front sur deux transports.** `ipc.ts` choisit Tauri ou la WebSocket selon l'endroit où il tourne. Les composants ne le savent pas. Un build web à part (configuration Vite) produit les fichiers que le serveur embarque.
- **Les commandes sans objet sur un serveur** (les 14 commandes à `AppHandle`, la voix) : l'interface web cache ce qui les appelle, et l'API les refuse avec une erreur claire. Prendre un `AppHandle` ne suffit pas à les reconnaître : `open_url`, `open_path` et `play_chime` supposent aussi un bureau sans en prendre (compte rendu du lot 0, § 3) ; sur un serveur, elles suivent la table « Ce qui change de machine » et, pour les liens de la vue Plugins, la décision juste en dessous ; `open_url` ailleurs reste à décider (compte rendu du lot 0, question 4).
- **Les plugins.** Les plugins livrés avec Escouade (les harness, les hébergeurs de code) tournent dans le cœur et marchent tels quels. La vue Plugins et le store (installer, mettre à jour) marchent aussi dans le navigateur dès cette version (décidé par le demandeur, 2026-11-14) : leurs appels à la fenêtre (dialogues, ouverture de liens, l'`invoke` de `src/lib/plugins.ts` hors de `ipc.ts`) passent par les deux transports ; c'est du travail en plus dans les lots 4 et 5.

### Sécurité

- Écoute sur `127.0.0.1` seulement. Pas d'option pour écouter ailleurs dans cette version (**mon choix**, conforme à `docs/REMOTE.md` : « jamais un port ouvert sur Internet »).
- `escouade-server pair` affiche une adresse à usage unique ; l'ouvrir dans le navigateur pose un jeton d'appareil. Les jetons se listent et se révoquent un par un (`escouade-server devices`).
- Contrôle de `Host` et d'`Origin` sur chaque requête et sur l'ouverture de la WebSocket, comme le fait déjà le serveur MCP. Par défaut, seuls `localhost` et `127.0.0.1` sont acceptés (le cas `ssh -L`). Une option `--origin https://nom` ajoute l'adresse d'un proxy ou de `tailscale serve` (confirmé par le demandeur) ; `pair` affiche alors cette adresse. Les notifications du navigateur demandent un contexte sûr : `localhost` ou https.
- Dans le lot 2, le serveur WebSocket de l'app de bureau ne s'allume que par une variable d'environnement qui porte aussi son jeton : il sert aux tests, pas aux utilisateurs (**mon choix**). L'appairage arrive au lot 5.
- L'interface dit que les identifiants des comptes vivent sur le serveur.

### Ce qui change de machine

| Quoi | Dans le navigateur |
| --- | --- |
| Choisir un dossier (« Nouveau projet ») | Un navigateur des dossiers du serveur, servi par le cœur ; cloner un dépôt est le cas courant |
| Fichiers joints, image collée, logo d'un projet | Envoyés au serveur |
| Fichiers envoyés par un agent | Téléchargés par le navigateur ; jamais ouverts sur le serveur |
| « Afficher dans l'Explorateur » | Caché |
| Notifications, son | L'API de notification du navigateur, à partir d'un événement du cœur |
| Connexion d'un compte | Le terminal du harness dans la page ; l'adresse de connexion s'ouvre dans le navigateur de l'utilisateur, le code se colle dans le terminal |
| Barre de titre, tray, menus du système | Absents ; l'en-tête de la page en tient lieu |

### Installation

Un binaire et un service systemd de l'utilisateur (`loginctl enable-linger`), posé par `escouade-server install`. Les données restent dans `~/.escouade`. Un verrou par fichier remplace `single-instance`, qui passe à l'exécution par le bus de session D-Bus (par `zbus`, en Rust pur, sans bibliothèque à la construction). La bibliothèque D-Bus que lie l'app vient de `tao`, par Tauri, et part avec lui (compte rendu du lot 0, § 3).

## Les lots

Chaque lot est une PR proposable à l'amont, relisible seule, et a son propre plan. Les lots 1 à 4 ne changent rien de visible dans l'app de bureau.

| Lot | Livre | Preuve |
| --- | --- | --- |
| 0. Essais | Le cœur lancé dans un process sans fenêtre, dans un conteneur Linux sans affichage ; la liste de ce qui suppose un bureau (D-Bus, cpal, GTK du presse-papiers) | Un compte rendu (`2026-11-14-mode-serveur-lot0-compte-rendu.md`), pas de code gardé |
| 1. `Host` | Le trait à la place de l'`AppHandle` du cœur ; le paramètre `R: Runtime` retiré du cœur ; un runtime à soi à la place de `tauri::async_runtime` (ce qui retire `tauri` d'`escouade-plugins`) ; les tests sur un faux `Host` ; aussi ce que `Core::load` demande encore à l'`AppHandle` (version, origines des fichiers, graine des plugins lue dans le dossier de ressources de Tauri) et les types de bureau que garde le cœur (compte rendu, § 5 et question 5). Environ 105 fichiers, 1 000 à 1 200 lignes touchées, dont environ 750 mécaniques (estimation du compte rendu, § 6) | Les tests existants passent, l'app de bureau est inchangée |
| 2. API distante | La liste unique des commandes, la sortie abstraite des flux, le serveur WebSocket (dans l'app de bureau, éteint par défaut) | Un test appelle une commande et reçoit un événement par la WebSocket |
| 3. Plusieurs abonnés et reprise | `hub.rs` à plusieurs abonnés, les tampons des terminaux et des lancements | Deux clients reçoivent le même événement ; un client reconnecté retrouve l'écran d'un terminal |
| 4. Front sur deux transports | `ipc.ts` sur Tauri ou WebSocket, le build web | Un parcours court (ouvrir un projet, lancer un agent sur un faux harness, répondre, ouvrir un terminal) passe dans Chromium sur la WebSocket, sous Linux ; pas toute la suite e2e (**mon choix**) |
| 5. `escouade-server` | Le binaire, l'appairage, le service, et la table « Ce qui change de machine » | Le parcours du but 3, dans un e2e sur un conteneur sans affichage |

La séparation par fonctionnalités Cargo de la voie (A) et son job de CI Linux sans GTK (125 à 205 lignes touchées, estimation du compte rendu, § 5, hors voix locale et recherche par le sens, non chiffré (question 3)) vont **soit à la fin du lot 2** (plus tôt, et protège plus tôt contre le retour de Tauri dans le cœur), **soit au début du lot 5** (avec le binaire) : à trancher par le demandeur.

Le premier plan couvre le lot 0 seul (confirmé par le demandeur) ; celui du lot 1 dépend de son compte rendu. Les PR partent de `upstream/main`, après avoir regardé où en est `release/2.3`.

## Laissé au constructeur

- La forme de la liste unique des commandes et le format des messages de la WebSocket.
- La bibliothèque du serveur HTTP et WebSocket (une déjà présente dans le dépôt est préférée).
- Le découpage du lot 5 s'il s'avère trop gros pour une PR.
- Les conventions du dépôt (titres de commits et de PR en français, style `feat(zone): …`, pages de `.escouade/spec` à tenir à jour) se lisent dans l'historique.

## Risques connus

- L'amont annonce ce chantier pour la 2.3 et pousse tous les jours : chaque lot se rebase sur `upstream/main` avant d'être proposé, et reste petit.
- La connexion d'un harness sans navigateur sur le serveur n'est essayée que pour Claude Code : son `/login` a marché dans un conteneur sans navigateur, et Escouade y lirait « connecté, jeton en date » (compte rendu du lot 0, § 4). Pas essayés : les autres harness, un vrai VPS (seulement arm64, dans Docker sur un Mac), le renouvellement du jeton dans le temps.
- Le mainteneur peut préférer un autre découpage : les lots 1 à 4 suivent son ordre pour limiter ce risque.
