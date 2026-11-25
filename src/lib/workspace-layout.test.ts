import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { planWith, planWithRunning, req } from '../test/workspace';
import { DISC, discOf, layoutFar } from './workspace-layout';

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
