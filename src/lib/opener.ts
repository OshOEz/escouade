import { t } from './i18n';
import { openUrl as inTab } from './platform';
import { errorText } from './refusal';
import { app } from './state.svelte';
import { desktop, invoke } from './transport';

/**
 * Opening outside Escouade, by the core (`outside.rs`): a link in the system's browser, a file in
 * its program, a folder in the file manager; Windows' under WSLg, where `xdg-open` usually has
 * nothing to open with. In the place of Tauri's opener, which goes to `xdg-open` there. A browser opens a link itself.
 */
export function openUrl(url: string | URL): Promise<void> {
  if (desktop) return invoke<void>('open_url', { url: String(url) });
  // A browser blocks, without a word, a tab opened with no click of the user's (a launch's page, the core's `openUrl`):
  // a toast's button gives it one.
  if (navigator.userActivation?.isActive === false) {
    app.toast(t('shell.app.pageToOpen', { url: String(url) }), 'info', {
      label: t('common.open'),
      onClick: () => void inTab(url).catch((e) => app.toast(errorText(e), 'error')),
    });
    return Promise.resolve();
  }
  return inTab(url);
}

/** A file in its program, or a folder in the file manager (see `openUrl`). */
export const openPath = (path: string): Promise<void> => invoke<void>('open_path', { path });
