import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { kit, planWith, planWithRunning, req, sub } from '../test/workspace';
import { planView } from './plan';
import type { Box } from './workspace-camera';
import {
  askMaxH,
  canvasOrder,
  cardMaxH,
  CARD,
  DISC,
  discOf,
  frameOf,
  GAP,
  layoutFar,
  layoutFarOpen,
  layoutNear,
  packRows,
  SUB,
  teamOf,
  TOOL,
  toolCards,
  waits,
} from './workspace-layout';

describe('the discs of the workspace', () => {
  it('says the state in one word', () => {
    expect(discOf(agent({ status: 'waiting' })).state).toBe('wait');
    expect(discOf(agent({ status: 'running' })).state).toBe('run');
    expect(discOf(agent({ status: 'error' })).state).toBe('error');
    expect(discOf(agent({ status: 'done' })).state).toBe('done');
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
    expect(discOf(a).satellites).toEqual([
      { kind: 'team', count: 3 },
      { kind: 'question', count: 2 },
    ]);
    expect(discOf(agent()).satellites).toEqual([]);
  });

  it('gathers the tools of the agent and its subagents', () => {
    const a = agent({
      canvas: { tools: [kit('plugin', 'github', 2), kit('skill', 'tdd')], subs: [{ call: 's1', tools: [kit('plugin', 'github', 3, 1)] }] },
    });
    expect(toolCards(a)).toEqual([
      { kind: 'skill', name: 'tdd', calls: 1, running: false, runningIn: null },
      { kind: 'plugin', name: 'github', calls: 5, running: true, runningIn: 's1' },
    ]);
    expect(toolCards(agent())).toEqual([]);
  });

  it('puts the tools satellite in its place', () => {
    const a = agent({
      status: 'waiting',
      requests: [req('q')],
      plan: planWithRunning(1, 0),
      canvas: { tools: [kit('skill', 'x')], subs: [] },
    });
    expect(discOf(a).satellites.map((s) => s.kind)).toEqual(['team', 'tools', 'question']);
  });

  it('lays the discs out in rows', () => {
    const discs = [1, 2, 3, 4, 5].map((i) => discOf(agent({ id: `a${i}` })));
    const { items, w, h } = layoutFar(discs, 4);
    expect(items[3]).toMatchObject({ x: 3 * (DISC.w + DISC.gap), y: 0 });
    expect(items[4]).toMatchObject({ x: 0, y: DISC.h + DISC.gap, w: DISC.w, h: DISC.h });
    expect(w).toBe(4 * DISC.w + 3 * DISC.gap);
    expect(h).toBe(2 * DISC.h + DISC.gap);
  });

  it('lays out nothing', () => {
    expect(layoutFar([])).toEqual({ items: [], w: 0, h: 0 });
  });
});

describe('the compass of the workspace', () => {
  it("sizes the north to the conversation's banner: a row a task, up to its list's height, less folded", () => {
    const north = (n: number, planOpen?: boolean) => frameOf(agent({ plan: planWith(Array(n).fill('pending')) }), { planOpen }).north!.h;
    expect(north(7)).toBeGreaterThan(north(3));
    expect(north(30)).toBe(north(40));
    expect(north(7, false)).toBeLessThan(north(3));
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
  it('puts the tools above the card, out of the way of the plan, with a link only while a call runs', () => {
    const a = agent({
      plan: planWith(['pending']),
      canvas: { tools: [kit('skill', 'tdd'), kit('plugin', 'github', 1, 1), kit('plugin', 'z')], subs: [] },
    });
    const f = frameOf(a);
    expect(f.west.map((t) => t.name)).toEqual(['tdd', 'github', 'z']);
    for (const t of f.west) {
      expect(t.y).toBeGreaterThanOrEqual(f.north!.h + GAP);
      expect(t.y + TOOL.h).toBeLessThan(f.centre.y);
      // The plan's link runs down the middle of the card: no tool card on it.
      expect(t.x + t.w < CARD.w / 2 || t.x > CARD.w / 2).toBe(true);
    }
    expect(f.links.filter((l) => l.id.startsWith('tool:'))).toEqual([expect.objectContaining({ id: 'tool:plugin:github', state: 'run' })]);
    const link = f.links.find((l) => l.id === 'tool:plugin:github')!;
    expect(link.y2).toBe(f.centre.y);
    expect(f.h).toBe(f.centre.y + f.centre.h);
  });
  it("sends the flow of a subagent's call to the subagent", () => {
    const a = agent({
      plan: planWith([], { agents: [sub('s1')] }),
      canvas: { tools: [], subs: [{ call: 's1', tools: [kit('plugin', 'github', 1, 1)] }] },
    });
    const f = frameOf(a);
    const s1 = f.team.find((t) => t.id === 's1')!;
    const link = f.links.find((l) => l.id === 'tool:plugin:github')!;
    expect([link.x2, link.y2]).toEqual([s1.x, s1.y + s1.h / 2]);
  });
  it('keeps the card where it was without tools', () => {
    expect(frameOf(agent()).centre.y).toBe(0);
    expect(frameOf(agent()).west).toEqual([]);
  });
  it('packs frames in rows', () => {
    const r = packRows(
      [
        { w: 1000, h: 10 },
        { w: 1000, h: 30 },
        { w: 1000, h: 20 },
      ],
      2100,
      50,
    );
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
  it('puts the links in the world, and a dotted one to the agent launched', () => {
    const a = frameOf(agent({ id: 'a', plan: planWith(['inProgress']) }));
    const b = frameOf(agent({ id: 'b' }));
    const r = layoutNear([a, b], [['a', 'b']]);
    expect(r.items[1].x).toBe(a.w + GAP);
    expect(r.links.map((l) => l.state)).toEqual(['plan', 'launched']);
    expect(r.links[1].d.startsWith(`M ${a.w} `)).toBe(true);
    expect(r.links[1].d.endsWith(`${a.w + GAP} ${b.centre.y + b.centre.h / 2}`)).toBe(true);
  });

  it('computes the frame of an agent once, while it and its options stay the same', () => {
    const a = agent({ id: 'a', plan: planWith(['inProgress'], { agents: [sub('s1')] }) });
    const f = frameOf(a, { seenAt: 5 });
    expect(frameOf(a, { seenAt: 5 })).toBe(f);
    expect(frameOf({ ...a }, { seenAt: 5 })).toEqual(f);
    expect(frameOf(a, { seenAt: 5, teamOpen: true })).not.toBe(f);
  });
  it('draws the links of a frame away from the origin in the world', () => {
    const a = frameOf(agent({ id: 'a' }));
    const b = frameOf(agent({ id: 'b', plan: planWith(['inProgress']) }));
    const r = layoutNear([a, b]);
    const x = r.items[1].x + 360 / 2;
    expect(r.links[0].d).toBe(`M ${x} ${b.centre.y} L ${x} ${b.north!.h}`);
  });
});

describe('the cards that grow with a question', () => {
  const overlaps = (a: Box, b: Box) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

  it('moves the team down when a card grows', () => {
    const a = agent({ plan: planWith([], { agents: [sub('s1'), sub('s2')] }) });
    const f = frameOf(a, { teamH: { s1: 300 } });
    expect(f.team[1].y).toBe(f.team[0].y + 300 + 16);
    expect(f.team[0].h).toBe(300);
  });

  it('grows the frame with its card', () => {
    const a = agent();
    expect(frameOf(a, { centreH: 500 }).h).toBe(frameOf(a).h + 500 - CARD.h);
  });

  it('caps a card at 70 % of the window', () => {
    expect(cardMaxH(1000)).toBe(700);
  });

  it('caps the inset on screen, whatever the zoom', () => {
    expect(askMaxH(1000, 1, CARD.h)).toBe(700 - CARD.h);
    // At 200 %, half the world's pixels fill as much of the screen.
    expect(askMaxH(1000, 2, CARD.h)).toBe(350 - CARD.h);
    expect(askMaxH(1000, 0.5, CARD.h)).toBe(1400 - CARD.h);
    // Never less than a few lines, however near.
    expect(askMaxH(400, 3, CARD.h)).toBe(80);
  });

  it('tells an agent that waits', () => {
    expect(waits(agent({ status: 'waiting' }))).toBe(true);
    expect(waits(agent({ pending: ['q1'], requests: [req('q1')] }))).toBe(true);
    expect(waits(agent())).toBe(false);
  });

  it('unfolds a waiting agent among the discs, without overlap', () => {
    const discs = ['a1', 'a2', 'a3'].map((id) => discOf(agent({ id })));
    const open = new Map([['a2', frameOf(agent({ id: 'a2', status: 'waiting', requests: [req('q')] }))]]);
    const r = layoutFarOpen(discs, open);
    expect(r.frames.map((f) => f.agentId)).toEqual(['a2']);
    expect(r.discs.map((d) => d.agentId)).toEqual(['a1', 'a3']);
    const boxes = [...r.discs, ...r.frames];
    for (const p of boxes) for (const q of boxes) if (p !== q) expect(overlaps(p, q)).toBe(false);
    expect(r.w).toBeGreaterThan(0);
  });
});
