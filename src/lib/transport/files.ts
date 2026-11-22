// The bytes of files between a browser and its server, beside the WebSocket (src-tauri/src/remoteapi/files.rs): a file
// sent as the body of a request, a file fetched as a download. The device's token goes by `Authorization` (./device.ts).

import { authorization, deviceToken } from './device';

export interface SendOptions {
  /** Aborting it stops the sending: the server keeps nothing of it. */
  signal?: AbortSignal;
  /** Bytes sent so far. */
  onProgress?: (done: number) => void;
}

const address = (path: string, params: Record<string, string>) => `${path}?${new URLSearchParams(params)}`;

/** Files sent at once from a page; the others wait their turn here (the server takes a few, all clients together). */
const AT_ONCE = 2;
let sending = 0;
const waiting: (() => void)[] = [];

/** `job` run when a place is free; its place handed to the next when it ends. */
function inTurn<T>(job: () => Promise<T>): Promise<T> {
  const run = () =>
    job().finally(() => {
      const next = waiting.shift();
      if (next) next();
      else sending--;
    });
  if (sending < AT_ONCE) {
    sending++;
    return run();
  }
  return new Promise<T>((resolve, reject) => waiting.push(() => run().then(resolve, reject)));
}

/** `body` sent to `path` (`POST`), its answer's JSON; rejects with the server's reason when it refuses. */
export function send<T>(path: string, params: Record<string, string>, body: Blob, { signal, onProgress }: SendOptions = {}): Promise<T> {
  return inTurn(
    () =>
      new Promise<T>((resolve, reject) => {
        if (signal?.aborted) return reject(new DOMException('Aborted', 'AbortError'));
        // Not `fetch`: it says nothing of the bytes sent so far.
        const xhr = new XMLHttpRequest();
        xhr.open('POST', address(path, params));
        for (const [k, v] of Object.entries(authorization(deviceToken() ?? ''))) xhr.setRequestHeader(k, v);
        xhr.upload.onprogress = (e) => onProgress?.(e.loaded);
        // All its bytes gone, the server may have kept it: its answer is waited for, for the copy to be removed by its
        // path (`copyToDisk`), rather than left until the purge.
        let whole = false;
        xhr.upload.onload = () => (whole = true);
        xhr.onload = () => {
          // A proxy may answer 200 with a page of its own: refused, not left pending with its turn held.
          try {
            if (xhr.status === 200) return resolve(JSON.parse(xhr.responseText) as T);
          } catch {
            return reject(new Error(`${path}: not an answer of the server`));
          }
          reject(new Error(xhr.responseText || `${path}: ${xhr.status}`));
        };
        xhr.onerror = () => reject(new Error(`${path}: the server is out of reach`));
        xhr.onabort = () => reject(new DOMException('Aborted', 'AbortError'));
        signal?.addEventListener('abort', () => whole || xhr.abort(), { once: true });
        xhr.send(body);
      }),
  );
}

/** `what` (a `Blob`, or an address the page may read: `data:` or `blob:`) saved by the browser as `name`. */
export function save(what: Blob | string, name: string) {
  const url = typeof what === 'string' ? what : URL.createObjectURL(what);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.click();
  if (typeof what !== 'string') setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

/** The file of `path` (`GET`) saved by the browser as `name`; rejects when the server refuses it. */
export async function fetchFile(path: string, params: Record<string, string>, name: string): Promise<void> {
  const r = await fetch(address(path, params), { headers: authorization(deviceToken() ?? ''), credentials: 'omit' });
  if (!r.ok) throw new Error((await r.text()) || `${path}: ${r.status}`);
  save(await r.blob(), name);
}
