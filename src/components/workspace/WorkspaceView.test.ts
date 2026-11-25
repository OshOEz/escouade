import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, project, resetApp } from '../../test/ipc';
import { planWith, planWithRunning, req, sub } from '../../test/workspace';
import WorkspaceView from './WorkspaceView.svelte';

const show = () => render(WorkspaceView, { project: app.projects[0] });
/** The zoom the bar shows (jsdom has no size: the canvas opens fitted at its lowest zoom). */
const zoomShown = () => document.querySelector('.bar .pct')?.textContent;

/** Clicks « Zoomer » or « Dézoomer » until the bar says `z`; for 1, clicks the first disc, which brings it to 100 %. */
async function zoomTo(z: number) {
  const want = `${Math.round(z * 100)} %`;
  if (z === 1) return fireEvent.click(document.querySelector('.disc')!);
  for (let i = 0; i < 20 && zoomShown() !== want; i++) {
    const shown = parseInt(zoomShown() ?? '0');
    await fireEvent.click(screen.getByRole('button', { name: shown < z * 100 ? 'Zoomer' : 'Dézoomer' }));
  }
  expect(zoomShown()).toBe(want);
}

/**
 * The centre on screen of an agent, from the world's transform: its disc's `<g>` from afar, its card in its compass
 * near (positions written in `left`/`top` of the world).
 */
function centreOf(container: HTMLElement, agentId: string): [number, number] {
  const m = (container.querySelector('.world') as HTMLElement).style.transform.match(/translate\((.+)px, (.+)px\) scale\((.+)\)/)!;
  const [x, y, z] = m.slice(1).map(Number);
  const g = container.querySelector(`g[data-agent="${agentId}"]`);
  let wx: number;
  let wy: number;
  if (g) [wx, wy] = g.getAttribute('transform')!.match(/translate\((.+) (.+)\)/)!.slice(1).map(Number);
  else {
    const frame = container.querySelector(`div[data-agent="${agentId}"]`) as HTMLElement;
    const slot = frame.querySelector('[data-side="centre"]')!.parentElement as HTMLElement;
    const px = (el: HTMLElement, k: 'left' | 'top' | 'width' | 'height') => parseFloat(el.style[k]);
    wx = px(frame, 'left') + px(slot, 'left') + px(slot, 'width') / 2;
    wy = px(frame, 'top') + px(slot, 'top') + px(slot, 'height') / 2;
  }
  return [Math.round(wx * z + x), Math.round(wy * z + y)];
}

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

  it('shows the compasses from 45 %, the discs below', async () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
    show(); // jsdom : la toile s'ouvre au zoom le plus bas, donc de loin
    expect(screen.queryByRole('textbox', { name: 'Écrire à un' })).toBeNull();
    for (let i = 0; i < 3; i++) await fireEvent.click(screen.getByRole('button', { name: 'Zoomer' })); // 0,3 → 0,59
    expect(screen.getByRole('textbox', { name: 'Écrire à un' })).toBeTruthy();
  });

  it('keeps the agent in the middle when the altitude changes', async () => {
    // Three in a row: the middle one is at the centre of the screen (jsdom's is its corner), where the zoom keeps it.
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' }), agent({ id: 'a3' })] });
    const { container } = show();
    const before = centreOf(container, 'a2');
    await zoomTo(0.59);
    expect(container.querySelector('g[data-agent]')).toBeNull();
    expect(centreOf(container, 'a2')).toEqual(before);
  });

  it('zooms on the team when its satellite is clicked', async () => {
    resetApp({ agents: [agent({ id: 'a1', plan: planWithRunning(2, 0) })] });
    const { container } = show();
    await fireEvent.click(container.querySelector('.sat.team')!);
    expect(zoomShown()).toBe('100 %');
    expect(screen.getAllByRole('article', { name: /sous-agent/ }).length).toBe(2);
  });

  it("leaves the card's field alone", async () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
    app.openWorkspace('p1');
    show();
    await zoomTo(1);
    const field = screen.getByRole('textbox', { name: 'Écrire à un' });
    field.focus();
    for (const key of ['+', '-', '0', 'Escape']) await fireEvent.keyDown(field, { key });
    expect(zoomShown()).toBe('100 %');
    expect(app.workspaceOn).toBe(true);
    await fireEvent.pointerDown(field, { clientX: 0, clientY: 0 });
    expect(document.querySelector('.ws.pan')).toBeNull();
  });

  it('animates only the links of what runs', async () => {
    resetApp({ agents: [agent({ id: 'a1', plan: planWith([], { agents: [sub('s1'), sub('s2', 'done')] }) })] });
    const { container } = show();
    await zoomTo(1);
    await fireEvent.click(screen.getByRole('button', { name: /terminé/ }));
    expect(container.querySelectorAll('path.link.run')).toHaveLength(1);
    expect(container.querySelectorAll('path.link.done')).toHaveLength(1);
  });
});
