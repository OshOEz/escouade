// The bytes of files in a browser: sent as a request's body, fetched as a download, the device's token by `Authorization`.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fetchFile, send } from './files';

const TOKEN = 'abcdefghijklmnopqrstuvwxyz0123456789-_ABCDE';

/** The last request made, and what the test answers it. */
class FakeXhr {
  static last: FakeXhr;
  static all: FakeXhr[] = [];
  url = '';
  headers: Record<string, string> = {};
  body: unknown;
  status = 0;
  responseText = '';
  aborted = false;
  upload = {
    onprogress: null as ((e: { loaded: number }) => void) | null,
    onload: null as (() => void) | null,
  };
  onload: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onabort: (() => void) | null = null;
  constructor() {
    FakeXhr.last = this;
    FakeXhr.all.push(this);
  }
  open(method: string, url: string) {
    this.url = `${method} ${url}`;
  }
  setRequestHeader(k: string, v: string) {
    this.headers[k] = v;
  }
  send(body: unknown) {
    this.body = body;
  }
  abort() {
    this.aborted = true;
    this.onabort?.();
  }
  answer(status: number, text: string) {
    this.status = status;
    this.responseText = text;
    this.onload?.();
  }
}

describe('the files of a browser', () => {
  beforeEach(() => {
    localStorage.setItem('escouade-device', TOKEN);
    vi.stubGlobal('XMLHttpRequest', FakeXhr);
    FakeXhr.all = [];
  });
  afterEach(() => {
    localStorage.clear();
    vi.unstubAllGlobals();
  });

  it('sends a file as the body of a request, its name in the address, the token in its header', async () => {
    const file = new File(['hello'], 'à b&c.zip');
    const progress: number[] = [];
    const sent = send<{ path: string }>('/files/attach', { agent: 'draft', name: file.name }, file, {
      onProgress: (n) => progress.push(n),
    });
    const xhr = FakeXhr.last;
    expect(xhr.url).toBe('POST /files/attach?agent=draft&name=%C3%A0+b%26c.zip');
    expect(xhr.headers).toEqual({ Authorization: `Bearer ${TOKEN}` });
    expect(xhr.body).toBe(file);
    xhr.upload.onprogress!({ loaded: 3 });
    xhr.answer(200, '{"path":"/srv/a.zip"}');
    expect(await sent).toEqual({ path: '/srv/a.zip' });
    expect(progress).toEqual([3]);
  });

  it('says why the server refused it, and stops it when aborted', async () => {
    const refused = send('/files/attach', { agent: 'draft', name: 'a' }, new Blob(['x']));
    FakeXhr.last.answer(413, 'a dépasse 1 Mo');
    await expect(refused).rejects.toThrow('a dépasse 1 Mo');

    const stop = new AbortController();
    const cut = send('/files/attach', { agent: 'draft', name: 'a' }, new Blob(['x']), { signal: stop.signal });
    stop.abort();
    await expect(cut).rejects.toThrow('Aborted');
    expect(FakeXhr.last.aborted).toBe(true);
  });

  it('sends two files at a time from a page, the others waiting their turn', async () => {
    const sent = [1, 2, 3, 4, 5].map((i) => send<number>('/files/attach', { agent: 'draft', name: `${i}` }, new Blob(['x'])));
    const open = () => FakeXhr.all.filter((x) => x.body !== undefined && x.status === 0);
    expect(open()).toHaveLength(2);
    for (let done = 1; done <= 5; done++) {
      await vi.waitFor(() => expect(open().length).toBeGreaterThan(0));
      expect(open().length).toBeLessThanOrEqual(2);
      open()[0].answer(200, `${done}`);
    }
    expect((await Promise.all(sent)).sort()).toEqual([1, 2, 3, 4, 5]);
  });

  it('lets a file whose bytes all went get its answer when aborted then: the copy is known, to be removed', async () => {
    const stop = new AbortController();
    const late = send<{ path: string }>('/files/attach', { agent: 'draft', name: 'a' }, new Blob(['x']), { signal: stop.signal });
    const xhr = FakeXhr.last;
    xhr.upload.onload!();
    stop.abort();
    expect(xhr.aborted).toBe(false);
    xhr.answer(200, '{"path":"/srv/a"}');
    expect(await late).toEqual({ path: '/srv/a' });
  });

  it('fetches a file with the token and saves it under its name, never navigating to it', async () => {
    const fetch = vi.fn(async () => new Response('%PDF', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    // jsdom has neither.
    Object.assign(URL, { createObjectURL: () => 'blob:x', revokeObjectURL: () => {} });
    const clicked: { href: string; download: string }[] = [];
    const click = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
      clicked.push({ href: this.href, download: this.download });
    });
    await fetchFile('/files/sent', { agent: 'a1', item: 'toolu_1', index: '0' }, 'contrat.pdf');
    expect(fetch).toHaveBeenCalledWith('/files/sent?agent=a1&item=toolu_1&index=0', {
      headers: { Authorization: `Bearer ${TOKEN}` },
      credentials: 'omit',
    });
    expect(clicked).toEqual([{ href: 'blob:x', download: 'contrat.pdf' }]);

    vi.stubGlobal('fetch', async () => new Response(null, { status: 404 }));
    await expect(fetchFile('/files/sent', { agent: 'a1', item: 'x', index: '0' }, 'x')).rejects.toThrow('404');
    expect(clicked).toHaveLength(1);
    click.mockRestore();
  });
});
