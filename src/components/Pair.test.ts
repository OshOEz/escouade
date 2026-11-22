// The page of a browser not paired yet: the code typed, the token kept, a code refused said.
import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import { deviceToken } from '../lib/transport/device';
import Pair from './Pair.svelte';

describe('the pairing page', () => {
  beforeEach(() => {
    setLang('fr');
    localStorage.clear();
  });
  afterEach(() => vi.unstubAllGlobals());

  it('pairs with the code and the name typed', async () => {
    const fetch = vi.fn(async () => new Response(JSON.stringify({ token: 't'.repeat(43), id: 'a1' }), { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    const onPaired = vi.fn();
    render(Pair, { onPaired });
    await userEvent.type(screen.getByLabelText('Code d’appairage'), ' abcd-efgh ');
    const name = screen.getByLabelText('Nom de cet appareil');
    expect(name).toHaveValue('Navigateur');
    await userEvent.clear(name);
    await userEvent.type(name, 'Portable');
    await userEvent.click(screen.getByRole('button', { name: 'Appairer' }));
    await waitFor(() => expect(onPaired).toHaveBeenCalled());
    expect(JSON.parse((fetch.mock.calls[0] as unknown as [string, RequestInit])[1].body as string)).toEqual({
      code: 'abcd-efgh',
      name: 'Portable',
    });
    expect(deviceToken()).toBe('t'.repeat(43));
  });

  it('says a refused code, and one the server could not take', async () => {
    vi.stubGlobal('fetch', async () => new Response(null, { status: 403 }));
    const onPaired = vi.fn();
    render(Pair, { onPaired });
    await userEvent.type(screen.getByLabelText('Code d’appairage'), 'ABCD-EFGH');
    await userEvent.click(screen.getByRole('button', { name: 'Appairer' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Code refusé');
    expect(screen.getByLabelText('Code d’appairage')).toHaveValue('');
    vi.stubGlobal('fetch', async () => Promise.reject(new TypeError('network')));
    await userEvent.type(screen.getByLabelText('Code d’appairage'), 'ABCD-EFGH');
    await userEvent.click(screen.getByRole('button', { name: 'Appairer' }));
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('Le serveur ne répond pas'));
    expect(onPaired).not.toHaveBeenCalled();
    expect(deviceToken()).toBeNull();
  });
});
