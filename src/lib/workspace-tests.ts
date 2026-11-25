// The test runs of an agent and of its subagents (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md, « Choix
// validés »): each card of the canvas gets its latest run stuck under it. A run is a terminal call or a background command
// that is the board's test command, or looks like a test suite. The Kanban's launches are not tests. Pure.

import { exitCode, ownerOf } from './bgtasks';
import type { Agent, CommandRun } from './types';
import type { Ribbon, RibbonState } from './workspace-landing';

/** What a test suite's command starts with, word by word. Better one card too many than a run missed. */
export const TEST_PATTERNS: readonly string[] = [
  'cargo test',
  'cargo nextest',
  'npm test',
  'npm run test',
  'npm t',
  'pnpm test',
  'pnpm run test',
  'yarn test',
  'yarn run test',
  'bun test',
  'bun run test',
  'vitest',
  'jest',
  'pytest',
  'go test',
  'make test',
  'mvn test',
  'gradle test',
  './gradlew test',
  'dotnet test',
  'rspec',
  'phpunit',
];
/** A runner called by its path (`vendor/bin/phpunit`). */
const BY_PATH = ['/phpunit', '/rspec', '/pytest'];
/** What runs the command after it. */
const LAUNCHERS = ['npx', 'bunx', 'pnpm exec', 'yarn exec', 'python -m', 'python3 -m', 'uv run', 'poetry run', 'bundle exec'].map((l) =>
  l.split(' '),
);

const startsWith = (words: string[], prefix: string[]) => prefix.every((w, i) => words[i] === w);

export function looksLikeTests(command: string, testCommand = ''): boolean {
  const own = testCommand.trim();
  const line = command.trim();
  if (own && line.startsWith(own) && (line.length === own.length || /\s/.test(line[own.length]))) return true;
  return line.split(/&&|\|\||;|\|/).some((piece) => {
    let words = piece.trim().split(/\s+/);
    while (words.length && /^[A-Za-z_][A-Za-z0-9_]*=/.test(words[0])) words = words.slice(1);
    for (let launcher = LAUNCHERS.find((l) => startsWith(words, l)); launcher; launcher = LAUNCHERS.find((l) => startsWith(words, l))) {
      words = words.slice(launcher.length);
    }
    if (!words.length) return false;
    return TEST_PATTERNS.some((p) => startsWith(words, p.split(' '))) || BY_PATH.some((p) => words[0].endsWith(p));
  });
}

/** A test card: `under` is 'centre' (the agent's card) or the id of the subagent's. */
export type TestRun = Ribbon & { kind: 'tests'; under: string };

const run = (
  under: string,
  id: string,
  line: string,
  state: RibbonState,
  since: number,
  until: number | null,
  opens: Ribbon['opens'],
): TestRun => ({
  kind: 'tests',
  under,
  id: `tests:${id}`,
  number: null,
  state,
  since,
  until,
  line,
  steps: null,
  total: null,
  failed: null,
  where: '',
  opens,
});

const callRun = (under: string, c: CommandRun): TestRun =>
  run(under, c.call, c.command, c.endedAt == null ? 'running' : c.failed ? 'failure' : 'success', c.startedAt, c.endedAt ?? null, {
    kind: 'call',
    call: c.call,
  });

/** The latest test run of each card of the agent: its own first, then its subagents' in the plan's order. */
export function testsOf(agent: Agent, testCommand: string): TestRun[] {
  const plan = agent.plan;
  const runs: TestRun[] = [];
  for (const c of agent.canvas?.commands ?? []) runs.push(callRun('centre', c));
  for (const s of agent.canvas?.subs ?? []) for (const c of s.commands ?? []) runs.push(callRun(s.call, c));
  for (const t of plan?.bgTasks ?? []) {
    if (t.kind !== 'command' || !t.command) continue;
    const code = exitCode(t);
    const state: RibbonState = t.status === 'running' ? 'running' : t.status === 'done' && !code ? 'success' : 'failure';
    runs.push(run(ownerOf(plan, t)?.id ?? 'centre', t.id, t.command, state, t.startedAt, t.endedAt ?? null, { kind: 'task', task: t.id }));
  }
  const latest = new Map<string, TestRun>();
  for (const r of runs) {
    if (!looksLikeTests(r.line, testCommand)) continue;
    const was = latest.get(r.under);
    if (!was || (r.since ?? 0) > (was.since ?? 0)) latest.set(r.under, r);
  }
  const rank = (under: string) => {
    if (under === 'centre') return -1;
    const i = plan?.agents.findIndex((a) => a.id === under) ?? -1;
    return i < 0 ? Number.MAX_SAFE_INTEGER : i;
  };
  return [...latest.values()].sort((x, y) => rank(x.under) - rank(y.under));
}
