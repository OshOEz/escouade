# Workspace, lot 3 : les questions — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** répondre depuis la toile : la carte d'un agent qui attend grandit et montre l'encart jaune de la
conversation (`QuestionCard`, `PermissionCard` montés tels quels), la question d'un sous-agent s'affiche sur la carte du
sous-agent, et un agent qui attend est déplié à toutes les altitudes.

**Architecture:** un composant `AskInset` charge la conversation de l'agent tant qu'il attend (`conversationOf(id).show()`)
et monte les cartes de la conversation pour ses demandes. Les cartes mesurent leur hauteur ; le placement pur reçoit
ces hauteurs et décale les voisins. De loin, `layoutFarOpen` met la boussole d'un agent en attente à la place de son
disque, dans les mêmes rangées.

**Tech Stack:** Svelte 5 (runes), TypeScript, vitest + @testing-library/svelte, catalogues `t()` fr/en.

**Base:** la branche `feat/workspace` après le lot 2 (la boussole : `AgentCard`, `TeamCard`, `Compass`, `Links`,
`frameOf`, `layoutNear`, `packRows`, `submitDraft`).

**Spec:** `docs/superpowers/specs/2026-11-25-vue-workspace-design.md` (« La boussole, côté par côté › Au centre ›
La question », « Les trois altitudes › De près » (« Un agent qui attend l'utilisateur est toujours déplié »), « D'où
viennent les données », lot 3 de « Les lots », « Choix validés le 25 novembre »).

## Global Constraints

- « La toile monte les composants de la conversation » : `QuestionCard` et `PermissionCard` tels quels (plusieurs
  questions, réponses multiples, descriptions, « Envoyer ») ; aucune seconde logique de question ou de permission.
- La conversation d'un agent n'est chargée par la toile que tant qu'il attend, « le temps de sa question » ; aucune
  autre.
- « La question d'un sous-agent (`sub`) s'affiche sur la carte du sous-agent » ; la carte de l'agent prend un contour
  jaune et dit « question de <sous-agent> ».
- « La carte ne dépasse pas 70 % de la hauteur de la fenêtre (à 100 % de zoom) ; au-delà, l'encart défile dans la
  carte. »
- « Les objets voisins se décalent ; rien ne se chevauche. »
- « Un agent qui attend l'utilisateur est toujours déplié, quelle que soit l'altitude. »
- Écrire sur la carte d'un agent qui attend répond à sa question (chemin du lot 2, `routeOf`).
- Textes par `t()`, zone `workspace` ; couleurs par les variables de `src/app.css` (`--wait`, `--wait-bg`,
  `--wait-line`) ; `prefers-reduced-motion` fige le halo.
- Commits en Conventional Commits, en français.

## Review Focus

- **Deux agents qui attendent en même temps** : `Alt+1` ou `Ctrl+Entrée` ne répondent à aucun des deux à l'aveugle ;
  on répond au clic ou par Tab sur les boutons de la carte (Task 2, test « gives the answer keys up on the canvas »).
- **Une question très longue** (quatre questions avec descriptions) : la carte s'arrête à 70 % de la hauteur de la
  fenêtre et l'encart défile (Task 2).
- **La question répondue** : la conversation est relâchée par la toile (le retour de `show()` appelé) et la carte
  reprend sa taille (Task 2, test « lets the conversation go once answered » ; Task 1 pour le placement).
- **Un agent qui n'attend pas** : la toile ne lit pas sa conversation (Task 4, test « reads only the conversations of
  the agents that wait »).
- **La question d'un sous-agent** : sur la carte du sous-agent, pas sur celle de l'agent, qui dit « question de
  Explore » (Task 3).

---

### Task 1: Le placement qui suit la hauteur des cartes, et l'agent en attente déplié de loin

**Files:**
- Modify: `src/lib/workspace-layout.ts`
- Test: `src/lib/workspace-layout.test.ts`

**Interfaces:**
- Consumes (lot 2): `frameOf(agent: Agent, opts?: FrameOpts): Frame`, `type FrameOpts = { teamOpen?: boolean; seenAt?: number; centreH?: number }`,
  `packRows`, `layoutNear`, `type Link`, `GAP`, `SUB`, `CARD` ; (lot 1) `Disc`, `DISC`, `Placed`.
- Produces:
  - `FrameOpts` gagne `teamH?: Record<string, number>` — la hauteur mesurée d'une carte de l'équipe, par son `id` ; 0
    ou absente : la hauteur par défaut (`SUB.h`, `BG.h`).
  - `frameOf` place les cartes de l'équipe l'une sous l'autre selon ces hauteurs, et la carte du centre à
    `centreH ?? CARD.h` ; le cadre englobe tout.
  - `cardMaxH(windowH: number): number` — `Math.round(0.7 * windowH)` : la hauteur la plus grande d'une carte, à 100 %.
  - `waits(agent: Agent): boolean` — `agent.requests.length > 0`.
  - `layoutFarOpen(discs: Disc[], open: Map<string, Frame>, launched?: [string, string][], maxW = 2400): { discs: Placed<Disc>[]; frames: Placed<Frame>[]; links: Link[]; w: number; h: number }`
    — dans l'ordre de `discs`, un agent présent dans `open` est son cadre, les autres leur disque (`DISC.w`×`DISC.h`) ;
    `packRows` du tout (écart `GAP`) ; traits des cadres en coordonnées du monde. Sans cadre ouvert, les disques sont à
    leur place de `layoutFar`… à `maxW` près : c'est `packRows` qui range.

- [ ] **Step 1: Write the failing tests**

```ts
it('moves the team down when a card grows', () => {
  const a = agent({ plan: planWith([], { agents: [sub('s1'), sub('s2')] }) });
  const f = frameOf(a, { teamH: { s1: 300 } });
  expect(f.team[1].y).toBe(f.team[0].y + 300 + GAP);
});
it('grows the frame with its card', () => {
  const a = agent();
  expect(frameOf(a, { centreH: 500 }).h).toBe(frameOf(a).h + 500 - CARD.h);
});
it('caps a card at 70 % of the window', () => {
  expect(cardMaxH(1000)).toBe(700);
});
it('unfolds a waiting agent among the discs, without overlap', () => {
  const discs = ['a1', 'a2', 'a3'].map((id) => discOf(agent({ id })));
  const open = new Map([['a2', frameOf(agent({ id: 'a2', status: 'waiting', requests: [req('q')] }))]]);
  const r = layoutFarOpen(discs, open);
  expect(r.frames.map((f) => f.agentId)).toEqual(['a2']);
  expect(r.discs.map((d) => d.agentId)).toEqual(['a1', 'a3']);
  const boxes = [...r.discs, ...r.frames];
  for (const p of boxes) for (const q of boxes) if (p !== q) expect(overlaps(p, q)).toBe(false);
});
```

`overlaps(a: Box, b: Box)` est une aide locale au fichier de test.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: FAIL, `cardMaxH is not a function`.

- [ ] **Step 3: Implement in `src/lib/workspace-layout.ts`**

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/lib/workspace-layout.test.ts`
Expected: PASS, les tests des lots 1 et 2 compris.

- [ ] **Step 5: Commit**

```bash
git add src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts
git commit -m "feat(workspace): le placement suit la hauteur des cartes et déplie de loin l'agent qui attend"
```

### Task 2: L'encart des questions

**Files:**
- Create: `src/components/workspace/AskInset.svelte`
- Modify: `src/lib/resize.ts` (ajouter `observeHeight`)
- Modify: `src/lib/state.svelte.ts` (`keysFor`, l. ~769)
- Modify: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/components/workspace/AskInset.test.ts`, `src/lib/state.test.ts`

**Interfaces:**
- Consumes: `conversationOf`, `Conversation.show(): () => void` (`lib/conversations.svelte.ts`) ; `QuestionCard`
  (`{ item, agentId, pending, current }`) et `PermissionCard` (`{ item, agentId, pending, cwd, current }`)
  (`components/conv/`) ; `cardMaxH` (Task 1).
- Produces:
  - `<AskInset agent={Agent} sub={string | undefined} maxH={number} />` — les demandes de `agent.pending`, dans leur
    ordre, lues dans `conversationOf(agent.id).items` : celles dont `item.sub === sub` (sans `sub` : celles de
    l'agent lui-même, `!item.sub`). Rien quand il n'y en a pas. Tant que la conversation n'est pas lue
    (`!conv.loaded`), le texte `workspace.ask.loading`. Un `<div class="ask">` jaune (`--wait-bg`, `--wait-line`), de
    `max-height: {maxH}px` et `overflow: auto`.
  - Un `$effect` : tant que `agent.pending.length > 0`, `return conversationOf(agent.id).show()` ; la conversation est
    ainsi gardée en mémoire et relâchée quand la question part ou que l'encart disparaît.
  - `observeHeight(node: HTMLElement, onHeight: (height: number) => void): { destroy(): void }` dans
    `src/lib/resize.ts`, comme `observeWidth` (hauteur de bordure, `borderBoxSize` ou `getBoundingClientRect`).
  - `app.keysFor(agentId)` rend `false` quand `app.workspaceOn`.

Choix du plan : sur la toile, les touches qui répondent (`Alt+n`, `Ctrl+Entrée`) ne répondent pas : plusieurs cartes
sont à l'écran à la fois et une touche répondrait à toutes ; on répond au clic, ou au clavier par Tab puis Entrée sur
les boutons de la carte.

Textes : `ask: { loading: 'Chargement de la question…', fromSub: 'question de {name}' }`.

- [ ] **Step 1: Write the failing tests**

```ts
// AskInset.test.ts — fakeBackend({ get_conversation: () => [questionItem('q1'), questionItem('q2', { sub: 's1' })] })
it('shows the questions of the agent with the conversation\'s cards', async () => {
  resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['q1', 'q2'], requests: [req('q1'), req('q2')] })] });
  render(AskInset, { agent: app.agents.a1, sub: undefined, maxH: 700 });
  expect(await screen.findByText('Question q1 ?')).toBeTruthy();
  expect(screen.queryByText('Question q2 ?')).toBeNull(); // celle du sous-agent s1
});
it('shows those of a subagent on its card', async () => {
  render(AskInset, { agent: app.agents.a1, sub: 's1', maxH: 700 });
  expect(await screen.findByText('Question q2 ?')).toBeTruthy();
});
it('answers by the conversation\'s card', async () => {
  const backend = fakeBackend({ get_conversation: () => [questionItem('q1')] });
  render(AskInset, { agent: app.agents.a1, sub: undefined, maxH: 700 });
  await fireEvent.click(await screen.findByRole('button', { name: /Oui/ }));
  await fireEvent.click(screen.getByRole('button', { name: /Envoyer/ }));
  expect(backend.called('answer_question')[0].args).toMatchObject({ id: 'a1', requestId: 'q1' });
});
it('scrolls past its height', () => {
  const { container } = render(AskInset, { agent: app.agents.a1, sub: undefined, maxH: 420 });
  expect((container.querySelector('.ask') as HTMLElement).style.maxHeight).toBe('420px');
});
it('lets the conversation go once answered', async () => {
  const gone = vi.fn();
  vi.spyOn(conversationOf('a1'), 'show').mockReturnValue(gone);
  const { rerender } = render(AskInset, { agent: app.agents.a1, sub: undefined, maxH: 700 });
  await rerender({ agent: { ...app.agents.a1, status: 'running', pending: [], requests: [] }, sub: undefined, maxH: 700 });
  expect(gone).toHaveBeenCalled();
});
// state.test.ts
it('gives the answer keys up on the canvas', () => {
  resetApp({ agents: [agent({ id: 'a1' })] });
  expect(app.keysFor('a1')).toBe(true);
  app.openWorkspace('p1');
  expect(app.keysFor('a1')).toBe(false);
});
```

`questionItem(id, over?)` : une `QuestionItem` minimale (`kind: 'question'`, `toolUseId`, une question « Question
<id> ? » à deux options « Oui », « Non », `answers: null`, `ts`) ; l'ajouter à `src/test/workspace.ts` à côté de `req`,
pour le lot 3 et les suivants. Vérifier dans `QuestionCard.test.ts` le nom exact des boutons d'option et d'envoi, et
s'y conformer.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/AskInset.test.ts src/lib/state.test.ts -t "workspace|AskInset|answer keys|questions|scrolls|conversation"`
Expected: FAIL, composant introuvable et `keysFor` vrai sur la toile.

- [ ] **Step 3: Implement `observeHeight`, `keysFor`, the catalogue keys, then `AskInset.svelte`**

- [ ] **Step 4: Run them, and the suites of the cards and the state**

Run: `npx vitest run src/components/workspace src/components/conv/QuestionCard.test.ts src/components/conv/PermissionCard.test.ts src/lib/state.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/workspace/AskInset.svelte src/components/workspace/AskInset.test.ts src/lib/resize.ts src/lib/state.svelte.ts src/lib/state.test.ts src/lib/i18n src/test/workspace.ts
git commit -m "feat(workspace): l'encart des questions, avec les cartes de la conversation"
```

### Task 3: Les cartes qui grandissent

**Files:**
- Modify: `src/components/workspace/AgentCard.svelte`, `src/components/workspace/TeamCard.svelte`
- Test: `src/components/workspace/AgentCard.test.ts`, `src/components/workspace/TeamCard.test.ts`

**Interfaces:**
- Consumes: `AskInset`, `observeHeight` (Task 2) ; `cardMaxH`, `waits` (Task 1) ; `conversationOf` ; `AgentCard`,
  `TeamCard` (lot 2).
- Produces:
  - `<AgentCard agent={Agent} onsize={(h: number) => void} />` — `AskInset` (`sub` absent, `maxH` = `cardMaxH(innerHeight)`
    moins la hauteur des lignes de la carte) entre ses lignes et la barre de saisie ; quand une demande en attente a un
    `sub`, la ligne `workspace.ask.fromSub` avec le nom de ce sous-agent (`plan.agents` : `kind ?? title`) et le
    contour `--wait` ; `onsize` reçoit sa hauteur (`observeHeight`).
  - `<TeamCard agent={Agent} item={TeamItem} ontoggle={() => void} onsize={(id: string, h: number) => void} />` — un
    sous-agent dont une demande en attente a `sub === item.id` monte `<AskInset agent sub={item.id} maxH=… />`, prend
    le contour `--wait` ; `onsize(item.id, h)`.

- [ ] **Step 1: Write the failing tests**

```ts
// AgentCard.test.ts
it('grows with the question, between its lines and its field', async () => {
  fakeBackend({ get_conversation: () => [questionItem('q1')] });
  resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
  const { container } = render(AgentCard, { agent: app.agents.a1, onsize: () => {} });
  await screen.findByText('Question q1 ?');
  const order = [...container.querySelectorAll('.lines, .ask, input')].map((e) => e.className || e.tagName);
  expect(order[1]).toContain('ask');
  expect(order[2]).toBe('INPUT');
});
it('says the question of its subagent, and does not show it', async () => {
  fakeBackend({ get_conversation: () => [questionItem('q1', { sub: 's1' })] });
  resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['q1'], requests: [req('q1')], plan: planWith([], { agents: [{ ...sub('s1'), kind: 'Explore' }] }) })] });
  render(AgentCard, { agent: app.agents.a1, onsize: () => {} });
  expect(await screen.findByText('question de Explore')).toBeTruthy();
  expect(screen.queryByText('Question q1 ?')).toBeNull();
});
it('answers the question with the text typed in its field', async () => {
  const backend = fakeBackend({ get_conversation: () => [questionItem('q1')] });
  // … agent en attente sur q1 ; taper « Oui » dans « Écrire à un », Entrée
  expect(backend.called('answer_question')[0].args).toMatchObject({ id: 'a1', requestId: 'q1' });
});
// TeamCard.test.ts
it('shows the question of its subagent', async () => {
  fakeBackend({ get_conversation: () => [questionItem('q1', { sub: 's1' })] });
  const a = agent({ id: 'a1', status: 'waiting', pending: ['q1'], requests: [req('q1')] });
  resetApp({ agents: [a] });
  render(TeamCard, { agent: app.agents.a1, item: { kind: 'sub', id: 's1', sub: sub('s1') }, ontoggle: () => {}, onsize: () => {} });
  expect(await screen.findByText('Question q1 ?')).toBeTruthy();
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/AgentCard.test.ts src/components/workspace/TeamCard.test.ts`
Expected: FAIL, pas d'encart dans la carte.

- [ ] **Step 3: Mount `AskInset` in both cards, measure them**

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/components/workspace`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/workspace/AgentCard.svelte src/components/workspace/TeamCard.svelte src/components/workspace/AgentCard.test.ts src/components/workspace/TeamCard.test.ts
git commit -m "feat(workspace): la carte d'un agent ou d'un sous-agent grandit avec sa question"
```

### Task 4: L'agent en attente, toujours déplié

**Files:**
- Modify: `src/components/workspace/WorkspaceView.svelte`, `src/components/workspace/Compass.svelte`
- Test: `src/components/workspace/WorkspaceView.test.ts`

**Interfaces:**
- Consumes: `layoutFarOpen`, `waits`, `FrameOpts.teamH` (Task 1) ; `AgentCard.onsize`, `TeamCard.onsize` (Task 3) ;
  `Compass` (lot 2).
- Produces:
  - `<Compass frame agent ontoggle onsize={(id: string, h: number) => void} />` — relaie `onsize` de ses cartes (la
    carte de l'agent sous l'id `centre`).
  - Dans `WorkspaceView` : état local `heights: Record<string, Record<string, number>>` (par agent, par carte) ;
    `frameOf(a, { teamOpen, seenAt, centreH: heights[a.id]?.centre || undefined, teamH: heights[a.id] })`.
  - `far` : `layoutFarOpen(discs, new Map(order.filter(waits).map((a) => [a.id, frameOf(a, …)])), launched)` ; les
    disques et les boussoles ouvertes dans le même monde. `near` : inchangé.
  - Le compte « n questions » de la barre et le satellite question du disque comptent déjà les demandes des
    sous-agents (`agent.requests` les contient) : rien à changer, un test le fige.

- [ ] **Step 1: Write the failing tests**

```ts
it('unfolds a waiting agent from afar, the others stay discs', async () => {
  fakeBackend({ get_conversation: () => [questionItem('q1')] });
  resetApp({ agents: [agent({ id: 'a1', name: 'un' }), agent({ id: 'a2', name: 'deux', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
  show(); // de loin
  expect(await screen.findByText('Question q1 ?')).toBeTruthy();
  expect(screen.getByRole('button', { name: /^un,/ })).toBeTruthy();
  expect(screen.getByRole('textbox', { name: 'Écrire à deux' })).toBeTruthy();
});
it('reads only the conversations of the agents that wait', async () => {
  const backend = fakeBackend({ get_conversation: () => [questionItem('q1')] });
  resetApp({ agents: [agent({ id: 'a1', status: 'running' }), agent({ id: 'a2', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] });
  show();
  await zoomTo(1);
  await screen.findByText('Question q1 ?');
  expect(backend.called('get_conversation').map((c) => c.args.id)).toEqual(['a2']);
});
it('counts the questions of the subagents too', () => {
  resetApp({ agents: [agent({ id: 'a1', status: 'waiting', pending: ['q1'], requests: [req('q1')] })] }); // q1 a un `sub`
  show();
  expect(screen.getByText('1 question')).toBeTruthy();
});
it('folds the agent back once answered', async () => {
  // a2 en attente, puis app.agents.a2 = { ...a2, status: 'running', pending: [], requests: [] }
  await new Promise((r) => setTimeout(r, 0));
  expect(screen.queryByRole('textbox', { name: 'Écrire à deux' })).toBeNull();
});
```

`zoomTo` est l'aide du lot 2 dans ce fichier.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/components/workspace/WorkspaceView.test.ts`
Expected: FAIL, la question ne se voit pas de loin.

- [ ] **Step 3: Wire `Compass.svelte` and `WorkspaceView.svelte`**

- [ ] **Step 4: Run the whole suite and the type check**

Run: `npx vitest run && npm run check`
Expected: PASS ; seuls les échecs déjà présents sur la base.

- [ ] **Step 5: Try it**

Page d'aperçu jetable (lot 1) : trois agents, dont un qui attend une question à trois volets et un dont le sous-agent
demande une permission. De loin : la boussole de celui qui attend se voit, ses voisins se décalent. Répondre au clic ;
la carte se replie. Une question de 15 options : la carte s'arrête à 70 % de la fenêtre et défile. Capture d'écran.

- [ ] **Step 6: Commit**

```bash
git add src/components/workspace
git commit -m "feat(workspace): l'agent qui attend est déplié à toutes les altitudes"
```
