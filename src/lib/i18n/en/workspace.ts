import { defineZone } from '../types';

// The « Workspace »: the canvas of a project's agents, its discs seen from afar and its controls. The names of the
// agents and what they do are their own words: shown as they come, never translated.
export default defineZone('workspace', {
  open: 'Workspace',
  more: 'Other views of the project',
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
  discLabel: '{name}, {state}',
});
