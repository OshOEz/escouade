// `escouade-server` for the web interface's tests: the core without a window, on a data folder of the test's, serving
// the page (dist-web) and its WebSocket on 127.0.0.1. Built first: `cargo build --no-default-features --features
// server --bin escouade-server` (src-tauri) and `npm run build:web`.
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import type { Page } from '@playwright/test';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const SERVER = process.env.ESCOUADE_SERVER ?? path.join(ROOT, 'src-tauri', 'target', 'debug', 'escouade-server');
const WEB = path.join(ROOT, 'dist-web');
/** The desktop e2e's fake Claude Code (`exec node`): its scenarios answer « Bonjour », « question »…; alone, a sign-in. */
const FAKE_CLAUDE = path.join(ROOT, 'tests', 'web', 'fake-claude');

export interface Server {
  url: string;
  /** The folder of the test: the data folder, the home, the Git repository. */
  root: string;
  /** A Git repository with one commit on `main`, to make a project of. */
  repo: string;
  /** The home of the server's user, the folders the page browses: `projets/site` in it, a repository like `repo`. */
  home: string;
  /** The proxy the browser goes through (`newContext({ proxy })`): what cuts its network (`cut`). */
  proxy: { server: string; bypass: string };
  /** The network of the page cut (its connections closed, none opened again) or given back. */
  cut(off: boolean): void;
  /** Pairs the page's browser as a user does: `escouade-server pair` on the server, its code typed in the page. */
  pair(page: Page): Promise<void>;
  /** `escouade-server <args>` run on the server, as its user: what it printed. */
  cli(...args: string[]): string;
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

export interface Options {
  /** Served on a Unix socket, as the systemd unit does, reached by a tunnel to it (`ssh -L`): the page's port is the tunnel's. */
  socket?: boolean;
  /** The accounts of its settings: the machine's Claude Code (the fake one) by default; `[]`, none yet. */
  accounts?: unknown[];
}

export async function startServer(options: Options = {}): Promise<Server> {
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
      accounts: options.accounts ?? [{ id: 'principal', name: 'Principal', engine: 'claude', configDir: '', claudePath: '', active: true }],
    }),
  );
  // In the home: a client's project folder stays there (the server's bound, as its browser of folders).
  const repo = path.join(home, 'demo-api');
  const site = path.join(home, 'projets', 'site');
  for (const dir of [repo, site]) {
    fs.mkdirSync(dir, { recursive: true });
    git(dir, 'init', '-q', '-b', 'main');
    git(dir, 'config', 'user.email', 'e2e@test');
    git(dir, 'config', 'user.name', 'e2e');
    fs.writeFileSync(path.join(dir, 'README.md'), '# demo\n');
    git(dir, 'add', '-A');
    git(dir, 'commit', '-qm', 'init');
  }

  // On a socket, in a folder of the user's alone (as $XDG_RUNTIME_DIR): its tunnel listens on the page's port.
  const socket = options.socket ? path.join(root, 'run', 'server.sock') : null;
  if (socket) fs.mkdirSync(path.dirname(socket), { mode: 0o700 });
  const tunnel = socket ? await tunnelTo(socket) : null;
  const port = tunnel?.port ?? (await freePort());
  // No ESCOUADE_REMOTE_TOKEN: the devices paired by a code alone, as on a server.
  const { ESCOUADE_REMOTE_TOKEN: _, ESCOUADE_REMOTE_PORT: __, ...inherited } = process.env;
  const env = { ...inherited, HOME: home, ESCOUADE_DATA_DIR: data };
  const where = socket ? ['--socket', socket] : ['--port', String(port)];
  const child: ChildProcess = spawn(SERVER, [...where, '--web', WEB], {
    env: {
      ...env,
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
  const cli = (...args: string[]) => execFileSync(SERVER, args, { env, encoding: 'utf8' });
  return {
    url,
    root,
    repo,
    home,
    proxy: { server: `http://127.0.0.1:${network.port}`, bypass: '<-loopback>' },
    cut: network.cut,
    cli,
    async pair(page) {
      const out = cli('pair');
      const code = /\b[0-9A-Z]{4}-[0-9A-Z]{4}\b/.exec(out)?.[0];
      // On the socket, the tunnel it tells to open: `ssh -N -L 8765:<socket> <this server>`.
      if (!code || !out.includes(socket ? `:${socket} ` : `${url}/`)) throw new Error(`escouade-server pair: ${out}`);
      await page.goto(url);
      await page.getByLabel('Code d’appairage').fill(code);
      await page.getByLabel('Nom de cet appareil').fill('Chrome des tests');
      await page.getByRole('button', { name: 'Appairer' }).click();
      await page.waitForURL(url + '/');
    },
    log: () => fs.readFileSync(path.join(data, 'app.log'), 'utf8'),
    async stop() {
      if (child.exitCode === null) {
        const ended = new Promise((r) => child.once('exit', r));
        child.kill('SIGTERM');
        await ended;
      }
      network.close();
      tunnel?.close();
      fs.rmSync(root, { recursive: true, force: true });
    },
  };
}

/** What `ssh -L <port>:<socket>` does on the user's computer: a port of 127.0.0.1 whose connections go to the socket. */
async function tunnelTo(socket: string) {
  const open = new Set<net.Socket>();
  const tunnel = net.createServer((client) => {
    const up = net.connect(socket);
    for (const s of [client, up]) {
      open.add(s);
      s.on('close', () => open.delete(s));
      s.on('error', () => s.destroy());
    }
    client.pipe(up);
    up.pipe(client);
    up.on('close', () => client.destroy());
    client.on('close', () => up.destroy());
  });
  await new Promise<void>((resolve) => tunnel.listen(0, '127.0.0.1', resolve));
  return {
    port: (tunnel.address() as net.AddressInfo).port,
    close() {
      for (const s of open) s.destroy();
      tunnel.close();
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
