# Le mode serveur

Le mode serveur fait tourner Escouade sans fenêtre sur une machine Linux qui reste allumée, et ouvre la même interface dans un navigateur depuis ton poste. Cette page explique comment l’installer, y appairer un navigateur, et quoi faire quand le tunnel ne s’ouvre pas.

> [!WARNING]
> C’est une première version. Aucune version d’Escouade ne livre encore le programme `escouade-server` : il se construit depuis les sources. Il est prévu pour un seul utilisateur par serveur, sous Linux x64 ou arm64.

## Ce que c’est

Sur le serveur, le programme `escouade-server` fait tourner tout ce que fait l’app : les agents, Git, le Kanban, les terminaux, le serveur MCP. Il n’a pas de fenêtre et n’a besoin d’aucun bureau.

Sur ton poste, un navigateur affiche l’interface. Ce n’est qu’une vue : la fermer n’arrête rien. Tu peux en ouvrir plusieurs, sur plusieurs appareils, et retrouver les mêmes agents.

Le serveur n’écoute jamais sur Internet. Tu le joins par un tunnel SSH, avec les clés que tu utilises déjà pour administrer la machine, ou par un proxy HTTPS que tu tiens toi-même.

L’app de bureau ne change pas, et elle ne sait pas encore se connecter à un serveur : en mode serveur, le client est un navigateur.

## Avant de commencer

Il te faut :

- une machine Linux (Ubuntu 22.04, Debian 12, ou plus récent) où tu entres par SSH ;
- sur cette machine, le harness de tes agents ([Claude Code](harness) par exemple) et `git`, installés pour le compte qui fera tourner le serveur ;
- sur ton poste, un navigateur et la commande `ssh` (présente sous Windows 10 et plus, macOS et Linux).

### Un compte à part pour les agents

Les agents ont les droits du compte qui fait tourner le serveur. Si ce compte a `sudo` ou appartient au groupe `docker`, ils ont toute la machine. Sur un serveur qui héberge autre chose, crée un compte dédié, sans l’un ni l’autre. Depuis ton compte administrateur :

```
sudo adduser --disabled-password --gecos "" escouade
sudo install -d -m 700 -o escouade -g escouade /home/escouade/.ssh
sudo install -m 600 -o escouade -g escouade ~/.ssh/authorized_keys /home/escouade/.ssh/
```

La dernière ligne reprend tes clés SSH : `ssh escouade@<hôte>` entre alors dans ce compte. Fais toutes les étapes suivantes depuis lui, jamais en root.

## Construire le serveur

Sur une machine de la même architecture que le serveur, avec rustup, Node 22, git et un compilateur C :

```
rustup target add wasm32-wasip1
npm ci && npm run build:web
sh scripts/escouade-js.sh
npx --yes tsx@4.23.15 store/scripts/seed.ts fetch
cd src-tauri && cargo build --release --locked --no-default-features --features server --bin escouade-server
```

Tu obtiens le programme `src-tauri/target/release/escouade-server` et le dossier `dist-web/`, la page que le serveur sert. Copie les deux sur le serveur, par exemple dans `~/.local/bin/` et `~/.local/share/escouade-web/`.

## Installer le service

Sur le serveur, connecté par `ssh` avec le compte des agents :

```
escouade-server install --web ~/.local/share/escouade-web
systemctl --user daemon-reload
systemctl --user enable --now escouade-server
loginctl enable-linger
```

`install` écrit le service et affiche les commandes à lancer ensuite ; il ne lance rien lui-même. `loginctl enable-linger` fait tourner le service sans session ouverte, et le relance au démarrage de la machine. S’il est refusé, lance `sudo loginctl enable-linger <compte>` depuis un compte administrateur.

Les données sont celles de l’app, dans `~/.escouade`. Un seul serveur tourne par dossier de données : un second refuse de démarrer.

## Ouvrir le tunnel et appairer ton navigateur

1. Sur le serveur, lance `escouade-server pair`. Il affiche la ligne `ssh` à lancer, l’adresse à ouvrir et un code.
2. Sur ton poste, lance cette ligne `ssh -N -L 8765:… <hôte>` et laisse-la tourner.
3. Ouvre l’adresse que `pair` a donnée, `http://127.0.0.1:8765/`. La page affiche « Appairer ce navigateur ».
4. Tape le code et un nom pour ton appareil.

Le code sert une fois et vit 10 minutes. Un code faux se retape simplement.

> [!NOTE]
> Pour ton navigateur, `http://127.0.0.1:8765/` et `http://localhost:8765/` sont deux sites différents : un navigateur appairé sur l’un ne l’est pas sur l’autre. Garde toujours la même adresse.

Garde ce port local à Escouade seul. Ton navigateur y range le jeton de l’appareil, et tout autre programme servi plus tard sur ce port pourrait le lire.

## Premiers pas dans le navigateur

- **Ajouter un compte** : Réglages › Comptes et harness › « Ajouter un compte… ». Claude Code démarre dans un terminal de la page, sur le serveur. Il affiche une adresse : clique-la, connecte-toi dans ton navigateur, puis colle dans le terminal le code que la page te donne. Voir [les comptes](accounts).
- **Créer un projet** : « Nouveau projet » › « Parcourir… » montre les dossiers du serveur, sous le dossier personnel du compte. « Cloner ici » y clone un dépôt.
- **Lancer un agent**, fermer l’onglet, le rouvrir : l’agent a continué.

## Ce qui change par rapport à l’app

| Dans l’app | Dans le navigateur |
|---|---|
| Choisir un dossier par le sélecteur du système | Un navigateur des dossiers du serveur |
| Joindre un fichier par son chemin | Le fichier est envoyé au serveur |
| « Ouvrir » un fichier envoyé par un agent | Le fichier se télécharge sur ton poste |
| Notifications du système | Notifications du navigateur |
| Mise à jour intégrée | Absente : tu remplaces le programme et la page, puis tu relances le service |
| Dictée et lecture à voix haute | Absentes pour l’instant |
| Plugins « Depuis un dossier… » | Absent : le store et les dépôts Git restent |

Après une coupure de réseau, la page se reconnecte seule et retrouve tes agents et tes terminaux, avec ce qu’ils ont écrit entre-temps. Si le serveur a changé de version, elle te demande de la recharger.

## Gérer les appareils

Chaque navigateur appairé a son propre jeton. Sur le serveur :

- `escouade-server devices` liste les appareils, avec leur nom et la date où ils ont été vus ;
- `escouade-server devices revoke <id>` en coupe un tout de suite, connexions ouvertes comprises.

> [!WARNING]
> Le jeton d’un appareil vaut un terminal sur le serveur, sous le compte du service. Révoque un appareil perdu ou un navigateur que tu n’utilises plus.

Si la commande répond que la révocation n’a pas pu être écrite, l’appareil reste coupé tant que le serveur tourne, mais reviendrait à son redémarrage : libère de la place ou répare le disque, puis révoque de nouveau.

## Derrière un proxy HTTPS

À la place du tunnel, tu peux mettre le serveur derrière ton propre proxy (Caddy, nginx). Le service écoute alors sur un port, et tu lui déclares l’adresse publique :

```
escouade-server install --port 8765 --origin https://escouade.exemple.fr --web ~/.local/share/escouade-web
```

Avec Caddy :

```
escouade.exemple.fr {
	reverse_proxy 127.0.0.1:8765
}
```

`escouade-server pair` donne alors l’adresse `https://escouade.exemple.fr/`. Sur un port, n’importe quel compte de la machine peut joindre le serveur : réserve ce montage à une machine dont tu es le seul utilisateur.

## Ce qui vit sur le serveur

Tout l’état d’Escouade est dans `~/.escouade` : tes projets, tes conversations, et **les identifiants de tes comptes**, en clair dans des fichiers que seul le compte du service peut lire. Il n’y a pas de trousseau sur un serveur. Qui lit ce disque, une de ses sauvegardes ou la session SSH de ce compte a tes comptes. Voir aussi [les données](data).

## Si le tunnel ne s’ouvre pas

**VS Code est connecté au même serveur (Remote-SSH).** VS Code transfère tout seul le port de chaque adresse `http://localhost:…` ou `http://127.0.0.1:…` affichée dans un de ses terminaux, dont celle que donne `pair`. Le port local est alors pris, et ton `ssh -L` échoue. Deux issues :

- dire à VS Code d’ignorer ce port, dans ses réglages : `"remote.portsAttributes": { "8765": { "onAutoForward": "ignore" } }`, puis arrêter dans son panneau « Ports » le transfert déjà ouvert ;
- ou installer le service sur un port (`escouade-server install --port 8765 --web …`) et laisser VS Code le transférer.

**Sous Windows, `bind [127.0.0.1]:8765: Permission denied`.** Ce message veut souvent dire que le port est déjà pris. `Get-NetTCPConnection -State Listen -LocalPort 8765` montre par qui. Windows réserve aussi des plages de ports pour Hyper-V, WSL 2 et Docker : `netsh interface ipv4 show excludedportrange protocol=tcp` les liste. Choisis un port local hors de ces plages, par exemple `ssh -N -L 18765:… <hôte>`, puis ouvre `http://127.0.0.1:18765/`.

**Sous PowerShell, `hostname contains invalid characters`.** Un chemin de clé (`-i`) qui contient des espaces se met entre guillemets.

**Le serveur refuse la redirection vers une socket.** Un serveur SSH réglé avec `AllowStreamLocalForwarding no` refuse le tunnel vers la socket du service. Installe alors le service sur un port, et ouvre le tunnel vers ce port : `ssh -N -L 8765:127.0.0.1:8765 <hôte>`.

**Le service ne démarre pas.** `journalctl --user -u escouade-server -n 50` montre pourquoi. Voir aussi [le dépannage](troubleshooting).

## Tout retirer

```
systemctl --user disable --now escouade-server
escouade-server uninstall
```

Supprime ensuite le programme, le dossier de la page et, pour ne rien laisser, `~/.escouade`. Si tu avais créé un compte dédié : `sudo loginctl disable-linger escouade && sudo deluser --remove-home escouade`.

## Limites connues

- Pas encore de client de bureau qui se connecte à un serveur, ni de mise en page pour téléphone.
- Les serveurs de test que lancent tes agents ne sont pas relayés vers ton poste.
- L’état de l’interface est partagé entre tes appareils : le dernier qui écrit gagne.
- Le serveur tient 16 connexions à la fois ; une connexion très lente ou morte peut en garder une quelque temps.
- N’ouvre pas l’app de bureau sur le dossier de données d’un serveur qui tourne.
