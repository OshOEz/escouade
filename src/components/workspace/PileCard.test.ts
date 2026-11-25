import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import PileCard from './PileCard.svelte';

describe('a pile of a compass at mid-height', () => {
  beforeEach(() => setLang('fr'));

  it('says its side and its count, and unfolds on a click', async () => {
    const onunfold = vi.fn();
    render(PileCard, { pile: { side: 'west', count: 4, running: false }, onunfold });
    const pile = screen.getByRole('button', { name: 'Outils ×4' });
    expect(pile.getAttribute('aria-expanded')).toBe('false');
    await fireEvent.click(pile);
    expect(onunfold).toHaveBeenCalledWith('west');
  });

  it('rings when one of its items runs', () => {
    const { container } = render(PileCard, { pile: { side: 'east', count: 2, running: true }, onunfold: () => {} });
    expect(screen.getByRole('button', { name: 'Équipe ×2' })).toBeTruthy();
    expect(container.querySelector('button.running')).toBeTruthy();
  });
});
