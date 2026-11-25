import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { discOf, type Side } from '../../lib/workspace-layout';
import type { Agent } from '../../lib/types';
import { agent } from '../../test/ipc';
import { planWith, planWithRunning, req } from '../../test/workspace';
import DiscHost from './Disc.host.test.svelte';

const renderDisc = (a: Agent, onpick: (id: string, side?: Side) => void = () => {}) =>
  render(DiscHost, { disc: { ...discOf(a), x: 0, y: 0, w: 300, h: 300 }, agent: a, onpick });

describe('the disc of an agent', () => {
  beforeEach(() => setLang('fr'));

  it('names the agent and its state', () => {
    renderDisc(agent({ name: 'release-2-3', status: 'running' }));
    expect(screen.getByRole('button', { name: 'release-2-3, En cours' })).toBeTruthy();
  });

  it('shows the gauge, or says there is no plan', () => {
    renderDisc(agent({ plan: planWith(['done', 'done', 'done', 'pending', 'pending', 'pending', 'pending']) }));
    expect(screen.getByText('3/7 · 43 %')).toBeTruthy();
    cleanup();
    renderDisc(agent());
    expect(screen.getByText('sans plan')).toBeTruthy();
  });

  it('turns yellow and shows its questions when it waits', () => {
    const { container } = renderDisc(agent({ status: 'waiting', requests: [req('q1'), req('q2'), req('q3')] }));
    expect(container.querySelector('.halo')).toBeTruthy();
    expect(screen.getByText('questions')).toBeTruthy();
    expect(screen.getByText('3')).toBeTruthy();
  });

  it('cuts a long activity to fit in the disc', () => {
    renderDisc(agent({ status: 'running', activity: 'Lance cargo test -p escouade-plan --all-features' }));
    expect(screen.getByText('Lance cargo test -p escou…')).toBeTruthy();
  });

  it('tells a screen reader its progress and its questions', () => {
    renderDisc(
      agent({ name: 'audit', status: 'waiting', requests: [req('q1'), req('q2')], plan: planWith(['done', 'pending', 'pending']) }),
    );
    expect(screen.getByRole('button', { name: 'audit, Question, 1 tâche faite sur 3, 2 questions' })).toBeTruthy();
  });

  it('is picked by a click and by Enter', async () => {
    const onpick = vi.fn();
    renderDisc(agent({ id: 'a9' }), onpick);
    await fireEvent.keyDown(screen.getByRole('button'), { key: 'Enter' });
    await fireEvent.click(screen.getByRole('button'));
    expect(onpick).toHaveBeenNthCalledWith(1, 'a9');
    expect(onpick).toHaveBeenCalledTimes(2);
  });

  it('picks the side of a satellite', async () => {
    const onpick = vi.fn();
    const { container } = renderDisc(agent({ id: 'a9', plan: planWithRunning(1, 0) }), onpick);
    await fireEvent.click(container.querySelector('.sat.team')!);
    expect(onpick).toHaveBeenCalledWith('a9', 'east');
    expect(onpick).toHaveBeenCalledTimes(1);
  });
});
