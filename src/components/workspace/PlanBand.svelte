<script lang="ts">
  // The plan of an agent near, north of its card (spec 2026-11-25-vue-workspace-design.md, « Au nord », and « Choix
  // validés le 25 novembre, après le lot 1 ») : the conversation's own banner, as it is there, in a box of the canvas.
  import type { Agent } from '../../lib/types';
  import PlanBanner from '../PlanBanner.svelte';

  let { agent }: { agent: Agent } = $props();
</script>

<section class="north" data-side="north">
  <PlanBanner {agent} />
</section>

<style>
  /* A surface of the work area: the banner's own column, list and grip inside, its list scrolling past 232 px. */
  .north {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    height: 100%;
    overflow: hidden;
    border-radius: var(--r);
    background: var(--panel);
    box-shadow:
      0 0 0 1px var(--line),
      0 8px 24px -6px rgba(0, 0, 0, calc(0.5 * var(--shade)));
  }
  /* The banner's rule under it is the conversation's, where messages follow it. */
  .north > :global(.plan) {
    border-bottom: none;
  }
</style>
