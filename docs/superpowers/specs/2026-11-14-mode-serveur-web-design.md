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
- Cœur : 228 `#[tauri::command]` dans `src-tauri/src/commands.rs`. Ils prennent `State<Arc<Core>>` ; 7 prennent un `Channel` (1 pour les événements dans `subscribe`, 6 pour des sorties de terminal) ; 15 prennent un `AppHandle` (presse-papiers, enregistrer une image, quitter, mise à jour, `devpage`).
- `Core<R: Runtime>` garde un `AppHandle` (`core.rs:912`) pour les notifications, le tray, les menus et l'état de la voix. Les tests font déjà tourner le cœur sans WebView (`tauri::test::mock_app`).
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

- **Pas de Tauri à l'édition de liens du serveur.** `escouade-server` ne lie ni WebKitGTK ni GTK (`docs/REMOTE.md` : « sans WebKit ni GTK »). Le lot 0 dit par quelle voie : des fonctionnalités Cargo sur la crate actuelle, ou le cœur sorti dans une crate à lui. Le plan du lot 1 s'écrit après ce compte rendu.
- **Le cœur sans fenêtre.** Ce que le cœur demande à l'app (notifier, attirer l'attention, tray, menus, version, redémarrer) passe par un trait `Host`. L'app de bureau l'implémente avec Tauri ; le serveur l'implémente sans rien afficher et pousse à la place un événement que le navigateur rend.
- **Une seule liste des commandes**, servie à la fois par l'IPC de Tauri et par la WebSocket. Le moyen (macro, génération, table) est au constructeur ; la contrainte est qu'ajouter une commande à un seul endroit la rende disponible des deux côtés, et qu'un test échoue sinon.
- **Les flux.** Les 7 `Channel` deviennent une sortie abstraite qui écrit soit dans un `Channel` de Tauri, soit dans la WebSocket. Les sorties de terminal restent binaires, multiplexées sur la même connexion.
- **Plusieurs abonnés.** `hub.rs` diffuse à tous les navigateurs connectés. Une réponse donnée dans l'un (autorisation, question) se ferme dans les autres.
- **L'état d'interface partagé.** `UiState` reste unique : un navigateur le lit au chargement, et le dernier qui l'écrit gagne. Naviguer dans un navigateur ne déplace pas les autres en direct (confirmé par le demandeur ; `docs/REMOTE.md` vise à terme un état par client).
- **La reprise.** À la connexion, le client reçoit `InitialState` puis les événements. Chaque terminal et chaque lancement garde un tampon de sortie, comme les vues d'agent aujourd'hui (4 Mo), pour redessiner l'écran.
- **Le front sur deux transports.** `ipc.ts` choisit Tauri ou la WebSocket selon l'endroit où il tourne. Les composants ne le savent pas. Un build web à part (configuration Vite) produit les fichiers que le serveur embarque.
- **Les commandes sans objet sur un serveur** (les 15 à `AppHandle`, la voix) : l'interface web cache ce qui les appelle, et l'API les refuse avec une erreur claire.
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

Un binaire et un service systemd de l'utilisateur (`loginctl enable-linger`), posé par `escouade-server install`. Les données restent dans `~/.escouade`. Un verrou par fichier remplace `single-instance` (qui passe par D-Bus).

## Les lots

Chaque lot est une PR proposable à l'amont, relisible seule, et a son propre plan. Les lots 1 à 4 ne changent rien de visible dans l'app de bureau.

| Lot | Livre | Preuve |
| --- | --- | --- |
| 0. Essais | Le cœur lancé dans un process sans fenêtre, dans un conteneur Linux sans affichage ; la liste de ce qui suppose un bureau (D-Bus, cpal, GTK du presse-papiers) | Un compte rendu, pas de code gardé |
| 1. `Host` | Le trait à la place de l'`AppHandle` du cœur | Les tests existants passent, l'app de bureau est inchangée |
| 2. API distante | La liste unique des commandes, la sortie abstraite des flux, le serveur WebSocket (dans l'app de bureau, éteint par défaut) | Un test appelle une commande et reçoit un événement par la WebSocket |
| 3. Plusieurs abonnés et reprise | `hub.rs` à plusieurs abonnés, les tampons des terminaux et des lancements | Deux clients reçoivent le même événement ; un client reconnecté retrouve l'écran d'un terminal |
| 4. Front sur deux transports | `ipc.ts` sur Tauri ou WebSocket, le build web | Un parcours court (ouvrir un projet, lancer un agent sur un faux harness, répondre, ouvrir un terminal) passe dans Chromium sur la WebSocket, sous Linux ; pas toute la suite e2e (**mon choix**) |
| 5. `escouade-server` | Le binaire, l'appairage, le service, et la table « Ce qui change de machine » | Le parcours du but 3, dans un e2e sur un conteneur sans affichage |

Le premier plan couvre le lot 0 seul (confirmé par le demandeur) ; celui du lot 1 dépend de son compte rendu. Les PR partent de `upstream/main`, après avoir regardé où en est `release/2.3`.

## Laissé au constructeur

- La forme de la liste unique des commandes et le format des messages de la WebSocket.
- La bibliothèque du serveur HTTP et WebSocket (une déjà présente dans le dépôt est préférée).
- Le découpage du lot 5 s'il s'avère trop gros pour une PR.
- Les conventions du dépôt (titres de commits et de PR en français, style `feat(zone): …`, pages de `.escouade/spec` à tenir à jour) se lisent dans l'historique.

## Risques connus

- L'amont annonce ce chantier pour la 2.3 et pousse tous les jours : chaque lot se rebase sur `upstream/main` avant d'être proposé, et reste petit.
- La connexion d'un harness sans navigateur sur le serveur n'a jamais été essayée ; le lot 0 l'essaie pour Claude Code.
- Le mainteneur peut préférer un autre découpage : les lots 1 à 4 suivent son ordre pour limiter ce risque.
