import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Listing } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import ServerFolders from './ServerFolders.svelte';

/** The server's folders: a home, `projets` in it. */
const FOLDERS: Record<string, Listing> = {
  '/home/u': { path: '/home/u', parent: null, entries: [{ name: 'projets', dir: true, git: false }] },
  '/home/u/projets': {
    path: '/home/u/projets',
    parent: '/home/u',
    entries: [
      { name: 'api', dir: true, git: true },
      { name: 'ferme', dir: false, git: false },
    ],
  },
};

function server(more: Record<string, (args: any) => unknown> = {}) {
  return fakeBackend({ fs_browse: (a: { path?: string }) => FOLDERS[a.path ?? '/home/u'], ...more });
}

async function opened() {
  await userEvent.click(screen.getByRole('button', { name: 'Parcourir…' }));
  return screen.findByRole('group', { name: 'Dossiers du serveur' });
}

describe('ServerFolders', () => {
  beforeEach(() => resetApp());

  it('lists the home, goes down, up again, and chooses', async () => {
    const backend = server();
    const onpick = vi.fn();
    render(ServerFolders, { onpick });
    const box = await opened();
    expect(within(box).getByText('/home/u')).toBeInTheDocument();
    expect(within(box).getByRole('button', { name: 'Dossier parent' })).toBeDisabled();
    await userEvent.click(within(box).getByRole('button', { name: 'projets' }));
    await waitFor(() => expect(within(box).getByText('/home/u/projets')).toBeInTheDocument());
    // The keyboard's place kept: on the folder now shown, not lost on the page.
    await waitFor(() => expect(document.activeElement).toBe(within(box).getByText('/home/u/projets')));
    expect(backend.called('fs_browse').map((c) => c.args.path)).toEqual([undefined, '/home/u/projets']);
    // A repository is marked; a folder that cannot be read is shown, reached and told, and does not open.
    expect(within(box).getByRole('button', { name: /^api/ })).toHaveTextContent('git');
    const ferme = within(box).getByRole('button', { name: /^ferme/ });
    expect(ferme).toHaveAttribute('aria-disabled', 'true');
    expect(ferme).not.toBeDisabled();
    await userEvent.click(ferme);
    expect(backend.called('fs_browse')).toHaveLength(2);

    await userEvent.click(within(box).getByRole('button', { name: 'Dossier parent' }));
    await waitFor(() => expect(within(box).getByText('/home/u')).toBeInTheDocument());
    await userEvent.click(within(box).getByRole('button', { name: 'projets' }));
    await waitFor(() => expect(within(box).getByText('/home/u/projets')).toBeInTheDocument());
    await userEvent.click(within(box).getByRole('button', { name: 'Choisir ce dossier' }));
    expect(onpick).toHaveBeenCalledWith('/home/u/projets');
    // Chosen: the folders are put away.
    expect(screen.queryByRole('group', { name: 'Dossiers du serveur' })).not.toBeInTheDocument();
  });

  it('clones a repository into the folder shown, and chooses the clone', async () => {
    const backend = server({ fs_clone: () => '/home/u/demo' });
    const onpick = vi.fn();
    render(ServerFolders, { onpick });
    const box = await opened();
    await userEvent.type(within(box).getByRole('textbox', { name: 'Adresse du dépôt à cloner ici' }), '/srv/demo.git');
    await userEvent.click(within(box).getByRole('button', { name: 'Cloner ici' }));
    await waitFor(() => expect(onpick).toHaveBeenCalledWith('/home/u/demo'));
    expect(backend.called('fs_clone').map((c) => c.args)).toEqual([{ url: '/srv/demo.git', into: '/home/u' }]);
  });

  it('says when a folder holds more than is shown', async () => {
    server({
      fs_browse: () => ({ ...FOLDERS['/home/u'], truncated: true }),
    });
    render(ServerFolders, { onpick: vi.fn() });
    const box = await opened();
    expect(within(box).getByText('Dossier trop grand : seuls les premiers sont montrés')).toBeInTheDocument();
  });

  it('says a refusal, and keeps the folder shown', async () => {
    server({
      fs_browse: (a: { path?: string }) => {
        if (a.path) throw 'refusé';
        return FOLDERS['/home/u'];
      },
    });
    render(ServerFolders, { onpick: vi.fn() });
    const box = await opened();
    await userEvent.click(within(box).getByRole('button', { name: 'projets' }));
    expect(await within(box).findByRole('alert')).toHaveTextContent('refusé');
    expect(within(box).getByText('/home/u')).toBeInTheDocument();
  });
});
