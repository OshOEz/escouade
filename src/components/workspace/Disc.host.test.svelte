<script lang="ts">
  // A disc is SVG: the test draws it in an <svg>, as the canvas does.
  import type { Agent } from '../../lib/types';
  import type { Disc as DiscModel, Placed, Side } from '../../lib/workspace-layout';
  import Disc from './Disc.svelte';

  let { disc, agent, onpick }: { disc: Placed<DiscModel>; agent: Agent; onpick: (id: string, side?: Side) => void } = $props();
</script>

<svg width="300" height="300"><Disc {disc} {agent} {onpick} /></svg>
