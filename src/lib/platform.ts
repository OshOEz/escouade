// What differs between Windows, macOS and Linux in the interface: the main modifier key, how
// shortcuts are written, the name of the file manager. And what the interface asks of the system, the same in the
// desktop app's window (Tauri's) and in a browser (docs/REMOTE.md, « La fondation commune », point 5).
import type { Webview } from '@tauri-apps/api/webview';
import type { Window } from '@tauri-apps/api/window';
import { t } from './i18n';
import { system } from './platform-tauri';
import { desktop } from './transport';

export { desktop };

const done = () => Promise.resolve();

/** A browser's tab has no window of the app's to drive: its controls do nothing, it is never maximized. */
const tab = {
  isMaximized: () => Promise.resolve(false),
  onResized: () => Promise.resolve(() => {}),
  minimize: done,
  toggleMaximize: done,
  hide: done,
  setTheme: done,
  innerSize: () => Promise.resolve({ width: 0, height: 0 }),
  scaleFactor: () => Promise.resolve(1),
} as unknown as Window;

/** The page's zoom is the browser's own. */
const page = { setZoom: done } as unknown as Webview;

/** A folder or a file chosen in the system's dialog; in a browser, none (no one chose). */
export const open: typeof system.open = (options) => (desktop ? system.open(options) : Promise.resolve(null as never));

/** A file of the device chosen in the browser's own dialog (`accept`: its filter); null when none was. */
export function pickFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.onchange = () => resolve(input.files?.[0] ?? null);
    input.oncancel = () => resolve(null);
    input.click();
  });
}

/** An address in the system's browser; in a browser, in a tab of its own. */
export const openUrl: typeof system.openUrl = (url, openWith) => {
  if (desktop) return system.openUrl(url, openWith);
  // The core's rule and refusal for `open_url` (commands.rs): http, https, mailto and tel alone.
  const scheme = String(url).split(':')[0].toLowerCase();
  if (!['http', 'https', 'mailto', 'tel'].includes(scheme)) return Promise.reject(`not a link Escouade opens: ${scheme}:`);
  window.open(String(url), '_blank', 'noopener');
  return done();
};

export const getCurrentWindow = (): Window => (desktop ? system.getCurrentWindow() : tab);

export const getCurrentWebview = (): Webview => (desktop ? system.getCurrentWebview() : page);

/** An event of the window (the « Pages » window's); a browser has none. */
export const listen: typeof system.listen = (event, handler, options) =>
  desktop ? system.listen(event, handler, options) : Promise.resolve(() => {});

/** A system the app runs on, for what the interface names after it (its file manager). */
export type Os = 'windows' | 'mac' | 'linux';

/** The system of what the WebView says of it: its `navigator.platform`, else its user agent. */
export function osOf(platform: string): Os {
  if (/Mac/i.test(platform)) return 'mac';
  // At the start of a word: « darwin » holds « win ».
  return /\bWin/i.test(platform) ? 'windows' : 'linux';
}

/** The system the app runs on. */
export const OS: Os = typeof navigator !== 'undefined' ? osOf(navigator.platform || navigator.userAgent) : 'windows';

/** True on macOS. */
export const IS_MAC = OS === 'mac';

/** True on Windows. */
export const IS_WINDOWS = OS === 'windows';

/** The app's shortcut modifier is held alone: Cmd on macOS, Ctrl elsewhere (for a key or a click). */
export function primaryKey(e: { ctrlKey: boolean; metaKey: boolean }, mac = IS_MAC): boolean {
  return mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
}

/** A key of a shortcut, named in the language of the interface: given in either (`Shift` or `Maj`, `Enter` or `Entrée`). */
function keyName(key: string): string {
  if (/^(Shift|Maj)$/.test(key)) return t('format.keys.shift');
  if (/^(Enter|Entrée)$/.test(key)) return t('format.keys.enter');
  if (/^(Esc|Escape|Échap)$/.test(key)) return t('format.keys.escape');
  return key;
}

/**
 * A shortcut as written on this system and in the language of the interface: `Ctrl+Shift+F` (or `Ctrl+Maj+F`) is
 * `Ctrl+Maj+F` in French, `Ctrl+Shift+F` in English, `⇧⌘F` on macOS. `Ctrl+Tab` stays as is there: Cmd+Tab
 * switches applications.
 */
export function keyLabel(shortcut: string, mac = IS_MAC): string {
  const keys = shortcut.split('+');
  if (!mac || keys[0] !== 'Ctrl' || /Tab$/.test(shortcut)) return keys.map(keyName).join('+');
  const shift = /^(Shift|Maj)$/.test(keys[1] ?? '');
  const rest = keys.slice(shift ? 2 : 1).map(keyName);
  return (shift ? '⇧⌘' : '⌘') + rest.join('+');
}
