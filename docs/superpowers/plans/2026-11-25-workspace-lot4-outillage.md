# Workspace, lot 4 : l'outillage — plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** à l'ouest de chaque boussole, une carte par skill et par plugin que l'agent (ou un de ses sous-agents) a
utilisé, avec son nombre d'appels ; un flux part de la carte vers l'agent le temps d'un appel ; de loin, le satellite
« outils ».

**Architecture:** le cœur tient, pour chaque agent, un **résumé de toile** (`Canvas`, crate `escouade-plan`) mis à jour
à chaque appel d'outil et à chaque résultat, par les mêmes points d'entrée que le plan (`plan_tool_use`, `tool_done`) ;
il part avec l'agent (`AgentMeta::canvas`, donc `AgentView`). La fenêtre lit `agent.canvas` : une fonction pure en tire
les cartes, le placement les pose à l'ouest.

**Tech Stack:** Rust (serde, serde_json), Svelte 5, TypeScript, vitest, catalogues `t()` fr/en.

**Base:** la branche `feat/workspace` après le lot 3 (`frameOf` avec `FrameOpts`, `layoutNear`, `layoutFarOpen`,
`Compass`, `Links`, `AgentCard`, `TeamCard`, `AskInset`).

**Spec:** `docs/superpowers/specs/2026-11-25-vue-workspace-design.md` (« À l'ouest : l'outillage », « D'où viennent les
données » (« Choix validé : le cœur tient… un résumé de toile »), « De loin » (satellite outils), lot 4 de « Les lots »).

## Global Constraints

- « Le cœur tient, pour chaque agent, un résumé de toile qu'il envoie avec l'agent et met à jour à chaque appel
  d'outil : les skills et plugins utilisés (nom, nombre d'appels, appel en cours ou non), pour lui et pour chacun de
  ses sous-agents. » La toile ne charge aucune conversation pour l'outillage.
- « Un skill est un appel de l'outil `Skill` ; son nom est `input.skill`. » « Un plugin est un serveur d'outils : le
  `<serveur>` d'un outil `mcp__<serveur>__<outil>`, avec son nombre d'appels. »
- Cartes « plus carrées (coins de 3 px, liseré de couleur en haut) » ; skill : icône de parchemin, violet ; plugin :
  icône de boîte, bleu (`--info`).
- « Aucun trait permanent. Pendant qu'un appel est en cours (`status: 'running'`), la carte de l'outil a un anneau vert
  et un flux part d'elle vers l'agent ; à la fin de l'appel, le trait disparaît. »
- « Une seule couleur à ajouter, le violet des skills » : `--skill` dans `src/app.css`, thème sombre et clair.
- Satellite « outils » : « nombre de skills et de plugins utilisés », violet, à sa place dans l'ordre fixe (équipe,
  outils, livrables, CI, question).
- Ordre clavier nord, ouest, centre, est, sud : les cartes d'outil viennent avant la carte de l'agent dans le DOM.
- `prefers-reduced-motion` fige le flux.
- Pas de dépendance ajoutée, ni côté Rust (pas de `regex`), ni côté fenêtre.
- Builds Rust sur le Mac de l'utilisateur : dans Docker, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, une seule
  commande cargo à la fois.
- Commits en Conventional Commits, en français.

## Review Focus

- **Une trame reçue deux fois** (transcript relu, greffon relancé) : l'appel n'est compté qu'une fois (Task 1, test
  `a_call_seen_twice_counts_once`).
- **Le processus meurt pendant un appel** : plus d'appel « en cours », plus d'anneau ni de flux éternels (Task 1
  `close_running`, Task 2 test sur `detach`).
- **L'app redémarre** : le résumé revient, sans appel « en cours » (Task 1, test `nothing_runs_after_a_reload`).
- **Un agent qui appelle des centaines d'outils MCP de serveurs différents** : au plus 40 cartes par fil et 30 fils de
  sous-agents gardés (Task 1, test `keeps_at_most_40_tools_a_thread`).
- **Un outil qui tourne dans un sous-agent** : le flux va de la carte de l'outil à la carte du sous-agent quand elle est
  sur la toile, sinon à l'agent (Task 4).

---

### Task 1: Le résumé de toile, dans le cœur

**Files:**
- Create: `crates/escouade-plan/src/canvas.rs`
- Create: `crates/escouade-plan/src/canvas_tests.rs`
- Modify: `crates/escouade-plan/src/lib.rs` (déclarer `pub mod canvas;` et `#[cfg(test)] mod canvas_tests;`)
- Modify: `crates/escouade-plan/src/plan.rs` (`Seen::first_time`, l. ~331 : `fn` → `pub(crate) fn`)

**Interfaces:**
- Consumes: `escouade_plan::plan::{Change, Seen}`.
- Produces (`escouade_plan::canvas`) :
  - `pub const MAX_KIT: usize = 40;` `pub const MAX_THREADS: usize = 30;`
  - `pub enum KitKind { #[default] Skill, Plugin }` — `#[serde(rename_all = "lowercase")]`.
  - `pub struct KitUse { pub kind: KitKind, pub name: String, pub calls: u32, #[serde(skip_deserializing)] pub running: u32, pub last_at: i64 }`
    — `#[serde(rename_all = "camelCase", default)]`.
  - `pub struct ThreadKit { pub call: String, pub tools: Vec<KitUse> }`
  - `pub struct Canvas { pub tools: Vec<KitUse>, pub subs: Vec<ThreadKit>, #[serde(skip)] seen: Seen, #[serde(skip)] open: HashMap<String, (Option<String>, KitKind, String)> }`
    — `Debug, Clone, Default, Serialize, Deserialize`, `rename_all = "camelCase"`, `default`.
  - `pub fn kit_of(name: &str, input: &Value) -> Option<(KitKind, String)>` — `Skill` → `input["skill"]` (texte non
    vide, rogné) ; `mcp__<serveur>__<outil>` → `(Plugin, <serveur>)` ; le reste → `None`.
  - `impl Canvas` :
    - `pub fn on_tool_use(&mut self, parent: Option<&str>, id: &str, name: &str, input: &Value, now: i64) -> Change`
      — `kit_of` ; un appel déjà vu (`seen.first_time`) → `None` ; le fil (`parent` absent : `tools` ; sinon l'entrée
      `subs` de ce `call`, créée) ; l'entrée (kind, name) créée → `Change::Saved`, sinon `calls += 1`, `running += 1`,
      `last_at = now` → `Change::View` ; l'appel noté dans `open`.
    - `pub fn on_tool_result(&mut self, id: &str, is_error: bool, text: &str, now: i64) -> Change` (`is_error` et `text` servent au lot 5) — un appel de `open` : `running -= 1` (jamais
      sous 0), `last_at = now`, `Change::View` ; sinon `None`.
    - `pub fn close_running(&mut self, now: i64) -> Change` — tous les `running` à 0, `open` vidé ; `View` s'il y en avait (`now` sert au lot 5).
    - `pub fn is_empty(&self) -> bool`
  - Plafonds : au-delà de `MAX_KIT` entrées dans un fil, l'entrée au `last_at` le plus ancien part ; au-delà de
    `MAX_THREADS` fils de sous-agents, le plus ancien part (ordre d'arrivée).

Choix du plan : le résumé n'est jamais remis à zéro (ni par `/clear`, ni par un nouveau plan) : il dit ce que l'agent a
utilisé dans sa vie ; les plafonds bornent sa taille.

Choix du plan : un nouvel outil est sauvé (`Change::Saved`) ; un appel de plus ou sa fin ne fait qu'envoyer la vue
(`Change::View`), son compte part avec la sauvegarde suivante. Sauver l'agent à chaque appel d'outil coûterait une
écriture par appel.

- [ ] **Step 1: Write the failing tests** (`canvas_tests.rs`, dans le style de `plan_tests.rs`)

```rust
#[test]
fn a_skill_is_named_by_its_input_and_a_plugin_by_its_server() {
    assert_eq!(kit_of("Skill", &json!({"skill": " superpowers:writing-plans "})), Some((KitKind::Skill, "superpowers:writing-plans".into())));
    assert_eq!(kit_of("mcp__github__get_pr", &json!({})), Some((KitKind::Plugin, "github".into())));
    assert_eq!(kit_of("Read", &json!({})), None);
    assert_eq!(kit_of("Skill", &json!({})), None);
}

#[test]
fn counts_the_calls_and_says_which_one_runs() {
    let mut c = Canvas::default();
    assert_eq!(c.on_tool_use(None, "k1", "mcp__github__a", &json!({}), 1), Change::Saved);
    assert_eq!(c.on_tool_use(None, "k2", "mcp__github__b", &json!({}), 2), Change::View);
    assert_eq!((c.tools[0].calls, c.tools[0].running), (2, 2));
    assert_eq!(c.on_tool_result("k1", false, "", 3), Change::View);
    assert_eq!(c.tools[0].running, 1);
    assert_eq!(c.on_tool_result("nope", false, "", 3), Change::None);
}

#[test]
fn a_call_seen_twice_counts_once() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1);
    assert_eq!(c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1), Change::None);
    assert_eq!(c.tools[0].calls, 1);
}

#[test]
fn keeps_the_tools_of_each_subagent_apart() {
    let mut c = Canvas::default();
    c.on_tool_use(Some("a1"), "k1", "Skill", &json!({"skill": "x"}), 1);
    assert!(c.tools.is_empty());
    assert_eq!(c.subs[0].call, "a1");
    assert_eq!(c.subs[0].tools[0].name, "x");
}

#[test]
fn nothing_runs_after_a_reload_or_the_end_of_the_process() {
    let mut c = Canvas::default();
    c.on_tool_use(None, "k1", "Skill", &json!({"skill": "x"}), 1);
    let back: Canvas = serde_json::from_value(serde_json::to_value(&c).unwrap()).unwrap();
    assert_eq!((back.tools[0].calls, back.tools[0].running), (1, 0));
    assert_eq!(c.close_running(9), Change::View);
    assert_eq!(c.tools[0].running, 0);
}

#[test]
fn keeps_at_most_40_tools_a_thread() {
    let mut c = Canvas::default();
    for i in 0..45 {
        c.on_tool_use(None, &format!("k{i}"), &format!("mcp__s{i}__t"), &json!({}), i);
    }
    assert_eq!(c.tools.len(), MAX_KIT);
    assert!(c.tools.iter().all(|t| t.name != "s0"));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd src-tauri && cargo test -p escouade-plan canvas`
Expected: FAIL à la compilation, module `canvas` introuvable.

- [ ] **Step 3: Implement `crates/escouade-plan/src/canvas.rs`**

- [ ] **Step 4: Run them to see them pass, and the crate's tests**

Run: `cd src-tauri && cargo test -p escouade-plan`
Expected: PASS ; `cargo clippy -p escouade-plan -- -D warnings` sans avertissement.

- [ ] **Step 5: Commit**

```bash
git add crates/escouade-plan/src
git commit -m "feat(cœur): le résumé de toile d'un agent, ses skills et ses plugins"
```

### Task 2: Le résumé part avec l'agent

**Files:**
- Modify: `crates/escouade-model/src/model.rs` (`AgentMeta`, à côté de `plan`, l. ~2807)
- Modify: `src-tauri/src/lib.rs` (l. 215 : `use escouade_plan::{canvas, plan, planfiles};`)
- Modify: `src-tauri/src/agent.rs` (`canvas_apply` à côté de `plan_apply` l. ~660 ; là où le plan ferme ses lignes :
  `interrupt_plan` l. ~697, `AgentRt::new` l. ~483, la fin du processus l. ~1428)
- Modify: `src-tauri/src/agent/claude.rs` (`plan_tool_use`, l. ~147)
- Modify: `src-tauri/src/agent/items.rs` (`tool_done`, l. ~741)
- Modify: `src-tauri/src/agent/minimal.rs` (la branche `"done"`, l. ~86)
- Test: le module `tests` de `src-tauri/src/agent.rs` (section « the plan », aides `call` et `returns` l. ~4093-4110)

**Interfaces:**
- Consumes: `Canvas`, `Canvas::{on_tool_use, on_tool_result, close_running, is_empty}` (Task 1).
- Produces:
  - `AgentMeta::canvas: Option<Canvas>` — `#[serde(skip_serializing_if = "Option::is_none")]` ; `None` tant qu'il est
    vide (un agent enregistré avant ce lot se lit sans).
  - `AgentRt::canvas_apply(&mut self, fx: &mut Effects, f: impl FnOnce(&mut Canvas) -> Change)` — comme `plan_apply` :
    `View` → `fx.agent_changed`, `Saved` → `agent_changed` et `save`.
  - Appels : `plan_tool_use` → `canvas_apply(|c| c.on_tool_use(parent, id, name, input, now))` (tous les harness y
    passent) ; `tool_done` et la branche `"done"` de `minimal.rs` → `on_tool_result(id, error, &output, now_ms())` ; là où
    le plan ferme ses lignes → `close_running(now)` : dans `interrupt_plan` (donc `attach`, `detach`,
    `transcript.rs`), dans `AgentRt::new` (l. ~483, un agent rechargé) et à la fin du processus (l. ~1428).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_canvas_follows_the_skills_and_plugins_of_the_agent_and_its_subagents() {
    let mut a = rt();
    assert!(a.view().meta.canvas.is_none());
    let fx = call(&mut a, None, "k1", "Skill", json!({"skill": "superpowers:writing-plans"}));
    assert!(fx.agent_changed && fx.save);
    let c = a.view().meta.canvas.expect("a canvas");
    assert_eq!((c.tools[0].name.as_str(), c.tools[0].running), ("superpowers:writing-plans", 1));
    returns(&mut a, "k1", "ok", Value::Null);
    assert_eq!(a.view().meta.canvas.unwrap().tools[0].running, 0);
    call(&mut a, None, "a1", "Agent", json!({"description": "Lire", "prompt": "x"}));
    call(&mut a, Some("a1"), "m1", "mcp__github__get_pr", json!({}));
    let c = a.view().meta.canvas.unwrap();
    assert_eq!((c.subs[0].call.as_str(), c.subs[0].tools[0].name.as_str()), ("a1", "github"));
    assert_eq!(serde_json::to_value(a.view()).unwrap()["canvas"]["subs"][0]["tools"][0]["kind"], "plugin");
}
```

Et un second test, `a_new_process_ends_the_calls_of_the_canvas` : un appel `Skill` sans résultat, puis `a.detach()` ;
`running` vaut 0 et `calls` 1.

- [ ] **Step 2: Run them to see them fail**

Run: `cd src-tauri && cargo test --lib -- the_canvas_follows a_new_process_ends`
Expected: FAIL à la compilation, `no field canvas on AgentMeta`.

- [ ] **Step 3: Implement**

- [ ] **Step 4: Run them, and the plan's tests of the agent**

Run: `cd src-tauri && cargo test --lib agent::tests && cargo test -p escouade-model`
Expected: PASS ; comparer à la base pour les échecs déjà présents dans le conteneur (environ 90, voir la note du
dépôt sur les tests Rust dans Docker).

- [ ] **Step 5: Commit**

```bash
git add crates/escouade-model/src/model.rs src-tauri/src/lib.rs src-tauri/src/agent.rs src-tauri/src/agent
git commit -m "feat(cœur): le résumé de toile part avec l'agent, à chaque appel d'outil"
```

### Task 3: Les outils dans la fenêtre, et le satellite

**Files:**
- Modify: `src/lib/types.ts` (`Agent`, après `plan` l. ~2071 ; les types du résumé à côté de `PlanState`)
- Modify: `src/lib/workspace-layout.ts` (`discOf`, et `toolCards`)
- Modify: `src/lib/icons.ts` (deux icônes), `src/app.css` (`--skill`, l. ~65 sombre et l. ~178 clair)
- Modify: `src/components/workspace/Disc.svelte` (`satWord`), `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/lib/workspace-layout.test.ts`, `src/components/workspace/Disc.test.ts`

**Interfaces:**
- Consumes: `AgentMeta::canvas` sérialisé (Task 2) ; `discOf`, `Satellite` (lot 1).
- Produces:
  - `export interface KitUse { kind: 'skill' | 'plugin'; name: string; calls: number; running: number; lastAt: number }`
  - `export interface CanvasSummary { tools: KitUse[]; subs: { call: string; tools: KitUse[] }[] }`
  - `Agent.canvas?: CanvasSummary`
  - `type ToolCard = { kind: 'skill' | 'plugin'; name: string; calls: number; running: boolean; runningIn: string | null }`
    — `runningIn` : le `call` du sous-agent où l'appel tourne, `null` pour l'agent lui-même.
  - `toolCards(agent: Agent): ToolCard[]` — les entrées de l'agent et de ses sous-agents réunies par (kind, name),
    `calls` additionnés ; les skills puis les plugins, chacun par nom (ordre stable : une carte ne saute pas quand un
    compte change).
  - `discOf` ajoute `{ kind: 'tools', count: toolCards(agent).length }`, absent à 0, entre `team` et `question`.
  - `ICONS.scroll` (parchemin) et `ICONS.box` (boîte), au trait de 1,5 px sur la grille de 16 comme les autres.
  - `--skill` : `oklch(0.74 0.14 300)` en sombre, `oklch(0.5 0.16 300)` en clair (le violet des maquettes, à l'œil).

Textes : `sat.tools: { one: 'outil', other: 'outils' }`.

- [ ] **Step 1: Write the failing tests**

```ts
const kit = (kind: 'skill' | 'plugin', name: string, calls = 1, running = 0): KitUse => ({ kind, name, calls, running, lastAt: 1 });
it('gathers the tools of the agent and its subagents', () => {
  const a = agent({ canvas: { tools: [kit('plugin', 'github', 2), kit('skill', 'tdd')], subs: [{ call: 's1', tools: [kit('plugin', 'github', 3, 1)] }] } });
  expect(toolCards(a)).toEqual([
    { kind: 'skill', name: 'tdd', calls: 1, running: false, runningIn: null },
    { kind: 'plugin', name: 'github', calls: 5, running: true, runningIn: 's1' },
  ]);
  expect(toolCards(agent())).toEqual([]);
});
it('puts the tools satellite in its place', () => {
  const a = agent({ status: 'waiting', requests: [req('q')], plan: planWithRunning(1, 0), canvas: { tools: [kit('skill', 'x')], subs: [] } });
  expect(discOf(a).satellites.map((s) => s.kind)).toEqual(['team', 'tools', 'question']);
});
// Disc.test.ts
it('says the tools under their number', () => {
  renderDisc(agent({ canvas: { tools: [kit('skill', 'x'), kit('plugin', 'y')], subs: [] } }));
  expect(screen.getByText('outils')).toBeTruthy();
});
```

`kit` va dans `src/test/workspace.ts` (exporté) pour la Task 4.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-layout.test.ts src/components/workspace/Disc.test.ts`
Expected: FAIL, `toolCards is not a function`.

- [ ] **Step 3: Implement the types, `toolCards`, the satellite, the icons and the colour**

- [ ] **Step 4: Run them and the type check**

Run: `npx vitest run src/lib/workspace-layout.test.ts src/components/workspace src/lib/i18n && npm run check`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/lib/icons.ts src/app.css src/components/workspace/Disc.svelte src/components/workspace/Disc.test.ts src/lib/i18n src/test/workspace.ts
git commit -m "feat(workspace): les outils d'un agent lus dans son résumé de toile, et leur satellite"
```

### Task 4: Les cartes d'outil à l'ouest, et le flux pendant l'appel

**Files:**
- Create: `src/components/workspace/ToolCard.svelte`
- Modify: `src/lib/workspace-layout.ts` (`frameOf` : la colonne ouest, ses traits)
- Modify: `src/components/workspace/Compass.svelte`, `src/components/workspace/WorkspaceView.svelte` (`pick` côté
  `west`)
- Modify: `src/lib/i18n/fr/workspace.ts`, `src/lib/i18n/en/workspace.ts`
- Test: `src/lib/workspace-layout.test.ts`, `src/components/workspace/ToolCard.test.ts`,
  `src/components/workspace/WorkspaceView.test.ts`

**Interfaces:**
- Consumes: `toolCards`, `ToolCard` (type), `kit` (Task 3) ; `frameOf`, `Frame`, `Link`, `GAP`, `Placed` (lots 2-3).
- Produces:
  - `const TOOL = { w: 200, h: 72 }` ; `Frame` gagne `west: Placed<ToolCard>[]` — une colonne à gauche de la carte,
    dans l'ordre de `toolCards`, centrée sur sa hauteur.
  - Un trait par outil `running` seulement, `state: 'run'`, `id: 'tool:<kind>:<name>'`, qui part de la carte d'outil :
    vers la carte du sous-agent `runningIn` quand elle est dans `team`, sinon vers la carte de l'agent. Aucun trait
    pour un outil qui ne tourne pas.
  - `<ToolCard tool={ToolCard} />` — `<article data-side="west">`, `aria-label` `workspace.tool.label` ; icône
    `scroll` (skill, liseré `--skill`) ou `box` (plugin, liseré `--info`), coins de 3 px, le nom, `workspace.tool.calls` ;
    `running` : anneau `--run` et le mot `workspace.tool.running`.
  - `Compass` rend les `ToolCard` avant `AgentCard` dans le DOM ; `pick(agentId, 'west')` centre la colonne ouest.

Textes :

```ts
tool: {
  label: '{kind} {name}, {calls}',
  skill: 'Skill',
  plugin: 'Plugin',
  calls: { one: '{count} appel', other: '{count} appels' },
  running: 'en cours',
},
```

- [ ] **Step 1: Write the failing tests**

```ts
// workspace-layout.test.ts
it('puts the tools on the west, with a link only while a call runs', () => {
  const a = agent({ canvas: { tools: [kit('skill', 'tdd'), kit('plugin', 'github', 1, 1)], subs: [] } });
  const f = frameOf(a);
  expect(f.west.map((t) => t.name)).toEqual(['tdd', 'github']);
  expect(f.west[0].x + TOOL.w).toBeLessThan(f.centre.x);
  expect(f.links.filter((l) => l.id.startsWith('tool:'))).toEqual([expect.objectContaining({ id: 'tool:plugin:github', state: 'run' })]);
});
it('sends the flow of a subagent\'s call to the subagent', () => {
  const a = agent({ plan: planWith([], { agents: [sub('s1')] }), canvas: { tools: [], subs: [{ call: 's1', tools: [kit('plugin', 'github', 1, 1)] }] } });
  const f = frameOf(a);
  const s1 = f.team.find((t) => t.id === 's1')!;
  const link = f.links.find((l) => l.id === 'tool:plugin:github')!;
  expect(link.d.endsWith(`${s1.x} ${s1.y + s1.h / 2}`)).toBe(true);
});
// ToolCard.test.ts
it('says a skill, its calls and that it runs', () => {
  render(ToolCard, { tool: { kind: 'skill', name: 'tdd', calls: 3, running: true, runningIn: null } });
  expect(screen.getByRole('article', { name: 'Skill tdd, 3 appels' })).toBeTruthy();
  expect(screen.getByText('en cours')).toBeTruthy();
});
// WorkspaceView.test.ts
it('reaches the tools before the agent\'s card with the keyboard', async () => {
  resetApp({ agents: [agent({ id: 'a1', name: 'un', canvas: { tools: [kit('skill', 'tdd')], subs: [] } })] });
  const { container } = show();
  await zoomTo(1);
  const sides = [...container.querySelectorAll('[data-side]')].map((e) => e.getAttribute('data-side'));
  expect(sides.indexOf('west')).toBeLessThan(sides.indexOf('centre'));
});
```

La fin du chemin d'un trait (`d`) est le point d'arrivée, écrit `x y` : s'aligner sur la forme que `frameOf` a choisie au
lot 2, ou comparer avec une aide `endOf(d)` locale.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/lib/workspace-layout.test.ts src/components/workspace`
Expected: FAIL, `f.west` indéfini.

- [ ] **Step 3: Implement the west column in `frameOf`, then `ToolCard.svelte`, then `Compass` and `pick`**

- [ ] **Step 4: Run the whole suite and the type check**

Run: `npx vitest run && npm run check`
Expected: PASS ; seuls les échecs déjà présents sur la base.

- [ ] **Step 5: Try it**

App réelle si possible (le cœur change), sinon la page d'aperçu jetable avec un agent dont `canvas` est rempli à la main
et un appel `running` qui passe à 0 après 3 s : l'anneau et le flux apparaissent puis disparaissent. Thèmes sombre et
clair (le violet). Capture d'écran.

- [ ] **Step 6: Commit**

```bash
git add src/lib/workspace-layout.ts src/lib/workspace-layout.test.ts src/components/workspace src/lib/i18n
git commit -m "feat(workspace): les skills et les plugins à l'ouest, et le flux le temps d'un appel"
```
