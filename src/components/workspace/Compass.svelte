<script lang="ts">
  // An agent near on the « Workspace » (spec 2026-11-25-vue-workspace-design.md, « De près ») : its frame, placed in the
  // world, with its plan north, its card at the centre and its team east. Their order in the DOM is the keyboard's:
  // north, west (its tools, drawn above the card), centre, east, south (lot 5 puts the south after the team).
  // `onsize` tells the height of its cards, by their id (the agent's: `centre`): a card that grows moves the others.
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { readPrDetail } from '../../lib/workspace-ci.svelte';
  import type { Agent } from '../../lib/types';
  import type { Box, Size } from '../../lib/workspace-camera';
  import type { Landing } from '../../lib/workspace-landing';
  import type { Frame, Placed } from '../../lib/workspace-layout';
  import AgentCard from './AgentCard.svelte';
  import LandingZone from './LandingZone.svelte';
  import PlanBand from './PlanBand.svelte';
  import RibbonCard from './RibbonCard.svelte';
  import TeamCard from './TeamCard.svelte';
  import ToolCard from './ToolCard.svelte';

  let {
    frame,
    agent,
    landing = [],
    ontoggle,
    ontoggletools,
    onsize,
    zoom = 1,
  }: {
    frame: Placed<Frame>;
    agent: Agent;
    /** What lands in its zone, south (`landingOf`). */
    landing?: Landing[];
    ontoggle: (agentId: string) => void;
    ontoggletools?: (agentId: string) => void;
    onsize?: (id: string, h: number) => void;
    zoom?: number;
  } = $props();

  // Its open pull request's detail (checks, deployments) read when it is on screen, and when its checks change.
  const prKey = $derived.by(() => {
    const pr = agent.prs.find((p) => p.state === 'open');
    return pr ? `${pr.number}:${pr.checks}` : '';
  });
  $effect(() => {
    void prKey;
    untrack(() => void readPrDetail(agent));
  });
  const testOf = (under: string) => frame.tests.find((r) => r.under === under);
  const at = (b: Box | (Size & { x: number; y: number })) => `left:${b.x}px;top:${b.y}px;width:${b.w}px;height:${b.h}px`;
</script>

<!-- A test card right after the card it is stuck under: the keyboard reads it next. -->
{#snippet tests(under: string)}
  {@const run = testOf(under)}
  {#if run}
    <div class="slot" style={at(run)}><RibbonCard agentId={agent.id} item={run} side={under === 'centre' ? 'centre' : 'east'} /></div>
  {/if}
{/snippet}

<div class="compass" data-agent={frame.agentId} style={at(frame)}>
  {#if frame.north}<div class="slot" style={at(frame.north)}><PlanBand {agent} /></div>{/if}
  {#each frame.west as tool (`${tool.kind}:${tool.name}`)}
    <div class="slot" style={at(tool)}><ToolCard {tool} /></div>
  {/each}
  {#if frame.westMore}
    <div class="slot" style={at(frame.westMore)}>
      <button type="button" class="more" data-side="west" onclick={() => ontoggletools?.(agent.id)}>
        {frame.westMore.open ? t('workspace.tool.fold') : t('workspace.tool.more', { count: frame.westMore.hidden })}
      </button>
    </div>
  {/if}
  <div class="slot" style={at(frame.centre)}>
    <AgentCard {agent} shown={frame.team.filter((i) => i.kind === 'sub').map((i) => i.id)} onsize={(h) => onsize?.('centre', h)} {zoom} />
  </div>
  {@render tests('centre')}
  {#each frame.team as item (item.id)}
    <div class="slot" style={at(item)}><TeamCard {agent} {item} ontoggle={() => ontoggle(agent.id)} {onsize} {zoom} /></div>
    {@render tests(item.id)}
  {/each}
  <div class="slot" style={at(frame.south)}><LandingZone agentId={agent.id} items={landing} onsize={(h) => onsize?.('south', h)} /></div>
</div>

<style>
  .compass,
  .slot {
    position: absolute;
  }
  /* The pile of the tools: squared as their cards, dotted as a pile. */
  .more {
    width: 100%;
    height: 100%;
    border: none;
    border-radius: 3px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: inherit;
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
  }
  .more:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>
