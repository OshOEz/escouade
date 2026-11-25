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
  state: { run: 'Running', wait: 'Your turn', error: 'Error', done: 'Done', idle: 'Ready' },
  waiting: 'waits for your answer',
  noPlan: 'no plan',
  progress: '{done}/{total} · {percent} %',
  sat: {
    team: { one: 'team', other: 'team' },
    question: { one: 'question', other: 'questions' },
  },
  launched: 'launched',
  plan: {
    chip: 'Plan',
    count: '{done}/{total} tasks · {percent} %',
    folded: { one: '{count} done', other: '{count} done' },
  },
  team: {
    task: 'task {n}',
    write: 'Write to {name}',
    folded: '+ {done} ended · {failed} failed',
    foldedCapped: '+ 30+ ended · {failed} failed',
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
  discLabel: '{name}, {state}',
  discProgress: { one: '{count} of {total} task done', other: '{count} of {total} tasks done' },
});
