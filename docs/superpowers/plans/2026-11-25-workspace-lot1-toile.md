# Workspace, lot 1 : la toile et les disques — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ouvrir une toile « Workspace » depuis le bouton Éditeur, où chaque agent du projet est un disque avec ses
satellites équipe et question, qu'on déplace et qu'on zoome.

**Architecture:** un drapeau par projet dans `state.svelte.ts`, comme `board`/`docs`/`prs`. Le placement est une
fonction pure (`lib/workspace-layout.ts`) qui rend des boîtes pour chaque altitude ; la caméra est une fonction pure
(`lib/workspace-camera.ts`). La vue (`components/workspace/`) dessine les disques en SVG dans un monde transformé.

**Tech Stack:** Svelte 5 (runes), TypeScript, vitest + @testing-library/svelte, catalogues `t()` fr/en.

**Spec:** `docs/superpowers/specs/2026-11-25-vue-workspace-design.md` (sections « L'entrée et la sortie », « La
toile », « Les trois altitudes › De loin », lot 1 de « Les lots »).

## Global Constraints

- Aucune dépendance ajoutée (pas de bibliothèque de graphe ni de pan/zoom).
- Tout texte lu à l'écran vient de `t()`, en français (`src/lib/i18n/fr/workspace.ts`) et en anglais
  (`src/lib/i18n/en/workspace.ts`) ; `npm run check` doit passer.
- Couleurs uniquement par les variables de `src/app.css` (`--run`, `--wait`, `--del`, `--done`, `--raise`, `--line-2`,
  `--text`, `--muted`…), valables dans les deux thèmes.
- Zoom borné de 0,3 à 2,5 ; pas des boutons ×1,25 ; fond à points de 1,2 px tous les 18 px, qui suit la caméra.
- Seuil d'altitude : « de loin » sous 0,45 de zoom. Dans ce lot, la toile ne montre que la vue de loin à toutes les
  échelles ; `altitude()` existe déjà pour le lot 2.
- `prefers-reduced-motion` fige le halo et l'orbite.
- Commits en Conventional Commits, en français, comme l'historique (`feat(workspace): …`).

## Review Focus

- **Projet sans agent** : la toile montre un message « Aucun agent dans ce projet » et pas de toile vide muette.
- **Un agent archivé ou un agent d'un autre projet** n'apparaît jamais (`projectAgents` les exclut déjà ; le test le
  fige).
- **Changer de projet** avec la toile ouverte : la toile du nouveau projet n'est ouverte que si on l'y avait ouverte
  (drapeau par projet).
- **Échap dans un champ** (recherche ⌘K ouverte, par exemple) ne quitte pas la toile.
- **Ajuster avec une fenêtre minuscule ou un seul agent** : le zoom reste dans [0,3 ; 2,5] et le disque est centré.

---

### Task 1: L'état de la vue Workspace

**Files:**
- Modify: `src/lib/state.svelte.ts` (champs près de `prs` l. ~463, dérivés près de `prsOn` l. ~512, méthodes près de
  `openBoard`/`closePrs` l. ~1548-1650 ; `panesOn` l. 594)
- Test: `src/lib/state.test.ts` (nouveau `describe('workspace', …)` à côté de `describe('board', …)`)

**Interfaces:**
- Produces: `app.workspace: Record<string, boolean>`, `app.workspaceOn: boolean` (dérivé),
  `app.openWorkspace(projectId = app.ui.activeProject): void`, `app.closeWorkspace(projectId = app.ui.activeProject): void`.

- [ ] **Step 1: Write the failing tests**

```ts
describe('workspace', () => {
  beforeEach(() => resetApp({ projects: [project({ id: 'p1' }), project({ id: 'p2' })] }));

  it('opens in place of the board, the docs, the pull requests and the editor', () => {
    app.openBoard('p1');
    app.openWorkspace('p1');
    expect(app.workspaceOn).toBe(true);
    expect(app.boardOn).toBe(false);
    expect(app.docsOn).toBe(false);
    expect(app.prsOn).toBe(false);
    expect(app.editorOn).toBe(false);
    expect(app.panesOn).toBe(false);
  });

  it('is closed by the other views', () => {
    app.openWorkspace('p1');
    app.openDocs('p1');
    expect(app.workspaceOn).toBe(false);
  });

  it('closes and gives back the view underneath', () => {
    app.openWorkspace('p1');
    app.closeWorkspace('p1');
    expect(app.workspaceOn).toBe(false);
    expect(app.ui.view).toBe('project');
  });

  it('is per project', () => {
    app.openWorkspace('p1');
    app.selectProject('p2');
    expect(app.workspaceOn).toBe(false);
    app.selectProject('p1');
    expect(app.workspaceOn).toBe(true);
  });
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/state.test.ts -t workspace`
Expected: FAIL, `app.openWorkspace is not a function`.

- [ ] **Step 3: Implement**

`workspace = $state<Record<string, boolean>>({})` (non sauvé, comme `board`). `workspaceOn = $derived(this.ui.view ===
'project' && !!this.project && !!this.workspace[this.project.id])`. `openWorkspace` fait comme `openBoard` : projet
actif, `ui.view = 'project'`, `board`/`docs`/`prs` à false, `selectedTerm`/`selectedLaunch` à null,
`closeEditor(projectId)`, `persistUi()`, puis `workspace[projectId] = true`. `closeWorkspace` met le drapeau à false.
`openBoard`, `openDocs`, `openPrs` et `openEditor` mettent `workspace[projectId] = false`. `boardOn`, `docsOn`, `prsOn`
et `panesOn` ajoutent `&& !this.workspace[this.project.id]` ; `editorOn` aussi.

- [ ] **Step 4: Run them to see them pass, and the whole state suite**

Run: `npx vitest run src/lib/state.test.ts`
Expected: PASS, aucun test existant cassé.

- [ ] **Step 5: Commit**

```bash
git add src/lib/state.svelte.ts src/lib/state.test.ts
git commit -m "feat(workspace): l'état de la vue Workspace, par projet"
```

### Task 2: La caméra

**Files:**
- Create: `src/lib/workspace-camera.ts`
- Test: `src/lib/workspace-camera.test.ts`

**Interfaces:**
- Produces:
  - `type Camera = { x: number; y: number; zoom: number }`
  - `const ZOOM_MIN = 0.3`, `const ZOOM_MAX = 2.5`, `const ZOOM_STEP = 1.25`, `const DOT = 18`
  - `clampZoom(z: number): number`
  - `zoomAt(cam: Camera, factor: number, px: number, py: number): Camera` — le point d'écran (px, py) reste fixe
  - `pan(cam: Camera, dx: number, dy: number): Camera`
  - `fit(world: { w: number; h: number }, view: { w: number; h: number }, margin?: number): Camera` — marge 32 par défaut,
    zoom borné, monde centré ; un monde vide (`w` ou `h` à 0) rend `{ x: view.w / 2, y: view.h / 2, zoom: 1 }`
  - `type Altitude = 'far' | 'near'`, `const FAR_BELOW = 0.45`, `altitude(zoom: number): Altitude`

- [ ] **Step 1: Write the failing tests**

```ts
it('clamps the zoom', () => {
  expect(clampZoom(0.1)).toBe(0.3);
  expect(clampZoom(9)).toBe(2.5);
});
it('keeps the point under the pointer fixed', () => {
  const c = zoomAt({ x: 0, y: 0, zoom: 1 }, 2, 100, 50);
  expect(c).toEqual({ x: -100, y: -50, zoom: 2 });
  // le point du monde sous (100, 50) avant et après est le même
  expect((100 - c.x) / c.zoom).toBe(100);
});
it('does not move when the zoom is already at its bound', () => {
  expect(zoomAt({ x: 10, y: 20, zoom: 2.5 }, 2, 0, 0)).toEqual({ x: 10, y: 20, zoom: 2.5 });
});
it('fits a world in the view, centred', () => {
  const c = fit({ w: 1000, h: 500 }, { w: 1064, h: 800 });
  expect(c.zoom).toBe(1);
  expect(c.x).toBe(32);
  expect(c.y).toBe(150);
});
it('fits within the zoom bounds, even in a tiny view', () => {
  expect(fit({ w: 5000, h: 5000 }, { w: 200, h: 100 }).zoom).toBe(0.3);
  expect(fit({ w: 10, h: 10 }, { w: 2000, h: 2000 }).zoom).toBe(2.5);
});
it('centres an empty world', () => {
  expect(fit({ w: 0, h: 0 }, { w: 800, h: 600 })).toEqual({ x: 400, y: 300, zoom: 1 });
});
it('tells the altitude', () => {
  expect(altitude(0.44)).toBe('far');
  expect(altitude(0.45)).toBe('near');
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-camera.test.ts`
Expected: FAIL, module introuvable.

- [ ] **Step 3: Implement the functions in `src/lib/workspace-camera.ts`**

`fit` : `zoom = clampZoom(min((view.w - 2·margin) / w, (view.h - 2·margin) / h))`, puis
`x = (view.w - w·zoom) / 2`, `y = (view.h - h·zoom) / 2`.

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-camera.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-camera.ts src/lib/workspace-camera.test.ts
git commit -m "feat(workspace): la caméra de la toile, zoom autour du pointeur et ajustement"
```

### Task 3: Le modèle et le placement des disques

**Files:**
- Create: `src/lib/workspace-layout.ts`
- Test: `src/lib/workspace-layout.test.ts`

**Interfaces:**
- Consumes: `Agent`, `PlanState` (`src/lib/types.ts`) ; `planView(agent)` (`src/lib/plan.ts`) pour `done`/`total`.
- Produces:
  - `type DiscState = 'run' | 'wait' | 'error' | 'done' | 'idle'`
  - `type SatKind = 'team' | 'tools' | 'deliverables' | 'ci' | 'question'` (ordre fixe, de haut en bas)
  - `type Satellite = { kind: SatKind; count: number }`
  - `type Disc = { agentId: string; name: string; state: DiscState; done: number; total: number; satellites: Satellite[] }`
  - `discOf(agent: Agent): Disc`
  - `type Placed<T> = T & { x: number; y: number; w: number; h: number }`
  - `const DISC = { r: 92, orbit: 122, w: 300, h: 300, gap: 70 }`
  - `layoutFar(discs: Disc[], perRow?: number): { items: Placed<Disc>[]; w: number; h: number }` — rangées de `perRow`
    disques (défaut 4), dans l'ordre donné ; un tableau vide rend `{ items: [], w: 0, h: 0 }`

Règles de `discOf` (spec, « De loin ») :
- `state` : `waiting` → `wait` ; `error` → `error` ; `running` → `run` ; `done` → `done` ; sinon `idle`.
- `done`/`total` : ceux de `planView(agent)?.mini`, sinon 0/0 (« sans plan »).
- satellite `team` : sous-agents `running` de `plan.agents` + tâches de fond en cours de `plan.bgTasks` ; absent à 0.
- satellite `question` : `agent.requests.length` ; absent à 0.
- Les satellites `tools`, `deliverables`, `ci` ne sont jamais produits dans ce lot (leurs lots les ajoutent).

- [ ] **Step 1: Write the failing tests** (avec `agent()` de `src/test/ipc.ts`)

```ts
it('says the state in one word', () => {
  expect(discOf(agent({ status: 'waiting' })).state).toBe('wait');
  expect(discOf(agent({ status: 'running' })).state).toBe('run');
  expect(discOf(agent({ status: 'idle' })).state).toBe('idle');
});
it('counts the plan', () => {
  const a = agent({ plan: planWith(['done', 'done', 'done', 'inProgress', 'pending', 'pending', 'pending']) });
  expect(discOf(a)).toMatchObject({ done: 3, total: 7 });
});
it('has no gauge without a plan', () => {
  expect(discOf(agent())).toMatchObject({ done: 0, total: 0 });
});
it('shows only the satellites that have something to say, in order', () => {
  const a = agent({ status: 'waiting', requests: [req('q1'), req('q2')], plan: planWithRunning(2, 1) });
  expect(discOf(a).satellites).toEqual([{ kind: 'team', count: 3 }, { kind: 'question', count: 2 }]);
  expect(discOf(agent()).satellites).toEqual([]);
});
it('lays the discs out in rows', () => {
  const discs = [1, 2, 3, 4, 5].map((i) => discOf(agent({ id: `a${i}` })));
  const { items, w, h } = layoutFar(discs, 4);
  expect(items[4]).toMatchObject({ x: 0, y: DISC.h + DISC.gap });
  expect(w).toBe(4 * DISC.w + 3 * DISC.gap);
  expect(h).toBe(2 * DISC.h + DISC.gap);
});
it('lays out nothing', () => {
  expect(layoutFar([])).toEqual({ items: [], w: 0, h: 0 });
});
```

`planWith`, `planWithRunning` et `req` sont de petites aides locales au fichier de test qui fabriquent un `PlanState`
(tâches, sous-agents `running`, tâches de fond `running`) et un `PendingRequest` minimal ; ajouter `kind: 'command'` aux
tâches de fond.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: FAIL, module introuvable.

- [ ] **Step 3: Implement `discOf` and `layoutFar` in `src/lib/workspace-layout.ts`**

Une tâche de fond ou un sous-agent est « en cours » selon `isRunning` (exporté par `src/lib/bgtasks.ts`).

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts
git commit -m "feat(workspace): le modèle des disques et leur placement de loin"
```

### Task 4: Le disque

**Files:**
- Create: `src/components/workspace/Disc.svelte`
- Create: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts` ; Modify: `src/lib/i18n/fr/index.ts`,
  `src/lib/i18n/en/index.ts` (déclarer la zone)
- Test: `src/components/workspace/Disc.test.ts`

**Interfaces:**
- Consumes: `Placed<Disc>` (Task 3) ; `cardActivity(agent, 0)` (`src/lib/sidebar.ts`) pour la ligne d'activité.
- Produces: `<Disc disc={Placed<Disc>} agent={Agent} onpick={(agentId: string) => void} />`, un `<g>` SVG positionné en
  `translate(x + w/2, y + h/2)`.

Textes de la zone `workspace` (français ; l'anglais au même chemin) :

```ts
export default {
  open: 'Workspace',
  exit: 'Quitter le workspace',
  label: 'Toile des agents',
  empty: 'Aucun agent dans ce projet',
  count: { one: '{count} agent', other: '{count} agents' },
  questions: { one: '{count} question', other: '{count} questions' },
  zoomIn: 'Zoomer',
  zoomOut: 'Dézoomer',
  fit: 'Ajuster',
  state: { run: 'En cours', wait: 'À toi', error: 'Erreur', done: 'Terminé', idle: 'Prêt' },
  noPlan: 'sans plan',
  progress: '{done}/{total} · {percent} %',
  sat: {
    team: { one: 'équipe', other: 'équipe' },
    question: { one: 'question', other: 'questions' },
  },
  discLabel: '{name}, {state}',
} as const satisfies Tree;
```

Dessin (spec, « De loin ») : disque de rayon 92, contour 2 px de la couleur d'état (`--run`, `--wait`, `--del`, gris
pour `done`/`idle`) ; `wait` : fond `color-mix(--wait 9 %, --raise)` et un halo (cercle r = 98, trait 6, opacité qui
pulse) ; orbite r = 122 en pointillé, qui tourne si `run`. Dans le disque, de haut en bas : point + état en capitales,
nom en gras, ligne d'activité, jauge en segments (un par tâche, faites en `--run`) et `progress`, ou `noPlan`.
Satellites : des arcs entre les rayons 102 et 142, de 24° chacun, 4° d'écart, centrés sur 0° (flanc droit), dans
l'ordre de `satellites` ; dans chaque arc, le nombre et le mot de `sat`. Le `<g>` est un bouton (`role="button"`,
`tabindex="0"`, `aria-label={discLabel}`) ; clic et `Entrée` appellent `onpick(agentId)`.

- [ ] **Step 1: Write the failing tests**

```ts
it('names the agent and its state', () => {
  renderDisc(agent({ name: 'release-2-3', status: 'running' }));
  expect(screen.getByRole('button', { name: 'release-2-3, En cours' })).toBeTruthy();
});
it('shows the gauge, or says there is no plan', () => {
  renderDisc(agent({ plan: planWith(['done', 'done', 'done', 'pending', 'pending', 'pending', 'pending']) }));
  expect(screen.getByText('3/7 · 43 %')).toBeTruthy();
  cleanup();
  renderDisc(agent());
  expect(screen.getByText('sans plan')).toBeTruthy();
});
it('turns yellow and shows its questions when it waits', () => {
  const { container } = renderDisc(agent({ status: 'waiting', requests: [req('q1'), req('q2'), req('q3')] }));
  expect(container.querySelector('.halo')).toBeTruthy();
  expect(screen.getByText('questions')).toBeTruthy();
  expect(screen.getByText('3')).toBeTruthy();
});
it('is picked by a click and by Enter', async () => {
  const onpick = vi.fn();
  renderDisc(agent({ id: 'a9' }), onpick);
  await fireEvent.keyDown(screen.getByRole('button'), { key: 'Enter' });
  expect(onpick).toHaveBeenCalledWith('a9');
});
```

`renderDisc(a, onpick?)` rend `Disc` dans un `<svg>` avec `disc = { ...discOf(a), x: 0, y: 0, w: 300, h: 300 }` ;
`setLang('fr')` en `beforeEach`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/Disc.test.ts`
Expected: FAIL, composant introuvable.

- [ ] **Step 3: Write the catalogue zone, then `Disc.svelte`**

- [ ] **Step 4: Run the tests and the type check**

Run: `npx vitest run src/components/workspace/Disc.test.ts src/lib/i18n && npm run check`
Expected: PASS, et `check` sans erreur (le catalogue anglais a les mêmes clés).

- [ ] **Step 5: Commit**

```bash
git add src/components/workspace/Disc.svelte src/components/workspace/Disc.test.ts src/lib/i18n
git commit -m "feat(workspace): le disque d'un agent et ses satellites"
```

### Task 5: La toile, et son entrée par le bouton Éditeur

**Files:**
- Create: `src/components/workspace/WorkspaceView.svelte`
- Modify: `src/App.svelte` (l. ~232, une branche `{:else if app.workspaceOn}` avant `app.boardOn`)
- Modify: `src/components/TitleBar.svelte` (l. 94-103, le bouton Éditeur ouvre un menu)
- Test: `src/components/workspace/WorkspaceView.test.ts`, `src/components/TitleBar.test.ts`

**Interfaces:**
- Consumes: `app.workspaceOn`, `app.openWorkspace`, `app.closeWorkspace` (Task 1) ; `Camera`, `zoomAt`, `pan`, `fit`,
  `ZOOM_STEP`, `DOT` (Task 2) ; `discOf`, `layoutFar` (Task 3) ; `Disc` (Task 4) ; `app.projectAgents` ;
  `menu.showAt(el, items)` (`src/lib/menu.svelte.ts`).
- Produces: `<WorkspaceView project={Project} />`.

Comportement :
- Les agents sont `app.projectAgents` (ordre de la barre latérale, archivés exclus), passés à `discOf` puis `layoutFar`.
- À l'ouverture et au clic sur « Ajuster » (et sur `0`) : `fit` du monde dans la taille du conteneur. La caméra n'est
  pas mémorisée.
- Molette : `pan` ; Ctrl/⌘ + molette ou pincement : `zoomAt(cam, exp(-deltaY · 0.01), clientX, clientY)` ; glisser le
  fond : `pan` (pointer capture, curseur `grab`/`grabbing`) ; `+`/`-` : `zoomAt` ×`ZOOM_STEP` / ÷`ZOOM_STEP` au centre.
- Fond : `background-size: DOT·zoom` et `background-position: x y`, comme Kata.
- Barre flottante en haut au centre : `zoomOut`, le pourcentage, `zoomIn`, `fit`, puis `count` et `questions` (somme
  des `requests`, absente à 0).
- Bouton « Quitter le workspace » en haut à gauche → `app.closeWorkspace()`. `Échap` fait de même, sauf si l'élément
  actif est un champ (`input`, `textarea`, `[contenteditable]`) ou si un menu ou une boîte de dialogue est ouvert.
- Clic sur un disque : `zoomAt` pour amener ce disque au centre à un zoom de 1 (le lot 2 y mettra la boussole).
- Sans agent : le texte `empty` au centre, la barre et « Quitter » restent.
- Bouton Éditeur : son `onclick` appelle `menu.showAt(e.currentTarget, [{ label: t('common.editor'), icon: 'code',
  onClick: () => app.openEditor(...) }, { label: t('workspace.open'), icon: 'diagram',
  onClick: () => app.openWorkspace(activeProject.id) }])` ; la ligne cochée (`checked`) est celle de la vue ouverte.
  `onpointerdown` (le glisser vers un volet) ne change pas.

- [ ] **Step 1: Write the failing tests**

```ts
// WorkspaceView.test.ts
it('draws a disc per agent of the project, archived ones aside', () => {
  resetApp({ agents: [agent({ id: 'a1', name: 'un' }), agent({ id: 'a2', name: 'deux', archived: true }), agent({ id: 'a3', name: 'trois', projectId: 'p2' })] });
  render(WorkspaceView, { project: app.projects[0] });
  expect(screen.getByRole('button', { name: /^un,/ })).toBeTruthy();
  expect(screen.queryByRole('button', { name: /^deux,/ })).toBeNull();
  expect(screen.queryByRole('button', { name: /^trois,/ })).toBeNull();
});
it('says when there is no agent', () => {
  resetApp({ agents: [] });
  render(WorkspaceView, { project: app.projects[0] });
  expect(screen.getByText('Aucun agent dans ce projet')).toBeTruthy();
});
it('counts the agents and the questions', () => {
  resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2', status: 'waiting', requests: [req('q')] })] });
  render(WorkspaceView, { project: app.projects[0] });
  expect(screen.getByText('2 agents')).toBeTruthy();
  expect(screen.getByText('1 question')).toBeTruthy();
});
it('zooms with its buttons, within bounds', async () => {
  render(WorkspaceView, { project: app.projects[0] });
  for (let i = 0; i < 20; i++) await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
  expect(screen.getByText('250 %')).toBeTruthy();
});
it('closes on « Quitter » and on Escape, not on Escape in a field', async () => {
  app.openWorkspace('p1');
  render(WorkspaceView, { project: app.projects[0] });
  const field = document.body.appendChild(document.createElement('input'));
  field.focus();
  await fireEvent.keyDown(field, { key: 'Escape' });
  expect(app.workspaceOn).toBe(true);
  field.remove();
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(app.workspaceOn).toBe(false);
});

// TitleBar.test.ts
it('opens the editor or the workspace from the Éditeur menu', async () => {
  render(TitleBar);
  await fireEvent.click(screen.getByRole('button', { name: 'Ouvrir l’éditeur du projet' }));
  await fireEvent.click(screen.getByRole('menuitem', { name: /Workspace/ }));
  expect(app.workspaceOn).toBe(true);
});
```

`TitleBar` se rend sans props (`render(TitleBar)`, comme le fichier de test existant) ; monter aussi `ContextMenu` si
c'est lui qui rend les `menuitem`. En jsdom, la taille du conteneur est 0 : `fit` rend alors le cas « monde
vide » ou un zoom borné, ce qui suffit aux tests.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace src/components/TitleBar.test.ts`
Expected: FAIL, `WorkspaceView` introuvable et pas de `menuitem` « Workspace ».

- [ ] **Step 3: Write `WorkspaceView.svelte`, then wire `App.svelte` and `TitleBar.svelte`**

Monde : un `<div>` absolu transformé par `translate(x, y) scale(zoom)`, qui contient un `<svg>` de la taille du monde
avec un `<Disc>` par élément de `layoutFar`. Le conteneur a `role="application"` et `aria-label={t('workspace.label')}`.
Les commandes flottantes sont en dehors du monde.

- [ ] **Step 4: Run the tests, the whole suite and the type check**

Run: `npx vitest run && npm run check`
Expected: PASS partout ; seuls des échecs déjà présents sur la branche avant ce lot sont admis, à lister dans le
compte rendu.

- [ ] **Step 5: Try it in the app**

Lancer l'app (skill `run`), ouvrir un projet avec deux agents, cliquer « Éditeur » › « Workspace » : deux disques, la
barre, « Quitter ». Molette, Ctrl + molette, glisser, `+`, `-`, `0`, Échap. Capture d'écran dans le compte rendu.

- [ ] **Step 6: Commit**

```bash
git add src/components/workspace src/App.svelte src/components/TitleBar.svelte src/components/TitleBar.test.ts
git commit -m "feat(workspace): la toile des agents, ouverte depuis le bouton Éditeur"
```

---

## Les lots suivants

Chacun aura son plan, écrit quand le précédent est livré, à partir de la même spec :

2. La boussole (bascule par `altitude()`, carte d'agent et chemin d'envoi sorti de `Composer`, plan, équipe, flux).
3. Les questions dans la carte.
4. L'outillage, avec le résumé de toile dans le cœur.
5. La zone d'atterrissage (artefact, CI distante, CI locale, déploiement).
6. Le niveau du milieu et ses piles.
