import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import ToolCard from './ToolCard.svelte';

describe('the card of a tool on the workspace', () => {
  beforeEach(() => setLang('fr'));

  it('says a skill, its calls and that it runs', () => {
    const { container } = render(ToolCard, { tool: { kind: 'skill', name: 'tdd', calls: 3, running: true, runningIn: null } });
    expect(screen.getByRole('article', { name: 'Skill tdd, 3 appels' })).toBeTruthy();
    expect(screen.getByText('en cours')).toBeTruthy();
    expect(container.querySelector('article.skill.running')).toBeTruthy();
  });

  it('says a plugin, and nothing runs', () => {
    render(ToolCard, { tool: { kind: 'plugin', name: 'github', calls: 1, running: false, runningIn: null } });
    expect(screen.getByRole('article', { name: 'Plugin github, 1 appel' })).toBeTruthy();
    expect(screen.queryByText('en cours')).toBeNull();
  });
});
