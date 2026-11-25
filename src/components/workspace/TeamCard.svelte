<script lang="ts">
  // One of the team of an agent near, east of its card (spec 2026-11-25-vue-workspace-design.md, « À l'est ») : a
  // subagent, the agent's card in smaller (a field to write to it while it runs in the background), a background task in
  // dots, or the pile of the subagents that ended.
  import { bgDuration, kindLabel, rowTitle } from '../../lib/bgtasks';
  import type { DictationTarget } from '../../lib/dictation.svelte';
  import { insertAt } from '../../lib/dictation';
  import { fDur, fTok } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { modelLabel } from '../../lib/models';
  import { app } from '../../lib/state.svelte';
  import { canWriteSub, subDuration } from '../../lib/subagents';
  import type { Agent } from '../../lib/types';
  import type { TeamItem } from '../../lib/workspace-layout';
  import MicButton from '../MicButton.svelte';

  let { agent, item, ontoggle }: { agent: Agent; item: TeamItem; ontoggle: () => void } = $props();

  let field = $state<HTMLInputElement>();
  let wrap = $state<HTMLElement>();

  const sub = $derived(item.kind === 'sub' ? item.sub : null);
  const name = $derived(sub ? (sub.kind ?? sub.title) : '');
  // The rank of its task in the plan, when it is one of the plan's.
  const task = $derived.by(() => {
    const i = sub?.planTask ? (agent.plan?.tasks.findIndex((x) => x.id === sub.planTask) ?? -1) : -1;
    return i >= 0 ? i + 1 : null;
  });
  const canWrite = $derived(!!sub && canWriteSub(agent, sub));
  // What is typed to it is the app's, as in its view: it outlives the card and a change of altitude.
  const draftKey = $derived(sub ? `${agent.id}:${sub.id}` : '');
  const message = $derived(app.subDrafts[draftKey] ?? '');
  const setMessage = (v: string) => {
    if (v) app.subDrafts[draftKey] = v;
    else delete app.subDrafts[draftKey];
  };

  const dictated: DictationTarget = {
    kind: 'composer',
    id: `team-${Math.random().toString(36).slice(2)}`,
    el: () => wrap,
    insert: (said) => {
      const r = insertAt(message, field?.selectionStart ?? message.length, field?.selectionEnd ?? message.length, said);
      setMessage(r.text);
      queueMicrotask(() => {
        field?.focus();
        field?.setSelectionRange(r.caret, r.caret);
      });
    },
  };

  async function send() {
    const text = message.trim();
    if (!text || !sub || !canWrite) return;
    const sent = await app.run(api.messageSubagent(agent.id, sub.id, text).then(() => true));
    if (sent) {
      setMessage('');
      app.toast(t('subagents.writeSent', { name: agent.name }));
    }
  }

  function key(e: KeyboardEvent) {
    // The canvas's keys are not for it while one types.
    e.stopPropagation();
    if (e.key !== 'Enter' || e.isComposing) return;
    e.preventDefault();
    void send();
  }

  function open(e: MouseEvent) {
    if (!sub || (e.target as HTMLElement).closest('input, button')) return;
    app.openSubagent(agent.id, sub.id);
  }
</script>

{#if item.kind === 'folded'}
  <button type="button" class="folded" data-side="east" onclick={ontoggle}>
    {item.capped
      ? t('workspace.team.foldedCapped', { failed: item.failed })
      : t('workspace.team.folded', { done: item.done, failed: item.failed })}
  </button>
{:else if item.kind === 'bg'}
  {@const ms = bgDuration(item.row, app.now)}
  <article
    class="card bg {item.row.status}"
    data-side="east"
    aria-label={t('workspace.card.label', { name: kindLabel(item.row.kind), state: t(`bgtasks.status.${item.row.status}`) })}
  >
    <header><b>{kindLabel(item.row.kind)}</b>{#if ms !== null}<span class="dur mono">{fDur(ms)}</span>{/if}</header>
    <p class="cmd mono">{rowTitle(item.row)}</p>
  </article>
{:else if sub}
  {@const ms = subDuration(sub, app.now)}
  {@const word = t(`subagents.status.${sub.status ?? 'unknown'}`)}
  <article
    class="card sub {sub.status}"
    data-side="east"
    aria-label={t('workspace.card.label', { name, state: word })}
    ondblclick={open}
    bind:this={wrap}
  >
    <header>
      <span class="dot" aria-hidden="true"></span>
      <b class="name">{name}</b>
      {#if task}<span class="task">{t('workspace.team.task', { n: task })}</span>{/if}
      <span class="st">{word}</span>
    </header>
    <p class="meta mono">
      {[sub.model ? modelLabel(sub.model, app.models) : '', ms !== null ? fDur(ms) : '', sub.tokens ? fTok(sub.tokens) : '']
        .filter(Boolean)
        .join(' · ')}
    </p>
    {#if sub.doing}<p class="doing mono">{sub.doing}</p>{/if}
    {#if canWrite}
      <div class="bar">
        <input
          bind:this={field}
          value={message}
          aria-label={t('workspace.team.write', { name })}
          placeholder={t('workspace.team.write', { name })}
          oninput={(e) => setMessage(e.currentTarget.value)}
          onkeydown={key}
        />
        <MicButton target={dictated} panel={false} />
      </div>
    {/if}
  </article>
{/if}

<style>
  .card {
    --c: var(--line-2);
    display: flex;
    flex-direction: column;
    gap: 4px;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1.5px var(--c);
    color: var(--text);
    font-size: 12px;
  }
  .card.running {
    --c: var(--run);
  }
  .card.failed {
    --c: var(--del);
  }
  .card.bg {
    box-shadow: none;
    outline: 1.5px dashed var(--c);
    outline-offset: -1.5px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--c);
  }
  .name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .task {
    flex: none;
    padding: 0 6px;
    border-radius: 99px;
    background: var(--line);
    color: var(--soft);
    font-size: 10px;
  }
  .st,
  .dur {
    margin-left: auto;
    color: var(--muted);
    font-size: 10px;
    white-space: nowrap;
  }
  .running .st {
    color: var(--run);
  }
  .failed .st {
    color: var(--del);
  }
  p {
    margin: 0;
    overflow: hidden;
    color: var(--soft);
    font-size: 10.5px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-top: auto;
    padding: 2px 2px 2px 8px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .bar:focus-within {
    box-shadow: inset 0 0 0 1px var(--p);
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 11.5px;
    outline: none;
  }
  .folded {
    width: 100%;
    height: 100%;
    border: 0;
    border-radius: 99px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
  }
  .folded:hover {
    color: var(--text);
  }
</style>
