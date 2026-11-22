// `escouade-server` for the web interface's tests: the core without a window, on a data folder of the test's, serving
// the page (dist-web) and its WebSocket on 127.0.0.1. Built first: `cargo build --no-default-features --features
// server --bin escouade-server` (src-tauri) and `npm run build:web`.
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import type { BrowserContext } from '@playwright/test';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const SERVER = process.env.ESCOUADE_SERVER ?? path.join(ROOT, 'src-tauri', 'target', 'debug', 'escouade-server');
const WEB = path.join(ROOT, 'dist-web');
/** The desktop e2e's fake Claude Code (`exec node`): its scenarios answer « Bonjour », « question »… */
const FAKE_CLAUDE = path.join(ROOT, 'tests', 'fixtures', 'fake-claude');

export interface Server {
  url: string;
  /** The folder of the test: the data folder, the home, the Git repository. */
  root: string;
  /** A Git repository with one commit on `main`, to make a project of. */
  repo: string;
  /** The proxy the browser goes through (`newContext({ proxy })`): what cuts its network (`cut`). */
  proxy: { server: string; bypass: string };
  /** The network of the page cut (its connections closed, none opened again) or given back. */
  cut(off: boolean): void;
  /** Gives the browser's context its session (`POST /session` with the token): its cookie opens the WebSocket. */
  signIn(context: BrowserContext): Promise<void>;
  /** What the server wrote: its log in the data folder. */
  log(): string;
  stop(): Promise<void>;
}

const git = (cwd: string, ...args: string[]) => execFileSync('git', args, { cwd, stdio: 'pipe' });

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = net.createServer();
    probe.once('error', reject);
    probe.listen(0, '127.0.0.1', () => {
      const { port } = probe.address() as net.AddressInfo;
      probe.close(() => resolve(port));
    });
  });
}

export async function startServer(): Promise<Server> {
  for (const [what, file] of [
    ['the server', SERVER],
    ['the page', path.join(WEB, 'index.html')],
  ])
    if (!fs.existsSync(file)) throw new Error(`build ${what} first (missing ${file}): see playwright.web.config.ts`);
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-web-'));
  const data = path.join(root, 'data');
  const home = path.join(root, 'home');
  fs.mkdirSync(data);
  fs.mkdirSync(home);
  // The questions of a first start answered, French, nothing sent or updated, the fake Claude Code as the machine's
  // account: as the desktop e2e start (tests/e2e/fixture.ts).
  fs.writeFileSync(
    path.join(data, 'settings.json'),
    JSON.stringify({
      language: 'fr',
      telemetry: false,
      nameAsked: true,
      agentMode: 'acceptEdits',
      accountsVersion: 1,
      autoUpdate: false,
      sound: false,
      osNotifications: false,
      midTurnDelaySecs: 0,
      claudePath: FAKE_CLAUDE,
      accounts: [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
    }),
  );
  const repo = path.join(root, 'demo-api');
  fs.mkdirSync(repo);
  git(repo, 'init', '-q', '-b', 'main');
  git(repo, 'config', 'user.email', 'e2e@test');
  git(repo, 'config', 'user.name', 'e2e');
  fs.writeFileSync(path.join(repo, 'README.md'), '# demo\n');
  git(repo, 'add', '-A');
  git(repo, 'commit', '-qm', 'init');

  const port = await freePort();
  const token = randomBytes(24).toString('hex');
  const child: ChildProcess = spawn(SERVER, ['--web', WEB], {
    env: {
      ...process.env,
      HOME: home,
      ESCOUADE_DATA_DIR: data,
      ESCOUADE_REMOTE_PORT: String(port),
      ESCOUADE_REMOTE_TOKEN: token,
      // Never the machine's Claude Code: the fake one, its folders the test's.
      CLAUDE_CONFIG_DIR: path.join(root, 'claude'),
      FAKE_CLAUDE_LOG: path.join(root, 'fake-claude.jsonl'),
      FAKE_CLAUDE_HOME: path.join(root, 'fake-home'),
      SHELL: '/bin/bash',
    },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  let stderr = '';
  child.stderr!.on('data', (d) => (stderr += d));
  const url = `http://127.0.0.1:${port}`;
  const until = Date.now() + 30_000;
  for (;;) {
    if (child.exitCode !== null) throw new Error(`escouade-server ended (${child.exitCode}): ${stderr}`);
    const up = await fetch(`${url}/`).then(
      (r) => r.ok,
      () => false,
    );
    if (up) break;
    if (Date.now() > until) throw new Error(`escouade-server does not answer on ${url} after 30 s: ${stderr}`);
    await new Promise((r) => setTimeout(r, 200));
  }
  const network = await proxyTo(port);
  return {
    url,
    root,
    repo,
    proxy: { server: `http://127.0.0.1:${network.port}`, bypass: '<-loopback>' },
    cut: network.cut,
    async signIn(context) {
      const r = await context.request.post(`${url}/session`, { data: token, headers: { 'content-type': 'text/plain' } });
      if (r.status() !== 204) throw new Error(`POST /session: ${r.status()}`);
    },
    log: () => fs.readFileSync(path.join(data, 'app.log'), 'utf8'),
    async stop() {
      if (child.exitCode === null) {
        const ended = new Promise((r) => child.once('exit', r));
        child.kill('SIGTERM');
        await ended;
      }
      network.close();
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}

/**
 * A proxy in front of the server, the page's network: what it asks goes through (its requests, and its WebSocket by
 * `CONNECT`); cut, its open connections are closed and new ones refused, as when the network goes away.
 */
async function proxyTo(port: number) {
  let off = false;
  const open = new Set<net.Socket>();
  const keep = (s: net.Socket) => {
    open.add(s);
    s.on('close', () => open.delete(s));
    s.on('error', () => s.destroy());
  };
  const proxy = http.createServer((req, res) => {
    if (off) return void res.socket?.destroy();
    const up = http.request(req.url!, { method: req.method, headers: req.headers }, (r) => {
      res.writeHead(r.statusCode!, r.headers);
      r.pipe(res);
    });
    up.on('error', () => res.socket?.destroy());
    req.pipe(up);
  });
  proxy.on('connection', keep);
  proxy.on('connect', (_req, client: net.Socket, head: Buffer) => {
    if (off) return void client.destroy();
    const up = net.connect(port, '127.0.0.1', () => {
      client.write('HTTP/1.1 200 Connection Established\r\n\r\n');
      up.write(head);
      up.pipe(client);
      client.pipe(up);
    });
    keep(up);
    up.on('close', () => client.destroy());
    client.on('close', () => up.destroy());
  });
  await new Promise<void>((resolve) => proxy.listen(0, '127.0.0.1', resolve));
  return {
    port: (proxy.address() as net.AddressInfo).port,
    cut(cutOff: boolean) {
      off = cutOff;
      if (off) for (const s of open) s.destroy();
    },
    close() {
      for (const s of open) s.destroy();
      proxy.close();
    },
  };
}
