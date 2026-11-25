# Workspace, lot 2 : la boussole — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** au-delà de 45 % de zoom, chaque agent devient sa boussole : sa carte au centre avec une barre de saisie
qui écrit par le même chemin que la conversation, son plan au nord, son équipe à l'est, reliés par des traits qui ne
s'animent que quand quelque chose tourne.

**Architecture:** le chemin d'envoi de `Composer.svelte` et ses listes de modèles, d'efforts et de modes sortent dans
`lib/send.svelte.ts` et `lib/agent-options.ts`, appelés par `Composer` et par la carte. Le placement d'une boussole est
une fonction pure (`frameOf`, `layoutNear` dans `lib/workspace-layout.ts`) ; la caméra choisit l'ajustement et garde
l'agent du centre en place quand l'altitude change (`fitBest`, `keepAnchor`). Les cartes sont du HTML positionné dans
le monde, les traits un seul `<svg>` dessous.

**Tech Stack:** Svelte 5 (runes), TypeScript, vitest + @testing-library/svelte, catalogues `t()` fr/en.

**Base:** la branche `feat/workspace` telle que le lot 1 l'a laissée (06cebc59) : `WorkspaceView`, `Disc`,
`workspace-camera.ts`, `workspace-layout.ts`, `src/test/workspace.ts`, la zone `workspace` des catalogues.

**Spec:** `docs/superpowers/specs/2026-11-25-vue-workspace-design.md` (« De près : la boussole dépliée », « La boussole,
côté par côté » › centre, nord, est, « Construction », lot 2 de « Les lots », « Choix validés le 25 novembre »).

## Global Constraints

- Aucune dépendance ajoutée (« une bibliothèque de graphe : le dépôt n'en a pas et n'en a pas besoin »).
- Une seule logique d'envoi : « `Entrée` envoie par le même chemin que la conversation » ; `Composer` et la carte
  appellent la même fonction (`submitDraft`). Pas de seconde logique de question ni de permission.
- Le brouillon est celui de l'agent (`getDraft`, `setDraft`) : partagé avec la conversation dans les deux sens.
- Altitudes : « de loin » sous 0,45 de zoom (disques du lot 1) ; à partir de 0,45, la boussole. Le niveau du milieu
  (0,45 à 0,8) est le lot 6 : il n'existe pas dans ce lot.
- Zoom borné de 0,3 à 2,5 (lot 1, inchangé).
- « Un trait animé veut toujours dire « ça tourne en ce moment » » : aucun flux décoratif permanent.
- Les traits sont un seul `<svg>` sous les cartes ; les cartes sont du HTML positionné.
- Couleurs par les variables de `src/app.css` (`--run`, `--wait`, `--del`, `--done`, `--info`, `--raise`, `--line-2`…),
  dans les deux thèmes.
- Textes par `t()`, zone `workspace` (`src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`) ; `npm run check`
  passe (3 erreurs déjà présentes dans `apps/mobile`, admises).
- Clavier : chaque carte atteignable dans l'ordre nord, ouest, centre, est, sud (ordre du DOM) ; états dits en mots ;
  `prefers-reduced-motion` fige flux, halo et orbite.
- `PlanState` garde au plus 30 sous-agents : au plafond (`agents.length === 30`), la pile dit « 30+ terminés ».
- Bande du plan : au-delà de 9 tâches, les faites repliées en « n faites », puis la ou les tâches en cours et les
  suivantes.
- Commits en Conventional Commits, en français (`feat(workspace): …`).

## Review Focus

- **Texte tapé sur la carte d'un agent qui attend** : il répond à la question en attente, comme dans la conversation
  (Task 1, test « answers the question waiting »).
- **Envoi refusé** (pas de compte, erreur du cœur) : le texte revient dans le brouillon de l'agent, rien n'est perdu
  (Task 1, test « gives the text back when the message is refused »).
- **Franchir 45 % à la molette** : l'agent qui était au centre de l'écran y reste quand les disques deviennent des
  boussoles (Task 3 `keepAnchor`, Task 7 test « keeps the agent in the middle when the altitude changes »).
- **Un agent au plafond de 30 sous-agents, dont un échec** : la pile dit « + 30+ terminés · 1 échec » et l'échec non vu
  reste hors de la pile (Task 4).
- **Taper ou sélectionner dans le champ d'une carte** : `+`, `-`, `0`, Échap ne zooment pas et ne quittent pas, et
  glisser dans le champ ne déplace pas la toile (Task 7, test « leaves the card's field alone »).

---

### Task 1: Le chemin d'envoi partagé

**Files:**
- Modify: `src/lib/send.svelte.ts` (ajouter, à côté de `sendDraft` et `reportRefusal`)
- Modify: `src/components/Composer.svelte` (`send()` et `deliver()` l. ~566-662, `sending`/`heldFor` l. ~84-86 et
  l. ~948, `pendingItem` l. ~116, `paused` l. ~216)
- Test: `src/lib/send.test.ts` (nouveau) ; `src/components/Composer.test.ts`, `Composer.accounts.test.ts`,
  `Composer.images.test.ts` restent verts sans changement.

**Interfaces:**
- Consumes: `hold`, `holdModeOf`, `settled`, `type HoldMode` (`lib/outbox.svelte.ts`) ; `giveBack` (`lib/drafts.ts`) ;
  `conversationOf` (`lib/conversations.svelte.ts`) ; `api.answerQuestion`, `api.answerPermission` ; `connection`
  (`lib/connection.svelte.ts`).
- Produces (dans `src/lib/send.svelte.ts`) :
  - `pendingItemOf(agent: Agent): ConvItem | undefined` — l'élément de `conversationOf(agent.id).items` dont l'id est
    `agent.pending[0]` ; `undefined` sans demande.
  - `type Route = { kind: 'answer'; item: ConvItem } | { kind: 'hold'; mode: HoldMode } | { kind: 'send' }`
  - `routeOf(agent: Agent, body: string, pending: ConvItem | undefined): Route` — `answer` quand `body` n'est pas vide et
    `pending` existe ; sinon `hold` quand `holdModeOf(agent)` rend un mode ; sinon `send`.
  - `sendPaused(): boolean` — `connection.server !== null && connection.state !== 'open'`.
  - `outgoing: Record<string, 'sending' | 'worktree'>` (`$state`, par agent) — un envoi en route ; `'worktree'` quand le
    cœur le retient jusqu'à la préparation du worktree.
  - `type Submitted = { cleared: 'all' | 'text' | 'none'; done: Promise<void> }`
  - `submitDraft(agent: Agent, owner: string, text: string, files: DraftAttachment[]): Submitted` — ce que faisait
    `send()` après le test des commandes interactives : refuse (`cleared: 'none'`) quand `sendPaused()`, ou quand
    `outgoing[agent.id] === 'worktree'` (et alors `app.toast(t('composer.send.alreadyWaiting'))`) ; `hold` →
    `hold(...)`, `cleared: 'all'` ; sinon enchaîne derrière l'envoi précédent du même agent (une promesse par agent,
    au niveau du module), `settled(agent.id)`, puis l'ancien `deliver` ; `cleared: 'text'` quand il répond (les
    fichiers restent), `'all'` sinon.

- [ ] **Step 1: Write the failing tests** (`src/lib/send.test.ts`, mise en place comme `start()` de
  `src/lib/outbox.test.ts` l. 33-50 ; `fakeBackend` rend la conversation et enregistre les appels)

```ts
it('answers the question waiting with the text typed', async () => {
  const backend = await start([agent({ id: 'a1', status: 'waiting', pending: ['q1'] })], { items: [question('q1', 'Q ?')] });
  const s = submitDraft(app.agents.a1, 'a1', 'Oui', [file('a.png')]);
  await s.done;
  expect(backend.called('answer_question')[0].args).toMatchObject({ id: 'a1', requestId: 'q1', answers: { 'Q ?': 'Oui' } });
  expect(backend.called('send_message')).toHaveLength(0);
  expect(s.cleared).toBe('text');
});
it('sends a message otherwise, files and all', async () => {
  const backend = await start([agent({ id: 'a1' })]);
  const s = submitDraft(app.agents.a1, 'a1', 'Bonjour', []);
  await s.done;
  expect(backend.called('send_message')[0].args).toMatchObject({ id: 'a1', text: 'Bonjour' });
  expect(s.cleared).toBe('all');
});
it('holds a message typed mid-turn when the delay is on', async () => {
  const backend = await start([agent({ id: 'a1', status: 'running' })], { delay: 10 });
  expect(routeOf(app.agents.a1, 'x', undefined)).toEqual({ kind: 'hold', mode: 'delay' });
  submitDraft(app.agents.a1, 'a1', 'x', []);
  expect(heldOf('a1')).toHaveLength(1);
  expect(backend.called('send_message')).toHaveLength(0);
});
it('gives the text back when the message is refused', async () => {
  await start([agent({ id: 'a1' })], { send_message: () => { throw 'refusé'; } });
  await submitDraft(app.agents.a1, 'a1', 'Bonjour', []).done;
  expect(getDraft('a1').text).toBe('Bonjour');
});
it('sends nothing while the server connection is cut', () => {
  connection.server = 's1'; connection.state = 'closed';
  expect(submitDraft(app.agents.a1, 'a1', 'x', []).cleared).toBe('none');
});
```

`start(agents, opts)`, `question(id, text)` et `file(name)` sont des aides locales au fichier de test (une
`QuestionItem` minimale, une `DraftAttachment` d'image) ; remettre `connection` en `afterEach`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/send.test.ts`
Expected: FAIL, `submitDraft is not a function`.

- [ ] **Step 3: Implement the functions above in `src/lib/send.svelte.ts`, then make `Composer` call them**

`Composer.send()` garde : le test vide / `reading` / `copying`, les commandes interactives (`interactiveOf`), puis
`const s = submitDraft(agent, draftFor, text, files)` ; selon `s.cleared` il vide le texte, et les fichiers si `'all'`.
`deliver`, `inflight`, `heldFor` quittent le composant. Le bouton Envoyer lit `outgoing[agent.id]` au lieu de
`sending`. `pendingItem` devient `$derived(pendingItemOf(agent))`, `paused` devient `$derived(sendPaused())`.

- [ ] **Step 4: Run the new tests and every Composer suite**

Run: `npx vitest run src/lib/send.test.ts src/components/Composer src/lib/outbox.test.ts src/lib/drafts.test.ts`
Expected: PASS, aucun test existant modifié.

- [ ] **Step 5: Commit**

```bash
git add src/lib/send.svelte.ts src/lib/send.test.ts src/components/Composer.svelte
git commit -m "refactor(composer): le chemin d'envoi sort dans lib/send, pour être partagé"
```

### Task 2: Les choix de modèle, d'effort et de mode, sortis de Composer

**Files:**
- Create: `src/lib/agent-options.ts`
- Modify: `src/components/Composer.svelte` (`modelChoices`, `modelOptionsShown`, `modeOptions` l. ~131-185, la liste
  d'efforts l. ~892, `setOption` l. ~773-778)
- Modify: `src/lib/subagents.ts` et `src/components/SubagentView.svelte` (l. ~59, `canWrite`)
- Test: `src/lib/agent-options.test.ts`, `src/lib/subagents.test.ts`

**Interfaces:**
- Consumes: `capsOf`, `effortsOf`, `modelCost`, `modelMenu`, `modelsOf`, `modelTakes`, `modesOf`, `fromConfig`,
  `readingModels`, `searchesModels`, `engineName`, `speakerOf` (`lib/engines`) ; `EFFORTS`, `MODES`, `modeLabel`,
  `modeTitle`, `modelOptions` (`lib/models`).
- Produces:
  - `type OptionChoice = { value: string; label: string; detail?: string; title?: string; disabled?: boolean; reason?: string; favorite?: boolean; group?: string }`
  - `modelChoicesOf(agent: Agent): OptionChoice[]` — ce que `modelOptionsShown` rend aujourd'hui.
  - `effortChoicesOf(agent: Agent): OptionChoice[]` — `effortsOf(agent, EFFORTS)` en choix ; vide quand
    `!modelTakes(agent, agent.model, 'effort')`.
  - `modeChoicesOf(agent: Agent): OptionChoice[]` — ce que `modeOptions` rend aujourd'hui.
  - `setAgentOption(agentId: string, o: { model?: string; effort?: string; mode?: string }): void` — l'ancien
    `setOption` (mise à jour locale, toast du mode `bypassPermissions`, `app.run(api.setAgentOptions(...))`).
  - Dans `src/lib/subagents.ts` : `canWriteSub(agent: Agent, sub: { status: RunStatus | null; taskId?: string; background?: boolean }): boolean`
    — `running && !!taskId && background === true && !agent.terminal && !agent.archived` (la règle de `SubagentView`).

- [ ] **Step 1: Write the failing tests**

```ts
// agent-options.test.ts
it('greys a mode the harness lacks, with why', () => {
  resetApp({ agents: [agent({ id: 'a1', engine: 'copilot' })] }); // moteur de test à un seul mode, comme Composer.test
  const plan = modeChoicesOf(app.agents.a1).find((c) => c.value === 'plan');
  expect(plan).toMatchObject({ disabled: true });
  expect(plan?.reason).toBeTruthy();
});
it('offers no effort for a model that takes none', () => {
  expect(effortChoicesOf(agent({ model: 'haiku' }))).toEqual([]);
});
it('sets an option on the agent and tells the core', () => {
  const backend = fakeBackend();
  resetApp({ agents: [agent({ id: 'a1' })] });
  setAgentOption('a1', { effort: 'high' });
  expect(app.agents.a1.effort).toBe('high');
  expect(backend.called('set_agent_options')[0].args).toMatchObject({ id: 'a1', effort: 'high' });
});
// subagents.test.ts
it('lets one write to a subagent running in the background only', () => {
  const a = agent();
  expect(canWriteSub(a, { status: 'running', taskId: 't1', background: true })).toBe(true);
  expect(canWriteSub(a, { status: 'running', taskId: 't1', background: false })).toBe(false);
  expect(canWriteSub({ ...a, terminal: true }, { status: 'running', taskId: 't1', background: true })).toBe(false);
});
```

Pour le moteur à un seul mode et le modèle sans effort, reprendre les moteurs et modèles de test déjà utilisés par
`Composer.test.ts` (chercher ses cas « mode » et « effort ») plutôt que d'en inventer.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/agent-options.test.ts src/lib/subagents.test.ts`
Expected: FAIL, module `agent-options` introuvable et `canWriteSub` absent.

- [ ] **Step 3: Move the lists into `src/lib/agent-options.ts` and `canWriteSub` into `src/lib/subagents.ts`**

`Composer` garde ses `Dropdown` et les nourrit de `$derived(modelChoicesOf(agent))`, etc. ; `pick` appelle
`setAgentOption`. `SubagentView.canWrite` devient `$derived(!!info && canWriteSub(agent, info))`.

- [ ] **Step 4: Run them, and the Composer and SubagentView suites**

Run: `npx vitest run src/lib/agent-options.test.ts src/lib/subagents.test.ts src/components/Composer src/components/SubagentView.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/agent-options.ts src/lib/agent-options.test.ts src/lib/subagents.ts src/lib/subagents.test.ts src/components/Composer.svelte src/components/SubagentView.svelte
git commit -m "refactor(composer): les choix de modèle, d'effort et de mode sortent dans lib/agent-options"
```

### Task 3: La caméra entre deux altitudes

**Files:**
- Modify: `src/lib/workspace-camera.ts`
- Test: `src/lib/workspace-camera.test.ts`

**Interfaces:**
- Consumes: `Camera`, `clampZoom`, `fit`, `altitude`, `FAR_BELOW`, `ZOOM_MAX`, `type Altitude` (lot 1).
- Produces:
  - `type Size = { w: number; h: number }`, `type Box = { x: number; y: number; w: number; h: number }`
  - `fit(world: Size, view: Size, margin = 32, max = ZOOM_MAX): Camera` — le 4ᵉ paramètre borne le zoom par le haut.
  - `const ALTITUDES: { name: Altitude; min: number; max: number }[]` — du plus près au plus loin :
    `{ near, FAR_BELOW, ZOOM_MAX }`, `{ far, ZOOM_MIN, FAR_BELOW - 0.001 }`.
  - `fitBest(worlds: Record<Altitude, Size>, view: Size): Camera` — pour chaque altitude de `ALTITUDES`, dans l'ordre,
    `fit(worlds[name], view, 32, max)` ; rend la première dont le zoom est ≥ `min` ; la dernière (loin) sinon.
  - `keepAnchor(cam: Camera, from: Box, to: Box): Camera` — le centre de `to` (monde) tombe à l'écran où était celui de
    `from`, au zoom de `cam`.
  - `nearest<T extends Box>(items: T[], cam: Camera, view: Size): T | undefined` — la boîte dont le centre est le plus
    près du centre de l'écran.

- [ ] **Step 1: Write the failing tests**

```ts
it('caps the zoom of a fit', () => {
  expect(fit({ w: 10, h: 10 }, { w: 2000, h: 2000 }, 32, 0.44).zoom).toBe(0.44);
});
it('fits the compasses when they hold at 45 % or more, else the discs', () => {
  const view = { w: 1000, h: 800 };
  expect(fitBest({ far: { w: 300, h: 300 }, near: { w: 900, h: 600 } }, view).zoom).toBeGreaterThanOrEqual(0.45);
  const c = fitBest({ far: { w: 1500, h: 700 }, near: { w: 6000, h: 3000 } }, view);
  expect(altitude(c.zoom)).toBe('far');
});
it('keeps an agent where it was on screen', () => {
  const cam = { x: 100, y: 50, zoom: 1 };
  const c = keepAnchor(cam, { x: 0, y: 0, w: 300, h: 300 }, { x: 1000, y: 400, w: 800, h: 600 });
  // centre de `from` à l'écran : (250, 200) ; celui de `to`, (1400, 700), y tombe
  expect([1400 * c.zoom + c.x, 700 * c.zoom + c.y]).toEqual([250, 200]);
});
it('finds the box nearest the middle of the screen', () => {
  const items = [{ id: 'a', x: 0, y: 0, w: 100, h: 100 }, { id: 'b', x: 400, y: 0, w: 100, h: 100 }];
  expect(nearest(items, { x: 0, y: 0, zoom: 1 }, { w: 900, h: 100 })?.id).toBe('b');
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-camera.test.ts`
Expected: FAIL, `fitBest is not a function`.

- [ ] **Step 3: Implement in `src/lib/workspace-camera.ts`**

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-camera.test.ts`
Expected: PASS, les tests du lot 1 compris.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-camera.ts src/lib/workspace-camera.test.ts
git commit -m "feat(workspace): la caméra choisit son altitude et garde l'agent du centre en place"
```

### Task 4: Le placement d'une boussole

**Files:**
- Modify: `src/lib/workspace-layout.ts`
- Modify: `src/test/workspace.ts` (exporter `sub` et `bg`, aujourd'hui locales)
- Test: `src/lib/workspace-layout.test.ts`

**Interfaces:**
- Consumes: `Box`, `Size` (Task 3) ; `planView`, `type PlanRow` (`lib/plan.ts`) ; `bgRows`, `ownerOf`, `isRunning`,
  `type BgRow` (`lib/bgtasks.ts`) ; `drivenAgents` (`lib/later.ts`) ; `Placed`, `DISC`, `SatKind` (lot 1).
- Produces:
  - `type Side = 'north' | 'west' | 'centre' | 'east' | 'south'`
  - `const SIDE_OF: Record<SatKind, Side>` — `team: 'east'`, `tools: 'west'`, `deliverables: 'south'`, `ci: 'south'`,
    `question: 'centre'`.
  - `const CARD = { w: 360, h: 184 }`, `const SUB = { w: 240, h: 116 }`, `const BG = { w: 240, h: 86 }`,
    `const TILE = { w: 156, h: 96 }`, `const BAND_H = 150`, `const GAP = 48`, `const MAX_TILES = 9`
    (dimensions laissées par la spec à qui construit ; prises sur les maquettes, à régler à l'usage).
  - `type TeamItem = { kind: 'sub'; id: string; sub: SubAgent } | { kind: 'bg'; id: string; row: BgRow } | { kind: 'folded'; id: 'folded'; done: number; failed: number; capped: boolean }`
  - `type LinkState = 'run' | 'done' | 'failed' | 'plan' | 'launched'`
  - `type Link = { id: string; state: LinkState; d: string }` — `d` : le chemin SVG, en coordonnées du cadre puis du
    monde.
  - `type Frame = { agentId: string; w: number; h: number; centre: Box; north: Box | null; team: Placed<TeamItem>[]; links: Link[] }`
  - `type FrameOpts = { teamOpen?: boolean; seenAt?: number; centreH?: number }`
  - `bandTiles(rows: PlanRow[]): { folded: number; rows: PlanRow[] }` — `rows.length ≤ MAX_TILES` : tout, `folded: 0` ;
    sinon `folded` = les faites, `rows` = les non faites dans l'ordre, au plus `MAX_TILES - 1`.
  - `teamOf(agent: Agent, opts: FrameOpts): TeamItem[]` — sous-agents en cours (ordre du plan), puis tâches de fond en
    cours, puis, repliés sauf `teamOpen`, les terminés en un `folded` (`done` : les terminés sans échec, `failed` : les
    échecs, de la pile ; absent quand la pile est vide) ; un sous-agent `failed` dont `endedAt ?? startedAt` est après `seenAt` (défaut 0) reste hors
    de la pile, juste avant elle ; dépliés, les terminés suivent l'ordre de `sidebarSubs` ; `capped` quand
    `plan.agents.length >= 30`.

Choix du plan : « un échec non vu » = un échec arrivé depuis que l'on a déplié la pile de cet agent dans la toile ;
aucun déplié avant (session neuve) : tous les échecs sont vus hors de la pile. L'app ne garde pas de « vu » par
sous-agent, et 30 lignes au plus sont en jeu.

Choix du plan : à l'est, seules les tâches de fond en cours ont une carte ; les terminées restent dans la vue de
l'agent (la spec ne replie que les sous-agents, et le lot 5 pose celles qui sont des tests dans la zone d'atterrissage).
  - `frameOf(agent: Agent, opts?: FrameOpts): Frame` — coordonnées locales : bande du plan en haut (si `planView` a
    des tâches), la carte en dessous (hauteur `centreH ?? CARD.h`), l'équipe en colonne à droite ; un sous-agent dont
    `parent` est dans l'équipe, et une tâche de fond dont `ownerOf` est dans l'équipe, sont posés à droite de leur
    parent ; un trait par élément de l'équipe (`run` / `done` / `failed` selon son état), un trait `plan`.
  - `packRows<T extends Size>(items: T[], maxW: number, gap: number): { items: Placed<T>[]; w: number; h: number }` —
    en lignes, dans l'ordre, une ligne passe à la suivante quand elle dépasserait `maxW` ; chaque ligne a la hauteur de
    son plus haut élément.
  - `canvasOrder(agents: Agent[], all: Record<string, Agent>, tickets: Record<string, Ticket>): { order: Agent[]; launched: [from: string, to: string][] }`
    — un agent que `drivenAgents` dit lancé par un autre de la liste vient juste après lui.
  - `layoutNear(frames: Frame[], launched?: [string, string][], maxW = 2400): { items: Placed<Frame>[]; links: Link[]; w: number; h: number }`
    — `packRows` des cadres (écart `GAP`), traits de chaque cadre mis en coordonnées du monde, plus un trait
    `launched` (pointillé) du bord droit d'un cadre au bord gauche de celui qu'il a lancé.

- [ ] **Step 1: Write the failing tests**

```ts
it('folds the plan past 9 tasks', () => {
  const rows = planView(agent({ plan: planWith([...Array(6).fill('done'), 'inProgress', ...Array(5).fill('pending')]) }))!.rows;
  const b = bandTiles(rows);
  expect(b.folded).toBe(6);
  expect(b.rows[0].state).toBe('run');
  expect(b.rows).toHaveLength(6);
});
it('puts the team on the east, folds the ended, keeps a failure not seen', () => {
  const a = agent({ plan: planWith([], { agents: [sub('s1'), sub('s2', 'done'), sub('s3', 'failed')] }) });
  const kinds = teamOf(a, {}).map((i) => i.id);
  expect(kinds).toEqual(['s1', 's3', 'folded']);
  expect(teamOf(a, { seenAt: Infinity }).at(-1)).toMatchObject({ kind: 'folded', done: 1, failed: 1, capped: false });
  expect(teamOf(a, { teamOpen: true }).map((i) => i.id)).toEqual(['s1', 's2', 's3']);
});
it('says the pile is capped at 30 subagents', () => {
  const agents = Array.from({ length: 30 }, (_, i) => sub(`s${i}`, 'done'));
  expect(teamOf(agent({ plan: planWith([], { agents }) }), {}).at(-1)).toMatchObject({ capped: true, done: 30 });
});
it('lays a child subagent right of its parent', () => {
  const f = frameOf(agent({ plan: planWith([], { agents: [sub('s1'), { ...sub('s2'), parent: 's1' }] }) }));
  const [p, c] = f.team;
  expect(c.x).toBe(p.x + SUB.w + GAP);
  expect(c.y).toBe(p.y);
});
it('draws a moving link only to what runs', () => {
  const f = frameOf(agent({ plan: planWith(['inProgress'], { agents: [sub('s1'), sub('s2', 'failed')] }) }));
  expect(f.links.map((l) => l.state).sort()).toEqual(['failed', 'plan', 'run']);
  expect(f.north).not.toBeNull();
  expect(frameOf(agent()).north).toBeNull();
});
it('packs frames in rows', () => {
  const r = packRows([{ w: 1000, h: 10 }, { w: 1000, h: 30 }, { w: 1000, h: 20 }], 2100, 50);
  expect(r.items[2]).toMatchObject({ x: 0, y: 80 });
  expect(r).toMatchObject({ w: 2050, h: 100 });
});
it('puts an agent launched by another right after it', () => {
  const b = agent({ id: 'b', status: 'running' });
  const a = agent({ id: 'a', plan: planWith([], { driven: [{ agent: 'b', at: 1 }] }) });
  const c = agent({ id: 'c' });
  const r = canvasOrder([a, c, b], { a, b, c }, {});
  expect(r.order.map((x) => x.id)).toEqual(['a', 'b', 'c']);
  expect(r.launched).toEqual([['a', 'b']]);
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: FAIL, `bandTiles is not a function`.

- [ ] **Step 3: Implement in `src/lib/workspace-layout.ts`**

Un trait de l'équipe : une courbe de Bézier du bord droit de la carte au bord gauche de l'élément ; celui du plan, du
haut de la carte au bas de la bande. `discOf` et `layoutFar` ne changent pas.

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/test/workspace.ts
git commit -m "feat(workspace): le placement d'une boussole, son plan et son équipe"
```

### Task 5: La carte de l'agent et sa barre de saisie

**Files:**
- Create: `src/components/workspace/AgentCard.svelte`
- Modify: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/components/workspace/AgentCard.test.ts`

**Interfaces:**
- Consumes: `submitDraft`, `outgoing`, `sendPaused` (Task 1) ; `modelChoicesOf`, `effortChoicesOf`, `modeChoicesOf`,
  `setAgentOption` (Task 2) ; `getDraft`, `setDraft` (`lib/drafts.ts`) ; `interactiveOf` (`lib/interactive`) ;
  `cardActivity`, `cardSpend` (`lib/sidebar.ts`) ; `modelLabel` (`lib/models`) ; `fDur` (`lib/format`) ;
  `menu.showAt` (`lib/menu.svelte.ts`) ; `MicButton` et `insertAt` (`lib/dictation`) ; `app.selectAgent`.
- Produces: `<AgentCard agent={Agent} />` — un `<article data-side="centre">`, de hauteur libre (le lot 3 la fait
  grandir).

Contenu (spec, « Au centre ») : en-tête point + nom + état en mots (`workspace.state.*`) ; une ligne modèle · effort ·
durée (`fDur(activeMs + (activeSince ? app.now - activeSince : 0))`) · `cardSpend(agent)` ; la ligne d'activité
(`cardActivity`) et `progressLine` s'il y en a une ; en bas un `<input>` d'une ligne (`aria-label` `workspace.card.write`),
un bouton Paramètres (`workspace.card.settings`), `MicButton` (cible `kind: 'composer'`, `panel={false}`), et la
pastille `workspace.card.files` quand le brouillon a des fichiers. Contour `--run` quand il tourne, `--wait` + halo
quand il attend, `--del` en erreur.

Comportement :
- Le champ lit `getDraft(agent.id).text` à l'ouverture et écrit `setDraft(agent.id, { text, files })` à chaque
  frappe (les fichiers du brouillon gardés tels quels).
- `Entrée` : texte vide → rien ; `interactiveOf(agent.engine, body)` → `app.selectAgent(agent.id)`, le texte reste
  dans le brouillon ; sinon `submitDraft(agent, agent.id, text, getDraft(agent.id).files)` puis le champ suit
  `cleared`.
- Paramètres : `menu.showAt(button, [{ label: t('workspace.card.model'), submenu: modelChoicesOf(agent)… },
  { label: t('workspace.card.effort'), … }, { label: t('workspace.card.mode'), … }, { separator: true },
  { label: t('workspace.card.attach'), icon: 'clip', onClick: () => app.selectAgent(agent.id) }])` ; chaque entrée
  de sous-menu porte `checked` pour la valeur de l'agent et appelle `setAgentOption`. Sans effort possible, l'entrée
  Effort est `disabled`.
- Double-clic sur la carte (hors du champ) : `app.selectAgent(agent.id)`.

Choix du plan : « Joindre un fichier » ouvre la conversation de l'agent, où sont le trombone et la copie des fichiers ;
la carte ne refait pas la logique des pièces jointes (copie, origine, pastilles) et en montre le nombre.

Choix du plan : une commande interactive du harness (`/model`…) ouvre la conversation au lieu de partir, comme dans
`Composer` où elle n'est jamais envoyée en message.

Choix du plan : pour un harness à longue liste de modèles (`searchesModels`), le sous-menu montre les favoris et le
modèle de l'agent, puis « Tous les modèles… » qui ouvre la conversation ; un menu ne cherche pas.

Textes à ajouter (français ; l'anglais au même chemin) :

```ts
card: {
  write: 'Écrire à {name}',
  settings: 'Paramètres',
  model: 'Modèle',
  effort: 'Effort',
  mode: 'Mode',
  attach: 'Joindre un fichier',
  allModels: 'Tous les modèles…',
  files: { one: '{count} fichier', other: '{count} fichiers' },
  label: '{name}, {state}',
},
```

- [ ] **Step 1: Write the failing tests**

```ts
it('sends by Enter through the conversation\'s path', async () => {
  const backend = fakeBackend({ get_conversation: () => [] });
  resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
  render(AgentCard, { agent: app.agents.a1 });
  const field = screen.getByRole('textbox', { name: 'Écrire à un' }) as HTMLInputElement;
  await fireEvent.input(field, { target: { value: 'Bonjour' } });
  await fireEvent.keyDown(field, { key: 'Enter' });
  await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: 'Bonjour' }));
  expect(field.value).toBe('');
});
it('shares its draft with the conversation', () => {
  setDraft('a1', { text: 'déjà là', files: [image('a.png')] });
  render(AgentCard, { agent: app.agents.a1 });
  expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('déjà là');
  expect(screen.getByText('1 fichier')).toBeTruthy();
});
it('opens its conversation for an interactive command, the text kept', async () => {
  // `/model` est interactif pour Claude Code (interactiveOf)
  await typeAndEnter('/model');
  expect(app.workspaceOn).toBe(false);
  expect(getDraft('a1').text).toBe('/model');
});
it('offers model, effort, mode and « Joindre un fichier »', async () => {
  render(AgentCard, { agent: app.agents.a1 });
  await fireEvent.click(screen.getByRole('button', { name: 'Paramètres' }));
  expect(menu.open!.items.filter((i) => !i.separator).map((i) => i.label)).toEqual(['Modèle', 'Effort', 'Mode', 'Joindre un fichier']);
  expect(menu.open!.items[0].submenu!.some((i) => i.checked)).toBe(true);
});
it('opens its conversation on a double click', async () => {
  app.openWorkspace('p1');
  render(AgentCard, { agent: app.agents.a1 });
  await fireEvent.dblClick(screen.getByRole('article'));
  expect(app.workspaceOn).toBe(false);
  expect(app.ui.selectedAgent.p1).toBe('a1');
});
```

Le menu se lit dans `menu.open!.items` (comme `TitleBar.test.ts` au lot 1), `menu.close()` en `afterEach` ; `setLang('fr')`,
`fakeBackend({ get_conversation: () => [] })` et `resetApp({ agents: [agent({ id: 'a1', name: 'un' })] })` en
`beforeEach`. `image(name)` (une `DraftAttachment` d'image) et `typeAndEnter(text)` (rend la carte, tape, `Entrée`)
sont des aides locales au fichier.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/AgentCard.test.ts`
Expected: FAIL, composant introuvable.

- [ ] **Step 3: Write the catalogue keys, then `AgentCard.svelte`**

- [ ] **Step 4: Run the tests and the type check**

Run: `npx vitest run src/components/workspace/AgentCard.test.ts src/lib/i18n && npm run check`
Expected: PASS ; `check` sans erreur nouvelle.

- [ ] **Step 5: Commit**

```bash
git add src/components/workspace/AgentCard.svelte src/components/workspace/AgentCard.test.ts src/lib/i18n
git commit -m "feat(workspace): la carte de l'agent, qui écrit par le chemin de la conversation"
```

### Task 6: La bande du plan et les cartes de l'équipe

**Files:**
- Create: `src/components/workspace/PlanBand.svelte`, `src/components/workspace/TeamCard.svelte`
- Modify: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/components/workspace/PlanBand.test.ts`, `src/components/workspace/TeamCard.test.ts`

**Interfaces:**
- Consumes: `bandTiles`, `TeamItem` (Task 4) ; `canWriteSub` (Task 2) ; `planView` ; `kindLabel`, `rowTitle`,
  `bgDuration` (`lib/bgtasks.ts`) ; `subDuration` (`lib/subagents.ts`) ; `fTok`, `fDur` ; `api.messageSubagent` ;
  `app.subDrafts` ; `app.openSubagent` ; `MicButton`.
- Produces: `<PlanBand agent={Agent} />` (`data-side="north"`) ; `<TeamCard agent={Agent} item={TeamItem} ontoggle={() => void} />`
  (`data-side="east"`) — un `<article>` dont l'`aria-label` est `workspace.card.label` (nom, état en mots) pour un
  sous-agent ou une tâche de fond.

PlanBand (spec, « Au nord ») : la pastille `workspace.plan.chip`, le titre, `workspace.plan.count`, la jauge en
segments (un par tâche, faites en `--run`), puis une tuile par ligne de `bandTiles` (rang `n`, titre, `step`, une
jauge de `percent`) ; la tuile `run` cerclée de `--run`, les `done` estompées ; une tuile `workspace.plan.folded` en tête
quand `folded > 0`.

TeamCard :
- `sub` : nom (`sub.kind ?? sub.title`), pastille `workspace.team.task` quand `planTask` est le rang d'une tâche,
  modèle, durée, `fTok(tokens)`, `doing` ; état en mots (`subagents.status.*`) ; si `canWriteSub(agent, sub)`, un
  champ (`aria-label` `workspace.team.write`) lié à `app.subDrafts['<agentId>:<sub.id>']` et `MicButton` ; `Entrée`
  → `api.messageSubagent(agent.id, sub.id, text)` puis champ vidé et toast `subagents.writeSent`. Double-clic →
  `app.openSubagent(agent.id, sub.id)`.
- `bg` : contour pointillé ; `kindLabel(row.kind)`, `rowTitle(row)` en mono, la durée.
- `folded` : un bouton `workspace.team.folded` (« + n terminés · n échecs ») ou `workspace.team.foldedCapped` ;
  clic → `ontoggle()`.

```ts
plan: {
  chip: 'Plan',
  count: '{done}/{total} tâches · {percent} %',
  folded: { one: '{count} faite', other: '{count} faites' },
},
team: {
  task: 'tâche {n}',
  write: 'Écrire à {name}',
  folded: '+ {done} terminés · {failed} échecs',
  foldedCapped: '+ 30+ terminés · {failed} échecs',
},
```

- [ ] **Step 1: Write the failing tests**

```ts
// PlanBand.test.ts
it('shows the plan, its count and a tile per task', () => {
  render(PlanBand, { agent: agent({ plan: planWith(['done', 'done', 'done', 'inProgress', 'pending', 'pending', 'pending']) }) });
  expect(screen.getByText('3/7 tâches · 43 %')).toBeTruthy();
  expect(screen.getAllByRole('listitem')).toHaveLength(7);
});
it('folds the tasks done past 9', () => {
  render(PlanBand, { agent: agent({ plan: planWith([...Array(8).fill('done'), 'inProgress', 'pending']) }) });
  expect(screen.getByText('8 faites')).toBeTruthy();
});
// TeamCard.test.ts
it('lets one write to a subagent running in the background', async () => {
  const backend = fakeBackend();
  const s = { ...sub('s1'), kind: 'Explore', taskId: 't1', background: true };
  render(TeamCard, { agent: agent({ id: 'a1' }), item: { kind: 'sub', id: 's1', sub: s }, ontoggle: () => {} });
  const field = screen.getByRole('textbox', { name: 'Écrire à Explore' });
  await fireEvent.input(field, { target: { value: 'continue' } });
  await fireEvent.keyDown(field, { key: 'Enter' });
  expect(backend.called('message_subagent')[0].args).toEqual({ id: 'a1', call: 's1', text: 'continue' });
});
it('has no field for a subagent in the foreground', () => {
  render(TeamCard, { agent: agent(), item: { kind: 'sub', id: 's1', sub: sub('s1') }, ontoggle: () => {} });
  expect(screen.queryByRole('textbox')).toBeNull();
});
it('says the pile, and unfolds it on a click', async () => {
  const ontoggle = vi.fn();
  render(TeamCard, { agent: agent(), item: { kind: 'folded', id: 'folded', done: 4, failed: 1, capped: false }, ontoggle });
  await fireEvent.click(screen.getByRole('button', { name: '+ 4 terminés · 1 échecs' }));
  expect(ontoggle).toHaveBeenCalled();
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/PlanBand.test.ts src/components/workspace/TeamCard.test.ts`
Expected: FAIL, composants introuvables.

- [ ] **Step 3: Write the catalogue keys, `PlanBand.svelte`, then `TeamCard.svelte`**

- [ ] **Step 4: Run the tests and the type check**

Run: `npx vitest run src/components/workspace && npm run check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/workspace/PlanBand.svelte src/components/workspace/TeamCard.svelte src/components/workspace/PlanBand.test.ts src/components/workspace/TeamCard.test.ts src/lib/i18n
git commit -m "feat(workspace): le plan au nord et l'équipe à l'est"
```

### Task 7: La boussole sur la toile, les traits et les flux

**Files:**
- Create: `src/components/workspace/Compass.svelte`, `src/components/workspace/Links.svelte`
- Modify: `src/components/workspace/WorkspaceView.svelte`, `src/components/workspace/Disc.svelte`
- Test: `src/components/workspace/WorkspaceView.test.ts`, `src/components/workspace/Disc.test.ts`

**Interfaces:**
- Consumes: `fitBest`, `keepAnchor`, `nearest`, `altitude` (Task 3) ; `frameOf`, `layoutNear`, `canvasOrder`,
  `SIDE_OF`, `Side`, `Frame`, `Link` (Task 4) ; `AgentCard` (Task 5) ; `PlanBand`, `TeamCard` (Task 6).
- Produces:
  - `<Compass frame={Placed<Frame>} agent={Agent} ontoggle={(agentId: string) => void} />` — un `<div>` absolu en
    (`x`, `y`) du monde ; ses enfants dans l'ordre du DOM : `PlanBand`, `AgentCard`, les `TeamCard` (ordre clavier
    nord, centre, est ; les lots 4 et 5 insèrent l'ouest avant la carte et le sud après l'équipe).
  - `<Links links={Link[]} w={number} h={number} />` — le `<svg>` unique sous les cartes. `run` : trait `--run` en
    tirets qui défilent et un point qui va de l'agent vers l'élément (`<animateMotion>` sur le chemin) ; `done` : gris
    fixe ; `failed` : `--del` fixe ; `plan` : `--line-2` fixe ; `launched` : pointillé `--muted`, avec le mot
    `workspace.launched` au milieu. Sous `prefers-reduced-motion`, ni défilement ni point.
  - `Disc` : `onpick: (agentId: string, side?: Side) => void` ; un clic sur un arc de satellite appelle
    `onpick(agentId, SIDE_OF[kind])` (et ne remonte pas au disque).
  - Dans `WorkspaceView` : `alt = $derived(altitude(cam.zoom))` ; état local `teamOpen: Record<string, boolean>` et
    `seenAt: Record<string, number>` (déplier une pile met `seenAt[agentId] = Date.now()`).

Comportement de la vue :
- Ordre des agents : `canvasOrder(agents, app.agents, app.tickets)` pour les deux altitudes.
- `far` : les disques du lot 1, inchangés. `near` : `layoutNear(order.map((a) => frameOf(a, { teamOpen, seenAt })), launched)`,
  `<Links>` puis un `<Compass>` par cadre, dans le même monde transformé.
- Quand `alt` change, `cam = keepAnchor(cam, avant, après)` pour l'agent `nearest` du centre de l'écran dans l'ancien
  placement ; sa boîte d'ancrage est son disque de loin, sa carte (`frame.centre` en coordonnées du monde) de près :
  c'est la carte de l'agent qui ne bouge pas à l'écran, quoi qu'il pousse autour d'elle.
- « Ajuster » et `0` : `cam = fitBest({ far: farWorld, near: nearWorld }, size())`.
- `pick(agentId, side = 'centre')` : zoom 1, centré sur la boîte du côté `side` du cadre de l'agent (la carte pour
  `centre`, la bande pour `north`, la colonne de l'équipe pour `east`) ; un côté pas encore livré (`west`, `south`)
  centre la carte.
- Le glisser de la toile ne démarre pas sur une carte ni dans un champ (`down` ignore `input, textarea, article,
  [data-side]`).

Choix du plan : un satellite se clique à la souris mais n'est pas un arrêt de tabulation à lui seul ; au clavier, le
disque (déjà un bouton) amène la boussole, dont les cartes sont dans l'ordre nord, ouest, centre, est, sud. Un bouton
dans un bouton n'est pas lisible au lecteur d'écran.

Texte à ajouter : `launched: 'a lancé'`.

- [ ] **Step 1: Write the failing tests**

```ts
// WorkspaceView.test.ts
it('shows the compasses from 45 %, the discs below', async () => {
  resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
  show(); // jsdom : la toile s'ouvre au zoom le plus bas, donc de loin
  expect(screen.queryByRole('textbox', { name: 'Écrire à un' })).toBeNull();
  for (let i = 0; i < 3; i++) await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' })); // 0,3 → 0,59
  expect(screen.getByRole('textbox', { name: 'Écrire à un' })).toBeTruthy();
});
it('keeps the agent in the middle when the altitude changes', async () => {
  resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' })] });
  const { container } = show();
  const before = centreOf(container, 'a2'); // centre à l'écran du disque, puis du cadre, d'après les transforms
  await zoomTo(0.59);
  expect(centreOf(container, 'a2')).toEqual(before);
});
it('zooms on the team when its satellite is clicked', async () => {
  resetApp({ agents: [agent({ id: 'a1', plan: planWithRunning(2, 0) })] });
  const { container } = show();
  await fireEvent.click(container.querySelector('.sat.team')!);
  expect(zoomShown()).toBe('100 %');
  expect(screen.getAllByRole('article', { name: /sous-agent/ }).length).toBe(2);
});
it('leaves the card\'s field alone', async () => {
  resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
  app.openWorkspace('p1');
  show();
  await zoomTo(1);
  const field = screen.getByRole('textbox', { name: 'Écrire à un' });
  field.focus();
  for (const key of ['+', '-', '0', 'Escape']) await fireEvent.keyDown(field, { key });
  expect(zoomShown()).toBe('100 %');
  expect(app.workspaceOn).toBe(true);
  await fireEvent.pointerDown(field, { clientX: 0, clientY: 0 });
  expect(document.querySelector('.ws.pan')).toBeNull();
});
it('animates only the links of what runs', async () => {
  resetApp({ agents: [agent({ id: 'a1', plan: planWith([], { agents: [sub('s1'), sub('s2', 'done')] }) })] });
  const { container } = show();
  await zoomTo(1);
  await fireEvent.click(screen.getByRole('button', { name: /terminés/ }));
  expect(container.querySelectorAll('path.link.run')).toHaveLength(1);
  expect(container.querySelectorAll('path.link.done')).toHaveLength(1);
});
// Disc.test.ts
it('picks the side of a satellite', async () => {
  const onpick = vi.fn();
  const { container } = renderDisc(agent({ id: 'a9', plan: planWithRunning(1, 0) }), onpick);
  await fireEvent.click(container.querySelector('.sat.team')!);
  expect(onpick).toHaveBeenCalledWith('a9', 'east');
  expect(onpick).toHaveBeenCalledTimes(1);
});
```

`zoomTo(z)` clique « Zoomer » ou « Dézoomer » jusqu'à ce que la barre dise `${Math.round(z * 100)} %` (pas de 1,25
depuis 0,3 : 0,375 ; 0,47 ; 0,59 ; 0,73 ; 0,92 ; 1,14…) ; pour 1, cliquer un disque, qui amène le zoom à 100 %.
`centreOf(container, agentId)` lit le centre à l'écran de l'agent à partir de la transformée du monde : le `<g>` du
disque (`data-agent`), ou la carte `[data-side="centre"]` dans le `<div data-agent>` de la boussole (positions écrites
en `left`/`top` du monde). Les lots suivants s'en servent aussi.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace`
Expected: FAIL, pas de champ « Écrire à un » au-delà de 45 %.

- [ ] **Step 3: Write `Links.svelte`, `Compass.svelte`, then wire `WorkspaceView.svelte` and `Disc.svelte`**

- [ ] **Step 4: Run the whole suite and the type check**

Run: `npx vitest run && npm run check`
Expected: PASS ; seuls les échecs déjà présents sur la base (`sdk/plugin cli`, `apps/mobile`), à lister dans le compte
rendu.

- [ ] **Step 5: Try it**

Page d'aperçu jetable comme au lot 1 (Vite + faux backend + Chrome) : deux agents, l'un avec un plan de 12 tâches et
trois sous-agents dont un en cours. Molette jusqu'à 45 % : les disques deviennent des boussoles sans saut. Taper un
message sur une carte, `Entrée`, quitter la toile : il est dans la conversation. Capture d'écran dans le compte rendu,
en thème sombre et clair.

- [ ] **Step 6: Commit**

```bash
git add src/components/workspace
git commit -m "feat(workspace): la boussole de chaque agent au-delà de 45 %, avec ses traits et ses flux"
```
