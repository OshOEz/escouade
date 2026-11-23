# Essayer le mode serveur d'Escouade (version d'essai)

Un binaire `escouade-server` déjà construit et sa page web. Rien à compiler.
Il te faut : une machine Linux (Ubuntu 22.04 ou Debian 12, ou plus récent) où tu entres par SSH **sans être root**, et Claude Code installé dessus pour ce compte.

## Sur le serveur

```
mkdir -p ~/.local/bin ~/.local/share
cp escouade-server ~/.local/bin/
cp -r escouade-web ~/.local/share/escouade-web
~/.local/bin/escouade-server install --web ~/.local/share/escouade-web
systemctl --user daemon-reload
systemctl --user enable --now escouade-server
loginctl enable-linger
~/.local/bin/escouade-server pair
```

`pair` affiche un code et la ligne `ssh` à lancer. Si `enable-linger` est refusé : `sudo loginctl enable-linger <ton compte>`.

## Sur ton poste

1. Lance la ligne `ssh -N -L 8765:… <hôte>` que `pair` a donnée, et laisse-la tourner.
2. Ouvre `http://localhost:8765/`, tape le code et un nom pour ton appareil.
3. Réglages › Comptes et harness › « Ajouter un compte Claude Code… » : clique l'adresse affichée, colle le code qu'elle donne.
4. « Nouveau projet » › « Parcourir… » : choisis un dossier du serveur, ou « Cloner ici ».
5. Lance un agent, ferme l'onglet, rouvre-le : l'agent a continué.

## Ce que j'aimerais savoir

- À quelle étape ça coince, avec le message exact.
- `journalctl --user -u escouade-server -n 50` si le service ne démarre pas.
- Ce qui t'a surpris ou manqué dans le navigateur.

## Tout retirer

```
systemctl --user disable --now escouade-server
~/.local/bin/escouade-server uninstall
rm -r ~/.local/bin/escouade-server ~/.local/share/escouade-web
```

Les données sont dans `~/.escouade` : supprime ce dossier pour ne rien laisser.

Le détail et les limites connues : `docs/REMOTE.md` de la branche, « Le mode serveur ».
