import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// In a browser: no system's dialog, the server's folders in its place.
vi.mock('../../lib/platform', async (original) => {
  const real = await original<typeof import('../../lib/platform')>();
  return { ...real, desktop: false, open: vi.fn() };
});

import { open } from '../../lib/platform';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import NewProjectModal from './NewProjectModal.svelte';

describe('NewProjectModal in a browser', () => {
  beforeEach(() => {
    resetApp();
    app.modal = { kind: 'newProject' };
  });

  it('chooses the folder among the server’s, never by the system’s dialog', async () => {
    fakeBackend({
      fs_browse: () => ({ path: '/home/u', parent: null, entries: [{ name: 'api', dir: true, git: true }] }),
    });
    render(NewProjectModal);
    await userEvent.click(screen.getByRole('button', { name: 'Parcourir…' }));
    const box = await screen.findByRole('group', { name: 'Dossiers du serveur' });
    await userEvent.click(within(box).getByRole('button', { name: 'Choisir ce dossier' }));
    await waitFor(() => expect(screen.getByPlaceholderText(/^C:/)).toHaveValue('/home/u'));
    expect(open).not.toHaveBeenCalled();
  });
});
