import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { art, ribbon } from '../../test/workspace';
import LandingZone from './LandingZone.svelte';

describe('the landing zone', () => {
  beforeEach(() => setLang('fr'));

  it('is drawn even empty', () => {
    render(LandingZone, { agentId: 'a1', items: [], onsize: () => {} });
    expect(screen.getByText('Rien n’a encore atterri')).toBeTruthy();
    expect(screen.getByText('Zone d’atterrissage · 0 posés, 0 en approche')).toBeTruthy();
  });

  it('lands what is finished and keeps the rest above', () => {
    const { container } = render(LandingZone, { agentId: 'a1', items: [art(true), ribbon('ci', 'running')], onsize: () => {} });
    expect(screen.getByText('Zone d’atterrissage · 1 posés, 1 en approche')).toBeTruthy();
    expect(container.querySelectorAll('.coming .ribbon.stripes')).toHaveLength(1);
    expect(container.querySelectorAll('.landed article')).toHaveLength(1);
  });
});
