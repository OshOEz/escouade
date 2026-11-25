import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import type { KitUse, SubAgent } from './types';
import { art, bg, cmd, kit, planWith, planWithRunning, req, ribbon, sub } from '../test/workspace';
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
  SOUTH,
  SUB,
  teamOf,
  TEST,
  TOOL,
  toolCards,
  toolName,
  TOOLS_SHOWN,
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
      { kind: 'plugin', name: 'github', calls: 5, running: true, runningIn: 's1', firstAt: 1, lastAt: 1 },
      { kind: 'skill', name: 'tdd', calls: 1, running: false, runningIn: null, firstAt: 1, lastAt: 1 },
    ]);
    expect(toolCards(agent())).toEqual([]);
  });

  it('keeps the order of first use: a new tool never moves the others', () => {
    const at = (k: KitUse, firstAt: number) => ({ ...k, firstAt });
    const before = {
      tools: [at(kit('plugin', 'zeta'), 1), at(kit('skill', 'beta'), 5)],
      subs: [{ call: 's1', tools: [at(kit('skill', 'alpha'), 3)] }],
    };
    expect(toolCards(agent({ canvas: before })).map((c) => c.name)).toEqual(['zeta', 'alpha', 'beta']);
    const after = { ...before, tools: [...before.tools, at(kit('skill', 'aaa'), 9)] };
    expect(toolCards(agent({ canvas: after })).map((c) => c.name)).toEqual(['zeta', 'alpha', 'beta', 'aaa']);
  });

  it('names a plugin of Claude Code by its server, and keeps a name with « __ » whole', () => {
    expect(toolName({ kind: 'plugin', name: 'plugin_github_github' })).toBe('github');
    expect(toolName({ kind: 'plugin', name: 'plugin_superpowers_chrome' })).toBe('superpowers · chrome');
    expect(toolName({ kind: 'plugin', name: 'my__server' })).toBe('my__server');
    expect(toolName({ kind: 'skill', name: 'plugin_x_y' })).toBe('plugin_x_y');
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
    expect(
      f.links
        .filter((l) => !l.id.endsWith(':south'))
        .map((l) => l.state)
        .sort(),
    ).toEqual(['failed', 'plan', 'run']);
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
    expect(f.south!.y).toBe(f.centre.y + f.centre.h + GAP);
  });
  it('shows the most recent tools, the others in a pile that unfolds', () => {
    const tools = Array.from({ length: 10 }, (_, i) => ({ ...kit('skill', `s${i}`), firstAt: i, lastAt: i === 0 ? 99 : i }));
    const a = agent({ canvas: { tools, subs: [] } });
    const f = frameOf(a);
    expect(f.west).toHaveLength(TOOLS_SHOWN);
    expect(f.west.map((t) => t.name)).toEqual(['s0', 's3', 's4', 's5', 's6', 's7', 's8', 's9']);
    expect(f.westMore).toMatchObject({ hidden: 2, open: false });
    expect(f.westMore!.y + f.westMore!.h).toBeLessThan(f.centre.y);
    const open = frameOf(a, { toolsOpen: true });
    expect(open.west).toHaveLength(10);
    expect(open.westMore).toMatchObject({ hidden: 0, open: true });
    expect(discOf(a).satellites[0]).toEqual({ kind: 'tools', count: 10 });
    expect(frameOf(agent({ canvas: { tools: tools.slice(0, 8), subs: [] } })).westMore).toBeNull();
    // A call that runs keeps its card out of the pile, with its flow.
    const long = { tools: [{ ...kit('plugin', 'slow', 1, 1), firstAt: 0, lastAt: 0 }, ...tools], subs: [] };
    expect(frameOf(agent({ canvas: long })).west.map((t) => t.name)).toContain('slow');
  });

  /** The points of a path of straight segments (`M x y L x y …`). */
  const pointsOf = (d: string) => [...d.matchAll(/(-?[\d.]+) (-?[\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]);
  /** A segment, vertical or horizontal, passing inside a box (its edges excluded). */
  const crosses = ([x1, y1]: number[], [x2, y2]: number[], b: Box) =>
    Math.max(x1, x2) > b.x && Math.min(x1, x2) < b.x + b.w && Math.max(y1, y2) > b.y && Math.min(y1, y2) < b.y + b.h;
  const clear = (d: string, boxes: Box[]) => {
    const p = pointsOf(d);
    expect(d).not.toContain('C');
    for (let i = 1; i < p.length; i++)
      for (const b of boxes) expect(crosses(p[i - 1], p[i], b), `${p[i - 1]} → ${p[i]} in ${JSON.stringify(b)}`).toBe(false);
  };

  it('sends a flow down the aisle, under no card', () => {
    const tools = [kit('skill', 'a', 1, 1), kit('skill', 'b', 1, 1), kit('skill', 'c'), kit('skill', 'd'), kit('skill', 'e')];
    const f = frameOf(agent({ canvas: { tools, subs: [] } }));
    const r = layoutNear([f]);
    for (const id of ['tool:skill:a', 'tool:skill:b']) {
      const d = r.links.find((l) => l.id === id)!.d;
      clear(
        d,
        [...f.west, f.centre].filter((t) => !('name' in t) || `tool:skill:${t.name}` !== id),
      );
      const end = pointsOf(d).at(-1)!;
      expect(end[1]).toBe(f.centre.y);
      expect(end[0]).toBeGreaterThan(TOOL.w);
      expect(end[0]).toBeLessThan(TOOL.w + TOOL.aisle);
    }
  });

  it("sends the flow of a subagent's call around the cards to the subagent", () => {
    const tools = [kit('skill', 'a'), kit('skill', 'b'), kit('skill', 'c')];
    const a = agent({
      plan: planWith([], { agents: [sub('s1')] }),
      canvas: { tools, subs: [{ call: 's1', tools: [kit('plugin', 'github', 1, 1)] }] },
    });
    const f = frameOf(a);
    const gh = f.west.find((t) => t.name === 'github')!;
    const d = layoutNear([f]).links.find((l) => l.id === 'tool:plugin:github')!.d;
    clear(d, [...f.west.filter((t) => t !== gh), f.centre, ...f.team]);
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
    const links = r.links.filter((l) => !l.id.endsWith(':south'));
    expect(links.map((l) => l.state)).toEqual(['plan', 'launched']);
    expect(links[1].d.startsWith(`M ${a.w} `)).toBe(true);
    expect(links[1].d.endsWith(`${a.w + GAP} ${b.centre.y + b.centre.h / 2}`)).toBe(true);
  });

  it('computes the frame of an agent once, while it and its options stay the same', () => {
    const a = agent({ id: 'a', plan: planWith(['inProgress'], { agents: [sub('s1')] }) });
    const f = frameOf(a, { seenAt: 5 });
    expect(frameOf(a, { seenAt: 5 })).toBe(f);
    expect(frameOf({ ...a }, { seenAt: 5 })).toEqual(f);
    expect(frameOf(a, { seenAt: 5, teamOpen: true })).not.toBe(f);
  });
  it('keeps the frame of each altitude: crossing back to one computes nothing again', () => {
    const a = agent({ id: 'a', plan: planWith(['inProgress'], { agents: [sub('s1')] }) });
    const near = frameOf(a, { level: 'near' });
    const mid = frameOf(a, { level: 'mid' });
    expect(frameOf(a, { level: 'near' })).toBe(near);
    expect(frameOf(a, { level: 'mid' })).toBe(mid);
  });
  it('draws the links of a frame away from the origin in the world', () => {
    const a = frameOf(agent({ id: 'a' }));
    const b = frameOf(agent({ id: 'b', plan: planWith(['inProgress']) }));
    const r = layoutNear([a, b]);
    const x = r.items[1].x + 360 / 2;
    expect(r.links.find((l) => l.id === 'b:plan')!.d).toBe(`M ${x} ${b.centre.y} L ${x} ${b.north!.h}`);
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

describe('the deliverables and the CI', () => {
  it('says the deliverables and the worst CI on the disc', () => {
    const landing = [art(true), art(false), ribbon('deploy', 'success'), ribbon('ci', 'failure')];
    expect(discOf(agent(), landing).satellites).toEqual([
      { kind: 'deliverables', count: 2 },
      { kind: 'ci', count: 2, state: 'failure' },
    ]);
  });
});

describe('the test cards', () => {
  const team = (...subs: SubAgent[]) => planWith([], { agents: subs });

  it('sticks a test run under the card that runs it, and moves what is below', () => {
    const plain = agent({ plan: team(sub('s1'), sub('s2')) });
    const tested = agent({
      plan: plain.plan,
      canvas: { tools: [], subs: [{ call: 's1', tools: [], commands: [cmd('c1', 'npm test')] }], commands: [cmd('c0', 'cargo test')] },
    });
    const f = frameOf(tested);
    const under = (id: string) => f.tests.find((t) => t.under === id)!;
    expect(under('centre')).toMatchObject({ x: f.centre.x, y: f.centre.y + f.centre.h + TEST.gap, w: CARD.w, h: TEST.h });
    const s1 = f.team.find((i) => i.id === 's1')!;
    expect(under('s1')).toMatchObject({ x: s1.x, y: s1.y + s1.h + TEST.gap, w: s1.w });
    const was = frameOf(plain).team.find((i) => i.id === 's2')!;
    expect(f.team.find((i) => i.id === 's2')!.y - was.y).toBe(TEST.gap + TEST.h);
    expect(f.links.map((l) => l.id).filter((id) => id.includes('c0') || id.includes('c1'))).toEqual([]);
  });

  it('pushes the next row below the tests of a child', () => {
    const a = agent({
      plan: team(sub('s1'), { ...sub('s2'), parent: 's1' }, sub('s3')),
      canvas: { tools: [], subs: [{ call: 's2', tools: [], commands: [cmd('c', 'npm test')] }] },
    });
    const f = frameOf(a);
    const s2 = f.team.find((i) => i.id === 's2')!;
    const s3 = f.team.find((i) => i.id === 's3')!;
    expect(s2.y).toBe(f.team.find((i) => i.id === 's1')!.y);
    expect(s3.y).toBeGreaterThan(s2.y + s2.h + TEST.gap + TEST.h);
  });

  it('leaves a background test run out of the team: it is its test card', () => {
    const a = agent({
      plan: planWith([], {
        bgTasks: [
          { ...bg('t1'), command: 'npm test' },
          { ...bg('t2'), command: 'npm run dev' },
        ],
      }),
    });
    const f = frameOf(a);
    expect(f.team.map((i) => i.id)).toEqual(['t2']);
    expect(f.tests.map((t) => t.under)).toEqual(['centre']);
    const dev = planWith([], { bgTasks: [{ ...bg('t2'), command: 'npm run dev' }] });
    expect(frameOf(agent({ plan: dev }), { testCommand: 'npm run dev' }).team).toEqual([]);
  });

  it('keeps in the team a test run that is not its card: only the latest is', () => {
    const a = agent({
      plan: planWith([], {
        bgTasks: [
          { ...bg('t1'), command: 'npm test', startedAt: 1 },
          { ...bg('t2'), command: 'cargo test', startedAt: 2 },
        ],
      }),
    });
    const f = frameOf(a);
    expect(f.tests.map((t) => t.id)).toEqual(['tests:t2']);
    expect(f.team.map((i) => i.id)).toEqual(['t1']);
  });

  it('shows no test card for a subagent folded in the pile', () => {
    const a = agent({
      plan: team(sub('s1', 'done')),
      canvas: { tools: [], subs: [{ call: 's1', tools: [], commands: [cmd('c', 'npm test')] }] },
    });
    expect(frameOf(a).tests).toEqual([]);
  });
});

describe('the landing zone', () => {
  it('always draws the landing zone south, below the tests, with one link down to it', () => {
    const f = frameOf(agent({ canvas: { tools: [], subs: [], commands: [cmd('c0', 'cargo test')] } }));
    expect(f.south!.y).toBeGreaterThan(f.tests[0].y + f.tests[0].h);
    expect(f.south!.h).toBe(SOUTH.h);
    expect(f.links.filter((l) => l.id.endsWith(':south'))).toHaveLength(1);
    expect(frameOf(agent(), { southH: 300 }).south!.h).toBe(300);
  });
});

describe('the piles at mid-height', () => {
  const busy = () =>
    agent({
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
    expect(f.south).toBeNull();
    expect(f.northMini).toBe(true);
    expect(f.h).toBeLessThan(frameOf(busy(), { landing: [art(true)] }).h);
  });
  it("sets the west and south piles on the card's axis: the plan's line goes through their middle, not along an edge", () => {
    const f = frameOf(busy(), { level: 'mid', landing: [art(true)] });
    const axis = f.links.find((l) => l.id.endsWith(':plan'))!.x1;
    for (const p of f.piles.filter((p) => p.side !== 'east')) expect(p.x + p.w / 2).toBe(axis);
    for (const side of ['west', 'south']) {
      expect(f.links.find((l) => l.id.endsWith(`:pile:${side}`))!.x1).toBe(axis);
    }
  });
  it('unfolds the sides asked for', () => {
    const f = frameOf(busy(), { level: 'mid', unfolded: ['west'] });
    expect(f.west).toHaveLength(2);
    expect(f.piles.map((p) => p.side)).toEqual(['east']);
  });
  it('draws no pile of an empty side, not even of the landing zone', () => {
    const f = frameOf(agent(), { level: 'mid' });
    expect(f.piles).toEqual([]);
    expect(f.south).toBeNull();
    expect(f.links.filter((l) => l.id.endsWith(':south'))).toEqual([]);
  });
  it('animates the link to a pile that holds something running', () => {
    const f = frameOf(busy(), { level: 'mid' });
    expect(f.links.find((l) => l.id.endsWith(':pile:east'))?.state).toBe('run');
    expect(f.links.find((l) => l.id.endsWith(':pile:west'))?.state).not.toBe('run');
  });
  it('never folds a waiting agent', () => {
    const a = { ...busy(), status: 'waiting' as const, requests: [req('q')] };
    const f = frameOf(a, { level: 'mid' });
    expect(f.piles).toEqual([]);
    expect(f.northMini).toBe(false);
    expect(f.west).toHaveLength(2);
  });
});
