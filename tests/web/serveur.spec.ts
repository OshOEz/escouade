// What a user of `escouade-server` does from a browser, on a server installed as the systemd unit installs it (its Unix
// socket, reached by a tunnel): paired by a code, a Claude Code account signed in by its terminal, a project cloned, an
// agent that talks and asks, a terminal, the board, a file joined; the browser closed and opened again finds it all;
// the device revoked is cut off at once (within 2 s).
import fs from 'node:fs';
import path from 'node:path';
import { chromium, expect, test, type BrowserContext, type Page } from '@playwright/test';
import { startServer, type Server } from './server';

test.describe.configure({ mode: 'serial' });

let server: Server;
/** The browser's profile: what it keeps once closed (its storage, the device's token in it). */
let profile: string;
let browser: BrowserContext;
let page: Page;
const errors: string[] = [];

/** The browser opened on its profile, as playwright.web.config.ts launches it, its first tab watched. */
async function open() {
  browser = await chromium.launchPersistentContext(profile, {
    channel: 'chromium',
    args: ['--disable-webgl'],
    viewport: { width: 1400, height: 900 },
    proxy: server.proxy,
    locale: 'fr-FR',
  });
  page = browser.pages()[0] ?? (await browser.newPage());
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(m.text());
  });
  page.on('pageerror', (e) => errors.push(String(e)));
}

/** The row of the terminal shown that holds exactly `text` (xterm.js's DOM renderer, no WebGL). */
const row = (p: Page, text: string) => p.locator('main.tv .xterm-rows > div').filter({ hasText: new RegExp(`^\\s*${text}\\s*$`) });

test.beforeAll(async () => {
  // No account yet: the first thing the user does is sign one in.
  server = await startServer({ socket: true, accounts: [] });
  profile = path.join(server.root, 'profile');
  await open();
});

test.afterAll(async () => {
  await browser?.close();
  await server?.stop();
});

test('paired by a code through the tunnel to the socket', async () => {
  await page.goto(server.url);
  await expect(page.getByRole('heading', { name: 'Appairer ce navigateur' })).toBeVisible();
  await server.pair(page);
  await expect(page.getByRole('button', { name: /Ajouter un projet/ }).first()).toBeVisible();
  expect(server.cli('devices')).toContain('Chrome des tests');
  expect(errors).toEqual([]);
});

test('a Claude Code account signed in by its terminal: the address shown, the code pasted', async () => {
  await page.getByRole('button', { name: 'Réglages (Ctrl+,)' }).click();
  const settings = page.getByRole('dialog', { name: 'Réglages' });
  await settings.getByRole('navigation').getByRole('button', { name: 'Comptes et harness' }).click();
  await settings.getByRole('button', { name: 'Ajouter un compte Claude Code…' }).click();
  const add = page.getByRole('dialog', { name: 'Ajouter un compte Claude Code' });
  await add.getByRole('radio', { name: /Un nouveau compte/ }).check();
  await add.getByRole('textbox', { name: 'Nom' }).fill('Serveur');
  await add.getByRole('button', { name: 'Créer et se connecter' }).click();

  // `claude` in the page's terminal, on the server: its browser fails to open there, so it shows the address, long enough
  // to take several rows. A click on it opens it on this computer (not on the server), and the sign-in page gives the code.
  const signIn = page.getByRole('dialog', { name: 'Se connecter au compte « Serveur »' });
  const rows = signIn.locator('.xterm-rows');
  await expect(rows).toContainText('Paste code here if prompted');
  const address = rows.locator('> div').filter({ hasText: /^https:\/\/claude\.ai\/oauth\/authorize\?/ });
  // The address goes on over the next rows (a real one is some 400 characters).
  const lines = (await rows.locator('> div').allTextContents()).map((l) => l.trim());
  const at = lines.findIndex((l) => l.startsWith('https://claude.ai/oauth/authorize?'));
  expect(lines[at + 1], lines.join('\n')).toMatch(/^[\w%&=.+-]+/);
  await browser.route('https://claude.ai/**', (r) => r.fulfill({ body: 'Code: shown on the sign-in page' }));
  const popup = browser.waitForEvent('page');
  // The mouse itself: xterm.js's layers above the rows take its events. Over the link, xterm.js shows a pointer.
  const box = (await address.boundingBox())!;
  await page.mouse.move(box.x + 40, box.y + box.height / 2);
  await expect(signIn.locator('.xterm-cursor-pointer')).toHaveCount(1);
  await page.mouse.click(box.x + 40, box.y + box.height / 2);
  const opened = await popup;
  const state = new URL(opened.url()).searchParams.get('state');
  expect(state).toMatch(/^[0-9a-f]{32}$/);
  await opened.close();
  await signIn.locator('.xterm').click();
  await page.keyboard.type(`e2e-code#${state}`);
  await page.keyboard.press('Enter');
  await expect(signIn.getByRole('status')).toContainText('Connecté au compte ada@exemple.test', { timeout: 20_000 });
  await signIn.getByRole('button', { name: 'Terminé' }).click();
  await expect(settings.getByRole('list', { name: 'Comptes Claude Code' }).getByRole('listitem', { name: 'Serveur' })).toBeVisible();
  // Its sign-in is on the server, in its folder under the data folder.
  const signedIn = fs
    .readdirSync(path.join(server.root, 'data'), { recursive: true, encoding: 'utf8' })
    .filter((f) => f.endsWith('.credentials.json'));
  expect(signedIn).toHaveLength(1);
  await page.keyboard.press('Escape');
  await expect(settings).toBeHidden();
  expect(errors).toEqual([]);
});

test('a project cloned on the server, its agent talks and asks for a permission', async () => {
  await page
    .getByRole('button', { name: /Ajouter un projet/ })
    .first()
    .click();
  const item = page.getByRole('menuitem', { name: /Ajouter un projet/ });
  const dialog = page.getByRole('dialog', { name: 'Nouveau projet' });
  await expect(item.or(dialog).first()).toBeVisible();
  if (await item.isVisible()) await item.click();
  await dialog.getByRole('button', { name: 'Parcourir…' }).click();
  const box = dialog.getByRole('group', { name: 'Dossiers du serveur' });
  await box.getByRole('button', { name: 'projets' }).click();
  await box.getByRole('textbox', { name: 'Adresse du dépôt à cloner ici' }).fill(server.repo);
  await box.getByRole('button', { name: 'Cloner ici' }).click();
  const clone = path.join(fs.realpathSync(server.home), 'projets', 'demo-api');
  await expect(dialog.locator('input').first()).toHaveValue(clone);
  await expect(dialog.getByText(/Dépôt git détecté · branche main · propre/)).toBeVisible();
  await dialog.getByRole('button', { name: /^Créer sans l.assistant$/ }).click();
  await expect(dialog).toBeHidden();

  await page.locator('.agent.card').first().click();
  const composer = page.locator('.composer textarea');
  await composer.fill('Bonjour');
  await composer.press('Enter');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible({ timeout: 30_000 });
  // On the account signed in just before: its folder is the CLAUDE_CONFIG_DIR of the agent's Claude Code.
  const launches = fs
    .readFileSync(path.join(server.root, 'fake-claude.jsonl'), 'utf8')
    .trim()
    .split('\n')
    .map((l) => JSON.parse(l) as { argv: string[]; configDir: string | null });
  const stream = launches.filter((l) => l.argv.includes('stream-json')).at(-1);
  expect(fs.existsSync(path.join(stream?.configDir ?? '', '.credentials.json'))).toBe(true);
  await composer.fill('permission');
  await composer.press('Enter');
  const card = page.getByTestId('permission-pending');
  await expect(card).toContainText('rm -rf build');
  await card.getByRole('button', { name: 'Autoriser', exact: true }).click();
  await expect(page.getByText('Commande exécutée.')).toBeVisible();
  expect(errors).toEqual([]);
});

test('a file joined goes to the server, a terminal runs there, the board takes a ticket', async () => {
  const composer = page.locator('.composer textarea');
  await page
    .locator('.composer input[type=file]')
    .setInputFiles({ name: 'plans.zip', mimeType: 'application/zip', buffer: Buffer.alloc(2048, 0x62) });
  await expect(page.locator('.composer .atts .file')).toContainText('plans.zip');
  await expect(page.locator('.composer .atts .file.copying')).toHaveCount(0);
  await composer.fill('Lis ces fichiers');
  await composer.press('Enter');
  await expect(page.getByText('Lu : plans.zip=2048')).toBeVisible({ timeout: 30_000 });

  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /bash/i }).click();
  await page.locator('main.tv .xterm').click();
  await page.keyboard.type('echo "sur-le-$((40 + 2))"');
  await page.keyboard.press('Enter');
  await expect(row(page, 'sur-le-42')).toHaveCount(1);

  await page.getByRole('button', { name: /^Kanban/ }).click();
  await page.getByRole('button', { name: 'Nouveau ticket' }).click();
  await page.getByRole('textbox', { name: 'Titre du ticket' }).fill('Écrire la doc du serveur');
  await page.getByRole('button', { name: 'Ajouter', exact: true }).click();
  await expect(page.getByText('Écrire la doc du serveur')).toBeVisible();
  expect(errors).toEqual([]);
});

test('the browser closed and opened again: still paired, everything found again', async () => {
  await browser.close();
  await open();
  await page.goto(server.url);
  await expect(page.getByRole('tablist', { name: 'Projets ouverts' }).getByRole('tab')).toHaveCount(1);
  await page.getByRole('button', { name: /^Kanban/ }).click();
  await expect(page.getByText('Écrire la doc du serveur')).toBeVisible();
  await page.locator('.row-item.term', { hasText: 'bash-1' }).click();
  await expect(row(page, 'sur-le-42')).toHaveCount(1);
  await page.locator('.agent.card').first().click();
  await expect(page.getByText('Commande exécutée.')).toBeVisible();
  await expect(page.getByText('Lu : plans.zip=2048')).toBeVisible();
  expect(errors).toEqual([]);
});

test('the device revoked: its page is cut off at once and asks to be paired again', async () => {
  const id = /^(\S+)\s+Chrome des tests\b/m.exec(server.cli('devices'))?.[1];
  expect(id).toBeTruthy();
  expect(server.cli('devices', 'revoke', id!)).toContain('revoked');
  await expect(page.getByRole('heading', { name: 'Appairer ce navigateur' })).toBeVisible({ timeout: 2_000 });
  expect(await page.evaluate(() => localStorage.getItem('escouade-device'))).toBeNull();
  expect(server.cli('devices')).toContain('No device paired');
});
