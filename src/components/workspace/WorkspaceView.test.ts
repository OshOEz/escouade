import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../../test/ipc';
import { req } from '../../test/workspace';
import WorkspaceView from './WorkspaceView.svelte';

const show = () => render(WorkspaceView, { project: app.projects[0] });
/** The zoom the bar shows (jsdom has no size: the canvas opens fitted at its lowest zoom). */
const zoomShown = () => document.querySelector('.bar .pct')?.textContent;

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

  it('keeps its camera when an agent changes', async () => {
    resetApp({ agents: [agent({ id: 'a1', status: 'running' })] });
    show();
    const opened = zoomShown();
    await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
    const zoomed = zoomShown();
    expect(zoomed).not.toBe(opened);
    app.agents.a1 = { ...app.agents.a1, status: 'waiting' };
    await new Promise((r) => setTimeout(r, 0));
    expect(zoomShown()).toBe(zoomed);
  });

  it('leaves Ctrl and Cmd shortcuts to the app', async () => {
    resetApp({ agents: [agent()] });
    show();
    const opened = zoomShown();
    await fireEvent.keyDown(window, { key: '+', metaKey: true });
    await fireEvent.keyDown(window, { key: '=', ctrlKey: true });
    expect(zoomShown()).toBe(opened);
    await fireEvent.keyDown(window, { key: '+' });
    expect(zoomShown()).not.toBe(opened);
  });

  it('pans with the main button only', async () => {
    resetApp({ agents: [agent()] });
    show();
    const ws = screen.getByRole('application');
    const world = () => (document.querySelector('.world') as HTMLElement).style.transform;
    // jsdom's pointer events carry no coordinates: mouse events under their names do.
    const ptr = (type: string, init: MouseEventInit) => fireEvent(ws, new MouseEvent(type, { bubbles: true, ...init }));
    const before = world();
    await ptr('pointerdown', { button: 2, clientX: 0, clientY: 0 });
    await ptr('pointermove', { clientX: 50, clientY: 50 });
    await ptr('pointerup', {});
    expect(world()).toBe(before);
    await ptr('pointerdown', { button: 0, clientX: 0, clientY: 0 });
    await ptr('pointermove', { clientX: 50, clientY: 50 });
    expect(world()).not.toBe(before);
  });

  it('fits again when the window is resized, until the camera is moved', async () => {
    let width = 400;
    const spy = vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(() => width);
    const spyH = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockImplementation(() => 800);
    resetApp({ agents: [agent()] });
    show();
    const small = zoomShown();
    width = 1200;
    await fireEvent(window, new Event('resize'));
    const large = zoomShown();
    expect(large).not.toBe(small);
    await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' }));
    const placed = zoomShown();
    width = 400;
    await fireEvent(window, new Event('resize'));
    expect(zoomShown()).toBe(placed);
    spy.mockRestore();
    spyH.mockRestore();
  });

  it('does not close on Escape while a dialog or a popover is open', async () => {
    resetApp({ agents: [agent()] });
    app.openWorkspace('p1');
    show();
    for (const role of ['dialog', 'listbox']) {
      const pop = document.body.appendChild(document.createElement('div'));
      pop.setAttribute('role', role);
      await fireEvent.keyDown(window, { key: 'Escape' });
      expect(app.workspaceOn).toBe(true);
      pop.remove();
    }
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
