// What the « Workspace » shows of each agent, and where (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md,
// « Les trois altitudes »). Pure: the view draws what it gives.

import { bgRows, isRunning, ownerOf, type BgRow } from './bgtasks';
import { drivenAgents } from './later';
import { planView } from './plan';
import { sidebarSubs, subagentInfo } from './subagents';
import type { Agent, ConvItem, KitUse, SubAgent, Ticket } from './types';
import type { Box, Size } from './workspace-camera';
import type { Landing, RibbonState } from './workspace-landing';
import { looksLikeTests, testsOf, type TestRun } from './workspace-tests';

export type DiscState = 'run' | 'wait' | 'error' | 'done' | 'idle';
/** The satellites, in their order from top to bottom; each lot of the spec brings its own. */
export type SatKind = 'team' | 'tools' | 'deliverables' | 'ci' | 'question';
export type Satellite = { kind: SatKind; count: number; state?: RibbonState };

/** An agent seen from afar. `total` is 0 without a plan. */
export type Disc = { agentId: string; name: string; state: DiscState; done: number; total: number; satellites: Satellite[] };

export type Placed<T> = T & { x: number; y: number; w: number; h: number };

/** A disc's radius, its orbit's, the box it is drawn in and the gap between two boxes (px, at a zoom of 1). */
export const DISC = { r: 92, orbit: 122, w: 300, h: 300, gap: 70 } as const;

const STATE: Record<Agent['status'], DiscState> = { waiting: 'wait', error: 'error', running: 'run', done: 'done', idle: 'idle' };

export function discOf(agent: Agent, landing: Landing[] = []): Disc {
  const mini = planView(agent)?.mini;
  const team = (agent.plan?.agents.filter(isRunning).length ?? 0) + (agent.plan?.bgTasks.filter(isRunning).length ?? 0);
  const questions = agent.requests?.length ?? 0;
  const satellites: Satellite[] = [];
  if (team) satellites.push({ kind: 'team', count: team });
  const tools = toolCards(agent).length;
  if (tools) satellites.push({ kind: 'tools', count: tools });
  const deliverables = landing.filter((l) => l.kind === 'artifact').length;
  if (deliverables) satellites.push({ kind: 'deliverables', count: deliverables });
  // The remote CI and the deployments, the worst state first: the local tests are on their card.
  const ci = landing.flatMap((l) => (l.kind === 'ci' || l.kind === 'deploy' ? [l.state] : []));
  if (ci.length) {
    const state = (['failure', 'running', 'success'] as const).find((s) => ci.includes(s));
    satellites.push({ kind: 'ci', count: ci.length, state });
  }
  if (questions) satellites.push({ kind: 'question', count: questions });
  return {
    agentId: agent.id,
    name: agent.name,
    state: STATE[agent.status] ?? 'idle',
    done: mini?.done ?? 0,
    total: mini?.total ?? 0,
    satellites,
  };
}

/**
 * A skill or a plugin of an agent, its subagents' calls added; `runningIn`: the subagent's call a call of it runs in;
 * `firstAt`, `lastAt`: its first and its latest call, of all its threads.
 */
export type ToolCard = {
  kind: 'skill' | 'plugin';
  name: string;
  calls: number;
  running: boolean;
  runningIn: string | null;
  firstAt: number;
  lastAt: number;
};

/**
 * The name a tool is shown by. Claude Code names the server of a plugin `plugin_<plugin>_<server>`: its server, or
 * « <plugin> · <server> » when they differ (a plugin's name has no `_`). Any other name is shown whole.
 */
export function toolName(t: Pick<ToolCard, 'kind' | 'name'>): string {
  if (t.kind !== 'plugin' || !t.name.startsWith('plugin_')) return t.name;
  const rest = t.name.slice('plugin_'.length);
  const i = rest.indexOf('_');
  if (i <= 0 || i === rest.length - 1) return rest;
  const [plugin, server] = [rest.slice(0, i), rest.slice(i + 1)];
  return plugin === server ? server : `${plugin} · ${server}`;
}

/**
 * The tools of `agent` and of its subagents, one card a tool, in the order of their first use (a new one comes last:
 * it never moves the others; the agent's thread before its subagents' at the same time).
 */
export function toolCards(agent: Agent): ToolCard[] {
  const canvas = agent.canvas;
  if (!canvas) return [];
  const cards = new Map<string, ToolCard>();
  const threads: [string | null, KitUse[]][] = [[null, canvas.tools], ...canvas.subs.map((s): [string, KitUse[]] => [s.call, s.tools])];
  for (const [call, tools] of threads)
    for (const k of tools) {
      const key = `${k.kind}:${k.name}`;
      const first = k.firstAt ?? 0;
      const card = cards.get(key) ?? { kind: k.kind, name: k.name, calls: 0, running: false, runningIn: null, firstAt: first, lastAt: 0 };
      card.calls += k.calls;
      card.firstAt = Math.min(card.firstAt, first);
      card.lastAt = Math.max(card.lastAt, k.lastAt);
      if (k.running > 0 && !card.running) [card.running, card.runningIn] = [true, call];
      cards.set(key, card);
    }
  // A stable sort: same time, the order of the threads.
  return [...cards.values()].sort((a, b) => a.firstAt - b.firstAt);
}

/** The discs in rows of `perRow`, in the order given. */
export function layoutFar(discs: Disc[], perRow = 4): { items: Placed<Disc>[]; w: number; h: number } {
  if (!discs.length) return { items: [], w: 0, h: 0 };
  const items = discs.map((d, i) => ({
    ...d,
    x: (i % perRow) * (DISC.w + DISC.gap),
    y: Math.floor(i / perRow) * (DISC.h + DISC.gap),
    w: DISC.w,
    h: DISC.h,
  }));
  const cols = Math.min(perRow, discs.length);
  const rows = Math.ceil(discs.length / perRow);
  return { items, w: cols * DISC.w + (cols - 1) * DISC.gap, h: rows * DISC.h + (rows - 1) * DISC.gap };
}

// Near: the compass. Each agent is a frame, the agent's card in it and what belongs to it around (spec, « La boussole,
// côté par côté »). The sizes are left to who builds (taken from the mock-ups, to tune with use).

export type Side = 'north' | 'west' | 'centre' | 'east' | 'south';
/** The side of the compass a satellite stands for. */
export const SIDE_OF: Record<SatKind, Side> = { team: 'east', tools: 'west', deliverables: 'south', ci: 'south', question: 'centre' };

export const CARD = { w: 360, h: 128 } as const;
export const SUB = { w: 240, h: 100 } as const;
export const BG = { w: 240, h: 60 } as const;
/**
 * The conversation's plan banner (`PlanBanner`), north of the card, as measured in the window: its head and gauge
 * (`head`, 10 px less folded: no gap before a list), a row of 30 px a task in a list padded by 6 px, 232 px at most and
 * scrolling past it (its CSS).
 */
export const PLAN = { w: 560, head: 72, folded: 62, row: 30, pad: 6, list: 232 } as const;
export const GAP = 48;
/** A test card, stuck under the card that runs it: one line of command, its height fixed. */
export const TEST = { h: 76, gap: 8 } as const;
/** The landing zone, empty: its height until it is measured. */
export const SOUTH = { h: 132 } as const;
/**
 * A tool's card (« de la doc, pas un agent »): above the agent's card, two to a row around an aisle in the middle of the
 * card, where the plan's link runs; `row`: between two rows; `drop`: from the last row to the card, for a flow to show.
 */
export const TOOL = { w: 168, h: 60, aisle: 24, row: 12, drop: 32 } as const;
/** The tool cards shown, the most recently called; the others in a pile (« + n outils ») that unfolds them. */
export const TOOLS_SHOWN = 8;
/** The pile of the tools: `hidden` of them folded in it, or `open` (it folds them back). */
export type ToolPile = { hidden: number; open: boolean };
/** The pile of the ended subagents, and the gap between two cards of the team. */
const FOLDED_H = 40;
const STACK_GAP = 16;
/** The plan keeps 30 subagents at most: at that many, more may have ended. */
const PLAN_SUBS_MAX = 30;

export type TeamItem =
  | { kind: 'sub'; id: string; sub: SubAgent }
  | { kind: 'bg'; id: string; row: BgRow }
  | { kind: 'folded'; id: 'folded'; done: number; failed: number; capped: boolean };

/** `run` alone moves: a moving link always means « it runs right now ». */
export type LinkState = 'run' | 'done' | 'failed' | 'plan' | 'launched';
/** `d`: the SVG path, in the world's coordinates. */
export type Link = { id: string; state: LinkState; d: string };
/** A link of a frame, from (x1, y1) to (x2, y2) in the frame's coordinates: a curve, or a straight line. */
export type Edge = {
  id: string;
  state: LinkState;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  straight?: boolean;
  /** Straight segments through these points: the way around the cards. */
  via?: [number, number][];
};

/** `west`: the tools, drawn above the card (the user's choice); the name is the side's, for the keyboard's order. */
export type Frame = {
  agentId: string;
  w: number;
  h: number;
  centre: Box;
  north: Box | null;
  west: Placed<ToolCard>[];
  /** The pile of the tools past `TOOLS_SHOWN`, after them; none with fewer. */
  westMore: Placed<ToolPile> | null;
  team: Placed<TeamItem>[];
  /** The latest test run of each card, stuck under it (`under`: 'centre' or the subagent's id). */
  tests: Placed<TestRun>[];
  /** The landing zone: always there, below all the rest, as wide as the frame. */
  south: Box;
  links: Edge[];
};
/**
 * `teamOpen`: its pile of ended subagents unfolded; `seenAt`: when it was unfolded last; `centreH`: its card's height;
 * `planOpen`: its plan's banner unfolded, as in its conversation (default); `teamH`: the height measured of a card of
 * its team, by its id (none or 0: its size by default); `toolsOpen`: its pile of tools unfolded.
 */
export type FrameOpts = {
  teamOpen?: boolean;
  seenAt?: number;
  centreH?: number;
  planOpen?: boolean;
  teamH?: Record<string, number>;
  toolsOpen?: boolean;
  /** The board's test command (`testCommand`): a run of it is a test run. */
  testCommand?: string;
  /** The landing zone's height measured. */
  southH?: number;
};

/** The highest a card grows (an agent that waits, its questions in it), at a zoom of 1: past it, its inset scrolls. */
export const cardMaxH = (windowH: number): number => Math.round(0.7 * windowH);

/**
 * The highest the inset of a card of `cardH` grows, in the world's pixels: the card held at 70 % of the window on screen
 * at the camera's `zoom` (never less than a few lines). It reads the zoom alone, never the layout: no loop.
 */
export const askMaxH = (windowH: number, zoom: number, cardH: number): number => Math.max(80, Math.round(cardMaxH(windowH) / zoom) - cardH);

/** It waits for the user: a question or a permission. Such an agent is unfolded at every altitude. */
export const waits = (agent: Agent): boolean => agent.status === 'waiting' || agent.pending.length > 0;

/** A subagent's name on the canvas: its type, else its title (its plan's row, else its call in the conversation). */
export function subName(agent: Agent, items: ConvItem[], call: string): string {
  const info = subagentInfo(agent, items, call);
  return info?.kind ?? info?.title ?? call;
}

/**
 * The east of `agent`: its subagents running, its background tasks running, then the ended subagents folded in one pile
 * (unless unfolded). A failure since the pile was last unfolded stays out of it, just before it: the app keeps no « seen »
 * by subagent.
 */
export function teamOf(agent: Agent, opts: FrameOpts): TeamItem[] {
  const plan = agent.plan;
  const subs = plan?.agents ?? [];
  const item = (sub: SubAgent): TeamItem => ({ kind: 'sub', id: sub.id, sub });
  const out: TeamItem[] = subs.filter(isRunning).map(item);
  out.push(
    ...bgRows(plan)
      // A test run in the background is its test card, under the card that runs it.
      .filter((row) => isRunning(row) && !(row.kind === 'command' && looksLikeTests(row.command ?? '', opts.testCommand)))
      .map((row): TeamItem => ({ kind: 'bg', id: row.id, row })),
  );
  // In the sidebar's order, the latest ended first.
  const ended: SubAgent[] = sidebarSubs(agent)?.finished ?? [];
  if (opts.teamOpen) return [...out, ...ended.map(item)];
  const seenAt = opts.seenAt ?? 0;
  const unseen = (s: SubAgent) => s.status === 'failed' && (s.endedAt ?? s.startedAt) > seenAt;
  out.push(...ended.filter(unseen).map(item));
  const pile = ended.filter((s) => !unseen(s));
  if (pile.length) {
    const failed = pile.filter((s) => s.status === 'failed').length;
    out.push({ kind: 'folded', id: 'folded', done: pile.length - failed, failed, capped: subs.length >= PLAN_SUBS_MAX });
  }
  return out;
}

const defaultSize = (i: TeamItem): Size => (i.kind === 'sub' ? SUB : i.kind === 'bg' ? BG : { w: SUB.w, h: FOLDED_H });

/** A curve from (x1, y1), leaving to the right, to (x2, y2), arriving from the left. */
function curve(x1: number, y1: number, x2: number, y2: number): string {
  const mx = (x1 + x2) / 2;
  return `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`;
}

/** The link of an edge of a frame drawn at (dx, dy) of the world. */
function linkOf(e: Edge, dx: number, dy: number): Link {
  const [x1, y1, x2, y2] = [e.x1 + dx, e.y1 + dy, e.x2 + dx, e.y2 + dy];
  if (e.via) {
    const pts = e.via.map(([x, y]) => `L ${x + dx} ${y + dy}`).join(' ');
    return { id: e.id, state: e.state, d: `M ${x1} ${y1} ${pts} L ${x2} ${y2}` };
  }
  return { id: e.id, state: e.state, d: e.straight ? `M ${x1} ${y1} L ${x2} ${y2}` : curve(x1, y1, x2, y2) };
}

/** The frames computed, by agent object: one replaced (every update of the core replaces it) is computed again. */
const frames = new WeakMap<Agent, { key: string; frame: Frame }>();

/**
 * The compass of `agent`, in its own coordinates: the plan's band on top (with tasks), its card below, its team in a
 * column on the right. A subagent whose parent is in the team, and a background task whose subagent is, lie right of it.
 */
export function frameOf(agent: Agent, opts: FrameOpts = {}): Frame {
  const key = `${!!opts.teamOpen}|${opts.seenAt ?? 0}|${opts.centreH ?? ''}|${opts.planOpen !== false}|${JSON.stringify(opts.teamH ?? {})}|${!!opts.toolsOpen}|${opts.testCommand ?? ''}|${opts.southH ?? ''}`;
  const known = frames.get(agent);
  if (known?.key === key) return known.frame;
  const frame = computeFrame(agent, opts);
  frames.set(agent, { key, frame });
  return frame;
}

function computeFrame(agent: Agent, opts: FrameOpts): Frame {
  const tasks = planView(agent)?.rows ?? [];
  let north: Box | null = null;
  if (tasks.length) {
    const h = opts.planOpen === false ? PLAN.folded : PLAN.head + Math.min(tasks.length * PLAN.row + PLAN.pad, PLAN.list);
    north = { x: 0, y: 0, w: PLAN.w, h };
  }
  const top = north ? north.h + GAP : 0;
  const all = toolCards(agent);
  let shown = all;
  let pile: ToolPile | null = null;
  if (all.length > TOOLS_SHOWN) {
    if (opts.toolsOpen) pile = { hidden: 0, open: true };
    else {
      // The ones running first: a call that runs keeps its card, and its flow.
      const recent = new Set([...all].sort((a, b) => Number(b.running) - Number(a.running) || b.lastAt - a.lastAt).slice(0, TOOLS_SHOWN));
      shown = all.filter((t) => recent.has(t));
      pile = { hidden: all.length - TOOLS_SHOWN, open: false };
    }
  }
  const slot = (i: number) => ({
    x: (i % 2) * (TOOL.w + TOOL.aisle),
    y: top + Math.floor(i / 2) * (TOOL.h + TOOL.row),
    w: TOOL.w,
    h: TOOL.h,
  });
  const west: Placed<ToolCard>[] = shown.map((t, i) => ({ ...t, ...slot(i) }));
  const westMore = pile && { ...pile, ...slot(west.length) };
  const rows = Math.ceil((west.length + (westMore ? 1 : 0)) / 2);
  const centreY = rows ? top + rows * (TOOL.h + TOOL.row) - TOOL.row + TOOL.drop : top;
  const centre: Box = { x: 0, y: centreY, w: CARD.w, h: opts.centreH ?? CARD.h };
  const links: Edge[] = [];
  if (north) {
    const x = CARD.w / 2;
    links.push({ id: `${agent.id}:plan`, state: 'plan', x1: x, y1: centre.y, x2: x, y2: north.h, straight: true });
  }

  const items = teamOf(agent, opts);
  const ids = new Set(items.map((i) => i.id));
  const parentOf = (i: TeamItem): string | undefined => {
    const p = i.kind === 'sub' ? i.sub.parent : i.kind === 'bg' ? ownerOf(agent.plan, i.row)?.id : undefined;
    return p && p !== i.id && ids.has(p) ? p : undefined;
  };
  const placed = new Map<string, Placed<TeamItem>>();
  /** The right end of each row the column started, by the item that started it. */
  const rowEnd = new Map<string, { root: string; x: number }>();
  const colX = CARD.w + 2 * GAP;
  let colY = centre.y;
  const sizeOf = (i: TeamItem): Size => {
    const s = defaultSize(i);
    return opts.teamH?.[i.id] ? { w: s.w, h: opts.teamH[i.id] } : s;
  };
  const runs = new Map(testsOf(agent, opts.testCommand ?? '').map((r) => [r.under, r]));
  /** How far a card's test card reaches below it. */
  const below = (id: string) => (runs.has(id) ? TEST.gap + TEST.h : 0);
  const place = (i: TeamItem, x: number, y: number) => placed.set(i.id, { ...i, ...sizeOf(i), x, y });
  // Down the column, a row at a time: a root, then each child right of its parent, as long as one is placed; the next
  // root under the lowest card of the row, or its test card.
  for (const root of items) {
    if (parentOf(root)) continue;
    place(root, colX, colY);
    rowEnd.set(root.id, { root: root.id, x: colX + sizeOf(root).w });
    let bottom = colY + sizeOf(root).h + below(root.id);
    for (let more = true; more;) {
      more = false;
      for (const i of items) {
        const p = parentOf(i);
        if (placed.has(i.id) || !p || !placed.has(p)) continue;
        const row = rowEnd.get(p)!;
        const end = rowEnd.get(row.root)!;
        const y = placed.get(p)!.y;
        place(i, end.x + GAP, y);
        end.x += GAP + sizeOf(i).w;
        rowEnd.set(i.id, row);
        bottom = Math.max(bottom, y + sizeOf(i).h + below(i.id));
        more = true;
      }
    }
    colY = bottom + STACK_GAP;
  }
  const team = items.map((i) => placed.get(i.id)!);
  // No test card for a subagent that has no card (folded in the pile, forgotten by the plan).
  const tests: Placed<TestRun>[] = [];
  for (const [under, r] of runs) {
    const sub = placed.get(under);
    const card = under === 'centre' ? centre : sub?.kind === 'sub' ? sub : undefined;
    if (!card) continue;
    tests.push({ ...r, x: card.x, y: card.y + card.h + TEST.gap, w: card.w, h: TEST.h });
  }

  for (const i of team) {
    const p = parentOf(i);
    const from = p ? placed.get(p)! : centre;
    const state: LinkState =
      i.kind === 'bg'
        ? 'run'
        : i.kind === 'folded'
          ? 'done'
          : i.sub.status === 'running'
            ? 'run'
            : i.sub.status === 'failed'
              ? 'failed'
              : 'done';
    links.push({ id: `${agent.id}:${i.id}`, state, x1: from.x + from.w, y1: from.y + from.h / 2, x2: i.x, y2: i.y + i.h / 2 });
  }

  // A flow from a tool only while one of its calls runs, under no card: from the tool's side on the aisle, down the aisle
  // (off its middle, the plan's link), then to the agent's card, or along the gap above the card and down right of it to
  // the subagent it runs in when that one has a card.
  const aisle = TOOL.w + TOOL.aisle / 2;
  const gapY = centre.y - TOOL.drop / 2;
  for (const t of west) {
    if (!t.running) continue;
    const id = `tool:${t.kind}:${t.name}`;
    const left = t.x === 0;
    const [x1, y1] = [left ? t.x + t.w : t.x, t.y + t.h / 2];
    const lane = aisle + (left ? -6 : 6);
    const to = t.runningIn ? placed.get(t.runningIn) : undefined;
    if (to) {
      const x = CARD.w + GAP;
      const y2 = to.y + to.h / 2;
      links.push({
        id,
        state: 'run',
        x1,
        y1,
        x2: to.x,
        y2,
        via: [
          [lane, y1],
          [lane, gapY],
          [x, gapY],
          [x, y2],
        ],
      });
    } else links.push({ id, state: 'run', x1, y1, x2: lane, y2: centre.y, via: [[lane, y1]] });
  }

  const boxes: Box[] = [centre, ...(north ? [north] : []), ...west, ...(westMore ? [westMore] : []), ...team, ...tests];
  const w = Math.max(...boxes.map((b) => b.x + b.w));
  const south: Box = { x: 0, y: Math.max(...boxes.map((b) => b.y + b.h)) + GAP, w, h: opts.southH || SOUTH.h };
  // One line down to it, from the agent's card or its test card.
  const own = tests.find((r) => r.under === 'centre') ?? centre;
  const x = CARD.w / 2;
  links.push({ id: `${agent.id}:south`, state: 'plan', x1: x, y1: own.y + own.h, x2: x, y2: south.y, straight: true });
  return {
    agentId: agent.id,
    w,
    h: south.y + south.h,
    centre,
    north,
    west,
    westMore,
    team,
    tests,
    south,
    links,
  };
}

/** In rows, in order: an item that would pass `maxW` starts the next row; a row is as high as its highest. */
export function packRows<T extends Size>(items: T[], maxW: number, gap: number): { items: Placed<T>[]; w: number; h: number } {
  const out: Placed<T>[] = [];
  let x = 0;
  let y = 0;
  let rowH = 0;
  let w = 0;
  for (const it of items) {
    if (x > 0 && x + it.w > maxW) {
      x = 0;
      y += rowH + gap;
      rowH = 0;
    }
    out.push({ ...it, x, y, w: it.w, h: it.h });
    w = Math.max(w, x + it.w);
    rowH = Math.max(rowH, it.h);
    x += it.w + gap;
  }
  return { items: out, w, h: out.length ? y + rowH : 0 };
}

/** The agents in the canvas's order: one that another of them launched (`drivenAgents`) comes right after it. */
export function canvasOrder(
  agents: Agent[],
  all: Record<string, Agent>,
  tickets: Record<string, Ticket>,
): { order: Agent[]; launched: [from: string, to: string][] } {
  const order: Agent[] = [];
  const launched: [string, string][] = [];
  const shown = new Map(agents.map((a) => [a.id, a]));
  const placed = new Set<string>();
  const visit = (a: Agent) => {
    if (placed.has(a.id)) return;
    placed.add(a.id);
    order.push(a);
    for (const d of drivenAgents(a.plan, all, tickets, a.id)) {
      const next = shown.get(d.id);
      if (!next || placed.has(d.id)) continue;
      launched.push([a.id, d.id]);
      visit(next);
    }
  };
  for (const a of agents) visit(a);
  return { order, launched };
}

/** The frames in rows, their links in the world's coordinates, and a dotted link from an agent to each it launched. */
export function layoutNear(
  frames: Frame[],
  launched: [string, string][] = [],
  maxW = 2400,
): { items: Placed<Frame>[]; links: Link[]; w: number; h: number } {
  const packed = packRows(frames, maxW, GAP);
  const links: Link[] = packed.items.flatMap((f) => f.links.map((e) => linkOf(e, f.x, f.y)));
  const byId = new Map(packed.items.map((f) => [f.agentId, f]));
  for (const [from, to] of launched) {
    const a = byId.get(from);
    const b = byId.get(to);
    if (!a || !b) continue;
    const y = (f: Placed<Frame>) => f.y + f.centre.y + f.centre.h / 2;
    links.push(linkOf({ id: `launched:${from}:${to}`, state: 'launched', x1: a.x + a.w, y1: y(a), x2: b.x, y2: y(b) }, 0, 0));
  }
  return { ...packed, links };
}

/**
 * Afar, with the agents that wait unfolded (spec, « Un agent qui attend l'utilisateur est toujours déplié ») : in the
 * order of `discs`, an agent in `open` is its frame, the others their disc; all in rows, with no overlap.
 */
export function layoutFarOpen(
  discs: Disc[],
  open: Map<string, Frame>,
  maxW = 2400,
): { discs: Placed<Disc>[]; frames: Placed<Frame>[]; links: Link[]; w: number; h: number } {
  type Item = { disc: Disc; frame?: Frame; w: number; h: number };
  const items: Item[] = discs.map((disc) => {
    const frame = open.get(disc.agentId);
    return frame ? { disc, frame, w: frame.w, h: frame.h } : { disc, w: DISC.w, h: DISC.h };
  });
  const packed = packRows(items, maxW, GAP);
  const placedDiscs = packed.items.filter((i) => !i.frame).map((i) => ({ ...i.disc, x: i.x, y: i.y, w: i.w, h: i.h }));
  const frames = packed.items.filter((i) => i.frame).map((i) => ({ ...i.frame!, x: i.x, y: i.y }));
  const links = frames.flatMap((f) => f.links.map((e) => linkOf(e, f.x, f.y)));
  return { discs: placedDiscs, frames, links, w: packed.w, h: packed.h };
}
