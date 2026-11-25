# Workspace, lot 5 : la zone d'atterrissage — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** au sud de chaque boussole, une zone d'atterrissage toujours dessinée où se posent les artefacts publiés, la
CI distante, la CI locale et les déploiements de l'agent ; ce qui n'est pas fini plane au-dessus en pointillé ; de
loin, les satellites « livrables » et « CI ».

**Architecture:** le résumé de toile du cœur (lot 4) apprend les publications de l'outil `Artifact` et les dernières
commandes de terminal de l'agent. Une fonction pure, `landingOf` (`lib/workspace-landing.ts`), réunit ces données,
`BranchCi`, les contrôles et déploiements de la PR (`PrDetail`), les lancements de test du Kanban et les tâches de fond,
en cartes « artefact » et « ruban ». Trois composants les dessinent ; `frameOf` place la zone au sud.

**Tech Stack:** Rust (serde, serde_json), Svelte 5, TypeScript, vitest, catalogues `t()` fr/en.

**Base:** la branche `feat/workspace` après le lot 4 (`Canvas` dans `escouade-plan` avec
`on_tool_use(parent, id, name, input, now)`, `on_tool_result(id, is_error, text, now)`, `close_running(now)` ;
`AgentMeta::canvas`, `Agent.canvas?: CanvasSummary`, `toolCards`, `Frame.west`, `ToolCard`).

**Spec:** `docs/superpowers/specs/2026-11-25-vue-workspace-design.md` (« Au sud : la zone d'atterrissage », « D'où
viennent les données » et ses précisions CI distante, CI locale, déploiement, « De loin » (satellites livrables et CI),
lot 5 de « Les lots », « Choix validés le 25 novembre »).

## Global Constraints

- La zone : « contour pointillé, fond hachuré, repères d'angle, titre « Zone d'atterrissage · n posés, n en approche ».
  Elle est toujours dessinée ; vide, elle dit « Rien n'a encore atterri ». Un seul trait y descend depuis l'agent. »
- « Un livrable en approche (pas fini) plane au-dessus de son emplacement, dessiné en pointillé ; fini, il se pose. »
- Artefact : « une petite fenêtre de navigateur (trois points, l'adresse), le titre de la page, « Artefact · version n ·
  il y a x min », et « Ouvrir ↗ » » ; « une carte par adresse » ; titre = `title` de l'appel, sinon le nom du fichier
  publié ; « version n » = nombre de publications vues à cette adresse ; en approche tant que l'appel tourne, « sous le
  nom de son fichier ». Pas de miniature.
- Ruban : « coins de 6 px, un ruban animé en tête : rayures en biais vertes pour une CI, tirets bleus pour un
  déploiement. En échec, le ruban se fige et passe au rouge. » En-tête (genre, numéro, état, durée) ; une ligne ; les
  étapes en pastilles reliées, sinon « n contrôles, n en échec » ; en pied, où elle tourne et un bouton « Sortie » ou
  « Ouvrir ↗ ».
- CI locale, trois sources : un lancement de test du Kanban de l'agent (`test:<agent>:…`, « Sortie » ouvre `RunView`) ;
  une tâche de fond ou un appel de terminal dont la commande est le réglage `testCommand` du Kanban ou commence par lui ;
  une commande qui ressemble à une suite de tests (`cargo test`, `cargo nextest`, `npm test`, `npm run test`,
  `pnpm test`, `yarn test`, `bun test`, `vitest`, `jest`, `pytest`, `go test`, `make test`, `mvn test`,
  `gradle test`, `dotnet test`, `rspec`, `phpunit`…), « dans une fonction pure testée » ; pour ces deux-là « Sortie »
  ouvre `TaskView`. « Mieux vaut une carte de trop qu'une CI manquée. »
- CI distante : avec une PR, ses étapes sont les contrôles de la PR ; sans PR, le résumé de `BranchCi`, « n contrôles,
  n en échec ».
- Déploiement : se pose chez l'agent dont il déploie la PR ; « en approche tant que son état est en cours, posé une fois
  fini » ; les autres restent dans la vue « Environnements ».
- Satellites : livrables (nombre d'artefacts, bleu `--info`) ; CI (« CI » et son état en un mot, un déploiement de
  même ; vert, rouge en échec). Ordre fixe : équipe, outils, livrables, CI, question.
- Ordre clavier : la zone vient après l'équipe dans le DOM (nord, ouest, centre, est, sud).
- `prefers-reduced-motion` fige le ruban ; un ruban ne s'anime que tant que sa CI tourne.
- Pas de dépendance ajoutée (pas de `regex` côté Rust). Builds Rust dans Docker, `CARGO_BUILD_JOBS=1`,
  `CARGO_INCREMENTAL=0`.
- Textes par `t()`, zone `workspace` ; couleurs par les variables de `src/app.css`.
- Commits en Conventional Commits, en français.

## Review Focus

- **Une même page publiée trois fois** (même adresse) : une seule carte, « version 3 » (Task 1, test
  `a_page_published_again_is_one_artifact`).
- **Une commande qui contient « test » sans en être une** (`git commit -m "test"`, `cat test.txt`, `npm run build`) :
  pas de carte de CI (Task 2) ; à l'inverse `cd app && npx vitest run` en est une.
- **L'agent meurt pendant `cargo test` au premier plan** : la carte ne reste pas « en cours » pour toujours, elle se
  pose en échec (Task 1, test `a_command_cut_short_ends_failed`).
- **Agent sans PR, branche sans CI, rien publié** : la zone est là et dit « Rien n'a encore atterri » ; aucun satellite
  livrables ni CI (Task 3, Task 5).
- **La PR ne se lit pas** (hôte injoignable, quota) : la carte de CI distante garde l'état de `PullRequest.checks`, sans
  étapes, et rien ne casse (Task 5, test « keeps the pull request's state when its detail cannot be read »).

---

### Task 1: Les artefacts et les commandes dans le résumé de toile

**Files:**
- Modify: `crates/escouade-plan/src/canvas.rs`
- Test: `crates/escouade-plan/src/canvas_tests.rs` ; le module `tests` de `src-tauri/src/agent.rs` (aides `call`,
  `returns`)

**Interfaces:**
- Consumes (lot 4): `Canvas`, `on_tool_use`, `on_tool_result`, `close_running`, `Change`.
- Produces:
  - `pub const MAX_ARTIFACTS: usize = 20;` `pub const MAX_COMMANDS: usize = 10;` `pub const MAX_COMMAND: usize = 500;`
  - `pub struct Artifact { pub url: String, pub title: String, pub versions: u32, pub at: i64 }`
  - `pub struct Publishing { pub call: String, pub name: String, pub since: i64 }`
  - `pub struct CommandRun { pub call: String, pub command: String, pub started_at: i64, pub ended_at: Option<i64>, pub failed: bool }`
  - `Canvas` gagne `pub artifacts: Vec<Artifact>`, `pub publishing: Vec<Publishing>`, `pub commands: Vec<CommandRun>`
    (camelCase ; `skip_serializing_if = "Vec::is_empty"`).
  - `pub fn artifact_url(text: &str) -> Option<String>` — la première adresse `https://claude.ai/artifact/<id>` ou
    `https://<sous-domaine>.claude.ai/artifact/<id>` du texte (`<id>` : `[A-Za-z0-9_-]+`), précédée d'un début de texte
    ou d'un caractère qui n'est ni lettre, chiffre, `.`, `/`, `-` ; la règle de `ARTIFACT_URL` (`src/lib/tools.ts`
    l. ~120), écrite à la main.
  - `on_tool_use`, outil `Artifact` d'un fil quelconque, `input.action` absent ou `"publish"` : un `Publishing`
    (`name` = `input.title`, sinon le dernier segment de `input.file_path`) → `Saved`. Outil `Bash` du fil principal :
    un `CommandRun` (commande coupée à `MAX_COMMAND` caractères) → `View` ; au-delà de `MAX_COMMANDS`, la plus ancienne
    part.
  - `on_tool_result` : un `Publishing` de cet appel part ; s'il réussit et que `artifact_url(text)` trouve une adresse,
    l'`Artifact` de cette adresse prend `versions += 1`, `title` = `name` s'il n'est pas vide, `at = now` (créé à
    `versions: 1` sinon ; au-delà de `MAX_ARTIFACTS`, le plus ancien `at` part) → `Saved`. Un `CommandRun` de cet appel
    prend `ended_at = now`, `failed = is_error` → `View`.
  - `close_running(now)` : `publishing` vidé ; chaque `CommandRun` sans `ended_at` prend `ended_at = now`,
    `failed = true`.

Choix du plan : le cœur garde les dix dernières commandes de terminal du fil principal, sans les trier ; c'est la
fenêtre qui reconnaît les tests (la fonction pure de la spec, avec le réglage `testCommand` qu'elle a sous la main).

Choix du plan : un artefact publié par un sous-agent est un livrable de l'agent, et se pose dans sa zone.

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
    c.on_tool_result("p1", false, &format!("Published {URL}"), 2);
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
fn keeps_the_last_ten_commands_of_the_agent() {
    let mut c = Canvas::default();
    for i in 0..12 {
        c.on_tool_use(None, &format!("b{i}"), "Bash", &json!({"command": format!("echo {i}")}), i);
    }
    c.on_tool_use(Some("a1"), "sb", "Bash", &json!({"command": "cargo test"}), 20);
    assert_eq!(c.commands.len(), MAX_COMMANDS);
    assert_eq!(c.commands[0].command, "echo 2");
}

#[test]
fn a_command_cut_short_ends_failed() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "b1", "Bash", &json!({"command": "cargo test"}), 1);
    c.close_running(5);
    assert_eq!((c.commands[0].ended_at, c.commands[0].failed), (Some(5), true));
}
```

Et dans `src-tauri/src/agent.rs`, `the_canvas_lands_an_artifact_published` : `call(.., "Artifact", {"title": "R"})`,
`returns(.., "p1", "Published https://claude.ai/artifact/z9", Value::Null)` ; `a.view()` sérialisée a
`canvas.artifacts[0] == { "url": "https://claude.ai/artifact/z9", "title": "R", "versions": 1, … }`.

- [ ] **Step 2: Run them to see them fail**

Run: `cd src-tauri && cargo test -p escouade-plan canvas`
Expected: FAIL à la compilation, `artifact_url` introuvable.

- [ ] **Step 3: Implement in `crates/escouade-plan/src/canvas.rs`**

- [ ] **Step 4: Run them, and the agent's test**

Run: `cd src-tauri && cargo test -p escouade-plan && cargo test --lib -- the_canvas`
Expected: PASS ; clippy sans avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/escouade-plan/src src-tauri/src/agent.rs
git commit -m "feat(cœur): le résumé de toile garde les artefacts publiés et les dernières commandes"
```

### Task 2: Reconnaître une suite de tests

**Files:**
- Create: `src/lib/workspace-landing.ts`
- Test: `src/lib/workspace-landing.test.ts`

**Interfaces:**
- Produces: `looksLikeTests(command: string, testCommand?: string): boolean` — vrai quand `testCommand` (rogné, non
  vide) est la commande ou son début ; sinon quand un des morceaux de la commande (coupée sur `&&`, `||`, `;`, `|`),
  débarrassé des affectations `NOM=valeur` de tête et des lanceurs `npx`, `bunx`, `pnpm exec`, `yarn exec`,
  `python -m`, `python3 -m`, `uv run`, `poetry run`, `bundle exec`, commence par l'un des motifs de la spec (mot
  entier) : `cargo test`, `cargo nextest`, `npm test`, `npm run test`, `npm t`, `pnpm test`, `pnpm run test`,
  `yarn test`, `yarn run test`, `bun test`, `bun run test`, `vitest`, `jest`, `pytest`, `go test`, `make test`,
  `mvn test`, `gradle test`, `./gradlew test`, `dotnet test`, `rspec`, `phpunit`, ou un chemin qui finit par
  `/phpunit`, `/rspec`, `/pytest`.

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
it.each(['npm run build', 'cat test.txt', 'git commit -m "test"', 'ls tests/', 'cargo build --tests', 'echo jest'])(
  'sees none in %s',
  (c) => expect(looksLikeTests(c)).toBe(false),
);
it('takes the board\'s test command, and what starts with it', () => {
  expect(looksLikeTests('./scripts/check.sh --fast', './scripts/check.sh')).toBe(true);
  expect(looksLikeTests('./scripts/other.sh', './scripts/check.sh')).toBe(false);
  expect(looksLikeTests('anything', '  ')).toBe(false);
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-landing.test.ts`
Expected: FAIL, module introuvable.

- [ ] **Step 3: Implement `looksLikeTests` in `src/lib/workspace-landing.ts`**

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-landing.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-landing.ts src/lib/workspace-landing.test.ts
git commit -m "feat(workspace): reconnaître une commande qui lance des tests"
```

### Task 3: Ce qui se pose chez un agent

**Files:**
- Modify: `src/lib/types.ts` (`CanvasSummary` : `artifacts`, `publishing`, `commands`)
- Modify: `src/lib/workspace-landing.ts`
- Modify: `src/lib/workspace-layout.ts` (`Satellite`, `discOf`)
- Test: `src/lib/workspace-landing.test.ts`, `src/lib/workspace-layout.test.ts`

**Interfaces:**
- Consumes: `looksLikeTests` (Task 2) ; `parseTestId` (`lib/recipe.ts`) ; `exitCode` (`lib/bgtasks.ts`) ; `Agent`,
  `LaunchState`, `PrDetail`, `PrCheck`, `PrDeployment`, `BranchCi`, `PullRequest` (`lib/types.ts`) ; `discOf` (lot 1).
- Produces:
  - Dans `CanvasSummary` : `artifacts?: { url: string; title: string; versions: number; at: number }[]`,
    `publishing?: { call: string; name: string; since: number }[]`,
    `commands?: { call: string; command: string; startedAt: number; endedAt?: number | null; failed: boolean }[]`.
  - `type RibbonState = 'running' | 'success' | 'failure'`
  - `type Step = { name: string; state: RibbonState }`
  - `type Opens = { kind: 'run'; launch: string } | { kind: 'task'; task: string } | { kind: 'call'; call: string } | { kind: 'url'; url: string } | null`
  - `type ArtifactLanding = { kind: 'artifact'; id: string; url: string | null; title: string; versions: number; at: number; landed: boolean }`
  - `type Ribbon = { kind: 'ciLocal' | 'ciRemote' | 'deploy'; id: string; number: number | null; state: RibbonState; since: number | null; until: number | null; line: string; steps: Step[] | null; total: number | null; failed: number | null; where: string; opens: Opens; landed: boolean }`
  - `type Landing = ArtifactLanding | Ribbon`
  - `type LandingCtx = { launches: { id: string; command: string; state: LaunchState }[]; testCommand: string; pr: PrDetail | null }`
  - `landingOf(agent: Agent, ctx: LandingCtx): Landing[]` — dans cet ordre : artefacts posés (le plus récent d'abord)
    puis en approche (`publishing`, `url: null`, `title` = `name`) ; CI locale : lancements de `ctx.launches` dont
    `parseTestId(id)?.agentId === agent.id` (`opens: run`), tâches de fond `kind: 'command'` dont
    `looksLikeTests(command, testCommand)` (`opens: task`, état par `status` et `exitCode`), `canvas.commands` qui en
    ont l'air (`opens: call`) ; CI distante ; déploiements. `landed` = `state !== 'running'` (un artefact : `url !== null`).
  - CI distante : la première PR `open` de `agent.prs` → `number`, `line` = sa branche (`head`), `state` de `checks`
    (`pending` → `running`, `failure` → `failure`, sinon `success` ; `null` : pas de carte), `steps` = les `PrCheck` de
    `ctx.pr` (`pending` démarré → `running`, `cancelled` → `failure`) sinon `null`, `opens: { kind: 'url', url: pr.url }`.
    Sans PR : `agent.branchCi` avec `checks` non nul → `line` = `branch`, `steps: null`, `total`/`failed` =
    `checksTotal`/`checksFailed`, `where` = `host`, `opens` = l'adresse du premier échec, sinon `null`.
  - Déploiement : chaque `PrDeployment` de `ctx.pr` → `line` = `<head> → <environment>`, état (`success`/`active` →
    `success`, `failure`/`error`/`inactive` → `failure`, le reste → `running`), `opens` = `url`.
  - `Satellite` devient `{ kind: SatKind; count: number; state?: RibbonState }` ; `discOf(agent: Agent, landing: Landing[] = [])`
    ajoute `deliverables` (nombre d'artefacts, absent à 0) et `ci` (`count` = rubans CI et déploiements ; `state` : le
    pire, `failure` > `running` > `success` ; absent sans ruban), à leur place dans l'ordre fixe.

Choix du plan : la ligne d'une CI distante dit la branche, pas le commit ; ni `PullRequest`, ni `BranchCi`, ni
`PrDetail` ne portent le commit dans la fenêtre. De même « version → environnement » devient « branche →
environnement » : `PrDeployment` ne dit pas de version.

Choix du plan : un déploiement ne se pose que par la PR de l'agent (`PrDetail.deployments`, que l'hébergeur lit sur la
branche de la PR). « Son commit est sur la branche de l'agent » demanderait au cœur l'historique de chaque branche ; un
agent sans PR n'a pas de déploiement sur la toile, les déploiements restent dans « Environnements ».

Choix du plan : un lancement de test du Kanban est en cours tant que `state.status === 'running'`, réussi s'il finit
`done` avec le code 0, en échec sinon (`crashed`, `stopped`, autre code).

- [ ] **Step 1: Write the failing tests**

```ts
const ctx = (over: Partial<LandingCtx> = {}): LandingCtx => ({ launches: [], testCommand: '', pr: null, ...over });
it('lands an artifact by its address, and keeps one being published above', () => {
  const a = agent({ canvas: { tools: [], subs: [], artifacts: [{ url: 'https://claude.ai/artifact/z', title: 'R', versions: 2, at: 5 }], publishing: [{ call: 'p', name: 'brouillon.html', since: 6 }] } });
  expect(landingOf(a, ctx())).toEqual([
    expect.objectContaining({ kind: 'artifact', title: 'R', versions: 2, landed: true }),
    expect.objectContaining({ kind: 'artifact', url: null, title: 'brouillon.html', landed: false }),
  ]);
});
it('lands a local CI from the board, a background task and a terminal call', () => {
  const a = agent({
    id: 'a1',
    plan: planWith([], { bgTasks: [{ ...bg('t1'), command: 'npm test' }, { ...bg('t2'), command: 'npm run dev' }] }),
    canvas: { tools: [], subs: [], commands: [{ call: 'c1', command: 'cargo test', startedAt: 1, endedAt: 9, failed: false }] },
  });
  const launches = [{ id: 'test:a1:run:0', command: 'npm run dev', state: launch('running') }];
  const opens = landingOf(a, ctx({ launches })).filter((l) => l.kind === 'ciLocal').map((l) => (l as Ribbon).opens);
  expect(opens).toEqual([{ kind: 'run', launch: 'test:a1:run:0' }, { kind: 'task', task: 't1' }, { kind: 'call', call: 'c1' }]);
});
it('takes the steps of the pull request, else the branch\'s summary', () => {
  const pr = { ...pullRequest(), number: 12, head: 'feat/x', checks: 'failure' as const, url: 'https://h/pr/12' };
  const detail = { ...prDetail(), checks: [check('lint', 'success'), check('test', 'failure')], deployments: [{ environment: 'staging', state: 'in_progress', url: 'https://s' }] };
  const r = landingOf(agent({ prs: [pr] }), ctx({ pr: detail }));
  expect(r[0]).toMatchObject({ kind: 'ciRemote', number: 12, state: 'failure', line: 'feat/x', steps: [{ name: 'lint', state: 'success' }, { name: 'test', state: 'failure' }] });
  expect(r[1]).toMatchObject({ kind: 'deploy', line: 'feat/x → staging', state: 'running', landed: false });
  const b = landingOf(agent({ branchCi: { ...branchCi(), checks: 'failure', checksTotal: 5, checksFailed: 2 } }), ctx());
  expect(b[0]).toMatchObject({ kind: 'ciRemote', steps: null, total: 5, failed: 2 });
});
it('lands nothing for an agent with nothing', () => {
  expect(landingOf(agent(), ctx())).toEqual([]);
});
// workspace-layout.test.ts
it('says the deliverables and the worst CI on the disc', () => {
  const a = agent();
  const landing = [art(true), art(false), ribbon('ciLocal', 'success'), ribbon('ciRemote', 'failure')];
  expect(discOf(a, landing).satellites).toEqual([
    { kind: 'deliverables', count: 2 },
    { kind: 'ci', count: 2, state: 'failure' },
  ]);
});
```

`launch(status)`, `pullRequest()`, `prDetail()`, `check(name, state)`, `branchCi()`, `art(landed)` et
`ribbon(kind, state)` : petites aides à ajouter à `src/test/workspace.ts` (des objets minimaux des types de
`lib/types.ts`), avec `bg` exporté au lot 2.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-landing.test.ts src/lib/workspace-layout.test.ts`
Expected: FAIL, `landingOf is not a function`.

- [ ] **Step 3: Implement the types, `landingOf`, and the two satellites in `discOf`**

- [ ] **Step 4: Run them and the type check**

Run: `npx vitest run src/lib/workspace-landing.test.ts src/lib/workspace-layout.test.ts && npm run check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/workspace-landing.ts src/lib/workspace-landing.test.ts src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/test/workspace.ts
git commit -m "feat(workspace): ce qui se pose chez un agent, artefacts, CI et déploiements"
```

### Task 4: La zone, la carte d'artefact et la carte à ruban

**Files:**
- Create: `src/components/workspace/LandingZone.svelte`, `src/components/workspace/ArtifactCard.svelte`,
  `src/components/workspace/RibbonCard.svelte`
- Modify: `src/components/workspace/Disc.svelte` (`satWord` des satellites `deliverables` et `ci`, couleur de `ci`)
- Modify: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/components/workspace/LandingZone.test.ts`, `src/components/workspace/Disc.test.ts`

**Interfaces:**
- Consumes: `Landing`, `ArtifactLanding`, `Ribbon`, `Opens` (Task 3) ; `fAgo`, `fDur` (`lib/format`) ; `openUrl`
  (`lib/opener`) ; `app.selectLaunch`, `app.openTask`, `app.selectAgent` ; `conversationOf(id).reveal(call)`.
- Produces:
  - `<LandingZone agentId={string} items={Landing[]} />` — `<section data-side="south">` : contour pointillé, fond hachuré
    (`repeating-linear-gradient` de `--line-2`), quatre repères d'angle, titre `workspace.land.title` ; vide :
    `workspace.land.empty`. Les posés dans une rangée, les en approche au-dessus, en pointillé, décalés de 14 px.
  - `<ArtifactCard item={ArtifactLanding} />` — la fenêtre (trois points, l'adresse sans `https://`), le titre,
    `workspace.land.artifact` ; « Ouvrir ↗ » (`workspace.land.open`) → `openUrl(url)` ; en approche, pas de bouton.
  - `<RibbonCard agentId={string} item={Ribbon} />` — coins de 6 px ; ruban `.ribbon.ci` (rayures en biais `--run`) ou
    `.ribbon.deploy` (tirets `--info`), animé seulement si `running`, `.failure` figé en `--del` ; en-tête genre
    (`workspace.land.kind.*`), `#number`, état en mots (`workspace.land.state.*`), durée ; la ligne ; les pastilles des
    `steps` reliées, sinon `workspace.land.checks` quand `total` est connu ; en pied `where` et le bouton :
    `opens.kind` `run` → `app.selectLaunch(launch)`, `task` → `app.openTask(agentId, task)`, `call` →
    `app.selectAgent(agentId)` puis `conversationOf(agentId).reveal(call)` (libellé `workspace.land.output`) ; `url` →
    `openUrl(url)` (libellé `workspace.land.open`).

Choix du plan : la sortie d'un appel de terminal au premier plan s'ouvre dans la conversation, sur l'appel ; `TaskView`
ne montre que des tâches de fond, et cet appel n'en est pas une.

Textes :

```ts
land: {
  title: 'Zone d’atterrissage · {landed} posés, {coming} en approche',
  empty: 'Rien n’a encore atterri',
  artifact: 'Artefact · version {n} · {ago}',
  open: 'Ouvrir ↗',
  output: 'Sortie',
  checks: '{total} contrôles, {failed} en échec',
  kind: { ciLocal: 'CI locale', ciRemote: 'CI', deploy: 'Déploiement' },
  state: { running: 'en cours', success: 'réussie', failure: 'en échec' },
},
sat: { deliverables: { one: 'livrable', other: 'livrables' }, ci: 'CI' },
```

(`sat.deliverables` et `sat.ci` s'ajoutent à `sat` du lot 1 ; vérifier l'apostrophe typographique du catalogue
existant et s'y tenir.)

- [ ] **Step 1: Write the failing tests**

```ts
it('is drawn even empty', () => {
  render(LandingZone, { agentId: 'a1', items: [] });
  expect(screen.getByText('Rien n’a encore atterri')).toBeTruthy();
  expect(screen.getByText('Zone d’atterrissage · 0 posés, 0 en approche')).toBeTruthy();
});
it('lands what is finished and keeps the rest above', () => {
  const { container } = render(LandingZone, { agentId: 'a1', items: [art(true), ribbon('ciLocal', 'running')] });
  expect(screen.getByText('Zone d’atterrissage · 1 posés, 1 en approche')).toBeTruthy();
  expect(container.querySelectorAll('.coming .ribbon.ci')).toHaveLength(1);
});
it('opens a local CI\'s output where it is', async () => {
  resetApp({ agents: [agent({ id: 'a1' })] });
  app.openWorkspace('p1');
  render(LandingZone, { agentId: 'a1', items: [{ ...ribbon('ciLocal', 'success'), opens: { kind: 'task', task: 't1' } }] });
  await fireEvent.click(screen.getByRole('button', { name: 'Sortie' }));
  expect(app.taskView.a1).toBe('t1');
  expect(app.workspaceOn).toBe(false);
});
it('freezes a failed ribbon in red', () => {
  const { container } = render(LandingZone, { agentId: 'a1', items: [ribbon('ciRemote', 'failure')] });
  expect(container.querySelector('.ribbon.failure')).toBeTruthy();
  expect(screen.getByText('en échec')).toBeTruthy();
});
// Disc.test.ts
it('says the CI in a word, red when it fails', () => {
  const a = agent();
  const { container } = render(DiscHost, { disc: { ...discOf(a, [ribbon('ciRemote', 'failure')]), x: 0, y: 0, w: 300, h: 300 }, agent: a, onpick: () => {} });
  expect(screen.getByText('CI')).toBeTruthy();
  expect(container.querySelector('.sat.ci.failure')).toBeTruthy();
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/LandingZone.test.ts src/components/workspace/Disc.test.ts`
Expected: FAIL, composant introuvable.

- [ ] **Step 3: Write the catalogue keys, `ArtifactCard`, `RibbonCard`, `LandingZone`, then the satellites of `Disc`**

- [ ] **Step 4: Run them and the type check**

Run: `npx vitest run src/components/workspace src/lib/i18n && npm run check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/workspace src/lib/i18n
git commit -m "feat(workspace): la zone d'atterrissage, la carte d'artefact et la carte à ruban"
```

### Task 5: La zone au sud de la boussole, et les données qu'elle lit

**Files:**
- Create: `src/lib/workspace-ci.svelte.ts`
- Modify: `src/lib/workspace-layout.ts` (`frameOf` : `south`, son trait unique)
- Modify: `src/components/workspace/Compass.svelte`, `src/components/workspace/WorkspaceView.svelte`
- Test: `src/lib/workspace-ci.test.ts`, `src/lib/workspace-layout.test.ts`,
  `src/components/workspace/WorkspaceView.test.ts`

**Interfaces:**
- Consumes: `landingOf`, `LandingCtx`, `Landing` (Task 3) ; `LandingZone` (Task 4) ; `api.agentPrDetail(id, number, hostRepo?)`
  → `PrRead<PrDetail>` (`lib/ipc.ts`) ; `app.launches`, `app.launchOf(project, id)` ; `project.board.testCommand` ;
  `frameOf`, `FrameOpts`, `layoutNear`, `layoutFarOpen` (lots 2-4).
- Produces:
  - `prDetails: Record<string, PrDetail>` (`$state`, par agent) et `readPrDetail(agent: Agent): Promise<void>` dans
    `src/lib/workspace-ci.svelte.ts` — lit la première PR `open` de l'agent ; garde `data` quand elle vient, ne touche à
    rien sur une `failure` ou une erreur ; sans PR ouverte, oublie celle de l'agent.
  - `FrameOpts` gagne `landing?: Landing[]` et `southH?: number` ; `Frame` gagne `south: Box` — toujours là, sous la
    carte et l'équipe, de la largeur du cadre ; un trait `plan`-gris unique de la carte au haut de la zone,
    `id: 'south'`.
  - `Compass` rend `<LandingZone>` après les `TeamCard` (ordre clavier) et relaie sa hauteur par `onsize('south', h)`.
  - `WorkspaceView` : `landingCtx(agent)` = `{ launches: … de app.launches pour les ids `test:<agent>:…` (commande par
    `app.launchOf(project, id)`), testCommand: project.board.testCommand, pr: prDetails[agent.id] ?? null }` ; passe
    `landing` à `frameOf` et à `discOf` ; `pick(agentId, 'south')` centre la zone ; à la vue de près, un `$effect` par
    agent montré appelle `readPrDetail(agent)` à l'ouverture et quand `${pr.number}:${pr.checks}` change.

Choix du plan : la toile ne lit le détail d'une PR (ses contrôles, ses déploiements) que pour les boussoles à l'écran, à
leur ouverture puis quand l'état de ses contrôles change ; de loin, le satellite CI s'en tient à `PullRequest.checks` et
à `BranchCi`. Une lecture par agent visible, pas une par agent du projet.

- [ ] **Step 1: Write the failing tests**

```ts
// workspace-ci.test.ts
it('reads the detail of the agent\'s open pull request', async () => {
  const backend = fakeBackend({ agent_pr_detail: () => ({ data: { ...prDetail(), checks: [check('ci', 'success')] } }) });
  const a = agent({ id: 'a1', prs: [{ ...pullRequest(), number: 7, state: 'open' }] });
  await readPrDetail(a);
  expect(backend.called('agent_pr_detail')[0].args).toMatchObject({ id: 'a1', number: 7 });
  expect(prDetails.a1.checks[0].name).toBe('ci');
});
it('keeps the pull request\'s state when its detail cannot be read', async () => {
  fakeBackend({ agent_pr_detail: () => ({ failure: { kind: 'unreachable', message: 'x' } }) });
  await readPrDetail(agent({ id: 'a2', prs: [{ ...pullRequest(), state: 'open' }] }));
  expect(prDetails.a2).toBeUndefined();
});
// workspace-layout.test.ts
it('always draws the landing zone south, with one link down to it', () => {
  const f = frameOf(agent());
  expect(f.south.y).toBeGreaterThan(f.centre.y + f.centre.h);
  expect(f.links.filter((l) => l.id === 'south')).toHaveLength(1);
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

- [ ] **Step 4: Run the whole suite and the type check**

Run: `npx vitest run && npm run check`
Expected: PASS ; seuls les échecs déjà présents sur la base.

- [ ] **Step 5: Try it**

App réelle de préférence (le cœur change) : un agent qui publie un artefact deux fois à la même adresse (une carte,
« version 2 »), qui lance `npm test` au premier plan (le ruban tourne puis se pose), une PR avec une CI rouge (ruban figé
rouge, étapes en pastilles). Satellites livrables et CI de loin. Thèmes sombre et clair, et `prefers-reduced-motion`.
Captures d'écran.

- [ ] **Step 6: Commit**

```bash
git add src/lib/workspace-ci.svelte.ts src/lib/workspace-ci.test.ts src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/components/workspace
git commit -m "feat(workspace): la zone d'atterrissage au sud de chaque boussole"
```
