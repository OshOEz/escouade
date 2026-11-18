import { fileURLToPath } from 'node:url';
import { defineConfig, mergeConfig, normalizePath } from 'vite';
import desktop from './vite.config.js';

// The interface for a browser, served by the core's server (npm run build:web): the same page and components, without
// Tauri. Its two modules, read only in the desktop app's window (src/lib/transport, src/lib/platform.ts), are left out.
// Vite's ids are written with `/`, Windows' paths with `\`.
const own = (file: string) => normalizePath(fileURLToPath(new URL(file, import.meta.url)));
const WITHOUT_TAURI: Record<string, string> = {
  [own('./src/lib/transport/tauri.ts')]: 'export const transport = null;',
  [own('./src/lib/platform-tauri.ts')]: 'export const system = null;',
};

export default mergeConfig(
  desktop,
  defineConfig({
    plugins: [{ name: 'without-tauri', enforce: 'pre', load: (id: string) => WITHOUT_TAURI[id] }],
    build: { outDir: 'dist-web' },
  }),
);
