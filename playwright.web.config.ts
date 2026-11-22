import { defineConfig } from '@playwright/test';

// The web interface in a browser, against `escouade-server` (npm run test:web): the server built without the desktop
// (`cargo build --no-default-features --features server --bin escouade-server`) and the page (`npm run build:web`) first; tests/web/server.ts.
export default defineConfig({
  testDir: 'tests/web',
  testMatch: '**/*.spec.ts',
  timeout: 120_000,
  expect: { timeout: 15_000 },
  workers: 1,
  retries: 0,
  reporter: process.env.CI ? [['github'], ['list']] : 'list',
  use: {
    browserName: 'chromium',
    // Chrome for Testing itself, not its headless shell.
    channel: 'chromium',
    // No WebGL: xterm.js then draws its rows in the page, where the test reads them (its WebGL canvas holds no text).
    launchOptions: { args: ['--disable-webgl'] },
    trace: 'retain-on-failure',
  },
});
