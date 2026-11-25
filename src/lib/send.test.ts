import { afterEach, describe, expect, it } from 'vitest';
import { agent, fakeBackend, project, SETTINGS, BARE_ENGINE, CLAUDE_ENGINE } from '../test/ipc';
import type { DraftAttachment } from './attachments';
import { connection, type ServerInfo } from './connection.svelte';
import { conversationOf, dropConversation } from './conversations.svelte';
import { forgetDraft, getDraft } from './drafts';
import { cancelHeld, heldOf } from './outbox.svelte';
import { routeOf, submitDraft } from './send.svelte';
import { app } from './state.svelte';
import type { Agent, ConvItem, InitialState, QuestionItem } from './types';

const question = (id: string, text: string): QuestionItem =>
  ({
    kind: 'question',
    id,
    toolUseId: `tu-${id}`,
    questions: [{ question: text, options: [] }],
    answers: null,
    ts: 1,
  }) as unknown as QuestionItem;

const file = (name: string): DraftAttachment => ({ kind: 'image', name, mediaType: 'image/png', data: 'AA==', size: 1 });

/** The app started on these agents; `items`: the conversation of each; `delay`: the delay of a message typed mid-turn. */
async function start(agents: Agent[], opts: { items?: ConvItem[]; delay?: number } & Record<string, unknown> = {}) {
  const { items = [], delay = 0, ...handlers } = opts;
  const initial: InitialState = {
    projects: [project()],
    agents,
    ui: { activeProject: 'p1', view: 'project', selectedAgent: {} },
    settings: { ...SETTINGS, midTurnDelaySecs: delay },
    usage: { todayCost: 0, accounts: [], current: { claude: 'principal' } },
    git: {},
    shells: [],
    terminals: [],
    tickets: [],
    claudeFound: true,
    version: '0.1.0',
    models: [],
    engines: [CLAUDE_ENGINE, BARE_ENGINE],
  };
  const backend = fakeBackend({
    subscribe: () => initial,
    get_conversation: () => items,
    ...(handlers as Record<string, (args: any) => unknown>),
  });
  await app.init();
  for (const a of agents) await conversationOf(a.id).load();
  return backend;
}

describe('submitDraft', () => {
  afterEach(() => {
    connection.server = null;
    connection.state = 'local';
    for (const h of heldOf('a1')) cancelHeld(h.id);
    dropConversation('a1');
    forgetDraft('a1');
  });

  it('answers the question waiting with the text typed', async () => {
    const backend = await start([agent({ id: 'a1', status: 'waiting', pending: ['q1'] })], { items: [question('q1', 'Q ?')] });
    const s = submitDraft(app.agents.a1, 'a1', 'Oui', [file('a.png')]);
    await s.done;
    expect(backend.called('answer_question')[0].args).toMatchObject({ id: 'a1', requestId: 'q1', answers: { 'Q ?': 'Oui' } });
    expect(backend.called('send_message')).toHaveLength(0);
    expect(s.cleared).toBe('text');
  });

  it('sends a message otherwise, files and all', async () => {
    const backend = await start([agent({ id: 'a1' })]);
    const s = submitDraft(app.agents.a1, 'a1', 'Bonjour', []);
    await s.done;
    expect(backend.called('send_message')[0].args).toMatchObject({ id: 'a1', text: 'Bonjour' });
    expect(s.cleared).toBe('all');
  });

  it('holds a message typed mid-turn when the delay is on', async () => {
    const backend = await start([agent({ id: 'a1', status: 'running' })], { delay: 10 });
    expect(routeOf(app.agents.a1, 'x', undefined)).toEqual({ kind: 'hold', mode: 'delay' });
    submitDraft(app.agents.a1, 'a1', 'x', []);
    expect(heldOf('a1')).toHaveLength(1);
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('gives the text back when the message is refused', async () => {
    await start([agent({ id: 'a1' })], {
      send_message: () => {
        throw 'refusé';
      },
    });
    await submitDraft(app.agents.a1, 'a1', 'Bonjour', []).done;
    expect(getDraft('a1').text).toBe('Bonjour');
  });

  it('sends nothing while the server connection is cut', async () => {
    await start([agent({ id: 'a1' })]);
    connection.server = { id: 's1' } as ServerInfo;
    connection.state = 'lost';
    expect(submitDraft(app.agents.a1, 'a1', 'x', []).cleared).toBe('none');
  });
});
