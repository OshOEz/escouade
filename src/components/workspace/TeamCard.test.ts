import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { sub } from '../../test/workspace';
import TeamCard from './TeamCard.svelte';

describe('the team of an agent on the workspace', () => {
  let backend: ReturnType<typeof fakeBackend>;
  beforeEach(() => {
    setLang('fr');
    backend = fakeBackend();
    resetApp();
  });

  it('lets one write to a subagent running in the background', async () => {
    const s = { ...sub('s1'), kind: 'Explore', taskId: 't1', background: true };
    render(TeamCard, { agent: agent({ id: 'a1' }), item: { kind: 'sub', id: 's1', sub: s }, ontoggle: () => {} });
    const field = screen.getByRole('textbox', { name: 'Écrire à Explore' });
    await fireEvent.input(field, { target: { value: 'continue' } });
    await fireEvent.keyDown(field, { key: 'Enter' });
    expect(backend.called('message_subagent')[0].args).toEqual({ id: 'a1', call: 's1', text: 'continue' });
  });

  it('has no field for a subagent in the foreground', () => {
    render(TeamCard, { agent: agent(), item: { kind: 'sub', id: 's1', sub: sub('s1') }, ontoggle: () => {} });
    expect(screen.queryByRole('textbox')).toBeNull();
  });

  it('says the pile, and unfolds it on a click', async () => {
    const ontoggle = vi.fn();
    render(TeamCard, { agent: agent(), item: { kind: 'folded', id: 'folded', done: 4, failed: 1, capped: false }, ontoggle });
    await fireEvent.click(screen.getByRole('button', { name: '+ 4 terminés · 1 échecs' }));
    expect(ontoggle).toHaveBeenCalled();
  });
});
