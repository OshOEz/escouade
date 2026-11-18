// The core's remote API over a WebSocket, as its server speaks it (src-tauri/src/remoteapi/wire.rs): a greeting, then
// requests `{id, cmd, args, stream?}` answered by `{id, ok}` or `{id, err: {kind, message}}`, the events `{event}` after
// `subscribe`, and a terminal's bytes in binary frames under their stream number (4 bytes, big-endian).

import type { Flow, Transport } from './index';

/** The path the server upgrades to its WebSocket, on the page's own host. */
export const PATH = '/';

/** The version of the wire this page speaks; the server's greeting says its own. */
export const PROTOCOL = 1;

/** The largest request the server reads (`MAX_REQUEST`, server.rs): a bigger one makes it close the connection. */
export const MAX_REQUEST = 2 * 1024 * 1024;

/** What the transport uses of a WebSocket: the browser's, or a fake one in the tests. */
export type Socket = Pick<WebSocket, 'binaryType' | 'send' | 'close' | 'onmessage' | 'onclose'>;

/** A flow of the core in the browser: its number goes in the request in the place of the argument. */
class StreamFlow<T> implements Flow<T> {
  onmessage: (message: T) => void = () => {};
  constructor(readonly stream: number) {}
}

interface Call {
  cmd: string;
  flow?: StreamFlow<unknown>;
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
}

/** The transport over the sockets `open` gives: one at a time, opened at the first call after none or a lost one. */
export function wsTransport(open: () => Socket) {
  let socket: Socket | null = null;
  /** The greeting was read: the requests go at once, else they wait for it in their order. */
  let ready = false;
  let waiting: string[] = [];
  const calls = new Map<number, Call>();
  const flows = new Map<number, StreamFlow<unknown>>();
  /** The stream of each terminal by its id (the `TermInfo` its call gave), let go at its `terminalExit`. */
  const terminals = new Map<string, number>();
  let events: StreamFlow<unknown> | null = null;
  let nextId = 1;
  let nextStream = 1;
  const closeListeners = new Set<(reason: string) => void>();

  function lose(reason: string) {
    socket = null;
    ready = false;
    waiting = [];
    events = null;
    flows.clear();
    terminals.clear();
    const lost = [...calls.values()];
    calls.clear();
    for (const call of lost) call.reject(new Error(`${call.cmd}: ${reason}`));
    for (const listener of closeListeners) listener(reason);
  }

  function receive(s: Socket, data: string | ArrayBuffer) {
    if (typeof data !== 'string') {
      // One copy: the bytes without their stream number, an ArrayBuffer as Tauri's channel gives them in the window.
      flows.get(new DataView(data).getUint32(0))?.onmessage(data.slice(4));
      return;
    }
    const frame = JSON.parse(data);
    if ('hello' in frame) {
      if (frame.hello.protocol !== PROTOCOL) {
        lose(`the server speaks protocol ${frame.hello.protocol}, this page protocol ${PROTOCOL}`);
        s.close();
        return;
      }
      ready = true;
      for (const request of waiting) s.send(request);
      waiting = [];
    } else if ('event' in frame) {
      if (frame.event?.type === 'terminalExit' && terminals.has(frame.event.id)) {
        flows.delete(terminals.get(frame.event.id)!);
        terminals.delete(frame.event.id);
      }
      events?.onmessage(frame.event);
    } else {
      const call = calls.get(frame.id);
      if (!call) return;
      calls.delete(frame.id);
      if ('ok' in frame) {
        if (call.flow && call.cmd !== 'subscribe' && typeof frame.ok?.id === 'string') terminals.set(frame.ok.id, call.flow.stream);
        return call.resolve(frame.ok);
      }
      if (call.flow) flows.delete(call.flow.stream);
      if (call.flow && events === call.flow) events = null;
      const { kind, message } = frame.err;
      // `failed` is the command's own error, as Tauri rejects with it in the window: given as it is.
      if (kind === 'failed') call.reject(message);
      else call.reject(new Error(`${call.cmd}: ${kind}: ${typeof message === 'string' ? message : JSON.stringify(message)}`));
    }
  }

  function connect(): Socket {
    const s = open();
    s.binaryType = 'arraybuffer';
    s.onmessage = (e) => {
      if (socket === s) receive(s, e.data);
    };
    s.onclose = () => {
      if (socket === s) lose('the connection to the server was lost');
    };
    return s;
  }

  return {
    invoke<T>(cmd: string, args: Record<string, unknown> | Uint8Array = {}): Promise<T> {
      // Tauri's raw request (an attachment's bytes) has no place on this wire.
      if (args instanceof Uint8Array) return Promise.reject(new Error(`${cmd}: desktop: a raw request is the desktop app's alone`));
      return new Promise<T>((resolve, reject) => {
        const id = nextId++;
        let flow: StreamFlow<unknown> | undefined;
        const rest: Record<string, unknown> = {};
        for (const [key, value] of Object.entries(args)) {
          if (value instanceof StreamFlow) flow = value;
          else rest[key] = value;
        }
        const request = JSON.stringify(flow ? { id, cmd, args: rest, stream: flow.stream } : { id, cmd, args: rest });
        // Refused here rather than by the server, which would close the connection and lose every call and terminal.
        // A UTF-16 unit is 3 bytes at most: only a long request is measured.
        const size = request.length > MAX_REQUEST / 3 ? new TextEncoder().encode(request).length : 0;
        if (size > MAX_REQUEST) return reject(new Error(`${cmd}: args: too large for the server (${size} bytes, ${MAX_REQUEST} at most)`));
        socket ??= connect();
        if (flow && cmd === 'subscribe') events = flow;
        else if (flow) flows.set(flow.stream, flow);
        calls.set(id, { cmd, flow, resolve: resolve as (value: unknown) => void, reject });
        if (ready) socket.send(request);
        else waiting.push(request);
      });
    },
    channel: <T>(): Flow<T> => new StreamFlow<T>(nextStream++),
    /** `listener` is told of each lost connection, with why: what a reconnection hooks to. */
    onClose(listener: (reason: string) => void): () => void {
      closeListeners.add(listener);
      return () => closeListeners.delete(listener);
    },
  } satisfies Transport & { onClose: unknown };
}
