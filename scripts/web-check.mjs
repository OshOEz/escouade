#!/usr/bin/env node
// Checks the interface built for a browser (npm run build:web): no code of Tauri in it, only the transport's question
// whether it runs in the desktop app's window (src/lib/transport).
//   node scripts/web-check.mjs [dist-web]
// Prints each problem as a workflow error and exits 1, or prints a summary and exits 0.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/** The transport's question, as written or minified. */
const PRESENCE = /["'`]__TAURI_INTERNALS__["'`]\s*in\s+window/g;
/** What Tauri's own code calls: its internals, and its plugins' commands. */
const TAURI = /__TAURI_INTERNALS__|plugin:[a-z-]+\|/g;

/** What is wrong with the built `files` (`{ name, text }`); empty when no code of Tauri is in them. */
export function problems(files) {
  const found = [];
  for (const { name, text } of files) {
    const left = new Set(text.replace(PRESENCE, '').match(TAURI) ?? []);
    for (const what of left) found.push(`${name} holds Tauri's ${what}`);
  }
  return found;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const dir = process.argv[2] ?? 'dist-web';
  const files = fs
    .readdirSync(dir, { recursive: true })
    .map(String)
    .filter((name) => /\.(js|html)$/.test(name))
    .map((name) => ({ name, text: fs.readFileSync(path.join(dir, name), 'utf8') }));
  const found = files.length ? problems(files) : [`${dir} holds no script: build it first (npm run build:web)`];
  for (const problem of found) console.log(`::error::${problem}`);
  if (found.length) process.exitCode = 1;
  else console.log(`${dir}: ${files.length} scripts, no code of Tauri`);
}
