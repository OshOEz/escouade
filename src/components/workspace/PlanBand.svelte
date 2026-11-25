<script lang="ts">
  // The plan of an agent near, north of its card (spec 2026-11-25-vue-workspace-design.md, « Au nord ») : its title,
  // how far it is, and a tile per task; past 9 tasks, the ones done fold into one tile.
  import { t } from '../../lib/i18n';
  import { planView } from '../../lib/plan';
  import type { Agent } from '../../lib/types';
  import { bandTiles } from '../../lib/workspace-layout';

  let { agent }: { agent: Agent } = $props();

  const view = $derived(planView(agent));
  const band = $derived(view ? bandTiles(view.rows) : { folded: 0, rows: [] });
</script>

{#if view}
  <section class="band" data-side="north" aria-label={view.title || t('workspace.plan.chip')}>
    <header>
      <span class="chip">{t('workspace.plan.chip')}</span>
      {#if view.title}<b class="title">{view.title}</b>{/if}
      <span class="count mono">{t('workspace.plan.count', { done: view.done, total: view.total, percent: view.percent })}</span>
      <span class="gauge" aria-hidden="true">
        {#each view.rows as r (r.id)}<i class:on={r.state === 'done'}></i>{/each}
      </span>
    </header>
    <ol>
      {#if band.folded}
        <li class="tile folded">{t('workspace.plan.folded', { count: band.folded })}</li>
      {/if}
      {#each band.rows as r (r.id)}
        <li class="tile {r.state}">
          <span class="n mono">{r.n}</span>
          <span class="t">{r.title}</span>
          {#if r.step}<span class="step">{r.step}</span>{/if}
          {#if r.percent !== null}<span class="bar" aria-hidden="true"><i style:width="{r.percent}%"></i></span>{/if}
        </li>
      {/each}
    </ol>
  </section>
{/if}

<style>
  .band {
    display: flex;
    flex-direction: column;
    gap: 8px;
    box-sizing: border-box;
    height: 100%;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 12px;
  }
  .chip {
    padding: 1px 7px;
    border-radius: 99px;
    background: var(--p-soft);
    color: var(--p-ink);
    font-size: 10.5px;
    font-weight: 600;
  }
  .title {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .count {
    flex: none;
    color: var(--soft);
    font-size: 11px;
  }
  .gauge {
    display: flex;
    flex: none;
    gap: 2px;
  }
  .gauge i {
    width: 8px;
    height: 5px;
    border-radius: 3px;
    background: var(--line-2);
  }
  .gauge i.on {
    background: var(--run);
  }
  ol {
    display: flex;
    flex: 1;
    gap: 12px;
    min-height: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .tile {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 3px;
    box-sizing: border-box;
    width: 156px;
    padding: 8px 10px;
    overflow: hidden;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 11.5px;
  }
  .tile.run {
    box-shadow: inset 0 0 0 1.5px var(--run);
  }
  .tile.done,
  .tile.folded {
    opacity: 0.55;
  }
  .tile.folded {
    justify-content: center;
    color: var(--soft);
  }
  .n {
    color: var(--muted);
    font-size: 10px;
  }
  .t {
    display: -webkit-box;
    overflow: hidden;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .step {
    overflow: hidden;
    color: var(--muted);
    font-size: 10.5px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .bar {
    height: 3px;
    margin-top: auto;
    border-radius: 2px;
    background: var(--line-2);
  }
  .bar i {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--run);
  }
</style>
