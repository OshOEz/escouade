// The state in a browser (not the desktop app's window): what a lost connection to the server does, and its return.
import { describe, expect, it, vi } from 'vitest';
import { agent, CLAUDE_ENGINE, fakeBackend, project, SETTINGS } from '../test/ipc';
import { conversationOf } from './conversations.svelte';
import { app } from './state.svelte';
import type { ConvItem, InitialState, TermInfo } from './types';

/** The transport's hooks, as the state gives them; the page's transport is told to be a browser's. */
const hooks = vi.hoisted(() => ({
  close: null as ((reason: string, again: boolean) => void) | null,
  reopen: null as (() => void) | null,
  /** The subscription's channel: the events of the server. */
  channel: null as { onmessage: (e: unknown) => void } | null,
}));
/** The browser's notifications and chime: notifier.test.ts. */
const notifier = vi.hoisted(() => ({ notify: vi.fn(() => true), chime: vi.fn(), askOnFirstGesture: vi.fn() }));
vi.mock('./notifier', () => notifier);
vi.mock('./transport', async (original) => ({
  ...(await original<typeof import('./transport')>()),
  desktop: false,
  onClose: (listener: (reason: string, again: boolean) => void) => ((hooks.close = listener), () => {}),
  onReopen: (listener: () => void) => ((hooks.reopen = listener), () => {}),
}));
/** The attach of a terminal: its xterm.js is no part of these tests. */
const attach = vi.hoisted(() => vi.fn());
vi.mock('./terminals', async (original) => ({ ...(await original<typeof import('./terminals')>()), attachTerminal: attach }));

const term = (id: string): TermInfo => ({ id, projectId: 'p1', name: `bash-${id}`, shell: 'bash' });
const text = (id: string, t: string): ConvItem => ({ kind: 'text', id, text: t, streaming: false });

/** Starts the app in a browser; `snapshot` is what each subscription gets, changed by the test between connections. */
async function start(over: Partial<InitialState> = {}, handlers: Record<string, (args: any) => unknown> = {}) {
  const snapshot: { now: InitialState } = {
    now: {
      projects: [project(), project({ id: 'p2', name: 'studio-web' })],
      agents: [agent()],
      ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
      settings: SETTINGS,
      usage: { todayCost: 0, accounts: [], current: { claude: 'principal' } },
      git: {},
      shells: [],
      terminals: [],
      tickets: [],
      claudeFound: true,
      version: '0.1.0',
      models: [],
      engines: [CLAUDE_ENGINE],
      ...over,
    },
  };
  const backend = fakeBackend({
    subscribe: (a: any) => ((hooks.channel = a.channel), snapshot.now),
    get_conversation: () => [],
    ...handlers,
  });
  app.terminals = [];
  app.exitedTerms = {};
  app.toasts = [];
  await app.init();
  /** The connection lost, then back: the state read again. */
  const cut = async () => {
    hooks.close!('the connection to the server was lost', true);
    const subscribed = backend.called('subscribe').length;
    hooks.reopen!();
    await vi.waitFor(() => expect(backend.called('subscribe')).toHaveLength(subscribed + 1));
    // Its terminals attached, its conversations asked: what the reconnection does after the snapshot.
    await new Promise((r) => setTimeout(r, 0));
  };
  return { backend, snapshot, cut };
}

describe('AppState in a browser', () => {
  it("the server's notification is the browser's, a toast when the browser shows none; its chime plays", async () => {
    await start();
    expect(notifier.askOnFirstGesture).toHaveBeenCalled();
    hooks.channel!.onmessage({ type: 'chime' });
    expect(notifier.chime).toHaveBeenCalledTimes(1);
    hooks.channel!.onmessage({ type: 'notify', id: 4, title: 'demo', body: 'Tour fini' });
    expect(notifier.notify).toHaveBeenCalledWith(4, 'demo', 'Tour fini');
    expect(app.toasts).toEqual([]);
    notifier.notify.mockReturnValueOnce(false);
    hooks.channel!.onmessage({ type: 'notify', id: 5, title: 'demo', body: 'Tour fini' });
    expect(app.toasts.at(-1)).toMatchObject({ text: 'demo — Tour fini', kind: 'info' });
  });

  it('a terminal whose attach failed in a stuttering cut is not ended; the next reconnection attaches it', async () => {
    attach.mockReset().mockResolvedValue(undefined);
    const { cut } = await start({ terminals: [term('t1')] });
    expect(app.terminals.map((x) => x.id)).toEqual(['t1']);
    // Back, then lost again before its attach is answered.
    attach.mockRejectedValueOnce(new Error('term_attach: the connection to the server was lost'));
    await cut();
    expect(app.exitedTerms).toEqual({});
    // Back for good: attached, still running.
    await cut();
    expect(attach).toHaveBeenCalledTimes(3);
    expect(app.exitedTerms).toEqual({});
    expect(app.terminals.map((x) => x.id)).toEqual(['t1']);
  });

  it('a terminal is ended when the snapshot no longer has it, and running again once attached', async () => {
    attach.mockReset().mockResolvedValue(undefined);
    const { snapshot, cut } = await start({ terminals: [term('t1'), term('t2')] });
    // Told ended by mistake (an earlier stutter): attached again, it is running.
    app.exitedTerms.t1 = null;
    snapshot.now = { ...snapshot.now, terminals: [term('t1')] };
    await cut();
    expect(app.exitedTerms).toEqual({ t2: null });
  });

  it('the conversation shown is read again: its ops of the cut were lost', async () => {
    attach.mockReset().mockResolvedValue(undefined);
    let items = [text('m:0', 'Bonjour')];
    const { cut } = await start({}, { get_conversation: () => items });
    const c = conversationOf('a1');
    c.show();
    await vi.waitFor(() => expect(c.loaded).toBe(true));
    items = [text('m:0', 'Bonjour'), text('m:1', 'Écrit pendant la coupure')];
    await cut();
    await vi.waitFor(() => expect(c.items.map((i) => i.id)).toEqual(['m:0', 'm:1']));
  });

  it('a project removed elsewhere during the cut is not left shown', async () => {
    attach.mockReset().mockResolvedValue(undefined);
    const { snapshot, cut } = await start({ ui: { activeProject: 'p2', view: 'project', selectedAgent: {} } });
    expect(app.ui.activeProject).toBe('p2');
    snapshot.now = { ...snapshot.now, projects: [project()] };
    await cut();
    expect(app.ui.activeProject).toBe('p1');
  });

  it('a server of another protocol: the page asks to be loaded again, it does not say it tries again', async () => {
    attach.mockReset().mockResolvedValue(undefined);
    await start();
    hooks.close!('the server speaks protocol 2, this page protocol 1', true);
    hooks.close!('the server speaks protocol 2, this page protocol 1', false);
    expect(app.toasts.map((x) => x.text)).toEqual(['Le serveur ne parle plus la langue de cette page : rechargez-la.']);
  });
});
