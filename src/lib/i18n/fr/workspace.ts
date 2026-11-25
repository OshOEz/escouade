import type { Tree } from '../types';

// The « Workspace »: the canvas of a project's agents, its discs seen from afar and its controls. The names of the
// agents and what they do are their own words: shown as they come, never translated.
export default {
  /** The entry of the menu of the « Éditeur » button, and that button when the editor is off. */
  open: 'Workspace',
  /** The arrow beside « Éditeur », which opens the menu of the project's views. */
  more: 'Autres vues du projet',
  exit: 'Quitter le workspace',
  label: 'Toile des agents',
  empty: 'Aucun agent dans ce projet',
  count: { one: '{count} agent', other: '{count} agents' },
  questions: { one: '{count} question', other: '{count} questions' },
  zoomIn: 'Zoomer',
  zoomOut: 'Dézoomer',
  zoom: '{percent} %',
  fit: 'Ajuster',
  state: { run: 'En cours', wait: 'À toi', error: 'Erreur', done: 'Terminé', idle: 'Prêt' },
  /** The line of a disc whose agent waits for an answer. */
  waiting: 'attend ta réponse',
  noPlan: 'sans plan',
  progress: '{done}/{total} · {percent} %',
  /** The word under a satellite's number. */
  sat: {
    team: { one: 'équipe', other: 'équipe' },
    question: { one: 'question', other: 'questions' },
  },
  /** A disc, for a screen reader: « release-2-3, En cours ». */
  discLabel: '{name}, {state}',
  /** What a screen reader hears after it: the plan, then the questions. */
  discProgress: { one: '{count} tâche faite sur {total}', other: '{count} tâches faites sur {total}' },
} as const satisfies Tree;
