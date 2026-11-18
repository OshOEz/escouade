import { clearMocks, mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setLang } from './i18n';
import { keyLabel, osOf, primaryKey } from './platform';

const key = (init: KeyboardEventInit) => new KeyboardEvent('keydown', init);

describe('platform', () => {
  it('tells the system from what the WebView says of it', () => {
    // WebView2, WKWebView, WebKitGTK.
    expect(osOf('Win32')).toBe('windows');
    expect(osOf('MacIntel')).toBe('mac');
    expect(osOf('Linux x86_64')).toBe('linux');
    // Without a platform, the user agent: the « darwin » jsdom says under macOS is no Windows.
    expect(osOf('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36')).toBe('windows');
    expect(osOf('Mozilla/5.0 (darwin) AppleWebKit/537.36 (KHTML, like Gecko) jsdom/26.1.0')).not.toBe('windows');
    expect(osOf('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15')).toBe('mac');
    expect(osOf('Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15')).toBe('linux');
  });

  it('uses Cmd on macOS and Ctrl elsewhere', () => {
    expect(primaryKey(key({ key: 'j', metaKey: true }), true)).toBe(true);
    expect(primaryKey(key({ key: 'j', ctrlKey: true }), true)).toBe(false);
    expect(primaryKey(key({ key: 'j', ctrlKey: true }), false)).toBe(true);
    expect(primaryKey(key({ key: 'j', metaKey: true }), false)).toBe(false);
  });

  it('writes shortcuts the macOS way', () => {
    expect(keyLabel('Ctrl+J', true)).toBe('⌘J');
    expect(keyLabel('Ctrl+Shift+F', true)).toBe('⇧⌘F');
    expect(keyLabel('Ctrl+Maj+L', true)).toBe('⇧⌘L');
    expect(keyLabel('Ctrl+Tab', true)).toBe('Ctrl+Tab');
    expect(keyLabel('Ctrl+J', false)).toBe('Ctrl+J');
  });

  it('names the keys in French, whichever way the shortcut is written', () => {
    expect(keyLabel('Ctrl+Shift+F', false)).toBe('Ctrl+Maj+F');
    expect(keyLabel('Ctrl+Enter', false)).toBe('Ctrl+Entrée');
    expect(keyLabel('Ctrl+Maj+Entrée', false)).toBe('Ctrl+Maj+Entrée');
    expect(keyLabel('Esc', false)).toBe('Échap');
    expect(keyLabel('Ctrl+Entrée', true)).toBe('⌘Entrée');
    expect(keyLabel('Ctrl+,', false)).toBe('Ctrl+,');
  });

  it('names the keys in English once the interface is', () => {
    setLang('en');
    expect(keyLabel('Ctrl+Maj+F', false)).toBe('Ctrl+Shift+F');
    expect(keyLabel('Ctrl+Entrée', false)).toBe('Ctrl+Enter');
    expect(keyLabel('Échap', false)).toBe('Esc');
    expect(keyLabel('Ctrl+Maj+Entrée', true)).toBe('⇧⌘Enter');
    expect(keyLabel('Alt+1', false)).toBe('Alt+1');
  });
});

/** The module anew, in the window of the desktop app or in a browser: the façade chooses as it is loaded. */
async function load(desktop: boolean) {
  const w = window as { __TAURI_INTERNALS__?: unknown };
  const internals = w.__TAURI_INTERNALS__;
  if (!desktop) delete w.__TAURI_INTERNALS__;
  vi.resetModules();
  try {
    return await import('./platform');
  } finally {
    w.__TAURI_INTERNALS__ = internals;
  }
}

describe('what the interface asks of the system', () => {
  afterEach(() => clearMocks());

  it('in a browser: no dialog, no window to drive, a link in a tab of its own', async () => {
    const platform = await load(false);
    expect(platform.desktop).toBe(false);
    const opened = vi.spyOn(window, 'open').mockReturnValue(null);
    await platform.openUrl('https://example.com/a');
    expect(opened).toHaveBeenCalledWith('https://example.com/a', '_blank', 'noopener');
    // The core's rule: http, https, mailto and tel alone, refused with its words.
    for (const ok of ['HTTP://example.com', 'mailto:a@b.c', 'tel:+33100000000']) await platform.openUrl(ok);
    for (const [bad, scheme] of [
      ['javascript:alert(1)', 'javascript'],
      ['file:///etc/passwd', 'file'],
      ['vscode://file/x', 'vscode'],
    ])
      await expect(platform.openUrl(bad)).rejects.toBe(`not a link Escouade opens: ${scheme}:`);
    expect(opened).toHaveBeenCalledTimes(4);
    expect(await platform.open({ directory: true, multiple: false })).toBeNull();
    const win = platform.getCurrentWindow();
    expect(await win.isMaximized()).toBe(false);
    (await win.onResized(() => {}))();
    await win.minimize();
    await win.toggleMaximize();
    await win.hide();
    await win.setTheme('dark');
    await platform.getCurrentWebview().setZoom(1.25);
    (await platform.listen('devpages', () => {}))();
  });

  it('in the window: each one is Tauri’s, called with the same arguments', async () => {
    const platform = await load(true);
    expect(platform.desktop).toBe(true);
    const calls: [string, unknown][] = [];
    mockWindows('main');
    mockIPC((cmd, args) => void calls.push([cmd, args]));
    const { open } = await import('@tauri-apps/plugin-dialog');
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');
    const { listen } = await import('@tauri-apps/api/event');
    const run = async (f: {
      open: typeof open;
      openUrl: typeof openUrl;
      getCurrentWindow: typeof getCurrentWindow;
      getCurrentWebview: typeof getCurrentWebview;
      listen: typeof listen;
    }) => {
      await f.open({ directory: true, multiple: false, title: 'Dossier' });
      await f.openUrl('https://example.com/a');
      await f.getCurrentWindow().minimize();
      await f.getCurrentWindow().setTheme('dark');
      await f.getCurrentWebview().setZoom(1.25);
      await f.listen('devpages', () => {});
      return calls.splice(0).map(([cmd, args]) => [cmd, JSON.stringify(args).replace(/"handler":\d+/, '')]);
    };
    const theirs = await run({ open, openUrl, getCurrentWindow, getCurrentWebview, listen });
    expect(await run(platform)).toEqual(theirs);
    expect(theirs.map(([cmd]) => cmd)).toEqual([
      'plugin:dialog|open',
      'plugin:opener|open_url',
      'plugin:window|minimize',
      'plugin:window|set_theme',
      'plugin:webview|set_webview_zoom',
      'plugin:event|listen',
    ]);
  });
});
