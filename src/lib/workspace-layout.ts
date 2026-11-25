// What the « Workspace » shows of each agent, and where (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md,
// « Les trois altitudes »). Pure: the view draws what it gives.

import { isRunning } from './bgtasks';
import { planView } from './plan';
import type { Agent } from './types';

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
