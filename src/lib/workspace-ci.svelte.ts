// What the « Workspace » reads for its landing zones (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md,
// « D'où viennent les données »): the detail of an agent's open pull request (its checks, its deployments), for the
// compasses on screen, and the project's environments, followed while the canvas is open as the view « Environnements ».

import { envsView } from './envs-view.svelte';
import { api } from './ipc';
import { app } from './state.svelte';
import type { Agent, PrDetail } from './types';

/** The detail of each agent's open pull request, as last read well. */
export const prDetails = $state<Record<string, PrDetail>>({});

/** Reads the detail of the agent's open pull request; one that cannot be read leaves what was there. */
export async function readPrDetail(agent: Agent): Promise<void> {
  const pr = agent.prs.find((p) => p.state === 'open');
  if (!pr) {
    delete prDetails[agent.id];
    return;
  }
  try {
    const r = await api.agentPrDetail(agent.id, pr.number, pr.hostRepo);
    if (r.data) prDetails[agent.id] = r.data;
  } catch {
    // The host out of reach: the card keeps the pull request's state.
  }
}

/** How often the project is said followed (the core forgets it after 15 minutes). */
const KEEP_EVERY = 5 * 60 * 1000;

/** What `EnvironmentsView` does while open: its environments read, the core told to follow them. Returns the stop. */
export function followEnvironments(projectId: string): () => void {
  void (async () => {
    const p = await api.projectEnvironments(projectId).catch(() => null);
    if (p) app.environments[projectId] = p;
    // Read when never read, or read more than a minute ago.
    if (!p?.readAt || Date.now() - p.readAt > 60_000) void envsView.refresh(projectId);
  })();
  const keep = setInterval(() => void api.projectEnvironments(projectId).catch(() => {}), KEEP_EVERY);
  return () => clearInterval(keep);
}
