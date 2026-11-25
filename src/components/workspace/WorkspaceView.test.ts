import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../../test/ipc';
import { req } from '../../test/workspace';
import WorkspaceView from './WorkspaceView.svelte';

const show = () => render(WorkspaceView, { project: app.projects[0] });

describe('the workspace', () => {
  beforeEach(() => {
    setLang('fr');
    fakeBackend();
  });

  it('draws a disc per agent of the project, archived ones aside', () => {
    resetApp({
      projects: [project(), project({ id: 'p2', name: 'autre' })],
      agents: [
        agent({ id: 'a1', name: 'un' }),
        agent({ id: 'a2', name: 'deux', archived: true }),
        agent({ id: 'a3', name: 'trois', projectId: 'p2' }),
      ],
    });
    show();
    expect(screen.getByRole('button', { name: /^un,/ })).toBeTruthy();
    expect(screen.queryByRole('button', { name: /^deux,/ })).toBeNull();
    expect(screen.queryByRole('button', { name: /^trois,/ })).toBeNull();
  });

  it('says when there is no agent', () => {
    resetApp({ agents: [] });
    show();
    expect(screen.getByText('Aucun agent dans ce projet')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Quitter le workspace' })).toBeTruthy();
  });

  it('counts the agents and the questions', () => {
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2', status: 'waiting', requests: [req('q')] })] });
    show();
    expect(screen.getByText('2 agents')).toBeTruthy();
    expect(screen.getByText('1 question')).toBeTruthy();
  });

  it('zooms with its buttons, within bounds', async () => {
    resetApp({ agents: [agent()] });
    show();
    for (let i = 0; i < 20; i++) await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
    expect(screen.getByText('250 %')).toBeTruthy();
    for (let i = 0; i < 30; i++) await fireEvent.click(screen.getByRole('button', { name: 'Dézoomer' }));
    expect(screen.getByText('30 %')).toBeTruthy();
  });

  it('closes on « Quitter »', async () => {
    resetApp({ agents: [agent()] });
    app.openWorkspace('p1');
    show();
    await fireEvent.click(screen.getByRole('button', { name: 'Quitter le workspace' }));
    expect(app.workspaceOn).toBe(false);
  });

  it('closes on Escape, but not on Escape in a field', async () => {
    resetApp({ agents: [agent()] });
    app.openWorkspace('p1');
    show();
    const field = document.body.appendChild(document.createElement('input'));
    field.focus();
    await fireEvent.keyDown(field, { key: 'Escape' });
    expect(app.workspaceOn).toBe(true);
    field.remove();
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(app.workspaceOn).toBe(false);
  });
});
