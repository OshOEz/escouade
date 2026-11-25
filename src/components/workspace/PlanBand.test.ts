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

  it('shows the plan, its count and a tile per task', () => {
    render(PlanBand, { agent: agent({ plan: planWith(['done', 'done', 'done', 'inProgress', 'pending', 'pending', 'pending']) }) });
    expect(screen.getByText('3/7 tâches · 43 %')).toBeTruthy();
    expect(screen.getAllByRole('listitem')).toHaveLength(7);
  });

  it('folds the tasks done past 9', () => {
    render(PlanBand, { agent: agent({ plan: planWith([...Array(8).fill('done'), 'inProgress', 'pending']) }) });
    expect(screen.getByText('8 faites')).toBeTruthy();
  });
});
