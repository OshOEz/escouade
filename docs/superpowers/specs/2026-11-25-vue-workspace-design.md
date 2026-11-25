# La vue Workspace

Date : 2026-11-25. Conçue en cinq tours de maquettes avec l'utilisateur. Lu sur la branche `feat/mode-serveur-web`.
À construire après la sortie de la 2.3 ; d'ici là, la spec est découpée en cartes Kanban.

Maquettes de référence (pages privées de l'utilisateur) :

- composition et critique : https://claude.ai/artifact/GWg2kW7ZwEkYukE6osHHND
- disque de loin et zone d'atterrissage : https://claude.ai/artifact/8SQgeb4oeRusf2jYRfkhwj
- détail des cartes (question, artefact, CI, outils) : https://claude.ai/artifact/4cqWdgBQLpTRj6AmtGdLX5

## Le but

Voir d'un regard ce que font les agents d'un projet, comme une salle de contrôle : qui travaille, qui attend une
réponse, qui a lancé quoi, ce qui est sorti. Aujourd'hui, cette information est éparpillée : la barre latérale montre
un agent à la fois en détail, le plan est au-dessus de la conversation, les sous-agents sont une liste, la CI une
pastille.

La vue ne remplace rien. C'est une autre façon de regarder le même état, d'où l'on peut aussi répondre à une question
et écrire à un agent sans ouvrir sa conversation.

## Buts, hors périmètre, à éviter

Buts :

- une toile par projet, où chaque agent non archivé est un objet relié à ce qu'il utilise, lance et produit ;
- lisible de loin (six agents sur un écran) comme de près (un agent et tout son détail) ;
- répondre aux questions et aux permissions, et écrire à un agent, depuis la toile ;
- du mouvement seulement là où quelque chose se passe.

Hors périmètre (première version) :

- déplacer les cartes à la main et mémoriser leur place : Escouade range ;
- une toile de tous les projets ;
- une mini-carte, une miniature de la page d'un artefact, modifier le plan depuis la toile ;
- la toile dans un volet de l'écran partagé.

À éviter :

- des flux décoratifs permanents : un trait animé veut toujours dire « ça tourne en ce moment » ;
- une seconde logique de question, de permission ou d'envoi : la toile monte les composants de la conversation ;
- une bibliothèque de graphe : le dépôt n'en a pas et n'en a pas besoin (`WorkflowDiagram.svelte` dessine déjà des
  flèches et zoome à la main).

## Les approches

Trois compositions ont été maquettées.

- **La boussole** (retenue) : l'agent au centre, un sens par côté. L'effet « flux » reste, les traits sortent par
  quatre côtés au lieu d'un. Coût : un cadre est grand, deux agents dépliés remplissent un écran.
- **Les couloirs** : quatre bandes horizontales, une colonne par agent. Le plus dense, mais c'est un tableau et les flux
  disparaissent.
- **La scène et les coulisses** : un seul agent déplié. Le plus aéré, mais on ne voit plus deux agents travailler
  ensemble, ce qui est le but.

Le coût de la boussole (la place) est payé par les **altitudes** : le niveau de zoom change ce qui est montré. C'est
l'utilisateur qui a choisi la boussole et les trois altitudes.

## L'entrée et la sortie

- Le bouton « Éditeur » de la barre de titre (`TitleBar.svelte`) devient un menu à deux lignes, « Éditeur » et
  « Workspace », ouvert par `menu.showAt`. Le glisser du bouton vers un volet garde son sens actuel (l'éditeur).
- « Workspace » ouvre la toile du projet courant à la place de la zone de travail. La barre latérale reste.
- La toile a un bouton « Quitter le workspace » en haut à gauche, et `Échap` fait de même quand aucun champ n'a le
  focus. On revient à la vue d'avant.
- État : un drapeau par projet, à côté de `board`, `docs`, `prs` dans `state.svelte.ts`, avec `openWorkspace` et
  `closeWorkspace`. Comme les autres `open*`, il éteint les vues sœurs ; les autres `open*` l'éteignent.
- Double-clic sur une carte d'agent ou de sous-agent : quitte la toile et ouvre sa conversation (`selectAgent`,
  `openSubagent`).

## La toile

- Fond à points, repris du workspace de Kata avec les couleurs d'Escouade : `radial-gradient` d'un point de 1,2 px tous
  les 18 px, dont le pas et l'origine suivent la caméra.
- Se déplacer : glisser le fond, ou la molette. Zoomer : Ctrl/⌘ + molette ou pincement, autour du pointeur, de 30 % à
  250 %. Clavier : `+`, `-`, `0` (ajuster).
- Une barre flottante en haut au centre : `−`, le pourcentage, `+`, « Ajuster », puis « n agents · n questions ».
- À l'ouverture, la toile s'ajuste pour montrer tous les agents. La caméra n'est pas mémorisée.
- Les agents sont rangés par Escouade, dans l'ordre de la barre latérale, en lignes qui passent à la suivante. Un agent
  lancé par un autre (`drivenAgents`) est posé à sa droite, relié par un trait pointillé « a lancé ».

## Les trois altitudes

Le zoom choisit ce qu'on montre. Les seuils (mon choix, à régler à l'usage) : de loin sous 45 %, à mi-hauteur de 45 à
60 %, de près au-delà (60 % demandé par l'utilisateur le 26 novembre, au lieu de 80 %).

### De loin : le disque et ses satellites

D'après le croquis de l'utilisateur. Un disque par agent :

- en haut, un point et l'état en capitales (« En cours », « À toi », « Terminé », « Erreur ») ;
- le nom de l'agent, en gras ;
- une ligne de ce qu'il fait (`cardActivity`) ;
- la jauge du plan en segments, un par tâche, et « 3/7 · 43 % » ; « sans plan » sinon.

Le contour prend la couleur de l'état (`--run`, `--wait`, `--del`, gris quand il a fini). Un agent qui attend
l'utilisateur a un fond teinté de jaune et un halo qui pulse.

Les **satellites** sont des arcs posés sur le flanc droit du disque, centrés sur l'horizontale. Seuls ceux qui ont
quelque chose à dire sont montrés, toujours dans cet ordre de haut en bas :

| Satellite | Ce qu'il dit | Couleur |
|---|---|---|
| équipe | nombre de sous-agents et de tâches de fond en cours | vert |
| outils | nombre de skills et de plugins utilisés | violet |
| livrables | nombre d'artefacts | bleu |
| CI | « CI » et son état en un mot ; un déploiement de même | vert, rouge en échec |
| question | nombre de questions et de permissions en attente | jaune |

Une orbite en pointillé passe sous les satellites ; elle tourne tant que l'agent travaille. Un clic sur un satellite
zoome sur l'agent, du côté concerné.

### À mi-hauteur : des piles

La boussole, mais chaque côté est une pile : « Outils ×4 », « Équipe ×4 », « Livrables ×3 », et le plan réduit à son
titre et sa jauge. Un clic sur une pile la déplie sans changer le zoom.

### De près : la boussole dépliée

Un cadre par agent (un agent et tout ce qui lui appartient), l'agent au centre.

**Un agent qui attend l'utilisateur est toujours déplié**, quelle que soit l'altitude : sa carte et sa question se
voient sans zoomer.

## La boussole, côté par côté

### Au centre : la carte de l'agent

- En-tête : point d'état, nom, état en mots.
- Une ligne : modèle, effort, durée, jetons, coût (les données de la carte de la barre latérale).
- Une ligne de ce qu'il fait, et sa ligne d'avancement (`progressLine`) s'il en a une.
- En bas, la **barre de saisie** : un champ d'une ligne, un bouton Paramètres, un bouton Dicter. Paramètres ouvre un
  menu : modèle, effort, mode (`api.setAgentOptions`), « Joindre un fichier ». Dicter est le `MicButton` de la
  conversation, branché sur ce champ.
- `Entrée` envoie **par le même chemin que la conversation** : ce que `Composer.svelte` décide aujourd'hui avant
  `sendDraft` (le texte répond-il à la question en attente, le mode retenu, la boîte d'envoi, le brouillon rendu en
  cas de refus) est sorti dans une fonction partagée, appelée par les deux. C'est un remaniement de `Composer`, à
  faire dans le lot 2, avec ses tests.
- Le brouillon est celui de l'agent (`getDraft`, `setDraft`) : ce qu'on tape ici se retrouve dans sa conversation en
  quittant la toile, et l'inverse. Des fichiers déjà joints au brouillon sont dits par une pastille « n fichiers » à
  côté du champ.
- Contour vert quand il tourne, jaune avec un halo quand il attend, rouge en erreur.

**La question.** Quand l'agent a des demandes en attente (`requests`), sa carte grandit vers le bas et montre, entre
ses lignes et la barre de saisie, l'encart jaune de la conversation : `QuestionCard` et `PermissionCard` montés tels
quels, avec plusieurs questions, les réponses multiples, les descriptions, « Envoyer ». Ces deux composants prennent
un élément de conversation, pas `requests` : la toile charge la conversation d'un agent tant qu'il attend (voir « D'où
viennent les données »). La question d'un sous-agent (`sub`) s'affiche sur la carte du sous-agent, qui grandit comme
celle d'un agent ; la carte de l'agent prend un contour jaune et dit « question de <sous-agent> ». Dans la vue de
loin, le satellite question de l'agent compte aussi celles de ses sous-agents. La carte ne dépasse pas 70 %
de la hauteur de la fenêtre (à 100 % de zoom) ; au-delà, l'encart défile dans la carte. Les objets voisins se
décalent ; rien ne se chevauche.

### Au nord : le plan

Une bande horizontale au-dessus de l'agent, reliée par un trait : la pastille « Plan », son titre, « 3/7 tâches ·
43 % », la jauge en segments de `PlanBanner`, puis une tuile par tâche (numéro, titre, ce qu'elle fait, sa jauge). La
tâche en cours est cerclée de vert, les tâches faites sont estompées. Au-delà de 9 tâches, la bande montre les faites
repliées en « n faites », la ou les tâches en cours et les suivantes. Sans plan, pas de bande.

### À l'ouest : l'outillage

Les skills et les plugins que l'agent a utilisés, en cartes plus carrées (coins de 3 px, liseré de couleur en haut)
pour dire « de la doc, pas un agent » :

- **skill** : icône de parchemin, violet. Un skill est un appel de l'outil `Skill` ; son nom est `input.skill`.
- **plugin** : icône de boîte, bleu. Un plugin est un serveur d'outils : le `<serveur>` d'un outil
  `mcp__<serveur>__<outil>`, avec son nombre d'appels.

Aucun trait permanent. Pendant qu'un appel est en cours (`status: 'running'`), la carte de l'outil a un anneau vert et
un flux part d'elle vers l'agent ; à la fin de l'appel, le trait disparaît.

### À l'est : l'équipe

Les sous-agents et les tâches de fond de l'agent (`sidebarSubs`, `sidebarBg`), reliés à lui.

- Un sous-agent a la même carte que l'agent, en plus petit : nom (son type), pastille « tâche n » quand on la connaît
  (`planTask`), modèle, durée, jetons, ce qu'il fait (`doing`). Tant qu'il tourne, sa carte a un champ et Dicter, sans
  Paramètres (`api.messageSubagent`).
- Un sous-agent lancé par un sous-agent (`parent`) est posé à droite de son parent et relié à lui.
- Une tâche de fond est une carte au contour pointillé : son genre, sa commande, sa durée. Celle d'un sous-agent est
  posée à sa droite.
- En cours : trait vert animé, avec un point qui circule de l'agent vers lui. Terminé : trait gris fixe. Échec : rouge.
- Les sous-agents terminés sont repliés en une ligne « + n terminés · n échecs », qu'un clic déplie. Un échec non vu
  reste visible hors de la pile.

### Au sud : la zone d'atterrissage

Une zone matérialisée sous l'agent : contour pointillé, fond hachuré, repères d'angle, titre « Zone d'atterrissage ·
n posés, n en approche ». Elle est toujours dessinée ; vide, elle dit « Rien n'a encore atterri ». Un seul trait y
descend depuis l'agent.

Un livrable **en approche** (pas fini) plane au-dessus de son emplacement, dessiné en pointillé ; fini, il **se pose**.

Ce qui s'y pose :

- **L'artefact** : une petite fenêtre de navigateur (trois points, l'adresse), le titre de la page, « Artefact ·
  version n · il y a x min », et « Ouvrir ↗ » qui l'ouvre dans le navigateur. Une carte par adresse ; une nouvelle
  publication à la même adresse met la carte à jour. Source : les appels de l'outil `Artifact` et `artifactLink`
  (`lib/tools.ts`). Le titre est celui que l'appel donne (`title`), sinon le nom du fichier publié ; « version n »
  est le nombre de publications vues à cette adresse. Tant que l'appel tourne, l'adresse n'est pas connue :
  l'artefact est en approche, sous le nom de son fichier.
- **La famille « ruban »** : CI locale, CI distante, déploiement. Même carte, coins de 6 px, un ruban animé en tête :
  rayures en biais vertes pour une CI, tirets bleus pour un déploiement. En échec, le ruban se fige et passe au rouge.
  - en-tête : le genre, son numéro s'il en a un, l'état et la durée ;
  - une ligne : la commande (locale), ou la branche et le commit (distante), ou « version → environnement » ;
  - les étapes en pastilles reliées, quand on les connaît ; sinon « n contrôles, n en échec » ;
  - en pied : où elle tourne, et un bouton : « Sortie » (locale, ouvre `RunView` ou `TaskView`) ou « Ouvrir ↗ »
    (l'hébergeur, par l'adresse du contrôle ou du journal du déploiement).

## D'où viennent les données

La fenêtre ne charge les conversations qu'une à la fois, et les relâche après dix minutes
(`lib/conversations.svelte.ts`). Or l'outillage, les artefacts et la CI locale se lisent dans les appels d'outils.
Charger la conversation de chaque agent pour la toile coûterait en mémoire et en appels.

Choix validé : le cœur tient, pour chaque agent, un **résumé de toile** qu'il envoie avec l'agent et met à jour à chaque
appel d'outil : les skills et plugins utilisés (nom, nombre d'appels, appel en cours ou non), pour lui et pour chacun
de ses sous-agents, et ses artefacts (adresse, titre, nombre de publications, heure). Seule exception : la
conversation d'un agent qui attend est chargée par la toile, le temps de sa question, pour monter `QuestionCard` et
`PermissionCard` sans les réécrire.

| Besoin | Aujourd'hui | À faire |
|---|---|---|
| État, activité, coût d'un agent | `Agent`, `cardActivity`, `cardSpend` | rien |
| Plan | `planView` (`lib/plan.ts`) | rien |
| Sous-agents, tâches de fond | `PlanState.agents`, `bgTasks` ; `sidebarSubs` et `sidebarBg` ne servent que l'agent choisi et plafonnent leurs lignes | les lire pour tous les agents, sans plafond d'affichage |
| Agents pilotés | `drivenAgents` | rien |
| Questions, permissions | `QuestionCard`, `PermissionCard`, sur un élément de conversation | charger la conversation de l'agent qui attend |
| Écrire, régler, dicter | `Composer`, `setAgentOptions`, `MicButton` | sortir le chemin d'envoi de `Composer` |
| Skills, plugins, artefacts | dans les appels d'outils des conversations | **cœur** : le résumé de toile |
| CI distante | `BranchCi` (un état, un total, les échecs) ; `PrDetail.checks` pour la PR d'un agent | une carte ; ses étapes sont les contrôles de la PR quand l'agent en a une |
| CI locale | les lancements de test du Kanban (`test:<agent>:…`, vus dans `RunView`) ; les tâches de fond | les reconnaître (ci-dessous) |
| Déploiement | `Deployment`, `PrDeployment`, `app.environments`, la vue « Environnements » | une carte, rattachée à un agent |

Précisions :

- `PlanState` garde au plus 30 sous-agents et 40 tâches de fond : au plafond, la pile dit « 30+ terminés ».
- **CI distante** : sans PR, la carte montre le résumé de `BranchCi` de la branche de l'agent, « n contrôles, n en
  échec ».
- **CI locale** : trois sources. Un lancement de test du Kanban pour cet agent ; « Sortie » ouvre `RunView`. Une
  tâche de fond, ou un appel de terminal en cours, dont la commande est le réglage `testCommand` du Kanban ou commence
  par lui. Et une commande qui ressemble à une suite de tests, reconnue par une liste de motifs dans une fonction pure
  testée (`cargo test`, `cargo nextest`, `npm test`, `npm run test`, `pnpm test`, `yarn test`, `bun test`, `vitest`,
  `jest`, `pytest`, `go test`, `make test`, `mvn test`, `gradle test`, `dotnet test`, `rspec`, `phpunit`…). Pour
  ces deux-là, « Sortie » ouvre `TaskView`. Validé par l'utilisateur : mieux vaut une carte de trop qu'une CI
  manquée.
- **Déploiement** : le cœur les lit déjà. Un déploiement se pose chez un agent quand il vient de sa PR
  (`PrDeployment`) ou que son commit est sur la branche de l'agent. Les autres restent dans la vue « Environnements ».
  En approche tant que son état est en cours, posé une fois fini.

## Construction

- Le placement est une fonction pure, `lib/workspace-layout.ts` : de l'état des agents vers des boîtes et des traits,
  pour chaque altitude. Testée seule (vitest), comme `lib/workflow-diagram.ts`.
- Les traits sont un seul `<svg>` sous les cartes ; les cartes sont du HTML positionné, pour garder les champs, le
  focus et les composants existants.
- Composants sous `src/components/workspace/` : la vue, le disque, la carte d'agent, la bande du plan, la carte
  d'outil, la carte d'équipe, la zone d'atterrissage, la carte d'artefact, la carte à ruban.
- Couleurs et rayons : les variables de `app.css` (`--run`, `--wait`, `--del`, `--info`, `--raise`, `--line-2`…), dans
  les deux thèmes. Une seule couleur à ajouter, le violet des skills.
- Textes par `t()`, en français et en anglais.
- Accessibilité : chaque carte est atteignable au clavier dans l'ordre nord, ouest, centre, est, sud ; les états sont
  dits en mots et pas seulement en couleur ; `prefers-reduced-motion` fige flux, halo, orbite et ruban.
- Mode serveur : la vue lit l'état de la fenêtre, elle marche donc pareil pour une machine distante.

## Les lots

Chacun est livrable seul et devient une carte Kanban.

1. **La toile** : le menu du bouton Éditeur, l'état, le fond, se déplacer, zoomer, quitter, et la vue de loin (disques
   et satellites équipe et question ; les autres satellites arrivent avec leur lot). Le placement connaît les
   altitudes dès ce lot.
2. **La boussole** : la bascule à deux niveaux (disque sous 45 %, boussole au-delà), la carte de l'agent avec sa barre
   de saisie (et le chemin d'envoi sorti de `Composer`), le plan au nord, l'équipe à l'est, les traits et les flux.
3. **Les questions** : l'encart jaune dans la carte, et l'agent en attente toujours déplié.
4. **L'outillage** : le résumé de toile dans le cœur, les cartes de skill et de plugin à l'ouest, le flux pendant
   l'appel.
5. **La zone d'atterrissage** : la zone, l'artefact, la CI distante, la CI locale, le déploiement.
6. **Les altitudes** : le niveau du milieu, avec ses piles.

Un clic sur un satellite zoome sur l'agent ; il cadre le côté concerné une fois ce côté livré.

## Choix validés le 25 novembre

- Le cœur tient un résumé de toile par agent.
- Écrire depuis une carte passe par le chemin d'envoi sorti de `Composer`, partagé avec la conversation.
- La question d'un sous-agent s'affiche sur la carte du sous-agent.
- Une CI locale se reconnaît aussi aux commandes qui ressemblent à des tests.
- La spec reste dans le dépôt, à côté des autres specs.

## Choix validés le 25 novembre, après le lot 1

- Les lancements du Kanban (`test:<agent>:…`) sont les serveurs de la recette, pas des tests : ils ne comptent pas comme
  CI locale. Une CI locale est une commande de tests lancée par l'agent ou un sous-agent.
- Les tests ont leur case à part : une carte de tests collée sous la carte de l'agent ou du sous-agent qui les lance,
  et non dans la zone d'atterrissage.
- Un déploiement se rattache à un agent par sa PR, et aussi par sa branche : le cœur apprend à savoir si le commit d'un
  déploiement est sur la branche d'un agent (travail dans le cœur, lot 5).
- Écrire à un sous-agent suit la règle actuelle du cœur (un sous-agent en arrière-plan qui a un `taskId`).
- Les cartes à ruban montrent la branche, pas le commit ni la version, qu'aucune donnée de la fenêtre ne porte.
- Le bouton Éditeur ouvre un menu sur tout le bouton.
- La toile reprend le design d'Escouade 2.3, pas celui des maquettes : la carte d'un agent est celle de la barre latérale
  (point d'état, état en minuscules, modèle · durée, température, tokens · coût), le plan est le bandeau actuel (pastille
  « Plan », titre, « n/m tâches · %  », jauge en segments, lignes numérotées 01, 02… avec leur cercle d'état), posé
  au-dessus de l'agent ; surfaces, bordures, rayons et boutons sont ceux de la zone de travail, et le fond de la toile
  prend la couleur de la zone de travail, teintée par la couleur du projet.

## Changements du 26 novembre (après essai)

Demandés par l'utilisateur après avoir vu la toile :

1. **Plus de disques ni de satellites.** La vue de loin disparaît : en dessous de 60 %, la toile montre les boussoles en
   piles (jusqu'au zoom minimal, 30 %) ; à partir de 60 %, tout est déplié. Un agent qui attend reste déplié partout.
2. **Les agents s'empilent verticalement**, l'un sous l'autre, dans l'ordre de la barre latérale. Un agent lancé par un
   autre prend sa place dans cet ordre (le trait « a lancé » reste).
3. **Une toile de tous les projets**, depuis la « Vue d'ensemble » : un bouton « Workspace » y revient, et la toile y
   montre les agents de tous les projets ; les agents d'un même projet empilés en colonne, sous le nom du projet (avec
   sa couleur), et les projets côte à côte, dans l'ordre du rail, avec un espace net entre deux colonnes.
4. **À l'ouverture, la toile est à 100 %**, pas ajustée à l'écran : le premier agent (ou la première colonne) en haut à
   gauche, sous la barre flottante. « Ajuster » garde son rôle.

## Mes choix, à corriger

L'utilisateur n'a pas tranché ces points ; j'ai pris ma recommandation.

- La toile montre les agents du projet ouvert, sans les archivés.
- Les sous-agents terminés sont repliés.
- On peut écrire à un sous-agent tant qu'il tourne.
- La carte d'artefact n'a pas de miniature de la page.
- La caméra n'est pas mémorisée ; le seuil des piles est 60 % (il n'y a plus de vue de loin, 26 novembre).
- Un déploiement se pose chez l'agent dont il déploie la PR ou la branche.
- L'ordre des lots.

## Laissé à qui construit

- Les dimensions exactes des cartes et les écarts, à partir des maquettes.
- L'algorithme de rangement des cadres et l'évitement des chevauchements quand une carte grandit.
- Le dessin exact des deux icônes (parchemin, boîte) dans le jeu d'icônes de l'app.
- La durée des transitions entre altitudes.
