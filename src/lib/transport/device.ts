// This browser's device token (docs/REMOTE.md, « Le mode serveur »): given once by the server at the pairing
// (`POST /pair`), kept in this origin's storage (its port included: no other server of the machine reads it), and
// presented by the WebSocket's subprotocol and by `Authorization`. Never a cookie: a browser sends a cookie to every
// port of the host, whatever page asks.

const KEY = 'escouade-device';

/** The token, none before the pairing (or where the storage is refused). */
export function deviceToken(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

/** The WebSocket's subprotocols: the one the server answers, and the token's (`server.rs`, `SUBPROTOCOL`). */
export const protocols = (token: string): string[] => ['escouade', `bearer.${token}`];

/** The header that presents the token to the server's HTTP routes (a download asked by `fetch`). */
export const authorization = (token: string): Record<string, string> => ({ Authorization: `Bearer ${token}` });

/** Pairs this browser with `code` under `name`: false when the server refuses the code; throws when it is out of reach. */
export async function pair(code: string, name: string): Promise<boolean> {
  const r = await fetch('/pair', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ code, name }),
    credentials: 'omit',
  });
  if (r.status === 403) return false;
  if (!r.ok) throw new Error(`POST /pair: ${r.status}`);
  const { token } = (await r.json()) as { token: string };
  localStorage.setItem(KEY, token);
  return true;
}

/**
 * Whether the server still takes the token: false once the device is revoked, the token then forgotten (the page
 * pairs again). A server out of reach says nothing: true.
 */
export async function stillPaired(): Promise<boolean> {
  const token = deviceToken();
  if (!token) return false;
  try {
    const r = await fetch('/device', { headers: authorization(token), credentials: 'omit' });
    if (r.status !== 401) return true;
  } catch {
    return true;
  }
  try {
    localStorage.removeItem(KEY);
  } catch {
    // Nothing kept, nothing to forget.
  }
  return false;
}
