// Builders for the tests of the « Workspace »: plans and pending requests, smallest that the window accepts.
import type { BgTask, PendingRequest, PlanState, PlanTask, SubAgent } from '../lib/types';

const task = (i: number, status: PlanTask['status']): PlanTask => ({ id: String(i + 1), title: `Tâche ${i + 1}`, status });

export const sub = (id: string, status: SubAgent['status'] = 'running'): SubAgent => ({
  id,
  title: `sous-agent ${id}`,
  status,
  background: false,
  tools: 0,
  startedAt: 1,
});

export const bg = (id: string, status: BgTask['status'] = 'running'): BgTask => ({
  id,
  kind: 'command',
  title: `tâche ${id}`,
  status,
  startedAt: 1,
});

/** A plan of tasks in these states, and nothing else. */
export const planWith = (states: PlanTask['status'][], over: Partial<PlanState> = {}): PlanState => ({
  source: 'tools',
  tasks: states.map((s, i) => task(i, s)),
  agents: [],
  launched: 0,
  workflows: [],
  bgTasks: [],
  ...over,
});

/** A plan with no task, `subs` subagents and `bgs` background tasks running, and one of each ended. */
export const planWithRunning = (subs: number, bgs: number): PlanState =>
  planWith([], {
    agents: [...Array.from({ length: subs }, (_, i) => sub(`s${i}`)), sub('ended', 'done')],
    bgTasks: [...Array.from({ length: bgs }, (_, i) => bg(`b${i}`)), bg('ended', 'done')],
  });

export const req = (id: string, kind: PendingRequest['kind'] = 'question'): PendingRequest => ({
  id,
  kind,
  tool: 'AskUserQuestion',
  arg: '',
  description: null,
  reason: null,
  questions: [{ question: `Question ${id} ?`, options: ['Oui', 'Non'] }],
  cut: false,
  defaultNo: false,
});
