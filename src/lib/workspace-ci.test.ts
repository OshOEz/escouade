import { describe, expect, it, vi } from 'vitest';
import { agent, fakeBackend, pullRequest } from '../test/ipc';
import { check, prDetail, projectEnvs } from '../test/workspace';
import { followEnvironments, prDetails, readPrDetail } from './workspace-ci.svelte';
import { landingOf } from './workspace-landing';

describe('what the canvas reads of the CI', () => {
  it("reads the detail of the agent's open pull request", async () => {
    const backend = fakeBackend({ agent_pr_detail: () => ({ data: { ...prDetail(), checks: [check('ci', 'success')] } }) });
    await readPrDetail(agent({ id: 'a1', prs: [pullRequest({ number: 7, state: 'open' })] }));
    expect(backend.called('agent_pr_detail')[0].args).toMatchObject({ id: 'a1', number: 7 });
    expect(prDetails.a1.checks[0].name).toBe('ci');
  });

  it("keeps the pull request's state when its detail cannot be read", async () => {
    fakeBackend({ agent_pr_detail: () => ({ failure: { kind: 'unreachable', message: 'x' } }) });
    const a = agent({ id: 'a2', prs: [pullRequest({ state: 'open', checks: 'failure' })] });
    await readPrDetail(a);
    expect(prDetails.a2).toBeUndefined();
    expect(landingOf(a, { pr: null, environments: null })[0]).toMatchObject({ kind: 'ci', state: 'failure', steps: null });
  });

  it('follows the environments of the project while it is open', async () => {
    vi.useFakeTimers();
    const backend = fakeBackend({ project_environments: () => ({ ...projectEnvs(), projectId: 'p1', readAt: Date.now() }) });
    const stop = followEnvironments('p1');
    await vi.advanceTimersByTimeAsync(5 * 60 * 1000);
    expect(backend.called('project_environments')).toHaveLength(2);
    stop();
    await vi.advanceTimersByTimeAsync(5 * 60 * 1000);
    expect(backend.called('project_environments')).toHaveLength(2);
    vi.useRealTimers();
  });
});
