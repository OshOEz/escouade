<script lang="ts">
  // The « Workspace » of a project (spec 2026-11-25-vue-workspace-design.md, « La toile »): its agents on a dotted
  // canvas one moves and zooms, each a disc seen from afar, its compass from 45 %. The canvas is one world, transformed
  // by the camera; the controls float above it.
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { menu } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import {
    altitude,
    DOT,
    fitBest,
    keepAnchor,
    nearest,
    pan,
    ZOOM_STEP,
    zoomAt,
    type Altitude,
    type Box,
    type Camera,
    HUD_H,
  } from '../../lib/workspace-camera';
  import { canvasOrder, discOf, frameOf, layoutFarOpen, layoutNear, waits, type Side } from '../../lib/workspace-layout';
  import type { Agent } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Compass from './Compass.svelte';
  import Disc from './Disc.svelte';
  import Links from './Links.svelte';

  let { project }: { project: Project } = $props();

  const agents = $derived(app.projectAgents.filter((a) => a.projectId === project.id && !a.archived));
  const ordered = $derived(canvasOrder(agents, app.agents, app.tickets));
  /** Its pile of ended subagents unfolded, by agent; when it was unfolded last (a failure since stays out of the pile). */
  let teamOpen = $state<Record<string, boolean>>({});
  let seenAt = $state<Record<string, number>>({});
  /** The agents whose tools are all shown, their pile unfolded. */
  let toolsOpen = $state<Record<string, boolean>>({});
  /** The height measured of each card, by agent then by card (`centre`: the agent's): a card that grows moves the others. */
  let heights = $state<Record<string, Record<string, number>>>({});
  const frameAt = (a: Agent) =>
    frameOf(a, {
      teamOpen: teamOpen[a.id],
      toolsOpen: toolsOpen[a.id],
      seenAt: seenAt[a.id],
      // Its banner open or folded as in its conversation (PlanBanner: the reader's choice, else a window of 760 px).
      planOpen: app.planOpen[a.id] ?? window.innerHeight >= 760,
      centreH: heights[a.id]?.centre || undefined,
      teamH: heights[a.id],
      testCommand: project.board.testCommand,
    });
  function sized(agentId: string, id: string, h: number) {
    const mine = heights[agentId];
    if (h) {
      if (mine?.[id] !== h) heights[agentId] = { ...mine, [id]: h };
      return;
    }
    // 0: not laid out (yet), or gone: its size by default, and nothing kept of a card no longer there.
    if (!mine || !(id in mine)) return;
    const { [id]: _, ...rest } = mine;
    if (Object.keys(rest).length) heights[agentId] = rest;
    else delete heights[agentId];
  }
  // From afar, an agent that waits is unfolded among the discs (« toujours déplié, quelle que soit l'altitude »).
  const far = $derived(
    layoutFarOpen(
      ordered.order.map((a) => discOf(a)),
      new Map(ordered.order.filter(waits).map((a) => [a.id, frameAt(a)])),
    ),
  );
  const near = $derived(layoutNear(ordered.order.map(frameAt), ordered.launched));
  const questions = $derived(agents.reduce((n, a) => n + (a.requests?.length ?? 0), 0));

  let view: HTMLDivElement | undefined = $state();
  let cam = $state<Camera>({ x: 0, y: 0, zoom: 1 });
  const alt = $derived(altitude(cam.zoom));
  /** The compasses on screen: all of them near, those of the agents that wait from afar. */
  const shown = $derived(alt === 'far' ? far : { links: near.links, w: near.w, h: near.h, frames: near.items });
  const discAt = $derived(new Map(far.discs.map((d) => [d.agentId, d])));
  const compassAt = $derived(new Map(shown.frames.map((f) => [f.agentId, f])));
  let drag: { x: number; y: number } | null = $state(null);
  /** The user placed the camera: a resize of the window no longer fits it. */
  let moved = false;

  const size = () => ({ w: view?.clientWidth ?? 0, h: view?.clientHeight ?? 0 });
  /** The part of the canvas under the floating bar: where the cards are fitted, and whose middle the zoom keeps. */
  const below = () => {
    const s = size();
    return { w: s.w, h: Math.max(0, s.h - HUD_H) };
  };
  const centre = () => {
    const b = below();
    return [b.w / 2, HUD_H + b.h / 2] as const;
  };

  // Fitted on opening, and on « Ajuster »: the camera is not kept.
  function fitAll() {
    // Below the floating bar: a fitted card is never under it.
    const c = fitBest({ far, near }, below());
    cam = { ...c, y: c.y + HUD_H };
    moved = false;
  }
  const place = (next: Camera) => {
    cam = next;
    moved = true;
  };
  $effect(() => {
    // Only on opening: the agents changing must not move the camera the user placed.
    if (view) untrack(fitAll);
  });

  /** Where each agent is at an altitude: its disc from afar, its card near (the card does not move on screen). */
  const anchors = (a: Altitude): (Box & { agentId: string })[] => {
    const cards = (a === 'far' ? far.frames : near.items).map((f) => ({
      agentId: f.agentId,
      x: f.x + f.centre.x,
      y: f.y + f.centre.y,
      w: f.centre.w,
      h: f.centre.h,
    }));
    return a === 'far' ? [...far.discs, ...cards] : cards;
  };

  /** Zooms by `factor` around (px, py); past 45 %, the agent nearest the middle stays where it was on screen. */
  function zoomAround(factor: number, px: number, py: number) {
    let c = cam;
    const to = altitude(zoomAt(c, factor, px, py).zoom);
    if (to !== alt) {
      const from = nearest(anchors(alt), { ...c, y: c.y - HUD_H }, below());
      const there = from && anchors(to).find((b) => b.agentId === from.agentId);
      if (from && there) c = keepAnchor(c, from, there);
    }
    place(zoomAt(c, factor, px, py));
  }

  const zoomBy = (factor: number) => {
    const [px, py] = centre();
    zoomAround(factor, px, py);
  };

  function wheel(e: WheelEvent) {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      const r = view!.getBoundingClientRect();
      zoomAround(Math.exp(-e.deltaY * 0.01), e.clientX - r.left, e.clientY - r.top);
    } else place(pan(cam, -e.deltaX, -e.deltaY));
  }

  function down(e: PointerEvent) {
    // The main button alone: a right click keeps its menu, a middle click its own use.
    // Nor on a card or in its field: one selects text there.
    if (e.button > 0 || (e.target as Element).closest('button, [role="button"], input, textarea, article, [data-side]')) return;
    drag = { x: e.clientX, y: e.clientY };
    view?.setPointerCapture?.(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    place(pan(cam, e.clientX - drag.x, e.clientY - drag.y));
    drag = { x: e.clientX, y: e.clientY };
  }
  const up = () => (drag = null);

  // A disc picked: its compass at a zoom of 1, centred on the side asked (its card for a side not built yet).
  function pick(agentId: string, side: Side = 'centre') {
    const f = near.items.find((i) => i.agentId === agentId);
    if (!f) return;
    let b: Box = f.centre;
    if (side === 'north' && f.north) b = f.north;
    const span = (bs: Box[]): Box => {
      const x = Math.min(...bs.map((i) => i.x));
      const y = Math.min(...bs.map((i) => i.y));
      return { x, y, w: Math.max(...bs.map((i) => i.x + i.w)) - x, h: Math.max(...bs.map((i) => i.y + i.h)) - y };
    };
    if (side === 'east' && f.team.length) b = span(f.team);
    if (side === 'west' && f.west.length) b = span(f.west);
    const [px, py] = centre();
    place({ zoom: 1, x: px - (f.x + b.x + b.w / 2), y: py - (f.y + b.y + b.h / 2) });
  }

  function toggleTeam(agentId: string) {
    teamOpen[agentId] = !teamOpen[agentId];
    if (teamOpen[agentId]) seenAt[agentId] = Date.now();
  }

  // Escape leaves the canvas, unless the focus is in a field, or a menu, a dialog or a popover is open (it is theirs).
  const OVER = '[aria-modal="true"], [role="dialog"], [role="listbox"], [role="menu"]';
  const typing = (el: Element | null) =>
    !!el && (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || (el as HTMLElement).isContentEditable);
  function key(e: KeyboardEvent) {
    if (e.metaKey || e.ctrlKey || e.altKey || e.defaultPrevented) return;
    if (typing(document.activeElement) || menu.open || document.querySelector(OVER)) return;
    if (e.key === 'Escape') app.closeWorkspace(project.id);
    else if (e.key === '+' || e.key === '=') zoomBy(ZOOM_STEP);
    else if (e.key === '-') zoomBy(1 / ZOOM_STEP);
    else if (e.key === '0') fitAll();
    else return;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={key} onresize={() => !moved && fitAll()} />

<div
  class="ws"
  class:pan={!!drag}
  bind:this={view}
  role="application"
  aria-label={t('workspace.label')}
  style:background-size="{DOT * cam.zoom}px {DOT * cam.zoom}px"
  style:background-position="{cam.x}px {cam.y}px"
  onwheel={wheel}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
>
  {#if agents.length}
    <div class="world" style:transform="translate({cam.x}px, {cam.y}px) scale({cam.zoom})">
      {#if shown.frames.length}<Links links={shown.links} w={shown.w} h={shown.h} />{/if}
      <!-- One piece per agent, in the canvas's order: the keyboard goes from one to the next as the eye does. -->
      {#each ordered.order as a (a.id)}
        {@const d = alt === 'far' ? discAt.get(a.id) : undefined}
        {@const f = compassAt.get(a.id)}
        {#if d}
          <svg class="one" width="1" height="1" overflow="visible"><Disc disc={d} agent={a} onpick={pick} /></svg>
        {:else if f}
          <Compass
            frame={f}
            agent={a}
            ontoggle={toggleTeam}
            ontoggletools={(id) => (toolsOpen[id] = !toolsOpen[id])}
            onsize={(id, h) => sized(a.id, id, h)}
            zoom={cam.zoom}
          />
        {/if}
      {/each}
    </div>
  {:else}
    <p class="empty">{t('workspace.empty')}</p>
  {/if}

  <button class="hud exit btn" onclick={() => app.closeWorkspace(project.id)}>
    <Icon name="left" size={14} /><span>{t('workspace.exit')}</span>
  </button>
  <div class="hud bar">
    <button aria-label={t('workspace.zoomOut')} onclick={() => zoomBy(1 / ZOOM_STEP)}>−</button>
    <span class="pct">{t('workspace.zoom', { percent: Math.round(cam.zoom * 100) })}</span>
    <button aria-label={t('workspace.zoomIn')} onclick={() => zoomBy(ZOOM_STEP)}>+</button>
    <i></i>
    <button onclick={fitAll}>{t('workspace.fit')}</button>
    <i></i>
    <span class="info">{t('workspace.count', { count: agents.length })}</span>
    {#if questions}<span class="info q">{t('workspace.questions', { count: questions })}</span>{/if}
  </div>
</div>

<style>
  /* The work area's own surface (App.svelte, `.work`), with discreet dots. */
  .ws {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    cursor: grab;
    touch-action: none;
    background-color: var(--panel);
    background-image: radial-gradient(var(--line-2) 1px, transparent 1px);
  }
  .ws.pan {
    cursor: grabbing;
    user-select: none;
  }
  .world {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
  /* A disc's own: drawn at its place in the world, past its 1 px box. */
  .world svg.one {
    position: absolute;
    left: 0;
    top: 0;
    display: block;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--muted);
  }
  /* Above the canvas, with the shadow of the app's floating surfaces. */
  .hud {
    position: absolute;
    top: 14px;
    z-index: 1;
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 8px 24px -6px rgba(0, 0, 0, calc(0.5 * var(--shade)));
  }
  .exit {
    left: 16px;
    background: var(--raise);
  }
  /* A pill of the app: its buttons are those of the composer's chips. */
  .bar {
    left: 50%;
    display: flex;
    align-items: center;
    gap: 2px;
    height: 32px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--raise);
    color: var(--soft);
    font-size: 12.5px;
    white-space: nowrap;
    transform: translateX(-50%);
  }
  .bar button {
    height: 24px;
    padding: 0 10px;
    border: none;
    border-radius: 999px;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: 550;
    cursor: pointer;
  }
  .bar button:hover {
    background: var(--hover);
  }
  .bar i {
    width: 1px;
    height: 14px;
    margin: 0 4px;
    background: var(--line-2);
  }
  .pct {
    min-width: 44px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .info {
    padding: 0 6px;
    color: var(--muted);
  }
  .info.q {
    color: var(--wait);
  }
</style>
