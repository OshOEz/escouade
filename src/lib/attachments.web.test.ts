// The files of a message in a browser: sent whole to its server, which keeps the copy; a message carries what its
// server reads of a request, the rest goes by its path.
import { describe, expect, it, vi } from 'vitest';
import { fakeBackend } from '../test/ipc';
import { MAX_TOTAL, copyToDisk, planAttachment } from './attachments';
import type { AttachedFile } from './types';

const send = vi.hoisted(() => vi.fn());
vi.mock('./transport', async (original) => ({ ...(await original<typeof import('./transport')>()), desktop: false }));
vi.mock('./transport/files', () => ({ send }));

const MB = 1024 * 1024;
const copy: AttachedFile = {
  name: 'a.zip',
  path: '/srv/data/a1/1f2e/a.zip',
  claudePath: '/srv/data/a1/1f2e/a.zip',
  size: 3,
  origin: false,
};
const settings = { attachMaxMb: 0, attachWarnMb: 0, attachBigText: 'path' } as const;

describe('in a browser', () => {
  it('sends a file whole to its server, never by the chunks of the window', async () => {
    const backend = fakeBackend({});
    send.mockResolvedValueOnce(copy);
    const f = new File(['abc'], 'a.zip');
    const progress = () => {};
    expect(await copyToDisk(f, 'a1', { onProgress: progress })).toEqual(copy);
    expect(send).toHaveBeenCalledWith('/files/attach', { agent: 'a1', name: 'a.zip' }, f, { signal: undefined, onProgress: progress });
    expect(backend.calls.map((c) => c.cmd)).toEqual([]);
  });

  it('removes a copy cancelled as it was finishing', async () => {
    const backend = fakeBackend({ attach_discard: () => null });
    const stop = new AbortController();
    send.mockImplementationOnce(async () => (stop.abort(), copy));
    await expect(copyToDisk(new File(['abc'], 'a.zip'), 'a1', { signal: stop.signal })).rejects.toThrow('Aborted');
    expect(backend.calls).toEqual([{ cmd: 'attach_discard', args: { owner: 'a1', path: copy.path } }]);
  });

  it('keeps a message under what its server reads of a request: a big image goes by its path', () => {
    expect(MAX_TOTAL).toBe(MB);
    const image = (size: number) => {
      const f = new File(['x'], 'capture.png', { type: 'image/png' });
      Object.defineProperty(f, 'size', { value: size });
      return f;
    };
    expect(planAttachment(image(MB / 2), { settings, messageBytes: 0 })).toEqual({ kind: 'image', warn: false });
    expect(planAttachment(image(MB / 2), { settings, messageBytes: MB / 2 + 1 })).toEqual({ kind: 'file', warn: false });
    expect(planAttachment(image(3 * MB), { settings, messageBytes: 0 })).toEqual({ kind: 'file', warn: false });
  });
});
