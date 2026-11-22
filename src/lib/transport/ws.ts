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

/** The wait before a new connection after a lost one: doubled at each failed try, up to `RETRY_MAX`. */
export const RETRY_FIRST = 500;
export const RETRY_MAX = 10_000;
/** How long a connection is held before the wait goes back to `RETRY_FIRST`: a server that greets then closes is no loop. */
export const RETRY_HELD = 5_000;

/**
 * The beat: a call every `BEAT_EVERY` on a greeted connection, unanswered within `BEAT_WAIT` closes it and the reconnection
 * starts. A network gone without a word (a laptop asleep, a NAT forgetting) closes nothing, and a page cannot send a Ping.
 * `data_dir` reads a path the core holds: no lock, nothing written.
 */
export const BEAT_EVERY = 30_000;
export const BEAT_WAIT = 10_000;
const BEAT_CMD = 'data_dir';

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

/**
 * The transport over the sockets `open` gives: one at a time, opened at the first call. A lost one is opened again by
 * itself, after a wait that grows at each failed try and is reset by a connection held `RETRY_HELD`; never again after a
 * server of another protocol (the page is older or newer than it).
 */
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
  const closeListeners = new Set<(reason: string, again: boolean) => void>();
  const reopenListeners = new Set<() => void>();
  /** A connection was lost since the last greeting: the next greeting is a reconnection. */
  let wasLost = false;
  let retryIn = RETRY_FIRST;
  let retry: ReturnType<typeof setTimeout> | undefined;
  let held: ReturnType<typeof setTimeout> | undefined;
  let beats: ReturnType<typeof setInterval> | undefined;
  /** Why no connection is tried any more: a server of another protocol. */
  let refused: string | null = null;

  /** `again`: a new connection is tried after the wait; else none ever (`refused`). */
  function lose(reason: string, again = true) {
    clearTimeout(held);
    clearInterval(beats);
    socket = null;
    ready = false;
    waiting = [];
    events = null;
    flows.clear();
    terminals.clear();
    const lost = [...calls.values()];
    calls.clear();
    for (const call of lost) call.reject(new Error(`${call.cmd}: ${reason}`));
    wasLost = true;
    if (!again) refused = reason;
    for (const listener of closeListeners) listener(reason, again);
    if (!again || retry !== undefined) return;
    retry = setTimeout(() => {
      retry = undefined;
      socket ??= connect();
    }, retryIn);
    retryIn = Math.min(retryIn * 2, RETRY_MAX);
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
        lose(`the server speaks protocol ${frame.hello.protocol}, this page protocol ${PROTOCOL}`, false);
        s.close();
        return;
      }
      ready = true;
      held = setTimeout(() => (retryIn = RETRY_FIRST), RETRY_HELD);
      beats = setInterval(() => beat(s), BEAT_EVERY);
      for (const request of waiting) s.send(request);
      waiting = [];
      if (wasLost) {
        wasLost = false;
        for (const listener of reopenListeners) listener();
      }
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

  function beat(s: Socket) {
    if (socket !== s) return;
    const late = setTimeout(() => {
      if (socket !== s) return;
      // Lost first: its `onclose`, which may come much later on a dead network, is then another socket's.
      lose(`the server did not answer within ${BEAT_WAIT / 1000} s`);
      s.close();
    }, BEAT_WAIT);
    const answered = () => clearTimeout(late);
    invoke(BEAT_CMD).then(answered, answered);
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

  function invoke<T>(cmd: string, args: Record<string, unknown> | Uint8Array = {}): Promise<T> {
    // Tauri's raw request (an attachment's bytes) has no place on this wire.
    if (args instanceof Uint8Array) return Promise.reject(new Error(`${cmd}: desktop: a raw request is the desktop app's alone`));
    if (refused) return Promise.reject(new Error(`${cmd}: ${refused}`));
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
  }

  return {
    invoke,
    channel: <T>(): Flow<T> => new StreamFlow<T>(nextStream++),
    /** `listener` is told of each lost connection, with why and whether another is tried: what a reconnection hooks to. */
    onClose(listener: (reason: string, again: boolean) => void): () => void {
      closeListeners.add(listener);
      return () => closeListeners.delete(listener);
    },
    /** `listener` is told when the server greets again after a lost connection: what subscribes again hooks to. */
    onReopen(listener: () => void): () => void {
      reopenListeners.add(listener);
      return () => reopenListeners.delete(listener);
    },
  } satisfies Transport & { onClose: unknown; onReopen: unknown };
}
