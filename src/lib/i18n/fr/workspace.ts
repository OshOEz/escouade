import type { Tree } from '../types';

// The « Workspace »: the canvas of a project's agents, its discs seen from afar and its controls. The names of the
// agents and what they do are their own words: shown as they come, never translated.
export default {
  /** The entry of the menu of the « Éditeur » button, and that button when the editor is off. */
  open: 'Workspace',
  exit: 'Quitter le workspace',
  label: 'Toile des agents',
  empty: 'Aucun agent dans ce projet',
  count: { one: '{count} agent', other: '{count} agents' },
  questions: { one: '{count} question', other: '{count} questions' },
  zoomIn: 'Zoomer',
  zoomOut: 'Dézoomer',
  zoom: '{percent} %',
  fit: 'Ajuster',
  /** The line of a disc whose agent waits for an answer. */
  waiting: 'attend ta réponse',
  noPlan: 'sans plan',
  progress: '{done}/{total} · {percent} %',
  /** The word under a satellite's number. */
  sat: {
    team: { one: 'équipe', other: 'équipe' },
    question: { one: 'question', other: 'questions' },
  },
  /** The word on the dotted link from an agent to one it launched. */
  launched: 'a lancé',
  /** The team, east of the card: its subagents and background tasks. */
  team: {
    task: 'tâche {n}',
    write: 'Écrire à {name}',
    folded: { one: '+ {count} terminé', other: '+ {count} terminés' },
    /** The plan keeps 30 subagents at most: more may have ended. */
    foldedCapped: '+ 30+ terminés',
    failed: { one: '{count} échec', other: '{count} échecs' },
  },
  /** The card of an agent near (its compass's centre) and its bar to write to it. */
  card: {
    write: 'Écrire à {name}',
    settings: 'Paramètres',
    model: 'Modèle',
    effort: 'Effort',
    mode: 'Mode',
    attach: 'Joindre un fichier',
    allModels: 'Tous les modèles…',
    files: { one: '{count} fichier', other: '{count} fichiers' },
    label: '{name}, {state}',
  },
  /** A disc, for a screen reader: « release-2-3, En cours ». */
  discLabel: '{name}, {state}',
  /** What a screen reader hears after it: the plan, then the questions. */
  discProgress: { one: '{count} tâche faite sur {total}', other: '{count} tâches faites sur {total}' },
} as const satisfies Tree;
