# Essayer le mode serveur d'Escouade (version d'essai)

Un binaire `escouade-server` déjà construit et sa page web. Rien à compiler.
Il te faut : une machine Linux (Ubuntu 22.04 ou Debian 12, ou plus récent) où tu entres par SSH **sans être root**, et Claude Code installé dessus pour ce compte.

## Un compte à part (conseillé)

Les agents ont les droits du compte qui fait tourner le serveur. Si ton compte a `sudo` ou est dans le groupe `docker`, ils ont toute la machine. Depuis ton compte administrateur :

```
sudo adduser --disabled-password --gecos "" escouade
sudo install -d -m 700 -o escouade -g escouade /home/escouade/.ssh
sudo install -m 600 -o escouade -g escouade ~/.ssh/authorized_keys /home/escouade/.ssh/
```

Puis `ssh escouade@<hôte>`, installe Claude Code dans ce compte, et fais tout le reste depuis lui.

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
2. Ouvre l'adresse que `pair` a donnée (`http://127.0.0.1:8765/` ; `localhost` est une autre origine pour le navigateur), tape le code et un nom pour ton appareil.
3. Réglages › Comptes et harness › « Ajouter un compte Claude Code… » : clique l'adresse affichée, colle le code qu'elle donne.
4. « Nouveau projet » › « Parcourir… » : choisis un dossier du serveur, ou « Cloner ici ».
5. Lance un agent, ferme l'onglet, rouvre-le : l'agent a continué.

**Le `ssh -L` échoue ?**
- Avec VS Code Remote-SSH ouvert sur le même serveur, VS Code prend le port 8765 dès que `pair` affiche son adresse. Mets `"remote.portsAttributes": { "8765": { "onAutoForward": "ignore" } }` dans ses réglages et arrête ce transfert dans le panneau Ports. Ou installe le service sur un port (`install --port 8765 --web …`) et laisse VS Code le transférer.
- Sous Windows, `bind [127.0.0.1]:8765: Permission denied` veut souvent dire « port déjà pris » : `Get-NetTCPConnection -State Listen -LocalPort 8765`. Sous PowerShell, mets entre guillemets un chemin de clé (`-i`) qui contient des espaces.

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
