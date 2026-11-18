// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { problems } from './web-check.mjs';

describe('the web build without Tauri', () => {
  it('lets the transport ask whether it is in the window', () => {
    expect(problems([{ name: 'assets/index.js', text: 'const d=typeof window<"u"&&"__TAURI_INTERNALS__"in window;' }])).toEqual([]);
    // As Vite minifies it.
    expect(problems([{ name: 'assets/index.js', text: 'var d=typeof window<`u`&&`__TAURI_INTERNALS__`in window,' }])).toEqual([]);
  });

  it('finds Tauri’s own code: its internals called, its plugins’ commands', () => {
    const found = problems([
      {
        name: 'assets/index.js',
        text: '"__TAURI_INTERNALS__"in window;async function i(c,a){return window.__TAURI_INTERNALS__.invoke(c,a)}',
      },
      { name: 'assets/dialog.js', text: 'i("plugin:dialog|open",{options:o})' },
      { name: 'assets/other.js', text: 'nothing of Tauri' },
    ]);
    expect(found).toHaveLength(2);
    expect(found[0]).toContain('assets/index.js');
    expect(found[1]).toContain('plugin:dialog|');
  });
});
