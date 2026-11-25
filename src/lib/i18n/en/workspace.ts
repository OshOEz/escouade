import { defineZone } from '../types';

// The « Workspace »: the canvas of a project's agents, its discs seen from afar and its controls. The names of the
// agents and what they do are their own words: shown as they come, never translated.
export default defineZone('workspace', {
  open: 'Workspace',
  exit: 'Leave the workspace',
  label: 'Canvas of the agents',
  empty: 'No agent in this project',
  count: { one: '{count} agent', other: '{count} agents' },
  questions: { one: '{count} question', other: '{count} questions' },
  zoomIn: 'Zoom in',
  zoomOut: 'Zoom out',
  zoom: '{percent} %',
  fit: 'Fit',
  waiting: 'waits for your answer',
  noPlan: 'no plan',
  progress: '{done}/{total} · {percent} %',
  sat: {
    team: { one: 'team', other: 'team' },
    tools: { one: 'tool', other: 'tools' },
    question: { one: 'question', other: 'questions' },
  },
  launched: 'launched',
  tool: {
    label: '{kind} {name}, {calls}',
    skill: 'Skill',
    plugin: 'Plugin',
    calls: { one: '{count} call', other: '{count} calls' },
    running: 'running',
    tags: 'Tools: {names}',
  },
  team: {
    task: 'task {n}',
    write: 'Write to {name}',
    folded: { one: '+ {count} ended', other: '+ {count} ended' },
    foldedCapped: '+ 30+ ended',
    failed: { one: '{count} failed', other: '{count} failed' },
  },
  card: {
    write: 'Write to {name}',
    settings: 'Settings',
    model: 'Model',
    effort: 'Effort',
    mode: 'Mode',
    attach: 'Attach a file',
    allModels: 'All models…',
    files: { one: '{count} file', other: '{count} files' },
    label: '{name}, {state}',
  },
  ask: { loading: 'Loading the question…', fromSub: 'question from {name}' },
  discLabel: '{name}, {state}',
  discProgress: { one: '{count} of {total} task done', other: '{count} of {total} tasks done' },
});
