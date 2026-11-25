<script lang="ts">
  // The « Workspace » of a project (spec 2026-11-25-vue-workspace-design.md, « La toile »): its agents on a dotted
  // canvas one moves and zooms, each a disc seen from afar. The canvas is one world, transformed by the camera; the
  // controls float above it.
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { menu } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import { DOT, fit, pan, ZOOM_STEP, zoomAt, type Camera } from '../../lib/workspace-camera';
  import { discOf, layoutFar } from '../../lib/workspace-layout';
  import Icon from '../Icon.svelte';
  import Disc from './Disc.svelte';

  let { project }: { project: Project } = $props();

  const agents = $derived(app.projectAgents.filter((a) => a.projectId === project.id && !a.archived));
  const world = $derived(layoutFar(agents.map(discOf)));
  const byId = $derived(new Map(agents.map((a) => [a.id, a])));
  const questions = $derived(agents.reduce((n, a) => n + (a.requests?.length ?? 0), 0));

  let view: HTMLDivElement | undefined = $state();
  let cam = $state<Camera>({ x: 0, y: 0, zoom: 1 });
  let drag: { x: number; y: number } | null = $state(null);
  /** The user placed the camera: a resize of the window no longer fits it. */
  let moved = false;

  const size = () => ({ w: view?.clientWidth ?? 0, h: view?.clientHeight ?? 0 });
  const centre = () => {
    const s = size();
    return [s.w / 2, s.h / 2] as const;
  };

  // Fitted on opening, and on « Ajuster »: the camera is not kept.
  function fitAll() {
    cam = fit(world, size());
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

  const zoomBy = (factor: number) => {
    const [px, py] = centre();
    place(zoomAt(cam, factor, px, py));
  };

  function wheel(e: WheelEvent) {
    e.preventDefault();
    if (e.ctrlKey || e.metaKey) {
      const r = view!.getBoundingClientRect();
      place(zoomAt(cam, Math.exp(-e.deltaY * 0.01), e.clientX - r.left, e.clientY - r.top));
    } else place(pan(cam, -e.deltaX, -e.deltaY));
  }

  function down(e: PointerEvent) {
    // The main button alone: a right click keeps its menu, a middle click its own use.
    if (e.button > 0 || (e.target as Element).closest('button, [role="button"]')) return;
    drag = { x: e.clientX, y: e.clientY };
    view?.setPointerCapture?.(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    place(pan(cam, e.clientX - drag.x, e.clientY - drag.y));
    drag = { x: e.clientX, y: e.clientY };
  }
  const up = () => (drag = null);

  // A disc picked: it comes to the centre at a zoom of 1 (its compass, lot 2, will open there).
  function pick(agentId: string) {
    const d = world.items.find((i) => i.agentId === agentId);
    if (!d) return;
    const [px, py] = centre();
    place({ zoom: 1, x: px - (d.x + d.w / 2), y: py - (d.y + d.h / 2) });
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
  {#if world.items.length}
    <div class="world" style:transform="translate({cam.x}px, {cam.y}px) scale({cam.zoom})">
      <svg width={world.w} height={world.h} viewBox="0 0 {world.w} {world.h}" overflow="visible">
        {#each world.items as d (d.agentId)}
          {@const a = byId.get(d.agentId)}
          {#if a}<Disc disc={d} agent={a} onpick={pick} />{/if}
        {/each}
      </svg>
    </div>
  {:else}
    <p class="empty">{t('workspace.empty')}</p>
  {/if}

  <button class="hud exit" onclick={() => app.closeWorkspace(project.id)}>
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
  .ws {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    cursor: grab;
    touch-action: none;
    background-color: var(--well);
    background-image: radial-gradient(var(--line-2) 1.2px, transparent 1.2px);
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
  .world svg {
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
  .hud {
    position: absolute;
    top: 14px;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--raise);
    border: 1px solid var(--line-2);
    box-shadow: 0 6px 16px rgb(0 0 0 / 0.2);
    color: var(--soft);
  }
  .exit {
    left: 16px;
    padding: 6px 12px 6px 9px;
    border-radius: var(--r-sm);
    color: var(--text);
    cursor: pointer;
  }
  .bar {
    left: 50%;
    transform: translateX(-50%);
    padding: 4px 8px;
    border-radius: 999px;
    white-space: nowrap;
  }
  .bar button {
    border: none;
    background: none;
    color: var(--text);
    border-radius: 999px;
    padding: 4px 10px;
    cursor: pointer;
  }
  .bar button:hover,
  .exit:hover {
    background: var(--hover);
  }
  .bar i {
    width: 1px;
    height: 14px;
    background: var(--line-2);
    margin: 0 4px;
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
