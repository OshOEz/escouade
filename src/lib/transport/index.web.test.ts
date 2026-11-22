// The page's transport in a browser: the device's token goes by the WebSocket's subprotocol, to the page's own host;
// a revoked device makes the page pair again.
import { afterEach, describe, expect, it, vi } from 'vitest';

const TOKEN = 'abcdefghijklmnopqrstuvwxyz0123456789-_ABCDE';

/** The sockets the page opened: their address and subprotocols, and their `onclose`. */
const opened: { url: string; protocols: string[]; socket: { onclose: (() => void) | null } }[] = [];
class FakeSocket {
  binaryType = '';
  onmessage = null;
  onclose: (() => void) | null = null;
  constructor(url: string, protocols: string[]) {
    opened.push({ url, protocols, socket: this });
  }
  send() {}
  close() {}
}

async function browserTransport() {
  delete (window as any).__TAURI_INTERNALS__;
  vi.resetModules();
  vi.stubGlobal('WebSocket', FakeSocket);
  return import('./index');
}

describe('the transport in a browser', () => {
  afterEach(() => {
    (window as any).__TAURI_INTERNALS__ = {};
    vi.unstubAllGlobals();
    localStorage.clear();
    opened.length = 0;
  });

  it('presents the device token by the subprotocol, to the page’s own host', async () => {
    localStorage.setItem('escouade-device', TOKEN);
    const { desktop, invoke } = await browserTransport();
    expect(desktop).toBe(false);
    void invoke('data_dir').catch(() => {});
    expect(opened).toHaveLength(1);
    expect(opened[0].url).toBe(`ws://${location.host}/`);
    expect(opened[0].protocols).toEqual(['escouade', `bearer.${TOKEN}`]);
    expect(opened[0].url).not.toContain(TOKEN);
    expect(document.cookie).toBe('');
  });

  it('pairs again once the server says the device is revoked', async () => {
    localStorage.setItem('escouade-device', TOKEN);
    const { invoke } = await browserTransport();
    const reload = vi.fn();
    vi.stubGlobal('location', { host: location.host, protocol: 'http:', reload });
    vi.stubGlobal('fetch', async () => new Response(null, { status: 401 }));
    void invoke('data_dir').catch(() => {});
    opened[0].socket.onclose!();
    await vi.waitFor(() => expect(reload).toHaveBeenCalled());
    expect(localStorage.getItem('escouade-device')).toBeNull();
  });
});
