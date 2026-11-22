// The device's token in a browser: kept by the page, presented by the subprotocol and `Authorization`, never a cookie.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { authorization, deviceToken, pair, protocols, stillPaired } from './device';

const TOKEN = 'abcdefghijklmnopqrstuvwxyz0123456789-_ABCDE';

describe('the device token', () => {
  beforeEach(() => localStorage.clear());
  afterEach(() => vi.unstubAllGlobals());

  it('is kept by the page once paired, and presented without a cookie', async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ token: TOKEN, id: 'a1b2c3d4' }), { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    expect(deviceToken()).toBeNull();
    expect(await pair('ABCD-EFGH', 'Portable')).toBe(true);
    expect(deviceToken()).toBe(TOKEN);
    const [url, init] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe('/pair');
    expect(JSON.parse(init.body as string)).toEqual({ code: 'ABCD-EFGH', name: 'Portable' });
    expect(init.credentials).toBe('omit');
    expect(document.cookie).toBe('');
    expect(protocols(TOKEN)).toEqual(['escouade', `bearer.${TOKEN}`]);
    expect(authorization(TOKEN)).toEqual({ Authorization: `Bearer ${TOKEN}` });
  });

  it('is not kept when the code is refused', async () => {
    vi.stubGlobal('fetch', async () => new Response(null, { status: 403 }));
    expect(await pair('ABCD-EFGH', 'Portable')).toBe(false);
    expect(deviceToken()).toBeNull();
    vi.stubGlobal('fetch', async () => new Response(null, { status: 500 }));
    await expect(pair('ABCD-EFGH', 'Portable')).rejects.toThrow('500');
    expect(deviceToken()).toBeNull();
  });

  it('is forgotten once the server says the device is revoked, kept while it is out of reach', async () => {
    localStorage.setItem('escouade-device', TOKEN);
    const fetch = vi.fn(async () => new Response(null, { status: 204 }));
    vi.stubGlobal('fetch', fetch);
    expect(await stillPaired()).toBe(true);
    expect(fetch).toHaveBeenCalledWith('/device', { headers: { Authorization: `Bearer ${TOKEN}` }, credentials: 'omit' });
    vi.stubGlobal('fetch', async () => Promise.reject(new TypeError('network')));
    expect(await stillPaired()).toBe(true);
    expect(deviceToken()).toBe(TOKEN);
    vi.stubGlobal('fetch', async () => new Response(null, { status: 401 }));
    expect(await stillPaired()).toBe(false);
    expect(deviceToken()).toBeNull();
  });
});
