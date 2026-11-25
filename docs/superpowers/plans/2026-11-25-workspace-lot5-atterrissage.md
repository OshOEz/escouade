# Workspace, lot 5 : la zone d'atterrissage et les cartes de tests — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** au sud de chaque boussole, une zone d'atterrissage toujours dessinée où se posent les artefacts publiés, la
CI distante et les déploiements de l'agent ; sous la carte de l'agent ou d'un sous-agent, collée, la carte de ses
derniers tests ; de loin, les satellites « livrables » et « CI ».

**Architecture:** le résumé de toile du cœur (lot 4) apprend les publications de l'outil `Artifact` et les derniers
appels de terminal de chaque fil (l'agent, chaque sous-agent). Le cœur apprend aussi quels déploiements viennent de la
branche d'un agent (`ProjectEnvironments.branches`). Côté fenêtre, deux fonctions pures : `testsOf`
(`lib/workspace-tests.ts`, avec `looksLikeTests`) fait les cartes de tests, `landingOf` (`lib/workspace-landing.ts`)
les cartes de la zone. Une carte à ruban (`RibbonCard`) sert aux tests, à la CI et aux déploiements ; `frameOf` colle
les cartes de tests sous leur carte et pose la zone au sud.

**Tech Stack:** Rust (serde, serde_json, git en sous-processus), Svelte 5, TypeScript, vitest, catalogues `t()` fr/en.

**Spec:** `docs/superpowers/specs/2026-11-25-vue-workspace-design.md` (« Au sud : la zone d'atterrissage », « D'où
viennent les données » et ses précisions, « De loin » (satellites livrables et CI), et surtout « Choix validés le 25
novembre, après le lot 1 », qui l'emportent sur le reste de la spec).

**Base:** `feat/workspace` après les lots 3 et 4. Du lot 3 : `FrameOpts.teamH`, `cardMaxH`, `waits`, `layoutFarOpen`,
`onsize` de `AgentCard`/`TeamCard`/`Compass`, l'état `heights` de `WorkspaceView`. Du lot 4 : `Canvas` dans
`crates/escouade-plan/src/canvas.rs` (`on_tool_use(parent, id, name, input, now)`, `on_tool_result(id, is_error, text,
now)`, `close_running(now)`, `ThreadKit { call, tools }`, `MAX_THREADS`), `AgentMeta::canvas`, `Agent.canvas?:
CanvasSummary`, `kit` dans `src/test/workspace.ts`. Le code construit l'emporte sur un nom de ce plan ; un écart se
note « Ruling: ».

**Brief:** `.superpowers/brief-lot.md` (builds Rust, Docker, disque, aperçu, base des échecs connus).

## Global Constraints

- Les lancements du Kanban (`test:<agent>:…`) sont les serveurs de la recette : ils ne sont ni des tests ni une CI, et
  n'apparaissent nulle part dans ce lot.
- Une carte de tests est « collée sous la carte de l'agent ou du sous-agent qui les lance, et non dans la zone
  d'atterrissage ». Un passage de tests : une tâche de fond ou un appel de terminal de cet agent ou sous-agent dont la
  commande est le réglage `testCommand` du Kanban ou commence par lui, ou ressemble à une suite de tests (« une liste de
  motifs dans une fonction pure testée » : `cargo test`, `cargo nextest`, `npm test`, `npm run test`, `pnpm test`,
  `yarn test`, `bun test`, `vitest`, `jest`, `pytest`, `go test`, `make test`, `mvn test`, `gradle test`, `dotnet test`,
  `rspec`, `phpunit`…). « Mieux vaut une carte de trop qu'une CI manquée. »
- La carte de tests : le style ruban (rayures en biais vertes tant qu'ils tournent, rouge et figé en échec), la
  commande, la durée, et « Sortie » qui ouvre la sortie (`TaskView` pour une tâche de fond, la conversation sur l'appel
  pour un appel de terminal). Le placement la met sous la bonne carte et décale ce qui est dessous ; rien ne se
  chevauche.
- La zone garde : les artefacts, la CI distante, les déploiements. « Contour pointillé, fond hachuré, repères d'angle,
  titre « Zone d'atterrissage · n posés, n en approche ». Toujours dessinée ; vide, elle dit « Rien n'a encore atterri ».
  Un seul trait y descend depuis l'agent. » « En approche (pas fini) plane au-dessus de son emplacement, en pointillé ;
  fini, il se pose. »
- Artefact : une petite fenêtre de navigateur (trois points, l'adresse), le titre, « Artefact · version n · il y a x
  min », « Ouvrir ↗ » ; une carte par adresse ; titre = `title` de l'appel, sinon le nom du fichier publié ; « version
  n » = publications vues à cette adresse ; en approche tant que l'appel tourne, sous le nom de son fichier. Pas de
  miniature.
- Ruban : rayures en biais vertes pour tests et CI, tirets bleus pour un déploiement, animé seulement tant que ça tourne ;
  en échec, figé et rouge. Les cartes à ruban montrent la branche, jamais un commit ni une version.
- Un déploiement se pose chez un agent par sa PR, et aussi quand son commit est sur la branche de l'agent ; les autres
  restent dans « Environnements ». En approche tant que son état est en cours.
- Design d'Escouade 2.3 (commit a3de5e91) : surfaces, bordures, rayons de la carte d'équipe (`TeamCard.svelte`,
  `.card`), boutons `btn` de l'app, état en minuscules, jetons de `src/app.css` ; jamais le style des maquettes.
- Satellites, ordre fixe : équipe, outils, livrables, CI, question. Livrables : nombre d'artefacts, `--info`. CI :
  « CI » et son état, `--run`, `--del` en échec.
- Ordre clavier (DOM) : nord, ouest, centre, est, sud ; une carte de tests suit dans le DOM la carte sous laquelle elle
  est, avec le même `data-side`.
- `prefers-reduced-motion` fige tout ruban.
- Aucune dépendance ajoutée (pas de `regex` en Rust). Textes par `t()`, zone `workspace`, apostrophe typographique `’`
  comme les catalogues fr.
- Commits en Conventional Commits, en français.

## Review Focus

- **Une commande qui contient « test » sans en être** (`git commit -m "test"`, `cat test.txt`, `npm run build`) : pas
  de carte ; `cd app && npx vitest run` en est une (Task 4, `sees none in …`).
- **L'agent meurt pendant `cargo test` au premier plan** : la carte ne tourne pas pour toujours, elle finit en échec
  (Task 1, `a_command_cut_short_ends_failed`).
- **Un déploiement de production d'un commit de `main`, plus ancien que la branche de l'agent** : il est ancêtre de la
  branche mais ne se pose pas chez l'agent (Task 2, `a_deployment_lands_on_the_agents_whose_branch_has_its_commit`).
- **Une carte de tests sous un sous-agent posé à droite de son parent** : la ligne suivante de la colonne passe sous
  elle, sans chevauchement (Task 6, `pushes the next row below the tests of a child`).
- **La PR ne se lit pas** (hôte injoignable, quota) : la carte de CI garde l'état de `PullRequest.checks`, sans étapes
  (Task 7, `keeps the pull request's state when its detail cannot be read`).

---

### Task 1: Les artefacts et les appels de terminal dans le résumé de toile

**Files:**
- Modify: `crates/escouade-plan/src/canvas.rs`
- Test: `crates/escouade-plan/src/canvas_tests.rs` ; le module `tests` de `src-tauri/src/agent.rs` (aides `call`,
  `returns`)

**Interfaces:**
- Consumes (lot 4) : `Canvas`, `ThreadKit`, `on_tool_use`, `on_tool_result`, `close_running`, `is_empty`, `Change`,
  `MAX_THREADS`.
- Produces :
  - `pub const MAX_ARTIFACTS: usize = 20;` `pub const MAX_COMMANDS: usize = 5;` `pub const MAX_COMMAND: usize = 300;`
  - `pub struct Artifact { pub url: String, pub title: String, pub versions: u32, pub at: i64 }`
  - `pub struct Publishing { pub call: String, pub name: String, pub since: i64 }`
  - `pub struct CommandRun { pub call: String, pub command: String, pub started_at: i64, pub ended_at: Option<i64>, pub failed: bool }`
  - `Canvas` gagne `pub artifacts: Vec<Artifact>`, `pub publishing: Vec<Publishing>`, `pub commands: Vec<CommandRun>` ;
    `ThreadKit` gagne `pub commands: Vec<CommandRun>`. Tous camelCase, `default`, `skip_serializing_if = "Vec::is_empty"`.
  - `pub fn artifact_url(text: &str) -> Option<String>` — la première adresse `https://claude.ai/artifact/<id>` ou
    `https://<sous-domaine>.claude.ai/artifact/<id>` (`<id>` : `[A-Za-z0-9_-]+`), précédée d'un début de texte ou d'un
    caractère ni lettre, ni chiffre, ni `_`, `.`, `/`, `-` : la règle de `ARTIFACT_URL` (`src/lib/tools.ts` l. 120),
    écrite à la main.
  - `on_tool_use` (l'appel déjà vu ne compte pas, comme au lot 4) :
    - `Artifact`, de tout fil, `input.action` absent ou `"publish"` : un `Publishing` (`name` = `input.title` non vide,
      sinon le dernier segment de `input.file_path`) → `View`.
    - `Bash` sans `input.run_in_background == true` : un `CommandRun` dans le fil de l'appel (`commands` de `Canvas` si
      `parent` est absent, sinon de l'entrée `subs` de ce `call`, créée au besoin), commande rognée et coupée à
      `MAX_COMMAND` caractères (pas octets) → `View` ; au-delà de `MAX_COMMANDS` dans un fil, le plus ancien part.
  - `on_tool_result` : un `Publishing` de cet appel part ; s'il réussit et que `artifact_url(text)` trouve une adresse,
    l'`Artifact` de cette adresse prend `versions += 1`, `title` = `name`, `at = now` (créé à `versions: 1` sinon ;
    au-delà de `MAX_ARTIFACTS`, le plus ancien `at` part) → `Saved`, sinon `View`. Un `CommandRun` de cet appel (tout
    fil) prend `ended_at = Some(now)`, `failed = is_error` → `View`.
  - `close_running(now)` : `publishing` vidé ; tout `CommandRun` sans `ended_at` prend `ended_at = Some(now)`,
    `failed = true`.
  - `is_empty` compte aussi `artifacts` et `commands`.

Ce que le lot 4 laisse à changer dans `canvas.rs` (constaté à sa revue finale) :
- `on_tool_use` commence par `let Some((kind, kit)) = kit_of(name, input) else { return Change::None; };` puis
  `self.seen.first_time(id)` : un appel `Artifact` ou `Bash` repart avant d'être vu. Traiter `Artifact` et `Bash`
  avant ce `let … else` (chacun appelle d'abord `self.seen.first_time(id)` et rend `Change::None` s'il est déjà vu),
  puis garder le `let … else` pour les skills et les plugins.
- `open` (`HashMap<String, (Option<String>, KitKind, String)>`) ne tient que les appels de skills et de plugins :
  `on_tool_result` rend `Change::None` dès qu'il n'y trouve pas l'appel. Chercher le `Publishing` et le `CommandRun`
  de l'appel avant ce retour (ou ajouter à `open` une variante pour eux).
- `is_empty` ne compte que `tools` et les `tools` de `subs` : y ajouter `artifacts`, `commands`, `publishing` et les
  `commands` de chaque `ThreadKit`, sinon `canvas_apply` (agent.rs) jette un résumé qui n'a que des artefacts.
- `MAX_THREADS` évince d'abord un fil sans appel en cours : le test de `close_running` d'un fil doit compter ses
  `commands` sans `ended_at` comme « en cours ».
- `agent.rs` (`plan_tool_use`) ne passe plus au résumé un appel que la conversation tient déjà terminé (relu après un
  redémarrage) : rien à faire pour les artefacts et les commandes, la règle vaut pour eux.

Choix du plan : le cœur garde les cinq derniers appels de terminal de chaque fil sans les trier ; c'est la fenêtre qui
reconnaît les tests, avec le réglage `testCommand` qu'elle a sous la main (l'agent du cœur ne connaît pas son Kanban).
Limite : un passage de tests suivi de cinq autres appels de terminal du même fil n'a plus de carte ; une tâche de fond
de tests, elle, reste (le plan en garde 40).

Choix du plan : seul l'outil `Bash` (Claude Code) est lu ; les outils de terminal des autres harness n'ont pas de carte
de tests d'appel (leurs tâches de fond, si le plan les connaît, en ont une).

Choix du plan : un artefact publié par un sous-agent est un livrable de l'agent et se pose dans sa zone.

- [ ] **Step 1: Write the failing tests**

```rust
const URL: &str = "https://claude.ai/artifact/AbC_12-x";

#[test]
fn finds_the_address_an_artifact_was_published_at() {
    assert_eq!(artifact_url(&format!("Published to {URL}.")), Some(URL.into()));
    assert_eq!(artifact_url("see https://team.claude.ai/artifact/z9"), Some("https://team.claude.ai/artifact/z9".into()));
    assert_eq!(artifact_url("x/https://claude.ai/artifact/z9"), None);
    assert_eq!(artifact_url("nothing"), None);
}

#[test]
fn a_page_published_again_is_one_artifact() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "p1", "Artifact", &json!({"file_path": "/tmp/report.html"}), 1);
    assert_eq!(c.publishing[0].name, "report.html");
    assert_eq!(c.on_tool_result("p1", false, &format!("Published {URL}"), 2), Change::Saved);
    for (i, call) in ["p2", "p3"].iter().enumerate() {
        c.on_tool_use(None, call, "Artifact", &json!({"title": "Rapport", "file_path": "/tmp/report.html"}), 3 + i as i64);
        c.on_tool_result(call, false, &format!("Updated {URL}"), 4 + i as i64);
    }
    assert!(c.publishing.is_empty());
    assert_eq!(c.artifacts.len(), 1);
    assert_eq!((c.artifacts[0].versions, c.artifacts[0].title.as_str()), (3, "Rapport"));
}

#[test]
fn a_failed_or_other_artifact_call_lands_nothing() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "p1", "Artifact", &json!({"title": "x"}), 1);
    c.on_tool_result("p1", true, URL, 2);
    c.on_tool_use(None, "p2", "Artifact", &json!({"action": "list"}), 3);
    assert!(c.artifacts.is_empty() && c.publishing.is_empty());
}

#[test]
fn keeps_the_last_commands_of_each_thread() {
    let mut c = Canvas::default();
    for i in 0..7 {
        c.on_tool_use(None, &format!("b{i}"), "Bash", &json!({"command": format!("echo {i}")}), i);
    }
    c.on_tool_use(Some("a1"), "sb", "Bash", &json!({"command": " cargo test "}), 20);
    c.on_tool_use(None, "bg", "Bash", &json!({"command": "npm run dev", "run_in_background": true}), 21);
    assert_eq!(c.commands.len(), MAX_COMMANDS);
    assert_eq!(c.commands[0].command, "echo 2");
    assert_eq!((c.subs[0].call.as_str(), c.subs[0].commands[0].command.as_str()), ("a1", "cargo test"));
}

#[test]
fn a_command_ends_failed_or_not_by_its_result_and_is_cut_on_a_character() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "b1", "Bash", &json!({"command": "é".repeat(400)}), 1);
    c.on_tool_use(Some("a1"), "b2", "Bash", &json!({"command": "npm test"}), 2);
    assert_eq!(c.commands[0].command.chars().count(), MAX_COMMAND);
    assert_eq!(c.on_tool_result("b1", true, "exit code 101", 5), Change::View);
    c.on_tool_result("b2", false, "", 6);
    assert_eq!((c.commands[0].ended_at, c.commands[0].failed), (Some(5), true));
    assert_eq!((c.subs[0].commands[0].ended_at, c.subs[0].commands[0].failed), (Some(6), false));
}

#[test]
fn a_command_cut_short_ends_failed() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "b1", "Bash", &json!({"command": "cargo test"}), 1);
    c.close_running(5);
    assert_eq!((c.commands[0].ended_at, c.commands[0].failed), (Some(5), true));
}
```

Et dans `src-tauri/src/agent.rs`, `the_canvas_lands_an_artifact_published` : `call(&mut a, None, "p1", "Artifact",
json!({"title": "R"}))`, `returns(&mut a, "p1", "Published https://claude.ai/artifact/z9", Value::Null)` ;
`serde_json::to_value(a.view()).unwrap()["canvas"]["artifacts"][0]` a `url` `https://claude.ai/artifact/z9`, `title`
`R`, `versions` 1.

- [ ] **Step 2: Run them to see them fail**

Run: `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo test --manifest-path src-tauri/Cargo.toml -p escouade-plan canvas`
Expected: FAIL à la compilation, `artifact_url` introuvable.

- [ ] **Step 3: Implement in `crates/escouade-plan/src/canvas.rs`**

- [ ] **Step 4: Run the crate, its clippy, then the agent's test in Docker**

Run: `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo test --manifest-path src-tauri/Cargo.toml -p escouade-plan && CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 cargo clippy --manifest-path src-tauri/Cargo.toml -p escouade-plan -- -D warnings`
puis `sh spikes/lot0/run.sh --desktop-libs sh -c 'CARGO_BUILD_JOBS=1 cargo test --manifest-path src-tauri/Cargo.toml --lib -- the_canvas'`
Expected: PASS ; `df -h /` au-dessus de 40 Go libres.

- [ ] **Step 5: Commit**

```bash
rustfmt --edition 2021 crates/escouade-plan/src/canvas.rs crates/escouade-plan/src/canvas_tests.rs src-tauri/src/agent.rs
git add crates/escouade-plan/src src-tauri/src/agent.rs
git commit -m "feat(cœur): le résumé de toile garde les artefacts publiés et les derniers appels de terminal de chaque fil"
```

### Task 2: Les déploiements de la branche d'un agent, dans le cœur

**Files:**
- Modify: `src-tauri/src/deploys.rs` (`ProjectEnvironments`, `project_environments_read` l. ~310-380)
- Modify: `src-tauri/src/branchci.rs` (`ci_target`, l. 220 : `async fn` → `pub(crate) async fn`)
- Test: `src-tauri/src/deploys_tests.rs`

**Interfaces:**
- Consumes: `crate::forge::Deployment` (`environment`, `state`, `url`, `sha`, `created`, `log`) ; `client.environments`
  (la liste lue, du plus récent au plus ancien, aperçus compris) ; `Core::project_agents`, `Core::project`,
  `Core::ci_target` ; `git::text(cwd, args)` ; `AgentMeta::{archived, worktree, found_worktree}`.
- Produces:
  - `pub struct BranchDeployment { pub deployment: Deployment, pub agents: Vec<String> }` — `Debug, Clone, PartialEq,
    Serialize, Default`, camelCase.
  - `ProjectEnvironments.branches: Vec<BranchDeployment>` — `#[serde(skip_serializing_if = "Vec::is_empty")]`.
  - `pub fn branch_deployments(list: &[Deployment], own: &[(String, HashSet<String>)]) -> Vec<BranchDeployment>` — dans
    l'ordre de `list`, un déploiement au `sha` non vide est gardé avec les agents dont l'ensemble contient ce `sha` et
    qui n'ont pas encore de déploiement gardé dans cet environnement (le plus récent par environnement et par agent) ;
    sans agent, il part.
  - `async fn own_commits(&self, project: &Project) -> Vec<(String, HashSet<String>)>` (dans `impl Core` de
    `deploys.rs`) — pour chaque agent non archivé du projet avec un `worktree` sans `repos` ou un `found_worktree` :
    `git rev-list --max-count=200 <cible>..HEAD` dans son dossier, `<cible>` = `ci_target(project)` ; un échec
    (dossier parti, cible inconnue) laisse l'agent de côté.
  - `project_environments_read`, branche `Ok(list)` : `out.branches = branch_deployments(&list,
    &self.own_commits(&project).await)` ; un hôte sans déploiements vide `branches` avec `environments`.

Choix du plan : « le commit est sur la branche de l'agent » veut dire « parmi les commits propres de sa branche »
(`<cible>..HEAD`) : un vieux déploiement de `main` est ancêtre de toute branche et ne doit se poser chez personne. Un
appel `git rev-list` par agent et par lecture (toutes les 60 s au plus, et seulement pour un projet suivi) remplace un
`merge-base --is-ancestor` par déploiement : pas de cache à tenir.

Choix du plan : les aperçus (`Preview`, `pr-12`…), que `summarize` écarte des environnements, comptent ici : c'est le
déploiement le plus courant d'une branche.

Choix du plan : un agent d'un projet de plusieurs dépôts (`worktree.repos` non vide) n'a pas de déploiement par
branche (son dossier n'est pas un dépôt).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_deployment_lands_on_the_agents_whose_branch_has_its_commit() {
    let list = [
        dep("Preview", "success", "b2"),
        dep("Preview", "success", "b1"),
        dep("production", "success", "m1"),
        dep("staging", "in_progress", "c1"),
        dep("staging", "success", ""),
    ];
    let set = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<HashSet<_>>();
    let own = [("a1".to_string(), set(&["b1", "b2"])), ("a2".to_string(), set(&["c1"]))];
    let got = branch_deployments(&list, &own);
    assert_eq!(
        got.iter()
            .map(|b| (b.deployment.environment.as_str(), b.deployment.sha.as_str(), b.agents.clone()))
            .collect::<Vec<_>>(),
        [("Preview", "b2", vec!["a1".to_string()]), ("staging", "c1", vec!["a2".to_string()])]
    );
}

#[tokio::test]
async fn an_agents_branch_deployed_is_told_and_main_is_not() {
    use crate::core_tests::{git, harness};
    use serde_json::json;
    let h = harness("deploys-branch");
    let server = crate::integrations::fake::FakeServer::start().await;
    let (id, _, _) = crate::agentprs_tests::github_agent(&h, &server).await;
    let dir = std::path::PathBuf::from(h.agent(&id).worktree.unwrap().path);
    let own = git(&dir, &["rev-parse", "HEAD"]).trim().to_string();
    let main = git(&dir, &["rev-parse", "main"]).trim().to_string();
    let d = |n: u64, env: &str, sha: &str| {
        json!({"id": n, "environment": env, "sha": sha, "ref": "x",
               "created_at": "2026-11-01T10:00:00Z", "creator": {"login": "ada"}})
    };
    server.on("GET", "/repos/acme/demo/deployments", 200, json!([d(2, "Preview", &own), d(1, "production", &main)]));
    for n in [1, 2] {
        server.on("GET", &format!("/repos/acme/demo/deployments/{n}/statuses"), 200,
                  json!([{"state": "success", "environment_url": "https://example.com"}]));
    }
    let envs = h.core.project_environments_read(&h.agent(&id).project_id).await;
    assert_eq!(envs.error, None);
    assert_eq!(envs.branches.len(), 1, "main's production is no deployment of the agent");
    assert_eq!(envs.branches[0].deployment.sha, own);
    assert_eq!(envs.branches[0].agents, [id.clone()]);
    assert_eq!(serde_json::to_value(&envs).unwrap()["branches"][0]["agents"][0], json!(id));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `sh spikes/lot0/run.sh --desktop-libs sh -c 'CARGO_BUILD_JOBS=1 cargo test --manifest-path src-tauri/Cargo.toml --lib deploys::tests'`
Expected: FAIL à la compilation, `branch_deployments` introuvable.

- [ ] **Step 3: Implement `BranchDeployment`, `branch_deployments`, `own_commits`, and fill `branches`**

- [ ] **Step 4: Run the deploys' and branch CI's tests**

Run: `sh spikes/lot0/run.sh --desktop-libs sh -c 'CARGO_BUILD_JOBS=1 cargo test --manifest-path src-tauri/Cargo.toml --lib -- deploys:: branchci::'`
Expected: PASS ; un échec se juge contre la base (relancer le test seul, lire le code).

- [ ] **Step 5: Commit**

```bash
rustfmt --edition 2021 src-tauri/src/deploys.rs src-tauri/src/deploys_tests.rs src-tauri/src/branchci.rs
git add src-tauri/src/deploys.rs src-tauri/src/deploys_tests.rs src-tauri/src/branchci.rs
git commit -m "feat(cœur): un déploiement dit les agents dont la branche porte son commit"
```

### Task 3: Ce qui se pose chez un agent, et ses satellites

**Files:**
- Modify: `src/lib/types.ts` (`CanvasSummary` du lot 4 ; `ProjectEnvironments` l. ~1571 ; `BranchDeployment`)
- Create: `src/lib/workspace-landing.ts`, `src/lib/workspace-landing.test.ts`
- Modify: `src/lib/workspace-layout.ts` (`Satellite`, `discOf`), `src/lib/workspace-layout.test.ts`
- Modify: `src/test/workspace.ts` (aides)

**Interfaces:**
- Consumes: `Agent`, `PrDetail`, `PrCheck`, `PrDeployment`, `BranchCi`, `PullRequest`, `ProjectEnvironments`,
  `Deployment` (`lib/types.ts`) ; `discOf` (lot 1).
- Produces:
  - Types : `CommandRun = { call: string; command: string; startedAt: number; endedAt?: number | null; failed: boolean }` ;
    `CanvasSummary` gagne `artifacts?: { url: string; title: string; versions: number; at: number }[]`,
    `publishing?: { call: string; name: string; since: number }[]`, `commands?: CommandRun[]`, et chaque `subs[i]`
    `commands?: CommandRun[]` ; `BranchDeployment = { deployment: Deployment; agents: string[] }` ;
    `ProjectEnvironments.branches?: BranchDeployment[]`.
  - `type RibbonState = 'running' | 'success' | 'failure'` ; `type Step = { name: string; state: RibbonState }`
  - `type Opens = { kind: 'task'; task: string } | { kind: 'call'; call: string } | { kind: 'url'; url: string } | null`
  - `type Ribbon = { kind: 'tests' | 'ci' | 'deploy'; id: string; number: number | null; state: RibbonState; since: number | null; until: number | null; line: string; steps: Step[] | null; total: number | null; failed: number | null; where: string; opens: Opens }`
    — `since`/`until` en ms ; `where` = l'hôte (`new URL(…).host`) de l'adresse que le bouton ouvre, `''` sans adresse.
  - `type ArtifactLanding = { kind: 'artifact'; id: string; url: string | null; title: string; versions: number; at: number }`
  - `type Landing = ArtifactLanding | Ribbon` ; `landed(l: Landing): boolean` — un artefact : `url !== null` ; un ruban :
    `state !== 'running'`.
  - `type LandingCtx = { pr: PrDetail | null; environments: ProjectEnvironments | null }`
  - `landingOf(agent: Agent, ctx: LandingCtx): Landing[]` — dans cet ordre :
    1. artefacts posés (`canvas.artifacts`, le plus récent `at` d'abord), puis en approche (`canvas.publishing`,
       `url: null`, `title` = `name`, `versions: 0`, `at` = `since`) ;
    2. CI distante : la première PR `open` de `agent.prs` → `number`, `line` = `head`, état par `checks` (`pending` →
       `running`, `failure` → `failure`, `success` → `success` ; `null` : pas de carte), `steps` = les `PrCheck` de
       `ctx.pr` (`cancelled` ou `failure` → `failure`, `pending` → `running`, sinon `success`) sinon `null`, `since`/`until`
       = le plus petit `started` et, tout fini, le plus grand `finished` (×1000) sinon `null`, `opens` = `pr.url`. Sans
       PR ouverte : `agent.branchCi` au `checks` non nul → `number: null`, `line` = `branch`, `steps: null`,
       `total`/`failed` = `checksTotal`/`checksFailed`, `opens` = l'`url` du premier `failed`, sinon `null` ;
    3. déploiements : chaque `PrDeployment` de `ctx.pr` (`line` = `<head de la PR> → <environment>`, `opens` = `url`),
       puis chaque `ctx.environments.branches` dont `agents` contient `agent.id` et dont l'environnement n'a pas déjà de
       carte (`line` = `<branche de l'agent> → <environment>`, branche = `worktree.branch ?? foundWorktree.branch`,
       `since` = `created × 1000`, `opens` = `log ?? url`). État : `success`, `active`, `inactive` → `success` ;
       `failure`, `error` → `failure` ; le reste → `running`.
  - `Satellite` devient `{ kind: SatKind; count: number; state?: RibbonState }` ;
    `discOf(agent: Agent, landing: Landing[] = [])` ajoute `deliverables` (nombre d'artefacts, absent à 0) et `ci`
    (`count` = rubans `ci` et `deploy` ; `state` le pire : `failure` > `running` > `success` ; absent sans ruban), à leur
    place dans l'ordre fixe.

Choix du plan : un déploiement venu de la PR et un autre venu de la branche pour le même environnement ne font qu'une
carte, celle de la PR.

Choix du plan : `inactive` (un succès qu'un plus récent a remplacé) compte comme réussi, pas comme un échec.

Choix du plan : le satellite CI ne compte pas les tests locaux ; ils se voient sur leur carte, sous l'agent.

- [ ] **Step 1: Write the failing tests**

```ts
const ctx = (over: Partial<LandingCtx> = {}): LandingCtx => ({ pr: null, environments: null, ...over });
it('lands an artifact by its address, and keeps one being published above', () => {
  const a = agent({ canvas: { tools: [], subs: [], artifacts: [{ url: 'https://claude.ai/artifact/z', title: 'R', versions: 2, at: 5 }], publishing: [{ call: 'p', name: 'brouillon.html', since: 6 }] } });
  const r = landingOf(a, ctx());
  expect(r).toEqual([
    expect.objectContaining({ kind: 'artifact', title: 'R', versions: 2 }),
    expect.objectContaining({ kind: 'artifact', url: null, title: 'brouillon.html' }),
  ]);
  expect(r.map(landed)).toEqual([true, false]);
});
it('takes the steps of the pull request, else the branch\'s summary', () => {
  const pr = pullRequest({ number: 12, head: 'feat/x', checks: 'failure', url: 'https://h/pr/12', state: 'open' });
  const detail = { ...prDetail(), checks: [check('lint', 'success'), check('test', 'failure')], deployments: [{ environment: 'staging', state: 'in_progress', url: 'https://s' }] };
  const r = landingOf(agent({ prs: [pr] }), ctx({ pr: detail }));
  expect(r[0]).toMatchObject({ kind: 'ci', number: 12, state: 'failure', line: 'feat/x', where: 'h', steps: [{ name: 'lint', state: 'success' }, { name: 'test', state: 'failure' }] });
  expect(r[1]).toMatchObject({ kind: 'deploy', line: 'feat/x → staging', state: 'running' });
  const b = landingOf(agent({ branchCi: branchCi({ checks: 'failure', checksTotal: 5, checksFailed: 2 }) }), ctx());
  expect(b[0]).toMatchObject({ kind: 'ci', steps: null, total: 5, failed: 2 });
});
it('lands a deployment of the agent\'s branch, the pull request\'s first, never a commit', () => {
  const dep = (environment: string, sha: string, agents: string[]) => ({ deployment: { environment, state: 'success', url: 'https://p', sha, log: 'https://h/job/1' }, agents });
  const environments = { ...projectEnvs(), branches: [dep('Preview', 'abc1234', ['a1']), dep('staging', 'abc1234', ['a1']), dep('Preview', 'fff', ['a2'])] };
  const a = agent({ id: 'a1', worktree: { path: '/w', branch: 'feat/y', baseBranch: 'main' } });
  const r = landingOf(a, ctx({ environments }));
  expect(r.map((l) => (l as Ribbon).line)).toEqual(['feat/y → Preview', 'feat/y → staging']);
  expect(r[0]).toMatchObject({ state: 'success', opens: { kind: 'url', url: 'https://h/job/1' } });
  expect(JSON.stringify(r)).not.toContain('abc1234');
  const withPr = agent({ ...a, prs: [pullRequest({ head: 'feat/y', state: 'open', checks: null })] });
  const fromPr = { ...prDetail(), deployments: [{ environment: 'staging', state: 'success', url: 'https://s' }] };
  expect(landingOf(withPr, ctx({ pr: fromPr, environments })).filter((l) => l.kind === 'deploy').map((l) => (l as Ribbon).line)).toEqual(['feat/y → staging', 'feat/y → Preview']);
});
it('lands nothing for an agent with nothing', () => {
  expect(landingOf(agent(), ctx())).toEqual([]);
});
// workspace-layout.test.ts
it('says the deliverables and the worst CI on the disc', () => {
  const landing = [art(true), art(false), ribbon('deploy', 'success'), ribbon('ci', 'failure')];
  expect(discOf(agent(), landing).satellites).toEqual([
    { kind: 'deliverables', count: 2 },
    { kind: 'ci', count: 2, state: 'failure' },
  ]);
});
```

Aides à ajouter à `src/test/workspace.ts` (objets minimaux des types de `lib/types.ts`) : `prDetail()`,
`check(name, state)`, `projectEnvs()`, `art(landed)` (une `ArtifactLanding`, `url` nulle quand `landed` est faux),
`ribbon(kind, state)` (un `Ribbon` aux champs vides), `cmd(call, command, over?)` (un `CommandRun` fini, réussi,
`startedAt: 1`, `endedAt: 9`). `pullRequest` et `branchCi` sont déjà dans `src/test/ipc.ts`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-landing.test.ts src/lib/workspace-layout.test.ts`
Expected: FAIL, module `workspace-landing` introuvable.

- [ ] **Step 3: Implement the types, `landingOf`, `landed`, and the two satellites in `discOf`**

- [ ] **Step 4: Run them and the type check**

Run: `npx vitest run src/lib/workspace-landing.test.ts src/lib/workspace-layout.test.ts && npm run check`
Expected: PASS ; `check` n'a que les 3 erreurs connues d'apps/mobile.

- [ ] **Step 5: Commit**

```bash
npx prettier --write src/lib/types.ts src/lib/workspace-landing.ts src/lib/workspace-landing.test.ts src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/test/workspace.ts
git add src/lib/types.ts src/lib/workspace-landing.ts src/lib/workspace-landing.test.ts src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/test/workspace.ts
git commit -m "feat(workspace): ce qui se pose chez un agent, artefacts, CI distante et déploiements"
```

### Task 4: Reconnaître des tests, et la carte de tests de chaque fil

**Files:**
- Create: `src/lib/workspace-tests.ts`, `src/lib/workspace-tests.test.ts`

**Interfaces:**
- Consumes: `Ribbon`, `RibbonState` (Task 3) ; `CommandRun`, `Agent` (Task 3) ; `exitCode`, `ownerOf`
  (`lib/bgtasks.ts`).
- Produces:
  - `looksLikeTests(command: string, testCommand?: string): boolean` — vrai quand `testCommand` (rogné, non vide) est
    la commande rognée ou son début ; sinon quand un des morceaux de la commande (coupée sur `&&`, `||`, `;`, `|`),
    débarrassé des affectations `NOM=valeur` de tête et des lanceurs `npx`, `bunx`, `pnpm exec`, `yarn exec`,
    `python -m`, `python3 -m`, `uv run`, `poetry run`, `bundle exec`, commence par un motif (mot entier) : `cargo test`,
    `cargo nextest`, `npm test`, `npm run test`, `npm t`, `pnpm test`, `pnpm run test`, `yarn test`, `yarn run test`,
    `bun test`, `bun run test`, `vitest`, `jest`, `pytest`, `go test`, `make test`, `mvn test`, `gradle test`,
    `./gradlew test`, `dotnet test`, `rspec`, `phpunit`, ou un chemin qui finit par `/phpunit`, `/rspec`, `/pytest`.
    Les motifs sont une liste exportée, `TEST_PATTERNS`.
  - `type TestRun = Ribbon & { kind: 'tests'; under: string }` — `under` : `'centre'` (l'agent) ou l'`id` du sous-agent.
  - `testsOf(agent: Agent, testCommand: string): TestRun[]` — les passages : `canvas.commands` (`under: 'centre'`),
    `canvas.subs[i].commands` (`under` = `subs[i].call`), et les tâches de fond `kind: 'command'` de `plan.bgTasks`
    (`under` = `ownerOf(plan, t)?.id ?? 'centre'`), ceux dont `looksLikeTests(command, testCommand)` ; par `under`, le
    plus récent `startedAt` seul ; `'centre'` d'abord, puis dans l'ordre de `plan.agents`. Champs : `id` =
    `tests:<call ou id de tâche>`, `line` = la commande, `since` = `startedAt`, `until` = `endedAt ?? null`,
    `number`/`steps`/`total`/`failed` `null`, `where: ''`, `opens` = `{ kind: 'call', call }` (appel) ou
    `{ kind: 'task', task: id }` (tâche de fond). État : un appel sans `endedAt` → `running`, `failed` → `failure`,
    sinon `success` ; une tâche `running` → `running`, `done` avec `exitCode` nul ou 0 → `success`, sinon (`failed`,
    `stopped`, `interrupted`, autre code) → `failure`.

Choix du plan : une carte de tests par carte, celle du dernier passage ; les plus anciens ne se montrent pas.

Choix du plan : un passage arrêté à la main (`stopped`) se dit en échec : ses tests n'ont pas tous passé.

- [ ] **Step 1: Write the failing tests**

```ts
it.each([
  'cargo test -p escouade-plan',
  'cargo nextest run',
  'cd app && npx vitest run src/lib',
  'CI=1 npm test',
  'npm run test -- --watch=false',
  'python -m pytest -q',
  'go test ./...',
  'bundle exec rspec spec/a_spec.rb',
  'vendor/bin/phpunit',
  './gradlew test',
])('sees a test suite in %s', (c) => expect(looksLikeTests(c)).toBe(true));
it.each(['npm run build', 'cat test.txt', 'git commit -m "test"', 'ls tests/', 'cargo build --tests', 'echo jest', 'npm run dev'])(
  'sees none in %s',
  (c) => expect(looksLikeTests(c)).toBe(false),
);
it("takes the board's test command, and what starts with it", () => {
  expect(looksLikeTests('./scripts/check.sh --fast', './scripts/check.sh')).toBe(true);
  expect(looksLikeTests('./scripts/other.sh', './scripts/check.sh')).toBe(false);
  expect(looksLikeTests('anything', '  ')).toBe(false);
});
it('keeps the latest run of each card, a call or a background task', () => {
  const a = agent({
    plan: planWith([], {
      agents: [{ ...sub('s1'), taskId: 'k1' }],
      bgTasks: [{ ...bg('t1'), command: 'npm test', parent: 'k1', startedAt: 5 }, { ...bg('t2'), command: 'npm run dev' }],
    }),
    canvas: {
      tools: [],
      subs: [{ call: 's1', tools: [], commands: [cmd('c2', 'cargo test', { startedAt: 2 })] }],
      commands: [cmd('c0', 'cargo test'), cmd('c1', 'pytest -q', { startedAt: 3, endedAt: null }), cmd('c3', 'ls', { startedAt: 4 })],
    },
  });
  expect(testsOf(a, '')).toEqual([
    expect.objectContaining({ under: 'centre', line: 'pytest -q', state: 'running', opens: { kind: 'call', call: 'c1' } }),
    expect.objectContaining({ under: 's1', line: 'npm test', state: 'running', opens: { kind: 'task', task: 't1' } }),
  ]);
});
it('says a run failed by its result or its exit code', () => {
  const failedCall = agent({ canvas: { tools: [], subs: [], commands: [cmd('c', 'npm test', { failed: true })] } });
  expect(testsOf(failedCall, '')[0].state).toBe('failure');
  const done = (summary: string) => agent({ plan: planWith([], { bgTasks: [{ ...bg('t'), command: 'npm test', status: 'done', summary }] }) });
  expect(testsOf(done('Background command "x" completed (exit code 1)'), '')[0].state).toBe('failure');
  expect(testsOf(done('Background command "x" completed (exit code 0)'), '')[0].state).toBe('success');
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-tests.test.ts`
Expected: FAIL, module introuvable.

- [ ] **Step 3: Implement `TEST_PATTERNS`, `looksLikeTests` and `testsOf` in `src/lib/workspace-tests.ts`**

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-tests.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
npx prettier --write src/lib/workspace-tests.ts src/lib/workspace-tests.test.ts
git add src/lib/workspace-tests.ts src/lib/workspace-tests.test.ts
git commit -m "feat(workspace): reconnaître un passage de tests, le dernier de chaque agent et sous-agent"
```

### Task 5: La carte à ruban, la carte d'artefact et la zone

**Files:**
- Create: `src/components/workspace/RibbonCard.svelte`, `src/components/workspace/ArtifactCard.svelte`,
  `src/components/workspace/LandingZone.svelte`
- Modify: `src/components/workspace/Disc.svelte` (`satWord` de `deliverables` et `ci`, leurs couleurs, `.sat.ci.failure`)
- Modify: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/components/workspace/RibbonCard.test.ts`, `src/components/workspace/LandingZone.test.ts`,
  `src/components/workspace/Disc.test.ts`

**Interfaces:**
- Consumes: `Landing`, `ArtifactLanding`, `Ribbon`, `landed` (Task 3) ; `fAgo(seconds, now)`, `fDur(ms)`
  (`lib/format`) ; `openUrl` (`lib/opener`) ; `app.openTask`, `app.selectAgent`, `app.now` ; `conversationOf(id).reveal(call)`
  (`lib/conversations.svelte`) ; `observeHeight` (`lib/resize`).
- Produces:
  - `<RibbonCard agentId={string} item={Ribbon} side={'centre' | 'east' | 'south'} />` — `<article data-side={side}>`,
    `aria-label` `workspace.ribbon.label` ; la surface et le rayon de `.card` de `TeamCard` ; en tête un ruban de 4 px :
    `.ribbon.stripes` (rayures en biais `--run`, `tests` et `ci`) ou `.ribbon.dashes` (tirets `--info`, `deploy`),
    animé seulement avec `.running`, `.failure` figé en `--del` ; l'en-tête : genre (`workspace.ribbon.kind.*`),
    `#number` s'il y en a un, l'état en minuscules (`workspace.ribbon.state.*`), la durée (`fDur`, en cours :
    `app.now - since`, fini : `until - since`, rien sans `since`) ; la `line` en mono sur une ligne ; les `steps` en
    pastilles reliées, sinon `workspace.ribbon.checks` quand `total` n'est pas nul ; en pied `where` et le bouton `btn` :
    `opens.kind` `task` → `app.openTask(agentId, task)` ; `call` → `app.selectAgent(agentId)` puis
    `conversationOf(agentId).reveal(call)` (libellé `workspace.ribbon.output` pour les deux) ; `url` → `openUrl(url)`
    (libellé `workspace.land.open`) ; `null` : pas de bouton.
  - `<ArtifactCard item={ArtifactLanding} />` — un `<article data-side="south">` : la fenêtre (trois points, l'adresse sans `https://`, ou le titre en
    approche), le titre, `workspace.land.artifact` (`ago` = `fAgo(at / 1000, app.now)`), « Ouvrir ↗ » → `openUrl(url)` ;
    en approche, pas de bouton.
  - `<LandingZone agentId={string} items={Landing[]} onsize={(h: number) => void} />` — `<section data-side="south">` :
    contour pointillé `--line-2`, fond hachuré (`repeating-linear-gradient` de `--line-2`), quatre repères d'angle, titre
    `workspace.land.title` ; vide : `workspace.land.empty`. Les en approche dans un rang `.coming` au-dessus, en
    pointillé, décalés de 14 px ; les posés dans un rang `.landed` ; chaque rang passe à la ligne. `onsize` reçoit sa
    hauteur (`observeHeight`).
  - Disc : `deliverables` (`workspace.sat.deliverables`, `--info`) et `ci` (`workspace.sat.ci`, `--run`, `.failure` en
    `--del`).

Choix du plan : la sortie d'un appel de terminal s'ouvre dans la conversation de l'agent, sur l'appel (`reveal`
remonte à l'appel du sous-agent qui l'a fait, comme la recherche dans les conversations) ; `TaskView` ne montre que les
tâches de fond.

Choix du plan : les cartes à ruban prennent le rayon de la carte d'équipe (9 px), pas les 6 px des maquettes, comme le
veut le choix « design d'Escouade 2.3 ».

Textes (fr ; en en miroir) :

```ts
land: {
  title: 'Zone d’atterrissage · {landed} posés, {coming} en approche',
  empty: 'Rien n’a encore atterri',
  artifact: 'Artefact · version {n} · {ago}',
  open: 'Ouvrir ↗',
},
ribbon: {
  label: '{kind}, {state}',
  output: 'Sortie',
  checks: '{total} contrôles, {failed} en échec',
  kind: { tests: 'Tests', ci: 'CI', deploy: 'Déploiement' },
  state: { running: 'en cours', success: 'réussi', failure: 'en échec' },
},
```

et dans `sat` : `deliverables: { one: 'livrable', other: 'livrables' }`, `ci: 'CI'`.

- [ ] **Step 1: Write the failing tests**

```ts
// RibbonCard.test.ts
it('says a test run, its command and its state, and opens its output where it is', async () => {
  resetApp({ agents: [agent({ id: 'a1' })] });
  app.openWorkspace('p1');
  const item = { ...ribbon('tests', 'failure'), line: 'npm test', since: 0, until: 65_000, opens: { kind: 'task', task: 't1' } as const };
  const { container } = render(RibbonCard, { agentId: 'a1', item, side: 'centre' });
  expect(screen.getByRole('article', { name: 'Tests, en échec' })).toBeTruthy();
  expect(screen.getByText('npm test')).toBeTruthy();
  expect(container.querySelector('.ribbon.stripes.failure')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Sortie' }));
  expect(app.taskView.a1).toBe('t1');
  expect(app.workspaceOn).toBe(false);
});
it('opens a terminal call in the conversation, on the call', async () => {
  resetApp({ agents: [agent({ id: 'a1' })] });
  render(RibbonCard, { agentId: 'a1', item: { ...ribbon('tests', 'success'), opens: { kind: 'call', call: 'c1' } }, side: 'centre' });
  await fireEvent.click(screen.getByRole('button', { name: 'Sortie' }));
  expect(conversationOf('a1').jump).toBe('c1');
});
it('draws a deployment in dashes, moving only while it runs', () => {
  const { container } = render(RibbonCard, { agentId: 'a1', item: ribbon('deploy', 'running'), side: 'south' });
  expect(container.querySelector('.ribbon.dashes.running')).toBeTruthy();
});
// LandingZone.test.ts
it('is drawn even empty', () => {
  render(LandingZone, { agentId: 'a1', items: [], onsize: () => {} });
  expect(screen.getByText('Rien n’a encore atterri')).toBeTruthy();
  expect(screen.getByText('Zone d’atterrissage · 0 posés, 0 en approche')).toBeTruthy();
});
it('lands what is finished and keeps the rest above', () => {
  const { container } = render(LandingZone, { agentId: 'a1', items: [art(true), ribbon('ci', 'running')], onsize: () => {} });
  expect(screen.getByText('Zone d’atterrissage · 1 posés, 1 en approche')).toBeTruthy();
  expect(container.querySelectorAll('.coming .ribbon.stripes')).toHaveLength(1);
  expect(container.querySelectorAll('.landed article')).toHaveLength(1);
});
// Disc.test.ts
it('says the CI in a word, red when it fails', () => {
  const a = agent();
  const { container } = render(DiscHost, { disc: { ...discOf(a, [ribbon('ci', 'failure')]), x: 0, y: 0, w: 300, h: 300 }, agent: a, onpick: () => {} });
  expect(screen.getByText('CI')).toBeTruthy();
  expect(container.querySelector('.sat.ci.failure')).toBeTruthy();
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/RibbonCard.test.ts src/components/workspace/LandingZone.test.ts src/components/workspace/Disc.test.ts`
Expected: FAIL, composants introuvables.

- [ ] **Step 3: Write the catalogue keys, `RibbonCard`, `ArtifactCard`, `LandingZone`, then the satellites of `Disc`**

- [ ] **Step 4: Run them and the type check**

Run: `npx vitest run src/components/workspace src/lib/i18n && npm run check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
npx prettier --write src/components/workspace/RibbonCard.svelte src/components/workspace/ArtifactCard.svelte src/components/workspace/LandingZone.svelte src/components/workspace/*.test.ts src/components/workspace/Disc.svelte src/lib/i18n/fr/workspace.ts src/lib/i18n/en/workspace.ts
git add src/components/workspace src/lib/i18n/fr/workspace.ts src/lib/i18n/en/workspace.ts
git commit -m "feat(workspace): la carte à ruban, la carte d'artefact et la zone d'atterrissage"
```

### Task 6: La carte de tests collée sous sa carte

**Files:**
- Modify: `src/lib/workspace-layout.ts` (`FrameOpts`, `Frame`, `teamOf`, `computeFrame`, la clé de `frameOf`)
- Modify: `src/components/workspace/Compass.svelte`, `src/components/workspace/WorkspaceView.svelte`
- Test: `src/lib/workspace-layout.test.ts`, `src/components/workspace/WorkspaceView.test.ts`

**Interfaces:**
- Consumes: `testsOf`, `TestRun`, `looksLikeTests` (Task 4) ; `RibbonCard` (Task 5) ; `frameOf`, `teamOf`, `FrameOpts`
  (avec `teamH` du lot 3), `Placed`, `GAP` ; `project.board.testCommand`.
- Produces:
  - `export const TEST = { h: 76, gap: 8 } as const`.
  - `FrameOpts` gagne `testCommand?: string` ; `Frame` gagne `tests: Placed<TestRun>[]`.
  - `teamOf(agent, opts)` laisse de côté une tâche de fond en cours dont `looksLikeTests(command, opts.testCommand)` :
    c'est sa carte de tests.
  - `computeFrame` : `testsOf(agent, opts.testCommand ?? '')` ; un passage `under: 'centre'` se colle sous la carte de
    l'agent (`x = centre.x`, `y = centre.y + centre.h + TEST.gap`, `w = CARD.w`, `h = TEST.h`) ; un passage sous un
    sous-agent placé dans `team` se colle sous sa carte (même `x` et `w`) ; un passage dont le sous-agent n'est pas sur
    la toile (replié dans la pile, oublié du plan) ne se montre pas. Dans la colonne de l'est, un rang (une racine et ce
    qui est posé à sa droite) va jusqu'au plus bas de ses cartes et de leurs cartes de tests ; la racine suivante se
    pose sous ce bas, plus l'écart actuel. Aucun trait vers une carte de tests. Le cadre englobe tout.
  - La clé de `frameOf` compte `testCommand`.
  - `Compass` reçoit `testCommand` par le cadre seulement : il rend chaque `frame.tests` dans un `slot`, par
    `<RibbonCard agentId item side>` (`side` : `centre` pour `under: 'centre'`, sinon `east`), juste après dans le DOM la
    carte sous laquelle il est.
  - `WorkspaceView` passe `testCommand: project.board.testCommand` à chaque `frameOf`.

Choix du plan : la carte de tests a une hauteur fixe (`TEST.h`) : une ligne de commande, coupée à l'écran ; elle n'a
pas à être mesurée.

- [ ] **Step 1: Write the failing tests**

```ts
// workspace-layout.test.ts
const team = (...subs: SubAgent[]) => planWith([], { agents: subs });
it('sticks a test run under the card that runs it, and moves what is below', () => {
  const plain = agent({ plan: team(sub('s1'), sub('s2')) });
  const tested = agent({ plan: plain.plan, canvas: { tools: [], subs: [{ call: 's1', tools: [], commands: [cmd('c1', 'npm test')] }], commands: [cmd('c0', 'cargo test')] } });
  const f = frameOf(tested);
  const under = (id: string) => f.tests.find((t) => t.under === id)!;
  expect(under('centre')).toMatchObject({ x: f.centre.x, y: f.centre.y + f.centre.h + TEST.gap, w: CARD.w, h: TEST.h });
  const s1 = f.team.find((i) => i.id === 's1')!;
  expect(under('s1')).toMatchObject({ x: s1.x, y: s1.y + s1.h + TEST.gap, w: s1.w });
  const was = frameOf(plain).team.find((i) => i.id === 's2')!;
  expect(f.team.find((i) => i.id === 's2')!.y - was.y).toBe(TEST.gap + TEST.h);
  expect(f.links.map((l) => l.id).filter((id) => id.includes('c0') || id.includes('c1'))).toEqual([]);
});
it('pushes the next row below the tests of a child', () => {
  const a = agent({
    plan: team(sub('s1'), { ...sub('s2'), parent: 's1' }, sub('s3')),
    canvas: { tools: [], subs: [{ call: 's2', tools: [], commands: [cmd('c', 'npm test')] }] },
  });
  const f = frameOf(a);
  const s2 = f.team.find((i) => i.id === 's2')!;
  const s3 = f.team.find((i) => i.id === 's3')!;
  expect(s2.y).toBe(f.team.find((i) => i.id === 's1')!.y);
  expect(s3.y).toBeGreaterThan(s2.y + s2.h + TEST.gap + TEST.h);
});
it('leaves a background test run out of the team: it is its test card', () => {
  const a = agent({ plan: planWith([], { bgTasks: [{ ...bg('t1'), command: 'npm test' }, { ...bg('t2'), command: 'npm run dev' }] }) });
  const f = frameOf(a);
  expect(f.team.map((i) => i.id)).toEqual(['t2']);
  expect(f.tests.map((t) => t.under)).toEqual(['centre']);
  expect(frameOf(agent({ plan: a.plan }), { testCommand: 'npm run dev' }).team).toEqual([]);
});
it('shows no test card for a subagent folded in the pile', () => {
  const a = agent({ plan: team(sub('s1', 'done')), canvas: { tools: [], subs: [{ call: 's1', tools: [], commands: [cmd('c', 'npm test')] }] } });
  expect(frameOf(a).tests).toEqual([]);
});
// WorkspaceView.test.ts
it('draws the test card right after its card, in the keyboard order', async () => {
  resetApp({ agents: [agent({ id: 'a1', canvas: { tools: [], subs: [], commands: [cmd('c0', 'cargo test')] } })] });
  const { container } = show();
  await zoomTo(1);
  const cards = [...container.querySelectorAll('[data-agent="a1"] [data-side]')];
  const at = cards.findIndex((e) => e.getAttribute('aria-label')?.startsWith('Tests'));
  expect(cards[at - 1].getAttribute('data-side')).toBe('centre');
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-layout.test.ts src/components/workspace/WorkspaceView.test.ts`
Expected: FAIL, `f.tests` indéfini.

- [ ] **Step 3: Implement the tests in `frameOf` and `teamOf`, then `Compass` and `WorkspaceView`**

- [ ] **Step 4: Run the workspace's tests and the type check**

Run: `npx vitest run src/lib src/components/workspace && npm run check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
npx prettier --write src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/components/workspace/Compass.svelte src/components/workspace/WorkspaceView.svelte src/components/workspace/WorkspaceView.test.ts
git add src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/components/workspace
git commit -m "feat(workspace): la carte des tests collée sous l'agent ou le sous-agent qui les lance"
```

### Task 7: La zone au sud de la boussole, et les données qu'elle lit

**Files:**
- Create: `src/lib/workspace-ci.svelte.ts`, `src/lib/workspace-ci.test.ts`
- Modify: `src/lib/workspace-layout.ts` (`FrameOpts.southH`, `Frame.south`, son trait)
- Modify: `src/components/workspace/Compass.svelte`, `src/components/workspace/WorkspaceView.svelte`
- Test: `src/lib/workspace-layout.test.ts`, `src/components/workspace/WorkspaceView.test.ts`

**Interfaces:**
- Consumes: `landingOf`, `LandingCtx`, `Landing` (Task 3) ; `LandingZone` (Task 5) ; `api.agentPrDetail(id, number,
  hostRepo?)` → `PrRead<PrDetail>`, `api.projectEnvironments(projectId)` (`lib/ipc.ts`) ; `app.environments` ;
  `envsView.refresh` (`lib/envs-view.svelte.ts`) ; `heights` et `onsize` (lot 3).
- Produces:
  - Dans `src/lib/workspace-ci.svelte.ts` :
    - `prDetails: Record<string, PrDetail>` (`$state`, par agent) et `readPrDetail(agent: Agent): Promise<void>` — lit la
      première PR `open` de l'agent (`hostRepo` de la PR) ; garde `data` quand elle vient, ne touche à rien sur une
      `failure` ou une erreur ; sans PR ouverte, oublie celle de l'agent.
    - `followEnvironments(projectId: string): () => void` — ce que fait `EnvironmentsView.svelte` en s'ouvrant (l. 33-39) :
      `api.projectEnvironments` dans `app.environments`, `envsView.refresh` si jamais lu ou lu il y a plus d'une minute,
      puis `api.projectEnvironments` toutes les 5 minutes pour que le cœur garde le projet suivi ; renvoie l'arrêt.
  - `export const SOUTH = { h: 132 } as const` (la zone vide) ; `FrameOpts` gagne `southH?: number` (la hauteur mesurée de
    la zone) ; `Frame` gagne `south: Box` — toujours là, sous tout le reste du cadre (`GAP` plus bas), à `x: 0`, de la
    largeur du cadre ; un trait `state: 'plan'`, `id: '<agent>:south'`, droit, du bas de la carte de l'agent (ou de sa
    carte de tests) au haut de la zone. La clé de `frameOf` compte `southH`.
  - `Compass` reçoit `landing: Landing[]` et rend `<LandingZone agentId items={landing} onsize={(h) => onsize('south', h)} />`
    dans `frame.south`, en dernier dans le DOM.
  - `WorkspaceView` : `landingOf(a, { pr: prDetails[a.id] ?? null, environments: app.environments[project.id] ?? null })`
    par agent (un `$derived` en `Map`), passée à `discOf` et à `Compass` ; `southH: heights[a.id]?.south` dans
    `frameOf` ; `pick(agentId, 'south')` centre la zone ; `followEnvironments(project.id)` à l'ouverture, arrêté à la
    fermeture ; à la vue de près, un `$effect` par agent montré appelle `readPrDetail(agent)` à l'ouverture et quand
    `${pr.number}:${pr.checks}` change.

Attention : `ordered.order.map(discOf)` passerait l'index en second argument de `discOf` ; écrire
`(a) => discOf(a, landing.get(a.id))`.

Choix du plan : la toile ne lit le détail d'une PR (contrôles, déploiements) que pour les boussoles à l'écran, à leur
ouverture puis quand l'état de ses contrôles change ; de loin, le satellite CI s'en tient à `PullRequest.checks`, à
`BranchCi` et aux déploiements de branche.

Choix du plan : la toile ouverte suit les environnements du projet comme la vue « Environnements » ouverte (le cœur ne
lit les déploiements que d'un projet suivi) ; elle ne lit rien d'autre.

- [ ] **Step 1: Write the failing tests**

```ts
// workspace-ci.test.ts
it("reads the detail of the agent's open pull request", async () => {
  const backend = fakeBackend({ agent_pr_detail: () => ({ data: { ...prDetail(), checks: [check('ci', 'success')] } }) });
  await readPrDetail(agent({ id: 'a1', prs: [pullRequest({ number: 7, state: 'open' })] }));
  expect(backend.called('agent_pr_detail')[0].args).toMatchObject({ id: 'a1', number: 7 });
  expect(prDetails.a1.checks[0].name).toBe('ci');
});
it("keeps the pull request's state when its detail cannot be read", async () => {
  fakeBackend({ agent_pr_detail: () => ({ failure: { kind: 'unreachable', message: 'x' } }) });
  const a = agent({ id: 'a2', prs: [pullRequest({ state: 'open', checks: 'failure' })] });
  await readPrDetail(a);
  expect(prDetails.a2).toBeUndefined();
  expect(landingOf(a, { pr: null, environments: null })[0]).toMatchObject({ kind: 'ci', state: 'failure', steps: null });
});
it('follows the environments of the project while it is open', async () => {
  vi.useFakeTimers();
  const backend = fakeBackend({ project_environments: () => ({ ...projectEnvs(), projectId: 'p1', readAt: Date.now() }) });
  const stop = followEnvironments('p1');
  await vi.advanceTimersByTimeAsync(5 * 60 * 1000);
  expect(backend.called('project_environments')).toHaveLength(2);
  stop();
  await vi.advanceTimersByTimeAsync(5 * 60 * 1000);
  expect(backend.called('project_environments')).toHaveLength(2);
  vi.useRealTimers();
});
// workspace-layout.test.ts
it('always draws the landing zone south, below the tests, with one link down to it', () => {
  const f = frameOf(agent({ canvas: { tools: [], subs: [], commands: [cmd('c0', 'cargo test')] } }));
  expect(f.south.y).toBeGreaterThan(f.tests[0].y + f.tests[0].h);
  expect(f.south.h).toBe(SOUTH.h);
  expect(f.links.filter((l) => l.id.endsWith(':south'))).toHaveLength(1);
  expect(frameOf(agent(), { southH: 300 }).south.h).toBe(300);
});
// WorkspaceView.test.ts
it('shows the landing zone south of every compass', async () => {
  resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' })] });
  const { container } = show();
  await zoomTo(1);
  expect(screen.getAllByText('Rien n’a encore atterri')).toHaveLength(2);
  const sides = [...container.querySelectorAll('[data-agent="a1"] [data-side]')].map((e) => e.getAttribute('data-side'));
  expect(sides.at(-1)).toBe('south');
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-ci.test.ts src/lib/workspace-layout.test.ts src/components/workspace/WorkspaceView.test.ts`
Expected: FAIL, module `workspace-ci` introuvable.

- [ ] **Step 3: Implement `workspace-ci.svelte.ts`, the south of `frameOf`, then wire `Compass` and `WorkspaceView`**

- [ ] **Step 4: Run the whole suite, the type check and prettier**

Run: `npx vitest run && npm run check && npx prettier --check <les fichiers touchés par le lot>`
Expected: seuls les échecs connus de la base (sdk/plugin/bin/cli.test.ts, apps/mobile/src/lib/appearance.test.ts ; 3
erreurs de `check` dans apps/mobile).

- [ ] **Step 5: Try it**

La page d'aperçu jetable du brief (`.superpowers/preview/`, jamais commitée), deux agents remplis à la main : l'un avec
un artefact publié deux fois (une carte, « version 2 ») et un autre en approche, une PR à la CI rouge (ruban figé rouge,
étapes en pastilles), un déploiement de branche en cours (tirets bleus qui bougent) ; l'autre avec `cargo test` en
cours sous sa carte et `npm test` en échec sous un sous-agent, la colonne décalée d'autant. Vue de loin : satellites
livrables et CI. Thèmes sombre et clair, et `prefers-reduced-motion`. Captures dans le dossier du brief ; arrêter Vite.

- [ ] **Step 6: Commit**

```bash
npx prettier --write src/lib/workspace-ci.svelte.ts src/lib/workspace-ci.test.ts src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/components/workspace/Compass.svelte src/components/workspace/WorkspaceView.svelte src/components/workspace/WorkspaceView.test.ts
git add src/lib/workspace-ci.svelte.ts src/lib/workspace-ci.test.ts src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/components/workspace
git commit -m "feat(workspace): la zone d'atterrissage au sud de chaque boussole"
```

---

Pour le lot 6 : `FrameOpts` n'a pas de `landing` ; ses piles du sud prendront le nombre de cartes dans la `Map` de
`landingOf` de `WorkspaceView`, et ses cartes de tests restent collées à leur carte (elles ne sont pas une pile).
