// What the « Workspace » shows of each agent, and where (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md,
// « Les trois altitudes »). Pure: the view draws what it gives.

import { bgRows, isRunning, ownerOf, type BgRow } from './bgtasks';
import { drivenAgents } from './later';
import { planView, type PlanRow } from './plan';
import { sidebarSubs } from './subagents';
import type { Agent, SubAgent, Ticket } from './types';
import type { Box, Size } from './workspace-camera';

export type DiscState = 'run' | 'wait' | 'error' | 'done' | 'idle';
/** The satellites, in their order from top to bottom; each lot of the spec brings its own. */
export type SatKind = 'team' | 'tools' | 'deliverables' | 'ci' | 'question';
export type Satellite = { kind: SatKind; count: number };

/** An agent seen from afar. `total` is 0 without a plan. */
export type Disc = { agentId: string; name: string; state: DiscState; done: number; total: number; satellites: Satellite[] };

export type Placed<T> = T & { x: number; y: number; w: number; h: number };

/** A disc's radius, its orbit's, the box it is drawn in and the gap between two boxes (px, at a zoom of 1). */
export const DISC = { r: 92, orbit: 122, w: 300, h: 300, gap: 70 } as const;

const STATE: Record<Agent['status'], DiscState> = { waiting: 'wait', error: 'error', running: 'run', done: 'done', idle: 'idle' };

export function discOf(agent: Agent): Disc {
  const mini = planView(agent)?.mini;
  const team = (agent.plan?.agents.filter(isRunning).length ?? 0) + (agent.plan?.bgTasks.filter(isRunning).length ?? 0);
  const questions = agent.requests?.length ?? 0;
  const satellites: Satellite[] = [];
  if (team) satellites.push({ kind: 'team', count: team });
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

export const CARD = { w: 360, h: 184 } as const;
export const SUB = { w: 240, h: 116 } as const;
export const BG = { w: 240, h: 86 } as const;
export const TILE = { w: 156, h: 96 } as const;
export const BAND_H = 150;
export const GAP = 48;
/** The tiles a plan's band shows at most: past them, the tasks done fold into one. */
export const MAX_TILES = 9;
/** The pile of the ended subagents, and the gap between two tiles of the band and two cards of the team. */
const FOLDED_H = 40;
const TILE_GAP = 12;
const STACK_GAP = 16;
/** The plan keeps 30 subagents at most: at that many, more may have ended. */
const PLAN_SUBS_MAX = 30;

export type TeamItem =
  | { kind: 'sub'; id: string; sub: SubAgent }
  | { kind: 'bg'; id: string; row: BgRow }
  | { kind: 'folded'; id: 'folded'; done: number; failed: number; capped: boolean };

/** `run` alone moves: a moving link always means « it runs right now ». */
export type LinkState = 'run' | 'done' | 'failed' | 'plan' | 'launched';
/** `d`: the SVG path, in the frame's coordinates, then the world's. */
export type Link = { id: string; state: LinkState; d: string };

export type Frame = { agentId: string; w: number; h: number; centre: Box; north: Box | null; team: Placed<TeamItem>[]; links: Link[] };
/** `teamOpen`: its pile of ended subagents unfolded; `seenAt`: when it was unfolded last; `centreH`: its card's height. */
export type FrameOpts = { teamOpen?: boolean; seenAt?: number; centreH?: number };

/** The tiles of the plan's band: all of them up to 9, else the tasks done folded and the others, in order. */
export function bandTiles(rows: PlanRow[]): { folded: number; rows: PlanRow[] } {
  if (rows.length <= MAX_TILES) return { folded: 0, rows };
  const left = rows.filter((r) => r.state !== 'done');
  return { folded: rows.length - left.length, rows: left.slice(0, MAX_TILES - 1) };
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
  out.push(...bgRows(plan).filter(isRunning).map((row): TeamItem => ({ kind: 'bg', id: row.id, row })));
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

const sizeOf = (i: TeamItem): Size => (i.kind === 'sub' ? SUB : i.kind === 'bg' ? BG : { w: SUB.w, h: FOLDED_H });

/** A curve from (x1, y1), leaving to the right, to (x2, y2), arriving from the left. */
function curve(x1: number, y1: number, x2: number, y2: number): string {
  const mx = (x1 + x2) / 2;
  return `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`;
}

/** The path `d` moved by (dx, dy): its points are pairs « x y », as `curve` and the plan's line write them. */
const shift = (d: string, dx: number, dy: number) => d.replace(/(-?[\d.]+) (-?[\d.]+)/g, (_, x, y) => `${+x + dx} ${+y + dy}`);

/**
 * The compass of `agent`, in its own coordinates: the plan's band on top (with tasks), its card below, its team in a
 * column on the right. A subagent whose parent is in the team, and a background task whose subagent is, lie right of it.
 */
export function frameOf(agent: Agent, opts: FrameOpts = {}): Frame {
  const rows = planView(agent)?.rows ?? [];
  let north: Box | null = null;
  if (rows.length) {
    const b = bandTiles(rows);
    const n = b.rows.length + (b.folded ? 1 : 0);
    north = { x: 0, y: 0, w: Math.max(CARD.w, n * (TILE.w + TILE_GAP) + TILE_GAP), h: BAND_H };
  }
  const centre: Box = { x: 0, y: north ? BAND_H + GAP : 0, w: CARD.w, h: opts.centreH ?? CARD.h };
  const links: Link[] = [];
  if (north) {
    const x = CARD.w / 2;
    links.push({ id: `${agent.id}:plan`, state: 'plan', d: `M ${x} ${centre.y} L ${x} ${north.h}` });
  }

  const items = teamOf(agent, opts);
  const parentOf = (i: TeamItem): string | undefined => {
    const p = i.kind === 'sub' ? i.sub.parent : i.kind === 'bg' ? ownerOf(agent.plan, i.row)?.id : undefined;
    return p && p !== i.id && items.some((o) => o.id === p) ? p : undefined;
  };
  const placed = new Map<string, Placed<TeamItem>>();
  /** The right end of each row the column started, by the item that started it. */
  const rowEnd = new Map<string, { root: string; x: number }>();
  const colX = CARD.w + 2 * GAP;
  let colY = centre.y;
  const place = (i: TeamItem, x: number, y: number) => placed.set(i.id, { ...i, ...sizeOf(i), x, y });
  // The column first, then each child right of its parent, as long as one is placed.
  for (const i of items)
    if (!parentOf(i)) {
      place(i, colX, colY);
      rowEnd.set(i.id, { root: i.id, x: colX + sizeOf(i).w });
      colY += sizeOf(i).h + STACK_GAP;
    }
  for (let more = true; more; ) {
    more = false;
    for (const i of items) {
      const p = parentOf(i);
      if (placed.has(i.id) || !p || !placed.has(p)) continue;
      const row = rowEnd.get(p)!;
      const end = rowEnd.get(row.root)!;
      place(i, end.x + GAP, placed.get(p)!.y);
      end.x += GAP + sizeOf(i).w;
      rowEnd.set(i.id, row);
      more = true;
    }
  }
  const team = items.map((i) => placed.get(i.id)!);

  for (const i of team) {
    const p = parentOf(i);
    const from = p ? placed.get(p)! : centre;
    const state: LinkState =
      i.kind === 'bg' ? 'run' : i.kind === 'folded' ? 'done' : i.sub.status === 'running' ? 'run' : i.sub.status === 'failed' ? 'failed' : 'done';
    links.push({ id: `${agent.id}:${i.id}`, state, d: curve(from.x + from.w, from.y + from.h / 2, i.x, i.y + i.h / 2) });
  }

  const boxes: Box[] = [centre, ...(north ? [north] : []), ...team];
  return {
    agentId: agent.id,
    w: Math.max(...boxes.map((b) => b.x + b.w)),
    h: Math.max(...boxes.map((b) => b.y + b.h)),
    centre,
    north,
    team,
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
  const shown = new Set(agents.map((a) => a.id));
  const visit = (a: Agent) => {
    if (order.includes(a)) return;
    order.push(a);
    for (const d of drivenAgents(a.plan, all, tickets, a.id)) {
      if (!shown.has(d.id) || order.some((o) => o.id === d.id)) continue;
      launched.push([a.id, d.id]);
      visit(agents.find((x) => x.id === d.id)!);
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
  const links: Link[] = packed.items.flatMap((f) => f.links.map((l) => ({ ...l, d: shift(l.d, f.x, f.y) })));
  const byId = new Map(packed.items.map((f) => [f.agentId, f]));
  for (const [from, to] of launched) {
    const a = byId.get(from);
    const b = byId.get(to);
    if (!a || !b) continue;
    links.push({
      id: `launched:${from}:${to}`,
      state: 'launched',
      d: curve(a.x + a.w, a.y + a.centre.y + a.centre.h / 2, b.x, b.y + b.centre.y + b.centre.h / 2),
    });
  }
  return { ...packed, links };
}
