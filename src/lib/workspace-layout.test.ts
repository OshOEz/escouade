import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { planWith, planWithRunning, req, sub } from '../test/workspace';
import { planView } from './plan';
import { bandTiles, canvasOrder, DISC, discOf, frameOf, GAP, layoutFar, layoutNear, packRows, SUB, teamOf } from './workspace-layout';

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
  it('puts the links in the world, and a dotted one to the agent launched', () => {
    const a = frameOf(agent({ id: 'a', plan: planWith(['inProgress']) }));
    const b = frameOf(agent({ id: 'b' }));
    const r = layoutNear([a, b], [['a', 'b']]);
    expect(r.items[1].x).toBe(a.w + GAP);
    expect(r.links.map((l) => l.state)).toEqual(['plan', 'launched']);
    expect(r.links[1].d.startsWith(`M ${a.w} `)).toBe(true);
    expect(r.links[1].d.endsWith(`${a.w + GAP} ${b.centre.y + b.centre.h / 2}`)).toBe(true);
  });
});
