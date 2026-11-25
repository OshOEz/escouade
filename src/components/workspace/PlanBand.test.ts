import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { planWith } from '../../test/workspace';
import PlanBand from './PlanBand.svelte';

describe('the plan of an agent on the workspace', () => {
  beforeEach(() => {
    setLang('fr');
    fakeBackend();
    resetApp();
  });

  it("shows the conversation's banner: its count, its share and a row per task", () => {
    render(PlanBand, { agent: agent({ plan: planWith(['done', 'done', 'done', 'inProgress', 'pending', 'pending', 'pending']) }) });
    expect(screen.getAllByText('3/7 tâches').length).toBeGreaterThan(0); // its count, and what a screen reader is told
    expect(screen.getByText('43 %')).toBeTruthy();
    expect(screen.getAllByRole('listitem')).toHaveLength(7);
    expect(screen.getByText('04')).toBeTruthy();
  });

  it('keeps every task of a long plan, in its list that scrolls', () => {
    render(PlanBand, { agent: agent({ plan: planWith(['done', 'inProgress', ...Array(10).fill('pending')]) }) });
    expect(screen.getAllByRole('listitem')).toHaveLength(12);
  });
});
