import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { conversationOf } from '../../lib/conversations.svelte';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, resetApp } from '../../test/ipc';
import { ribbon } from '../../test/workspace';
import RibbonCard from './RibbonCard.svelte';

describe('the card with a ribbon', () => {
  beforeEach(() => setLang('fr'));

  it('says a test run, its command and its state, and opens its output where it is', async () => {
    resetApp({ agents: [agent({ id: 'a1' })] });
    app.openWorkspace('p1');
    const item = { ...ribbon('tests', 'failure'), line: 'npm test', since: 0, until: 65_000, opens: { kind: 'task', task: 't1' } as const };
    const { container } = render(RibbonCard, { agentId: 'a1', item, side: 'centre' });
    expect(screen.getByRole('article', { name: 'Tests, en échec' })).toBeTruthy();
    expect(screen.getByText('npm test')).toBeTruthy();
    expect(container.querySelector('.ribbon.stripes.failure')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Sortie' }));
    expect(app.taskView.a1).toBe('t1');
    expect(app.workspaceOn).toBe(false);
  });

  it('opens a terminal call in the conversation, on the call', async () => {
    resetApp({ agents: [agent({ id: 'a1' })] });
    render(RibbonCard, { agentId: 'a1', item: { ...ribbon('tests', 'success'), opens: { kind: 'call', call: 'c1' } }, side: 'centre' });
    await fireEvent.click(screen.getByRole('button', { name: 'Sortie' }));
    expect(conversationOf('a1').jump).toBe('c1');
  });

  it('draws a deployment in dashes, moving only while it runs', () => {
    const { container } = render(RibbonCard, { agentId: 'a1', item: ribbon('deploy', 'running'), side: 'south' });
    expect(container.querySelector('.ribbon.dashes.running')).toBeTruthy();
  });

  it('says a test run stopped before its end « arrêté », still and grey', () => {
    resetApp({ agents: [agent({ id: 'a1' })] });
    const { container } = render(RibbonCard, { agentId: 'a1', item: ribbon('tests', 'stopped'), side: 'centre' });
    expect(screen.getByRole('article', { name: 'Tests, arrêté' })).toBeTruthy();
    expect(container.querySelector('.ribbon.stopped:not(.running)')).toBeTruthy();
  });
});
