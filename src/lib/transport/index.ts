// Every call to the core goes through here: Tauri's IPC in the desktop app's window, the core's WebSocket server in a
// browser (docs/REMOTE.md, « La fondation commune », point 5).

import * as tauri from './tauri';
import { PATH, wsTransport } from './ws';

export interface Transport {
  /** `args` raw (bytes) with `options` is Tauri's raw request, the desktop app's alone. */
  invoke<T>(cmd: string, args?: Record<string, unknown> | Uint8Array, options?: { headers: Record<string, string> }): Promise<T>;
  /** A flow from the core, passed as a command's argument: events (JSON) or a terminal's bytes (ArrayBuffer). */
  channel<T>(): Flow<T>;
}

export interface Flow<T> {
  onmessage: (message: T) => void;
}

/** True in the window of the desktop app (its IPC is there); false in a browser. */
export const desktop = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

const ws = desktop ? null : wsTransport(() => new WebSocket(`${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}${PATH}`));

export const transport: Transport = ws ?? tauri.transport;
export const invoke: Transport['invoke'] = transport.invoke;
export const channel = <T>(): Flow<T> => transport.channel<T>();

/** `listener` is told when the browser's connection to the core is lost; never in the window. */
export const onClose = (listener: (reason: string) => void): (() => void) => ws?.onClose(listener) ?? (() => {});
