// What the interface asks of the system in the desktop app's window: Tauri's own functions (see `platform.ts`).

import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';

export const system = { getCurrentWebview, getCurrentWindow, listen, open, openUrl };
