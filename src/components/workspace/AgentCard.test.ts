import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DraftAttachment } from '../../lib/attachments';
import { connection, type ServerInfo } from '../../lib/connection.svelte';
import { dropConversation } from '../../lib/conversations.svelte';
import { forgetDraft, getDraft, setDraft } from '../../lib/drafts';
import { setLang } from '../../lib/i18n';
import { menu } from '../../lib/menu.svelte';
import { cardSpend } from '../../lib/sidebar';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import AgentCard from './AgentCard.svelte';

const image = (name: string): DraftAttachment => ({ kind: 'image', name, mediaType: 'image/png', data: 'AA==', size: 1 });

/** Renders the card of a1, types `text` in its field and presses Enter. */
async function typeAndEnter(text: string) {
  render(AgentCard, { agent: app.agents.a1 });
  const field = screen.getByRole('textbox', { name: 'Écrire à un' }) as HTMLInputElement;
  await fireEvent.input(field, { target: { value: text } });
  await fireEvent.keyDown(field, { key: 'Enter' });
  return field;
}

describe('the card of an agent on the workspace', () => {
  let backend: ReturnType<typeof fakeBackend>;
  beforeEach(() => {
    setLang('fr');
    backend = fakeBackend({ get_conversation: () => [] });
    resetApp({ agents: [agent({ id: 'a1', name: 'un' })] });
  });
  afterEach(() => {
    menu.close();
    forgetDraft('a1');
    dropConversation('a1');
    connection.server = null;
    connection.state = 'local';
  });

  it("sends by Enter through the conversation's path", async () => {
    const field = await typeAndEnter('Bonjour');
    await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: 'Bonjour' }));
    expect(field.value).toBe('');
  });

  it('shares its draft with the conversation', () => {
    setDraft('a1', { text: 'déjà là', files: [image('a.png')] });
    render(AgentCard, { agent: app.agents.a1 });
    expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('déjà là');
    expect(screen.getByText('1 fichier')).toBeTruthy();
  });

  it('opens its conversation for an interactive command, the text kept', async () => {
    app.openWorkspace('p1');
    // `/model` est interactif pour Claude Code (interactiveOf)
    await typeAndEnter('/model');
    expect(app.workspaceOn).toBe(false);
    expect(getDraft('a1').text).toBe('/model');
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('offers model, effort, mode and « Joindre un fichier »', async () => {
    render(AgentCard, { agent: app.agents.a1 });
    await fireEvent.click(screen.getByRole('button', { name: 'Paramètres' }));
    expect(menu.open!.items.filter((i) => !i.separator).map((i) => i.label)).toEqual(['Modèle', 'Effort', 'Mode', 'Joindre un fichier']);
    expect(menu.open!.items[0].submenu!.some((i) => i.checked)).toBe(true);
  });

  it('opens its conversation on a double click', async () => {
    app.openWorkspace('p1');
    render(AgentCard, { agent: app.agents.a1 });
    await fireEvent.dblClick(screen.getByRole('article'));
    expect(app.workspaceOn).toBe(false);
    expect(app.ui.selectedAgent.p1).toBe('a1');
  });

  it('answers the question waiting, its conversation not opened before', async () => {
    dropConversation('a1');
    let loaded!: (items: unknown[]) => void;
    const question = { kind: 'question', id: 'q1', toolUseId: 't', questions: [{ question: 'Q ?', options: [] }], answers: null, ts: 1 };
    backend = fakeBackend({ get_conversation: () => new Promise((r) => (loaded = r)) });
    resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting', pending: ['q1'] })] });
    // Not loaded yet: the text stays, nothing leaves as a new message.
    const field = await typeAndEnter('Oui');
    expect(backend.called('send_message')).toHaveLength(0);
    expect(field.value).toBe('Oui');
    loaded([question]);
    await vi.waitFor(() => expect(backend.called('get_conversation').length).toBeGreaterThan(0));
    await new Promise((r) => setTimeout(r, 0));
    await fireEvent.keyDown(field, { key: 'Enter' });
    await vi.waitFor(() => expect(backend.called('answer_question')[0]?.args).toMatchObject({ id: 'a1', requestId: 'q1', answers: { 'Q ?': 'Oui' } }));
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('says why a message is refused for want of an account', async () => {
    backend = fakeBackend({
      get_conversation: () => [],
      send_message: () => {
        throw 'no-account:codex|Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness.';
      },
    });
    const field = await typeAndEnter('Bonjour');
    await vi.waitFor(() => expect(app.toasts.map((x) => x.text)).toContain('Aucun compte Codex : ajoute-en un dans Réglages › Comptes et harness.'));
    await vi.waitFor(() => expect(field.value).toBe('Bonjour'));
  });

  it("leaves the app's shortcuts to the app while one types", async () => {
    const heard = vi.fn();
    window.addEventListener('keydown', heard);
    const field = await typeAndEnter('');
    await fireEvent.keyDown(field, { key: 'k', metaKey: true });
    window.removeEventListener('keydown', heard);
    expect(heard).toHaveBeenCalledWith(expect.objectContaining({ key: 'k', metaKey: true }));
  });

  it('sends nothing while a file of the draft is being copied', async () => {
    setDraft('a1', { text: '', files: [{ ...image('a.png'), kind: 'file', copying: true }] });
    const field = await typeAndEnter('Bonjour');
    await new Promise((r) => setTimeout(r, 0));
    expect(backend.called('send_message')).toHaveLength(0);
    expect(field.value).toBe('Bonjour');
  });

  it('says that sending waits while the connection to the server is cut', async () => {
    connection.server = { id: 's1' } as ServerInfo;
    connection.state = 'lost';
    await typeAndEnter('Bonjour');
    expect(screen.getByText('Envoi suspendu jusqu’à la reprise ; ton brouillon est gardé.')).toBeTruthy();
    expect(backend.called('send_message')).toHaveLength(0);
  });

  it('sends a draft of files alone', async () => {
    setDraft('a1', { text: '', files: [image('a.png')] });
    await typeAndEnter('');
    await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: '' }));
  });

  it("says its state as the sidebar's card does, with its spend", () => {
    resetApp({ agents: [agent({ id: 'a1', name: 'un', status: 'waiting' })] });
    const { container } = render(AgentCard, { agent: app.agents.a1 });
    expect(screen.getByText('Question')).toBeTruthy();
    expect(screen.getByRole('article', { name: 'un, Question' })).toBeTruthy();
    // Its spaces are the narrow ones of the French figures, which the text queries normalise away.
    expect(container.querySelector('.spend')?.textContent?.trim()).toBe(cardSpend(app.agents.a1));
  });

  it('sends by its « Envoyer » button too', async () => {
    render(AgentCard, { agent: app.agents.a1 });
    await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'Bonjour' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Envoyer' }));
    await vi.waitFor(() => expect(backend.called('send_message')[0]?.args).toMatchObject({ id: 'a1', text: 'Bonjour' }));
  });
});
