<script lang="ts">
  // The card of an agent near, at the centre of its compass (spec 2026-11-25-vue-workspace-design.md, « Au centre ») :
  // its state, its model and spend, what it does, and a bar to write to it. Enter sends by the conversation's path
  // (`submitDraft`), from the agent's own draft: what is typed here is in its conversation once the canvas is left.
  import { untrack } from 'svelte';
  import { conversationOf } from '../../lib/conversations.svelte';
  import { effortChoicesOf, modeChoicesOf, modelChoicesOf, setAgentOption, type OptionChoice } from '../../lib/agent-options';
  import { insertAt } from '../../lib/dictation';
  import type { DictationTarget } from '../../lib/dictation.svelte';
  import { getDraft, setDraft } from '../../lib/drafts';
  import { refreshModels, searchesModels } from '../../lib/engines';
  import { fDur } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { interactiveOf } from '../../lib/interactive';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { EFFORTS, modelLabel } from '../../lib/models';
  import { pendingItemOf, sendPaused, submitDraft } from '../../lib/send.svelte';
  import { tip } from '../../lib/tip';
  import { cardActivity, cardSpend } from '../../lib/sidebar';
  import { app } from '../../lib/state.svelte';
  import type { Agent } from '../../lib/types';
  import { discOf } from '../../lib/workspace-layout';
  import Icon from '../Icon.svelte';
  import MicButton from '../MicButton.svelte';

  let { agent }: { agent: Agent } = $props();

  const agentId = untrack(() => agent.id);
  let text = $state(getDraft(agentId).text);
  let files = $state(getDraft(agentId).files.length);
  let field = $state<HTMLInputElement>();
  let wrap = $state<HTMLElement>();

  const disc = $derived(discOf(agent));
  const stateWord = $derived(t(`workspace.state.${disc.state}`));
  const effort = $derived(EFFORTS.find((e) => e.value === agent.effort)?.label);
  const duration = $derived(fDur(agent.activeMs + (agent.activeSince ? app.now - agent.activeSince : 0)));
  const activity = $derived.by(() => {
    const act = cardActivity(agent, 0);
    if (act?.kind === 'doing') return act.what ? `${act.verb} ${act.what}` : act.verb;
    if (act?.kind === 'thinking') return t('stats.overview.thinking');
    if (act?.kind === 'over') return `${t('nav.sidebar.over')} · ${act.goes}`;
    return disc.state === 'wait' ? t('workspace.waiting') : '';
  });

  // Its conversation is loaded while it waits: what is typed answers the request there, as in the conversation.
  const waits = $derived(agent.status === 'waiting' || agent.pending.length > 0);
  $effect(() => {
    if (waits) return untrack(() => conversationOf(agentId).show());
  });

  // A message given back to the draft (refused), or the draft changed by the conversation: read again.
  $effect(() => {
    if (!app.draftReturned[agentId]) return;
    untrack(read);
  });

  function read() {
    const d = getDraft(agentId);
    text = d.text;
    files = d.files.length;
  }

  function write(value: string) {
    text = value;
    setDraft(agentId, { text, files: getDraft(agentId).files });
  }

  const dictated: DictationTarget = {
    kind: 'composer',
    id: `card-${Math.random().toString(36).slice(2)}`,
    el: () => wrap,
    insert: (said) => {
      const r = insertAt(text, field?.selectionStart ?? text.length, field?.selectionEnd ?? text.length, said);
      write(r.text);
      queueMicrotask(() => {
        field?.focus();
        field?.setSelectionRange(r.caret, r.caret);
      });
    },
  };

  function send() {
    const body = text.trim();
    const d = getDraft(agentId);
    if (!body && !d.files.length) return;
    // An interactive command of its harness never goes as a message: its conversation opens, the text kept in the draft.
    if (interactiveOf(agent.engine, body)) return app.selectAgent(agentId);
    // Its request not read yet: the text would leave as a new message instead of answering it.
    if (agent.pending.length && !pendingItemOf(agent)) return;
    // A file still being copied has no path yet: it would be dropped on the way, as the composer waits for it.
    if (d.files.some((f) => f.copying)) return;
    const s = submitDraft(agent, agentId, text, d.files);
    if (s.cleared === 'none') return;
    setDraft(agentId, { text: '', files: s.cleared === 'all' ? [] : d.files });
    read();
    // Refused for want of an account: the conversation says it, which the canvas does not show.
    void s.done.then(() => {
      const missing = conversationOf(agentId).noAccount;
      if (missing) app.toast(missing.text, 'error');
    });
  }

  function key(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing) return;
    e.preventDefault();
    send();
  }

  const entries = (choices: OptionChoice[], current: string, set: (v: string) => void): MenuItem[] =>
    choices.map((c) => ({
      label: c.label,
      detail: c.detail,
      title: c.title,
      disabled: c.disabled,
      reason: c.reason,
      checked: c.value === current,
      onClick: () => set(c.value),
    }));

  function settings(e: MouseEvent) {
    refreshModels(agent);
    const many = searchesModels(agent);
    // A menu does not search: a long list shows the favorites and the agent's own, the rest in the conversation.
    const models = modelChoicesOf(agent).filter((c) => !many || c.favorite || c.value === agent.model);
    const efforts = effortChoicesOf(agent);
    const items: MenuItem[] = [
      {
        label: t('workspace.card.model'),
        submenu: [
          ...entries(models, agent.model, (model) => setAgentOption(agentId, { model })),
          ...(many ? [{ separator: true, label: '' }, { label: t('workspace.card.allModels'), onClick: () => app.selectAgent(agentId) }] : []),
        ],
      },
      {
        label: t('workspace.card.effort'),
        disabled: !efforts.length,
        submenu: efforts.length ? entries(efforts, agent.effort, (effort) => setAgentOption(agentId, { effort })) : undefined,
      },
      { label: t('workspace.card.mode'), submenu: entries(modeChoicesOf(agent), agent.mode, (mode) => setAgentOption(agentId, { mode })) },
      { separator: true, label: '' },
      // The clip and the copy of files are the conversation's: the card only counts them.
      { label: t('workspace.card.attach'), icon: 'clip', onClick: () => app.selectAgent(agentId) },
    ];
    menu.showAt(e.currentTarget as HTMLElement, items);
  }

  function open(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('input, button')) return;
    app.selectAgent(agentId);
  }
</script>

<article
  class="card {disc.state}"
  data-side="centre"
  aria-label={t('workspace.card.label', { name: agent.name, state: stateWord })}
  ondblclick={open}
  bind:this={wrap}
>
  <header>
    <span class="dot" aria-hidden="true"></span>
    <b class="name">{agent.name}</b>
    <span class="st">{stateWord}</span>
  </header>
  <p class="meta mono">
    {[modelLabel(agent.model, app.models), effort, duration, cardSpend(agent)].filter(Boolean).join(' · ')}
  </p>
  {#if activity}<p class="act mono">{activity}</p>{/if}
  {#if agent.progressLine}<p class="said">{agent.progressLine}</p>{/if}
  <!-- S10 (ESC-537): the connection to the server cut, sending waits; said as the composer says it. -->
  {#if sendPaused()}<p class="paused" role="status">{t('servers.composer.paused')}</p>{/if}
  <div class="bar">
    <input
      bind:this={field}
      value={text}
      aria-label={t('workspace.card.write', { name: agent.name })}
      placeholder={t('workspace.card.write', { name: agent.name })}
      oninput={(e) => write(e.currentTarget.value)}
      onkeydown={key}
    />
    {#if files}<span class="files">{t('workspace.card.files', { count: files })}</span>{/if}
    <button type="button" class="icon" aria-label={t('workspace.card.settings')} use:tip={t('workspace.card.settings')} onclick={settings}>
      <Icon name="sliders" size={14} />
    </button>
    <MicButton target={dictated} panel={false} />
  </div>
</article>

<style>
  .card {
    --c: var(--line-2);
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-sizing: border-box;
    width: 100%;
    min-height: 100%;
    padding: 14px 16px 12px;
    border-radius: 12px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1.5px var(--c);
    color: var(--text);
  }
  .card.run {
    --c: var(--run);
  }
  .card.wait {
    --c: var(--wait);
    background: color-mix(in oklch, var(--wait) 9%, var(--raise));
    animation: halo 1.8s ease-in-out infinite;
  }
  .card.error {
    --c: var(--del);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--c);
  }
  .name {
    overflow: hidden;
    font-size: 15px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .st {
    margin-left: auto;
    color: var(--c);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .idle .st,
  .done .st {
    color: var(--muted);
  }
  p {
    margin: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .meta {
    color: var(--soft);
    font-size: 11px;
  }
  .act {
    color: var(--muted);
    font-size: 11px;
  }
  .said {
    color: var(--soft);
    font-size: 12px;
  }
  .paused {
    color: var(--warn-ink);
    font-size: 11.5px;
    white-space: normal;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: auto;
    padding: 4px 4px 4px 10px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .bar:focus-within {
    box-shadow: inset 0 0 0 1px var(--p);
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 4px 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    outline: none;
  }
  .files {
    flex: none;
    padding: 1px 6px;
    border-radius: 99px;
    background: var(--line);
    color: var(--soft);
    font-size: 10.5px;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .icon:hover {
    background: var(--line);
    color: var(--text);
  }
  @keyframes halo {
    50% {
      box-shadow:
        inset 0 0 0 1.5px var(--c),
        0 0 0 6px color-mix(in oklch, var(--wait) 22%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card.wait {
      animation: none;
    }
  }
</style>
