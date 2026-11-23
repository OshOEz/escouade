---
title: "Mode serveur : escouade-server et l'interface dans un navigateur"
subject: "Mode serveur"
sources: ["docs/REMOTE.md (« La fondation commune », « Le mode serveur »)","docs/CRATES.md §16","src-tauri/src/headless_host.rs","src-tauri/src/remoteapi/server.rs","src-tauri/src/remoteapi/devices.rs","src/lib/transport/","tests/web/"]
tickets: []
decisions: ["le-mode-serveur-escouade-server-fait-tou"]
compared: null
---

## Ce que c'est

Escouade sans fenêtre, sur une machine qui reste allumée (un VPS, un poste au bureau) : `escouade-server` y fait tourner le cœur (agents, Git, Kanban, terminaux, serveur MCP) et sert la même interface à un navigateur, sur un autre poste. Fermer l'onglet, éteindre le poste ou perdre le réseau n'arrête aucun agent : le navigateur n'est qu'une vue.

C'est une première version : Linux x64 et arm64, construit depuis les sources (aucune release ne livre encore le binaire), un seul utilisateur par serveur. L'app de bureau ne change pas ; elle ne sait pas encore se connecter à un serveur. Le détail de l'installation, des choix et de ce qui reste à faire est dans `docs/REMOTE.md`.

## Comportement

### Le serveur

- `escouade-server` écoute sur une socket Unix (`--socket`, le défaut du service : `/run/user/<uid>/escouade/server.sock`, dans un dossier `0700`) ou sur un port de `127.0.0.1` (`--port`). Jamais sur une autre adresse : on le joint par un tunnel `ssh -L`, ou par un proxy HTTPS de l'utilisateur dont l'origine est déclarée (`--origin https://nom`, répétable).
- `--web <dossier>` donne la page à servir (le build web de l'interface). Les données sont celles de l'app : `~/.escouade`, ou `ESCOUADE_DATA_DIR`.
- **Un serveur par dossier de données** : un second sur le même dossier refuse de démarrer (code 3) et dit lequel le tient.
- `escouade-server install` écrit le service systemd de l'utilisateur et dit les commandes à lancer ensuite ; il ne lance rien lui-même. `uninstall` retire l'unité, et refuse tant que le service est activé. À l'arrêt du service, le serveur arrête ses agents comme l'app quand on la quitte.
- Il ne dépend ni de GTK, ni de WebKit, ni de D-Bus, ni d'ALSA.

### Les appareils

- Un navigateur ne voit rien avant d'être appairé : la page affiche « Appairer ce navigateur » (un code, le nom de l'appareil).
- `escouade-server pair`, lancé sur le serveur, donne le code et l'adresse à ouvrir. Le code sert une fois, vit 10 minutes, un seul à la fois ; 5 codes faux l'annulent.
- Appairé, le navigateur reçoit un jeton qui n'est qu'à lui. Le serveur n'en garde que l'empreinte, dans un fichier lisible de son seul propriétaire. Le jeton n'est jamais dans une adresse ni dans un journal.
- `escouade-server devices` liste les appareils (nom, appairé le, vu le). `escouade-server devices revoke <id>` en coupe un sur-le-champ, connexions ouvertes comprises. Si la révocation n'a pas pu être écrite sur le disque, la commande le dit et sort en erreur : l'appareil reste coupé tant que le serveur tourne, et il faut révoquer de nouveau une fois le disque réparé.
- Une page d'une autre origine que celle du serveur (ou qu'une origine déclarée) est refusée, jeton ou non.
- Ce jeton vaut un terminal sur le serveur, sous le compte du service : un appareil perdu se révoque.

### Dans le navigateur

- La même interface que l'app : projets, agents et conversations, Kanban, éditeur, Git, terminaux, réglages, plugins et store.
- **Plusieurs appareils à la fois** sur un même serveur : chacun voit les mêmes agents, et une réponse donnée sur l'un (une autorisation, une question) se ferme chez les autres. L'état de l'interface est partagé : le dernier qui écrit gagne.
- **Après une coupure**, la page se reconnecte seule et retrouve l'état et ses terminaux, ce qu'ils ont écrit entre-temps compris. Si le serveur a changé de version, elle dit de la recharger.
- Ce qui se faisait sur la machine du cœur se fait par le navigateur :
  - « Nouveau projet » › « Parcourir… » montre les dossiers du serveur, sous le dossier personnel du compte du service ; « Cloner ici » y clone un dépôt ;
  - un fichier joint, collé ou glissé est envoyé au serveur ; un fichier envoyé par un agent se télécharge sur le poste ;
  - les notifications sont celles du navigateur (sur `localhost` ou une origine `https`) ;
  - un lien s'ouvre dans un onglet du poste.
- Ce qui n'a pas de sens sans fenêtre est caché : la mise à jour de l'app, la voix, « Afficher dans l'Explorateur », « Depuis un dossier… » des plugins.

### Dans l'app de bureau

- Rien de visible ne change. Le cœur ne dépend plus de la fenêtre : ses commandes sont déclarées une fois, pour la fenêtre et pour l'API distante.
- L'API distante de l'app de bureau est éteinte ; elle ne s'allume que dans un build de debug.

## Limites

- Pas encore : le client de bureau qui se connecte à un serveur, la voix dans le navigateur, la mise à jour du serveur depuis l'interface, la mise en page mobile, le relais des ports des serveurs de test des agents, une release qui livre le binaire.
- Les identifiants des comptes vivent sur le serveur, en clair dans des fichiers lisibles du seul compte du service : il n'y a pas de trousseau sur un serveur.
- Les agents tournent sous le compte du serveur : rien ne les sépare du cœur qui les lance, ni de sa clé d'administration.
- Les autres limites connues (16 connexions à la fois, liens très lents, terminaux sous Windows, fichiers de plusieurs Go) sont listées dans `docs/REMOTE.md`, « Limites connues ».
