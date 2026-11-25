<script lang="ts">
  // The requests an agent waits on, in its card on the « Workspace » (spec 2026-11-25-vue-workspace-design.md, « La
  // question ») : the conversation's own cards, mounted as they are. `sub`: those of that subagent, on its card; none,
  // the agent's own. Its conversation is kept loaded by its card (AgentCard) while it waits.
  import { conversationOf } from '../../lib/conversations.svelte';
  import { t } from '../../lib/i18n';
  import type { Agent, PermissionItem, QuestionItem } from '../../lib/types';
  import PermissionCard from '../conv/PermissionCard.svelte';
  import QuestionCard from '../conv/QuestionCard.svelte';

  let { agent, sub, maxH }: { agent: Agent; sub: string | undefined; maxH: number } = $props();

  const conv = $derived(conversationOf(agent.id));
  const asks = $derived(
    agent.pending
      .map((id) => conv.items.find((i) => i.id === id))
      .filter(
        (i): i is QuestionItem | PermissionItem => (i?.kind === 'question' || i?.kind === 'permission') && (i.sub ?? undefined) === sub,
      ),
  );
</script>

{#if agent.pending.length && !conv.loaded && !sub}
  <p class="loading">{t('workspace.ask.loading')}</p>
{:else if asks.length}
  <!-- On the canvas, the wheel scrolls the inset rather than pans (a pinch still zooms). -->
  <!-- `current` false: no card takes the answer keys nor shows them, several may wait on screen at once. -->
  <div class="ask" style:max-height="{maxH}px" onwheel={(e) => !e.ctrlKey && !e.metaKey && e.stopPropagation()}>
    {#each asks as item (item.id)}
      {#if item.kind === 'question'}
        <QuestionCard {item} agentId={agent.id} pending current={false} />
      {:else}
        <PermissionCard {item} agentId={agent.id} cwd={agent.cwd} pending current={false} />
      {/if}
    {/each}
  </div>
{/if}

<style>
  .ask {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 6px;
    overflow: auto;
    overscroll-behavior: contain;
  }
  .loading {
    margin: 6px 0 0;
    padding-left: 16px;
    color: var(--muted);
    font-size: 12px;
  }
</style>
