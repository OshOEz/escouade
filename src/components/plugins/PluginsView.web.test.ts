// The view Plugins and the store in a browser: on the rail, installing from the store or a git repository; a folder of
// the system's dialog and the links `escouade://` stay the desktop app's.
import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../lib/platform', async (original) => ({ ...(await original<typeof import('../../lib/platform')>()), desktop: false }));

import { setLang } from '../../lib/i18n';
import { plugins } from '../../lib/plugins-view.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import ProjectRail from '../ProjectRail.svelte';
import PluginsView from './PluginsView.host.test.svelte';

beforeEach(() => {
  setLang('fr');
  resetApp({ projects: [project({ id: 'p1', name: 'demo-api' })] });
  plugins.list = [];
  plugins.page = { kind: 'installed' };
});

describe('the view Plugins in a browser', () => {
  it('is on the rail', async () => {
    fakeBackend();
    render(ProjectRail);
    screen.getByRole('button', { name: 'Plugins' }).click();
    expect(app.ui.view).toBe('plugins');
  });

  it('installs from a git repository; a folder of the system is the desktop app’s', async () => {
    fakeBackend({ plugins_list: () => [] });
    render(PluginsView);
    expect(await screen.findByRole('button', { name: 'Depuis un dépôt git…' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Depuis un dossier…' })).toBeNull();
    plugins.page = { kind: 'dev' };
    await vi.waitFor(() => expect(screen.queryByRole('button', { name: 'Charger un dossier…' })).toBeNull());
  });

  it('takes no links escouade://, which the desktop app alone is given', async () => {
    const backend = fakeBackend({ deep_links_take: () => [{ kind: 'plugin', id: 'dev.exemple.x' }] });
    await plugins.takeLinks();
    expect(backend.called('deep_links_take')).toEqual([]);
  });
});
