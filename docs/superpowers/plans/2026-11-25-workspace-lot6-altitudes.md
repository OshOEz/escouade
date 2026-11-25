# Workspace, lot 6 : les altitudes — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** entre 45 % et 80 % de zoom, chaque boussole se montre à mi-hauteur : ses côtés en piles (« Outils ×4 »,
« Équipe ×4 », « Livrables ×3 ») et son plan réduit à son titre et sa jauge ; un clic sur une pile la déplie sans
changer le zoom.

**Architecture:** la caméra connaît trois altitudes (`far`, `mid`, `near`) et `fitBest` les essaie de la plus proche à
la plus lointaine. `frameOf` reçoit le niveau (`mid` ou `near`) et les côtés dépliés ; à `mid`, un côté replié est une
pile. Les composants du lot 2 à 5 ne changent pas ; une carte de pile et une bande de plan réduite s'ajoutent.

**Tech Stack:** Svelte 5 (runes), TypeScript, vitest + @testing-library/svelte, catalogues `t()` fr/en.

**Base:** la branche `feat/workspace` après le lot 5 (`fitBest`, `ALTITUDES`, `keepAnchor`, `nearest` ; `frameOf` et
`FrameOpts` avec `teamOpen`, `seenAt`, `centreH`, `teamH`, `landing`, `southH` ; `Frame` avec `north`, `west`, `team`,
`south`, `links` ; `layoutNear`, `layoutFarOpen`, `waits` ; `Compass`, `PlanBand`, `ToolCard`, `TeamCard`,
`LandingZone`).

**Spec:** `docs/superpowers/specs/2026-11-25-vue-workspace-design.md` (« Les trois altitudes », « À mi-hauteur : des
piles », lot 6 de « Les lots », « Mes choix, à corriger » : seuils 45 % et 80 %).

## Global Constraints

- Seuils : « de loin sous 45 %, à mi-hauteur de 45 à 80 %, de près au-delà » (0,45 et 0,8 de zoom).
- « La boussole, mais chaque côté est une pile : « Outils ×4 », « Équipe ×4 », « Livrables ×3 », et le plan réduit à
  son titre et sa jauge. Un clic sur une pile la déplie sans changer le zoom. »
- « Un agent qui attend l'utilisateur est toujours déplié, quelle que soit l'altitude. »
- La caméra n'est pas mémorisée ; zoom borné de 0,3 à 2,5.
- Traits : un trait animé veut dire « ça tourne » ; un trait vers une pile ne s'anime que si un de ses éléments tourne.
- Ordre clavier nord, ouest, centre, est, sud ; une pile est un bouton qui dit son côté et son nombre.
- Textes par `t()`, zone `workspace` ; couleurs par les variables de `src/app.css` ; `prefers-reduced-motion` respecté.
- Commits en Conventional Commits, en français.

## Review Focus

- **Zoom pile à 45 % ou à 80 %** : 0,45 est à mi-hauteur, 0,8 de près (Task 1).
- **Déplier une pile** : le zoom ne bouge pas et l'agent reste à sa place à l'écran (Task 3, test « unfolds a pile
  without moving the camera »).
- **Un agent qui attend, à mi-hauteur** : il est déplié en entier, question comprise (Task 2, test « never folds a
  waiting agent »).
- **« Ajuster » avec six agents** : la toile choisit la mi-hauteur quand la boussole dépliée ne tient pas mais que les
  piles tiennent (Task 1).
- **Un sous-agent qui tourne dans une pile repliée** : le trait vers la pile s'anime (Task 2).

---

### Task 1: Trois altitudes

**Files:**
- Modify: `src/lib/workspace-camera.ts`
- Test: `src/lib/workspace-camera.test.ts` (le test `tells the altitude` du lot 1 change : 0,45 est `mid`)

**Interfaces:**
- Consumes (lots 1-2): `FAR_BELOW`, `ZOOM_MIN`, `ZOOM_MAX`, `fit`, `fitBest`, `ALTITUDES`, `Size`.
- Produces:
  - `type Altitude = 'far' | 'mid' | 'near'` ; `const NEAR_FROM = 0.8`.
  - `altitude(zoom: number): Altitude` — `< FAR_BELOW` → `far` ; `< NEAR_FROM` → `mid` ; sinon `near`.
  - `ALTITUDES` : `{ near, NEAR_FROM, ZOOM_MAX }`, `{ mid, FAR_BELOW, NEAR_FROM - 0.001 }`,
    `{ far, ZOOM_MIN, FAR_BELOW - 0.001 }` ; `fitBest(worlds: Record<Altitude, Size>, view: Size): Camera` inchangée
    dans son algorithme, elle essaie désormais trois mondes.

- [ ] **Step 1: Write the failing tests**

```ts
it('tells the altitude', () => {
  expect(altitude(0.44)).toBe('far');
  expect(altitude(0.45)).toBe('mid');
  expect(altitude(0.79)).toBe('mid');
  expect(altitude(0.8)).toBe('near');
});
it('fits the piles when the unfolded compasses do not hold', () => {
  const view = { w: 1200, h: 800 };
  const c = fitBest({ far: { w: 1500, h: 700 }, mid: { w: 2000, h: 1200 }, near: { w: 4000, h: 2600 } }, view);
  expect(altitude(c.zoom)).toBe('mid');
  expect(c.zoom).toBeLessThan(0.8);
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-camera.test.ts`
Expected: FAIL, `altitude(0.45)` rend `near`.

- [ ] **Step 3: Implement in `src/lib/workspace-camera.ts`**

Les appelants de `fitBest` (`WorkspaceView`) ne compilent plus sans le monde `mid` : c'est la Task 3 qui le leur donne ;
d'ici là, `npm run check` signale cet appel, et lui seul.

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-camera.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-camera.ts src/lib/workspace-camera.test.ts
git commit -m "feat(workspace): trois altitudes, la mi-hauteur entre 45 et 80 %"
```

### Task 2: Les piles dans le placement

**Files:**
- Modify: `src/lib/workspace-layout.ts`
- Test: `src/lib/workspace-layout.test.ts`

**Interfaces:**
- Consumes (lots 2-5): `frameOf`, `FrameOpts`, `Frame`, `Side`, `Link`, `toolCards`, `teamOf`, `waits`, `Landing`,
  `Placed`, `Box`, `GAP`.
- Produces:
  - `type Level = 'mid' | 'near'` ; `const PILE = { w: 184, h: 64 }`, `const BAND_MINI_H = 64`.
  - `type PileSide = 'west' | 'east' | 'south'` ; `type Pile = { side: PileSide; count: number; running: boolean }`
  - `FrameOpts` gagne `level?: Level` (défaut `near`) et `unfolded?: Side[]`.
  - `Frame` gagne `piles: Placed<Pile>[]` et `northMini: boolean` ; `Frame.south` (lot 5) devient `Box | null`,
    `null` à mi-hauteur quand le sud est replié (et le trait `south` avec lui).
  - `frameOf` à `level: 'mid'`, pour chaque côté `west`, `east`, `south` absent de `unfolded` : pas de cartes de ce
    côté, une pile à sa place si `count > 0` (`west` : `toolCards(agent).length` ; `east` : sous-agents et tâches de fond
    de `teamOf(agent, { teamOpen: true })` ; `south` : `landing.length`) ; `running` quand un de ses éléments tourne ;
    un trait vers la pile, `run` si `running`, sinon gris. `north` absent de `unfolded` : `northMini`, bande de hauteur
    `BAND_MINI_H`. Un agent qui `waits` est placé comme à `near`, quel que soit `level`.
  - `layoutNear` inchangée (elle range des cadres de toute taille).

Choix du plan : à mi-hauteur, un côté vide n'a pas de pile, pas même la zone d'atterrissage, qui reste « toujours
dessinée » de près ; une pile « Livrables ×0 » ne dirait rien.

Choix du plan : une pile dépliée le reste tant que la toile est ouverte (de près, tout est déplié de toute façon) ;
l'état n'est pas sauvé, comme la caméra.

- [ ] **Step 1: Write the failing tests**

```ts
const busy = () => agent({
  plan: planWith(['done', 'inProgress'], { agents: [sub('s1'), sub('s2', 'done')] }),
  canvas: { tools: [kit('skill', 'a'), kit('plugin', 'b')], subs: [] },
});
it('folds each side in a pile at mid-height', () => {
  const f = frameOf(busy(), { level: 'mid', landing: [art(true), art(true), art(false)] });
  expect(f.piles.map(({ side, count }) => ({ side, count }))).toEqual([
    { side: 'west', count: 2 },
    { side: 'east', count: 2 },
    { side: 'south', count: 3 },
  ]);
  expect(f.west).toEqual([]);
  expect(f.team).toEqual([]);
  expect(f.northMini).toBe(true);
  expect(f.h).toBeLessThan(frameOf(busy(), { landing: [art(true)] }).h);
});
it('unfolds the sides asked for', () => {
  const f = frameOf(busy(), { level: 'mid', unfolded: ['west'] });
  expect(f.west).toHaveLength(2);
  expect(f.piles.map((p) => p.side)).toEqual(['east']);
});
it('animates the link to a pile that holds something running', () => {
  const f = frameOf(busy(), { level: 'mid' });
  expect(f.links.find((l) => l.id === 'pile:east')?.state).toBe('run');
  expect(f.links.find((l) => l.id === 'pile:west')?.state).not.toBe('run');
});
it('never folds a waiting agent', () => {
  const a = { ...busy(), status: 'waiting' as const, requests: [req('q')] };
  expect(frameOf(a, { level: 'mid' }).piles).toEqual([]);
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: FAIL, `f.piles` indéfini.

- [ ] **Step 3: Implement in `src/lib/workspace-layout.ts`**

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: PASS, les tests des lots précédents compris.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts
git commit -m "feat(workspace): les côtés d'une boussole en piles à mi-hauteur"
```

### Task 3: La pile, la bande réduite, et la toile à mi-hauteur

**Files:**
- Create: `src/components/workspace/PileCard.svelte`
- Modify: `src/components/workspace/PlanBand.svelte` (prop `mini`), `src/components/workspace/Compass.svelte`,
  `src/components/workspace/WorkspaceView.svelte`
- Modify: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/components/workspace/PileCard.test.ts`, `src/components/workspace/PlanBand.test.ts`,
  `src/components/workspace/WorkspaceView.test.ts`

**Interfaces:**
- Consumes: `altitude`, `fitBest`, `keepAnchor`, `nearest` (Task 1, lot 2) ; `Level`, `Pile`, `FrameOpts.level`,
  `FrameOpts.unfolded`, `Frame.piles`, `Frame.northMini` (Task 2).
- Produces:
  - `<PileCard pile={Pile} onunfold={(side: PileSide) => void} />` — un `<button data-side={pile.side}>`, texte
    `workspace.pile.<side>`, `aria-expanded="false"` ; anneau `--run` quand `running`.
  - `<PlanBand agent mini={boolean} />` — `mini` : la pastille, le titre, `workspace.plan.count` et la jauge, sans tuiles.
  - `Compass` : rend les `PileCard` à leur côté dans l'ordre du DOM nord, ouest, centre, est, sud ; relaie
    `onunfold(agentId, side)`.
  - `WorkspaceView` : état local `unfolded: Record<string, Side[]>` ; `alt` à trois valeurs ; à `mid`,
    `frameOf(a, { …, level: 'mid', unfolded: unfolded[a.id] })` ; à `near`, `level: 'near'` ; « Ajuster » et `0` :
    `fitBest({ far, mid, near }, size())` ; le réancrage du lot 2 (`keepAnchor` sur l'agent `nearest`) joue à chaque
    changement d'altitude et quand une pile se déplie (l'agent cliqué garde sa place à l'écran).

Textes :

```ts
pile: { west: 'Outils ×{count}', east: 'Équipe ×{count}', south: 'Livrables ×{count}' },
```

- [ ] **Step 1: Write the failing tests**

```ts
// PileCard.test.ts
it('says its side and its count, and unfolds on a click', async () => {
  const onunfold = vi.fn();
  render(PileCard, { pile: { side: 'west', count: 4, running: false }, onunfold });
  await fireEvent.click(screen.getByRole('button', { name: 'Outils ×4' }));
  expect(onunfold).toHaveBeenCalledWith('west');
});
// PlanBand.test.ts
it('keeps only its title and its gauge when small', () => {
  render(PlanBand, { agent: agent({ plan: planWith(['done', 'pending']) }), mini: true });
  expect(screen.getByText('1/2 tâches · 50 %')).toBeTruthy();
  expect(screen.queryAllByRole('listitem')).toHaveLength(0);
});
// WorkspaceView.test.ts
it('shows piles at mid-height and the cards up close', async () => {
  resetApp({ agents: [agent({ id: 'a1', canvas: { tools: [kit('skill', 'a'), kit('plugin', 'b')], subs: [] } })] });
  show();
  await zoomTo(0.59);
  expect(screen.getByRole('button', { name: 'Outils ×2' })).toBeTruthy();
  await zoomTo(0.92);
  expect(screen.queryByRole('button', { name: 'Outils ×2' })).toBeNull();
  expect(screen.getByRole('article', { name: /Skill a/ })).toBeTruthy();
});
it('unfolds a pile without moving the camera', async () => {
  resetApp({ agents: [agent({ id: 'a1', canvas: { tools: [kit('skill', 'a')], subs: [] } }), agent({ id: 'a2' })] });
  const { container } = show();
  await zoomTo(0.59);
  const before = centreOf(container, 'a1');
  await fireEvent.click(screen.getByRole('button', { name: 'Outils ×1' }));
  expect(zoomShown()).toBe('59 %');
  expect(screen.getByRole('article', { name: /Skill a/ })).toBeTruthy();
  expect(centreOf(container, 'a1')).toEqual(before);
});
```

`zoomTo`, `zoomShown` et `centreOf` sont les aides du lot 2 dans ce fichier ; `kit` est dans `src/test/workspace.ts`
(lot 4). `centreOf` mesure le centre de la carte de l'agent (`[data-side="centre"]`) ; le réancrage du lot 2 se fait
déjà sur elle, et le dépliage d'une pile le déclenche aussi, avec la boîte `centre` du cadre avant et après.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace`
Expected: FAIL, pas de pile à 59 %.

- [ ] **Step 3: Write the catalogue keys, `PileCard`, `PlanBand`'s `mini`, then wire `Compass` and `WorkspaceView`**

- [ ] **Step 4: Run the whole suite and the type check**

Run: `npx vitest run && npm run check`
Expected: PASS ; seuls les échecs déjà présents sur la base ; `check` ne signale plus l'appel de `fitBest`.

- [ ] **Step 5: Try it**

Page d'aperçu jetable ou app : six agents. « Ajuster » : la toile s'ouvre à mi-hauteur si les boussoles dépliées ne
tiennent pas. Molette de 30 % à 250 % : disques, piles, boussoles, sans saut de l'agent du centre. Cliquer « Équipe ×4 »
d'un agent : elle se déplie, le zoom reste. Un agent en attente reste déplié à 30 % comme à 60 %. Capture d'écran aux
trois altitudes.

- [ ] **Step 6: Commit**

```bash
git add src/components/workspace src/lib/i18n
git commit -m "feat(workspace): le niveau du milieu, ses piles qu'un clic déplie"
```
