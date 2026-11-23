# Audit de sécurité du mode serveur avec Mantis, novembre 2026

Le mode serveur (`escouade-server`, l'API distante, le client web) a été audité le 23 novembre 2026 avec [Mantis](https://github.com/google/mantis), une chaîne d'audit de sécurité menée par des agents d'IA. L'audit a tourné au niveau le plus poussé (« Overkill ») : chaque fichier relu, puis trois tours complets de recherche, de reproduction et de correction. C'est un audit interne mené par un agent pour le projet. Il dit l'état d'une branche à une date, pas celui des versions suivantes.

Ce document est le compte rendu : le périmètre, la méthode, les points relevés et corrigés, et ce qui reste à faire. Le rapport brut de Mantis (constats détaillés, programmes de reproduction) n'est pas versionné, car il décrit des attaques pas à pas.

## Périmètre

- **Le code audité.** La branche du mode serveur, soit 37 commits depuis `main` (base `f29711a5`). Cela fait 193 fichiers source modifiés : Rust (`src-tauri/src/remoteapi/`, `headless_host.rs`, `bin/escouade-server.rs`, le cœur et ses commandes) et TypeScript/Svelte (le transport web, le client).
- **Les fichiers voisins.** Le serveur atteint aussi du code que la branche n'a pas touché : `git.rs`, `projectlogo.rs`, `docs.rs`, `docsimport.rs`, `workflowfiles.rs`. Le deuxième et le troisième tour les ont ajoutés.
- **Hors périmètre.** Le reste du dépôt Escouade, les tests, la documentation, le site.

## Méthode

Mantis enchaîne quatorze étapes, chacune confiée à un agent :

1. **Comprendre le code.** Historique git, index du code, architecture, modèle de menaces.
2. **Chercher.** Un plan, puis une investigation par fichier source, sur 193 à 198 fichiers selon le tour.
3. **Trier.** Dédoublonnage, relecture, puis critique : chaque constat est-il exploitable en production ?
4. **Prouver.** Une reproduction dans Docker sans réseau, avec une copie exacte du code fautif.
5. **Corriger.** Un correctif. Un second agent, indépendant, essaie ensuite de le contourner sur une copie isolée du dépôt.
6. **Calibrer et capitaliser.** Un score de risque, puis un bilan qui sert à replanifier le tour suivant.

Trois tours ont été faits :

| Tour | Axe principal | Constats retenus |
|---|---|---|
| 1 | Tout le code du mode serveur | 7 |
| 2 | La surface accessible sans authentification, les fichiers voisins des constats du tour 1 | 4 de plus |
| 3 | Le contenu d'un dépôt cloné vu comme une entrée hostile | 8 de plus |

Les reproductions et les contre-attaques ont tourné dans des conteneurs Docker sans réseau, sur des fichiers factices. Rien du code cible n'a tourné sur le poste. Le serveur complet n'a pas été compilé pendant l'audit : chaque reproduction recopie le fragment fautif dans un petit programme, en citant le fichier et la ligne d'origine.

## Ce que l'audit a trouvé

**La surface exposée au réseau a tenu sur les trois tours.** Les contrôles Host et Origin, le refus du DNS rebinding, CORS, l'appairage par code, le jeton d'appareil, la page web statique et le confinement des fichiers envoyés n'ont montré aucun contournement.

Les vrais points étaient ailleurs :

- **Le contenu d'un dépôt cloné.** C'est la principale découverte. Un dépôt ajouté comme projet ou mis à jour par un pull pouvait faire lire, écrire ou effacer un fichier hors du projet, ou faire tomber le serveur, avec seulement des liens symboliques commités, des fichiers énormes ou des chemins écrits dans un fichier de configuration.
- **La disponibilité du serveur.** Un fichier spécial lu sans garde (une FIFO, `/dev/zero`), un appel bloqué, des connexions occupées.
- **Les révocations.** Révoquer un appareil ne coupait ni ses transferts en cours ni ses connexions inactives.

19 constats ont été retenus, aucun critique ni élevé. 28 autres ont été écartés : ce sont souvent de vrais bugs, mais pas des failles, puisqu'un appareil appairé a déjà l'équivalent d'un shell sur le serveur (`docs/REMOTE.md`).

## Points corrigés

Les 19 constats sont corrigés sur la branche `mantis/audit-2026-11-23_11h17`.

« Vérifié » veut dire qu'une contre-attaque indépendante a échoué contre le correctif, sur une copie isolée du dépôt. « Proposé » veut dire qu'aucune contre-attaque n'a pu être menée.

### Contenu d'un dépôt cloné

| Point | Correctif | État |
|---|---|---|
| Les médias d'un ticket partagé étaient importés en suivant les liens symboliques du dossier du ticket (`sharing.rs`, `import_media`) | Liens et fichiers non réguliers refusés à tous les niveaux, taille copiée plafonnée | Vérifié |
| Les tickets partagés étaient lus sans limite de taille ni de nombre, et `import_media` relisait le même fichier sans limite (`sharing.rs`, `read_dir`) | 64 Kio par fichier, 2 000 fichiers par dossier, rapprochement linéaire, `import_media` reprend le texte déjà lu | Corrigé (le premier correctif avait été contourné par la relecture, celle-ci est maintenant bornée) |
| La suppression d'un ticket partagé suivait un dossier de tickets lié (`sharing.rs`, `remove_file_of`) | Même garde que les fonctions voisines (`unsafe_dir`), dossier lié jamais effacé | Vérifié |
| `.escouade.yml` était lu à travers les liens, sans limite de taille (`projectfile.rs`) | Fichier régulier seulement, `O_NOFOLLOW`, 1 Mio au plus, même garde à l'écriture | Vérifié |
| L'analyse de `.escouade.yml` prenait un temps quadratique sous le verrou des projets (`projectfile.rs`) | Table des lignes calculée une fois par analyse | Proposé |
| Un workflow commité pouvait désigner n'importe quel chemin comme fichier d'état (`workflowfiles.rs`, `scan_of`) | Chemins relatifs seulement, dans le projet, sans lien, 2 Mo au plus | Vérifié |
| Les fichiers de spec importés étaient lus en entier (`docsimport.rs`) | 2 Mio par fichier, 32 Mio par import, fichiers réguliers seulement | Vérifié |
| Le dossier de mémoire du projet était lu sans limite (`memory.rs`) | 500 fichiers et 8 Mio au plus | Vérifié |
| `git show` et `git diff` chargeaient toute la sortie de git avant de la couper (`git.rs`) | Lecture plafonnée à 64 Mo, git arrêté au-delà | Proposé (contre-attaque interrompue) |
| La vue Cursor écrivait sa règle de relais à travers des liens commités (`tui.rs`) | Liens refusés sur tout le chemin, `O_NOFOLLOW`, suppression d'un fichier régulier seulement | Vérifié |

### Disponibilité du serveur

| Point | Correctif | État |
|---|---|---|
| Lire une FIFO bloquait un fil du runtime pour toujours (`fsedit.rs`, puis `git.rs` par un lien) | Un lecteur commun (`paths::read_regular`) : ouverture non bloquante, fichier régulier, taille plafonnée | Vérifié (le premier correctif avait été contourné par `git.rs`) |
| Un appel qui ne rend jamais la main gardait sa place d'appel (`remoteapi/server.rs`) | Appel annulé à la fermeture ou à la révocation de la connexion | Proposé |

### Appareils, appairage, révocation

| Point | Correctif | État |
|---|---|---|
| Cinq mauvais codes, envoyés par n'importe qui, annulaient l'appairage en cours (`remoteapi/devices.rs`) | Un mauvais code est refusé sans annuler le code en cours. `O`, `I` et `L` comptent pour `0`, `1` et `1` | Vérifié (force brute hors de portée : environ 2×10⁻⁵ de chances sur la vie d'un code) |
| Un appareil pouvait occuper les 16 connexions et empêcher sa propre révocation (`remoteapi/server.rs`) | 12 WebSockets au plus, 4 connexions réservées à la révocation et à l'appairage | Proposé |
| Révoquer un appareil ne coupait pas ses transferts de fichiers (`remoteapi/server.rs`) | Les transferts en cours et en attente sont coupés à la révocation | Proposé |
| Le mot de passe du proxy partait en clair à chaque navigateur appairé (`commands.rs`) | Retiré de l'état envoyé aux clients | Proposé |

### Client web

| Point | Correctif | État |
|---|---|---|
| Un lien ouvert depuis Escouade donnait l'adresse du serveur au site visité, par l'en-tête Referer (`platform.ts`) | `noreferrer` à l'ouverture et `Referrer-Policy: no-referrer` sur la page | Vérifié |
| Cette fuite, ajoutée aux cinq mauvais codes, permettait de bloquer l'appairage de l'extérieur | Cassée par les deux correctifs ci-dessus | Vérifié par ses deux maillons |
| Dans le navigateur, le bouton « Ouvrir » proposait aussi des programmes et des scripts (`SentFiles.svelte`) | Bouton masqué pour un fichier exécutable, comme sur le bureau | Vérifié |

## Vérification des correctifs

Chaque correctif a été écrit sur sa propre branche, puis tous ont été fusionnés et vérifiés ensemble sous Linux, dans Docker :

| Contrôle | Résultat |
|---|---|
| `cargo fmt --all --check` | OK |
| `cargo clippy --no-default-features --features server -p escouade --all-targets -- -D warnings` | OK |
| `cargo build --no-default-features --features server --bin escouade-server` | OK |
| `cargo test --no-default-features --features server -p escouade -p escouade-model` | Aucune régression (voir ci-dessous) |
| `vitest run` | 5 723 tests passés |
| `svelte-check`, `prettier --check` | OK |

Les tests Rust n'ont pas tourné dans l'environnement de la CI, mais dans un conteneur local. Une centaine de tests échouent déjà dans ce conteneur sur la base sans correctif (`b54b0f47`) : ce sont des tests d'intégration des harness et des plugins, sensibles au temps et à l'ordre. La comparaison s'est donc faite contre la base, dans ce même environnement :

- **Une régression trouvée et corrigée.** Le test d'intégration `a_device_paired_by_the_command_is_listed_then_revoked` attendait encore le message « cinq mauvais codes annulent l'appairage », que le correctif de l'appairage a retiré. Le test dit maintenant le nouveau comportement.
- **Les autres échecs ne viennent pas des correctifs.** Ceux qui n'apparaissaient que sur la branche corrigée ont été relancés seuls. Ils passent, ou échouent de la même façon sur la base dès qu'on les lance ensemble : `third_party`, `cursor`, et les groupes de processus d'`escouade-model`.
- Les tests ajoutés par les correctifs passent.

L'app de bureau (feature `desktop`) n'a pas été recompilée. La CI le fera.

## Ce qui reste

- **Des correctifs proposés sans contre-attaque.** Pour 6 d'entre eux, il manque une contre-attaque sur un serveur qui tourne vraiment.
- **Des limites connues, à traiter plus tard :**
  - `git::diff` d'un checkout et `lines_at` lisent encore sans plafond ;
  - `shared_resolve` relit un ticket sans limite, mais seulement à la demande de l'utilisateur ;
  - `write_shared_file` suit un dossier lié à l'écriture ;
  - il n'y a pas de limite par appareil ni de délai d'inactivité sur les WebSockets.
- **Les limites de la méthode.**
  - Pas de serveur complet pendant l'audit : les reproductions recopient le code fautif.
  - Les garde-fous de l'API du modèle ont interrompu quelques agents de reproduction : les constats concernés sont restés au stade de la lecture du code.

## La suite : un audit Mantis de tout Escouade

Cet audit ne couvrait que le mode serveur. **Un audit Mantis du dépôt Escouade entier est prévu plus tard** : l'app de bureau, l'hôte des plugins, les harness, le magasin et le site. Ses premières pistes :

- **Le contenu d'un dépôt cloné.** Il a donné la plupart des constats ici. Le même motif (liens symboliques commités, fichiers énormes, chemins écrits dans un fichier du dépôt) est à chercher dans tout le code qui lit un projet.
- **Le partage d'équipe et ses fichiers synchronisés.**
- **Les lectures de fichiers choisis par un agent ou par un client.**
