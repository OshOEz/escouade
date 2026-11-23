---
number: null
id: "le-mode-serveur-escouade-server-fait-tou"
title: "Le mode serveur : escouade-server fait tourner le cœur sans fenêtre et sert l'interface à des navigateurs appairés"
date: "2026-11-23"
state: "proposed"
origin: "document"
origin_ref: "docs/REMOTE.md"
pages: ["serveur/mode-serveur"]
tickets: []
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur/mode-serveur","added":49,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Un utilisateur a demandé à lancer Escouade sur un VPS et à le piloter de son poste, les agents tournant H24 (relayé le 2026-10-25). `docs/REMOTE.md` en a fait un chantier : une fondation commune avec le téléphone (le cœur détaché de Tauri, une API distante, plusieurs clients), puis le mode serveur. Il laissait quatre décisions ouvertes, dont le tunnel par défaut et la distribution du serveur. Jusqu'ici le cœur et l'interface se parlaient par l'IPC de Tauri, dans un seul process, et le cœur supposait une fenêtre.

## Décision

Un binaire `escouade-server`, pour Linux, fait tourner le cœur sans fenêtre et sert l'interface web, la même que celle de l'app, à des navigateurs. Il n'écoute que sur une socket Unix ou sur `127.0.0.1` : on le joint par un tunnel SSH (le défaut proposé par le chantier) ou par un proxy HTTPS dont l'origine est déclarée. Il s'installe en service systemd de l'utilisateur, un seul par dossier de données. Un navigateur s'appaire par un code donné sur le serveur (une fois, 10 minutes, 5 essais) et reçoit un jeton à lui, révocable sur-le-champ ; le serveur n'en garde que l'empreinte. Plusieurs appareils suivent le même serveur, et un client coupé retrouve l'état et ses terminaux. Le client vient d'abord dans un navigateur : le client de bureau (« Se connecter à un serveur ») passe après. L'app de bureau garde son IPC, et son API distante reste éteinte hors d'un build de debug.

## Conséquences

Le cœur ne nomme plus Tauri : un trait `Host` remplace l'`AppHandle`, les commandes sont déclarées une fois pour l'IPC et pour l'API distante, et le cœur se construit et se teste sous Linux sans GTK ni WebKit (la CI le garde). Le front appelle le cœur par un transport (l'IPC de Tauri ou une WebSocket) et le système par une façade ; il a un build web sans Tauri, joué dans Chromium par la CI. Le jeton d'un appareil vaut un terminal sur le serveur, sous le compte du service ; les identifiants des comptes y sont en clair dans des fichiers de ce compte, faute de trousseau ; les agents ne sont pas séparés du cœur qui les lance. Restent à faire : le client de bureau, la voix dans le navigateur, la mise à jour du serveur, le relais des ports des serveurs de test, la mise en page mobile, une release qui livre le binaire, et les durcissements listés dans `docs/REMOTE.md`, « Limites connues ».
