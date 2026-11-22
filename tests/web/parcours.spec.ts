// The web interface in Chrome for Testing, against `escouade-server`: one server for the whole course, its steps in order.
import fs from 'node:fs';
import path from 'node:path';
import { expect, test, type BrowserContext, type Locator, type Page } from '@playwright/test';
import { startServer, type Server } from './server';

test.describe.configure({ mode: 'serial' });

let server: Server;
let context: BrowserContext;
/** The first tab, then the second (step 2). */
let page: Page;
let other: Page;
/** What the pages wrote to their console as errors, and what they threw. */
const errors: string[] = [];

async function tab(): Promise<Page> {
  const p = await context.newPage();
  p.on('console', (m) => {
    if (m.type() === 'error') errors.push(m.text());
  });
  p.on('pageerror', (e) => errors.push(String(e)));
  return p;
}

/** The rows of the terminal shown: xterm.js's DOM renderer (no WebGL, playwright.web.config.ts). */
const screen = (p: Page) => p.locator('main.tv .xterm-rows');
/** The row of the terminal that holds exactly `text` (an output line, not the command that printed it). */
const row = (p: Page, text: string) =>
  screen(p)
    .locator('> div')
    .filter({ hasText: new RegExp(`^\\s*${text}\\s*$`) });

async function type(p: Page, line: string) {
  await p.locator('main.tv .xterm').click();
  await p.keyboard.type(line);
  await p.keyboard.press('Enter');
}

const openTerminal = (p: Page) => p.locator('.row-item.term', { hasText: 'bash-1' }).click();
const openBoard = (p: Page) => p.getByRole('button', { name: /^Kanban/ }).click();

async function newTicket(p: Page, title: string) {
  await expect(p.getByRole('button', { name: 'Nouveau ticket' })).toBeVisible();
  await p.getByRole('button', { name: 'Nouveau ticket' }).click();
  await p.getByRole('textbox', { name: 'Titre du ticket' }).fill(title);
  await p.getByRole('button', { name: 'Ajouter', exact: true }).click();
}

test.beforeAll(async ({ browser }) => {
  server = await startServer();
  context = await browser.newContext({ viewport: { width: 1400, height: 900 }, proxy: server.proxy, locale: 'fr-FR' });
  page = await tab();
});

test.afterAll(async () => {
  await context?.close();
  await server?.stop();
});

test('paired by a code, the token kept by the page and never in a cookie', async () => {
  await page.goto(server.url);
  await expect(page.getByRole('heading', { name: 'Appairer ce navigateur' })).toBeVisible();
  // A wrong code is said, and pairs nothing.
  await page.getByLabel('Code d’appairage').fill('0000-0000');
  await page.getByRole('button', { name: 'Appairer' }).click();
  await expect(page.getByRole('alert')).toContainText('Code refusé');
  // Chrome writes the refusal (403) to the console: that one is expected.
  expect(errors).toEqual([expect.stringContaining('403')]);
  errors.length = 0;
  await server.pair(page);
  await expect(page.getByRole('button', { name: /Ajouter un projet/ }).first()).toBeVisible();
  // Kept by this origin alone (its port included), presented by the WebSocket's subprotocol: no cookie at all, which the
  // browser would send to every port of 127.0.0.1.
  expect(await context.cookies()).toEqual([]);
  const token = await page.evaluate(() => localStorage.getItem('escouade-device'));
  expect(token).toMatch(/^[A-Za-z0-9_-]{43}$/);
  expect(errors).toEqual([]);
});

test('a project, a terminal and the board, found again after a reload', async () => {
  await page.goto(server.url);
  // The first state: no project.
  await expect(page.getByRole('button', { name: /Ajouter un projet/ }).first()).toBeVisible();
  expect(errors).toEqual([]);

  // « Nouveau projet », the path typed (a browser has no folder dialog).
  await page
    .getByRole('button', { name: /Ajouter un projet/ })
    .first()
    .click();
  const item = page.getByRole('menuitem', { name: /Ajouter un projet/ });
  const dialog = page.getByRole('dialog', { name: 'Nouveau projet' });
  await expect(item.or(dialog).first()).toBeVisible();
  if (await item.isVisible()) await item.click();
  await dialog.locator('input').first().fill(server.repo);
  await expect(dialog.getByText(/Dépôt git détecté · branche main · propre/)).toBeVisible();
  await dialog.getByRole('button', { name: /^Créer sans l.assistant$/ }).click();
  await expect(dialog).toBeHidden();

  // A terminal: what is typed in it runs on the server, its output is on the screen.
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /bash/i }).click();
  await expect(screen(page)).toBeVisible();
  await type(page, 'echo bonjour');
  await expect(row(page, 'bonjour')).toHaveCount(1);

  await openBoard(page);
  await expect(page.getByRole('button', { name: 'Nouveau ticket' })).toBeVisible();

  // Loaded again: the project, and the terminal with its screen, then what follows.
  await page.reload();
  await expect(page.locator('.row-item.term', { hasText: 'bash-1' })).toBeVisible();
  await openTerminal(page);
  await expect(row(page, 'bonjour')).toHaveCount(1);
  await type(page, 'echo encore');
  await expect(row(page, 'encore')).toHaveCount(1);
  expect(errors).toEqual([]);
});

test('two tabs: an event of one shows in the other, which keeps its own view', async () => {
  await openBoard(page);
  other = await tab();
  await other.goto(server.url);
  await openBoard(other);

  // A ticket made in the second tab: the first gets it by the core's event, without reloading.
  await newTicket(other, 'Fait dans l’autre onglet');
  await expect(page.getByText('Fait dans l’autre onglet')).toBeVisible();

  // The second tab goes to the terminal: the first stays on the board. Opening a terminal writes no `set_ui` (the view
  // a tab shows is its own); the barrier is a second ticket made after it, whose event comes after anything it caused.
  await openTerminal(other);
  await expect(row(other, 'encore')).toHaveCount(1);
  await openBoard(other);
  await newTicket(other, 'Après le terminal');
  await expect(page.getByText('Après le terminal')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Nouveau ticket' })).toBeVisible();
  await expect(page.locator('main.tv')).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('the connection lost: said, opened again at a growing pace, and the terminal goes on', async () => {
  await other.close();
  await openTerminal(page);
  await expect(row(page, 'encore')).toHaveCount(1);
  // Written on the server while the page is cut off; cut once the command is on the server (its first line back).
  await type(page, 'echo debut; sleep 2; echo pendant-la-coupure');
  await expect(row(page, 'debut')).toHaveCount(1);
  const tries: number[] = [];
  page.on('websocket', () => tries.push(Date.now()));
  server.cut(true);
  await expect(page.getByText('La connexion au serveur est perdue')).toBeVisible();
  const cut = Date.now();
  await page.waitForTimeout(8000);
  // 0.5, 1, 2 and 4 s between the tries: a handful in 8 s, never a loop without a pause.
  const during = tries.filter((t) => t >= cut);
  expect(during.length).toBeGreaterThanOrEqual(3);
  expect(during.length).toBeLessThanOrEqual(5);
  const gaps = during.slice(1).map((t, i) => t - during[i]);
  for (const [i, gap] of gaps.entries()) if (i > 0) expect(gap).toBeGreaterThanOrEqual(gaps[i - 1] * 1.5);

  server.cut(false);
  await expect(page.getByText('La connexion au serveur est rétablie')).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText('La connexion au serveur est perdue')).toBeHidden();
  // Its screen again, what came while cut off included, then what follows.
  await expect(row(page, 'bonjour')).toHaveCount(1);
  await expect(row(page, 'pendant-la-coupure')).toHaveCount(1);
  await type(page, 'echo apres');
  await expect(row(page, 'apres')).toHaveCount(1);
});

test('an agent on the fake Claude Code: its reply, then its question answered', async () => {
  await page.locator('.agent.card').first().click();
  const box = page.locator('.composer textarea');
  await box.fill('Bonjour');
  await box.press('Enter');
  await expect(page.getByText('Bonjour, tu as dit : Bonjour')).toBeVisible({ timeout: 30_000 });
  await box.fill('question');
  await box.press('Enter');
  const card = page.getByTestId('question-pending');
  await expect(card).toContainText('Quelle base de données ?');
  await card.getByRole('button', { name: 'SQLite' }).click();
  await expect(page.getByText('Choix retenu : SQLite')).toBeVisible();
});

test('a project made from a folder chosen on the server, then from a repository cloned there', async () => {
  // What the cut of the network (above) wrote to the console was expected there.
  errors.length = 0;
  const tabs = page.getByRole('tablist', { name: 'Projets ouverts' }).getByRole('tab');
  await expect(tabs).toHaveCount(1);
  const projets = path.join(fs.realpathSync(server.home), 'projets');
  /** « Nouveau projet », its server's folders opened on `projets`. */
  async function browse() {
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
    await expect(box.getByText(projets, { exact: true })).toBeVisible();
    return { dialog, box };
  }
  async function create(dialog: Locator, folder: string) {
    await expect(dialog.locator('input').first()).toHaveValue(folder);
    await expect(dialog.getByText(/Dépôt git détecté · branche main · propre/)).toBeVisible();
    await dialog.getByRole('button', { name: /^Créer sans l.assistant$/ }).click();
    await expect(dialog).toBeHidden();
  }

  // Chosen: down into a repository, then « Choisir ce dossier ».
  let { dialog, box } = await browse();
  await box.getByRole('button', { name: /^site/ }).click();
  await expect(box.getByText(path.join(projets, 'site'), { exact: true })).toBeVisible();
  await box.getByRole('button', { name: 'Choisir ce dossier' }).click();
  await create(dialog, path.join(projets, 'site'));
  await expect(tabs).toHaveCount(2);

  // Cloned: a repository of the server's disk, into `projets`; the clone is the project's folder.
  ({ dialog, box } = await browse());
  await box.getByRole('textbox', { name: 'Adresse du dépôt à cloner ici' }).fill(server.repo);
  await box.getByRole('button', { name: 'Cloner ici' }).click();
  await create(dialog, path.join(projets, 'demo-api'));
  await expect(tabs).toHaveCount(3);
  expect(fs.existsSync(path.join(projets, 'demo-api', 'README.md'))).toBe(true);
  expect(errors).toEqual([]);
});
