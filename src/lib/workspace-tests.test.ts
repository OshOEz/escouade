import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { bg, cmd, planWith, sub } from '../test/workspace';
import { looksLikeTests, testsOf } from './workspace-tests';

describe('looksLikeTests', () => {
  it.each([
    'cargo test -p escouade-plan',
    'cargo nextest run',
    'cd app && npx vitest run src/lib',
    'CI=1 npm test',
    'npm run test -- --watch=false',
    'python -m pytest -q',
    'go test ./...',
    'bundle exec rspec spec/a_spec.rb',
    'vendor/bin/phpunit',
    './gradlew test',
  ])('sees a test suite in %s', (c) => expect(looksLikeTests(c)).toBe(true));

  it.each(['npm run build', 'cat test.txt', 'git commit -m "test"', 'ls tests/', 'cargo build --tests', 'echo jest', 'npm run dev'])(
    'sees none in %s',
    (c) => expect(looksLikeTests(c)).toBe(false),
  );

  it("takes the board's test command, and what starts with it", () => {
    expect(looksLikeTests('./scripts/check.sh --fast', './scripts/check.sh')).toBe(true);
    expect(looksLikeTests('./scripts/other.sh', './scripts/check.sh')).toBe(false);
    expect(looksLikeTests('anything', '  ')).toBe(false);
  });
});

describe('testsOf', () => {
  it('keeps the latest run of each card, a call or a background task', () => {
    const a = agent({
      plan: planWith([], {
        agents: [{ ...sub('s1'), taskId: 'k1' }],
        bgTasks: [
          { ...bg('t1'), command: 'npm test', parent: 'k1', startedAt: 5 },
          { ...bg('t2'), command: 'npm run dev' },
        ],
      }),
      canvas: {
        tools: [],
        subs: [{ call: 's1', tools: [], commands: [cmd('c2', 'cargo test', { startedAt: 2 })] }],
        commands: [cmd('c0', 'cargo test'), cmd('c1', 'pytest -q', { startedAt: 3, endedAt: null }), cmd('c3', 'ls', { startedAt: 4 })],
      },
    });
    expect(testsOf(a, '')).toEqual([
      expect.objectContaining({ under: 'centre', line: 'pytest -q', state: 'running', opens: { kind: 'call', call: 'c1' } }),
      expect.objectContaining({ under: 's1', line: 'npm test', state: 'running', opens: { kind: 'task', task: 't1' } }),
    ]);
  });

  it('says a run failed by its result or its exit code', () => {
    const failedCall = agent({ canvas: { tools: [], subs: [], commands: [cmd('c', 'npm test', { failed: true })] } });
    expect(testsOf(failedCall, '')[0].state).toBe('failure');
    const done = (summary: string) =>
      agent({ plan: planWith([], { bgTasks: [{ ...bg('t'), command: 'npm test', status: 'done', summary }] }) });
    expect(testsOf(done('Background command "x" completed (exit code 1)'), '')[0].state).toBe('failure');
    expect(testsOf(done('Background command "x" completed (exit code 0)'), '')[0].state).toBe('success');
  });
});
