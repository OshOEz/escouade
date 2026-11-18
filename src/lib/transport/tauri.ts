// Tauri's IPC, as the window always called it: a flow is a `Channel`, passed as a command's argument.

import { Channel, invoke } from '@tauri-apps/api/core';
import type { Transport } from './index';

export const transport: Transport = { invoke, channel: () => new Channel() };
