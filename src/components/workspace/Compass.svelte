<script lang="ts">
  // An agent near on the « Workspace » (spec 2026-11-25-vue-workspace-design.md, « De près ») : its frame, placed in the
  // world, with its plan north, its card at the centre and its team east. Their order in the DOM is the keyboard's:
  // north, west, centre, east, south (lots 4 and 5 put the west before the card and the south after the team).
  // `onsize` tells the height of its cards, by their id (the agent's: `centre`): a card that grows moves the others.
  import type { Agent } from '../../lib/types';
  import type { Box, Size } from '../../lib/workspace-camera';
  import type { Frame, Placed } from '../../lib/workspace-layout';
  import AgentCard from './AgentCard.svelte';
  import PlanBand from './PlanBand.svelte';
  import TeamCard from './TeamCard.svelte';

  let {
    frame,
    agent,
    ontoggle,
    onsize,
    zoom = 1,
  }: {
    frame: Placed<Frame>;
    agent: Agent;
    ontoggle: (agentId: string) => void;
    onsize?: (id: string, h: number) => void;
    zoom?: number;
  } = $props();

  const at = (b: Box | (Size & { x: number; y: number })) => `left:${b.x}px;top:${b.y}px;width:${b.w}px;height:${b.h}px`;
</script>

<div class="compass" data-agent={frame.agentId} style={at(frame)}>
  {#if frame.north}<div class="slot" style={at(frame.north)}><PlanBand {agent} /></div>{/if}
  <div class="slot" style={at(frame.centre)}>
    <AgentCard {agent} shown={frame.team.filter((i) => i.kind === 'sub').map((i) => i.id)} onsize={(h) => onsize?.('centre', h)} {zoom} />
  </div>
  {#each frame.team as item (item.id)}
    <div class="slot" style={at(item)}><TeamCard {agent} {item} ontoggle={() => ontoggle(agent.id)} {onsize} {zoom} /></div>
  {/each}
</div>

<style>
  .compass,
  .slot {
    position: absolute;
  }
</style>
