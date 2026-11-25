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
    tools: { one: 'outil', other: 'outils' },
    question: { one: 'question', other: 'questions' },
    deliverables: { one: 'livrable', other: 'livrables' },
    ci: 'CI',
  },
  /** The landing zone, south of a compass: what the agent delivered, its remote CI, its deployments. */
  land: {
    title: 'Zone d’atterrissage · {landed} posés, {coming} en approche',
    empty: 'Rien n’a encore atterri',
    artifact: 'Artefact · version {n} · {ago}',
    open: 'Ouvrir ↗',
  },
  /** A card with a ribbon: a test run, a CI, a deployment. */
  ribbon: {
    label: '{kind}, {state}',
    output: 'Sortie',
    checks: '{total} contrôles, {failed} en échec',
    kind: { tests: 'Tests', ci: 'CI', deploy: 'Déploiement' },
    state: { running: 'en cours', success: 'réussi', failure: 'en échec' },
  },
  /** The word on the dotted link from an agent to one it launched. */
  launched: 'a lancé',
  /** A skill or a plugin, on its card above the agent's, and as a tag on a subagent's card. */
  tool: {
    label: '{kind} {name}, {calls}',
    skill: 'Skill',
    plugin: 'Plugin',
    calls: { one: '{count} appel', other: '{count} appels' },
    running: 'en cours',
    /** The tags of a subagent's card: its tools, named for a screen reader. */
    tags: 'Outils de {name}',
    /** The pile of the tools past the most recent, and its button once unfolded. */
    more: { one: '+ {count} outil', other: '+ {count} outils' },
    fold: 'Replier les outils',
  },
  /** A side of a compass folded at mid-height, a button that unfolds it. */
  pile: {
    west: 'Outils ×{count}',
    east: 'Équipe ×{count}',
    south: 'Livrables ×{count}',
    /** Its name for a screen reader: what a click does. */
    unfold: { west: 'Déplier les outils ({count})', east: "Déplier l'équipe ({count})", south: 'Déplier les livrables ({count})' },
  },
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
  /** The requests of an agent that waits, in its card. `fromSub`: on the agent's card, a subagent's question is on its own. */
  ask: { loading: 'Chargement de la question…', fromSub: 'question de {name}' },
  /** A disc, for a screen reader: « release-2-3, En cours ». */
  discLabel: '{name}, {state}',
  /** What a screen reader hears after it: the plan, then the questions. */
  discProgress: { one: '{count} tâche faite sur {total}', other: '{count} tâches faites sur {total}' },
} as const satisfies Tree;
