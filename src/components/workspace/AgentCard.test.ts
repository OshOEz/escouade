import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DraftAttachment } from '../../lib/attachments';
import { forgetDraft, getDraft, setDraft } from '../../lib/drafts';
import { setLang } from '../../lib/i18n';
import { menu } from '../../lib/menu.svelte';
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
});
