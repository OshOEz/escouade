import { afterEach, describe, expect, it, vi } from 'vitest';

/** The opener and the app's state anew, in a browser: the façade chooses as it is loaded. */
async function inBrowser() {
  const w = window as { __TAURI_INTERNALS__?: unknown };
  const internals = w.__TAURI_INTERNALS__;
  delete w.__TAURI_INTERNALS__;
  vi.resetModules();
  try {
    return { ...(await import('./opener')), ...(await import('./state.svelte')) };
  } finally {
    w.__TAURI_INTERNALS__ = internals;
  }
}

/** The browser's word on whether the user just clicked or typed. */
const activation = (isActive: boolean) => Object.defineProperty(navigator, 'userActivation', { value: { isActive }, configurable: true });

describe('a link opened in a browser', () => {
  afterEach(() => {
    delete (navigator as { userActivation?: unknown }).userActivation;
    vi.restoreAllMocks();
  });

  it('opens at once after a click of the user', async () => {
    const { openUrl, app } = await inBrowser();
    const opened = vi.spyOn(window, 'open').mockReturnValue(null);
    activation(true);
    await openUrl('https://example.com/a');
    expect(opened).toHaveBeenCalledWith('https://example.com/a', '_blank', 'noopener');
    expect(app.toasts).toEqual([]);
  });

  it('without one, waits in a toast for the click the browser asks for', async () => {
    const { openUrl, app } = await inBrowser();
    const opened = vi.spyOn(window, 'open').mockReturnValue(null);
    activation(false);
    await openUrl('http://localhost:5173/');
    expect(opened).not.toHaveBeenCalled();
    const [toast] = app.toasts;
    expect(toast.text).toContain('http://localhost:5173/');
    toast.action!.onClick();
    expect(opened).toHaveBeenCalledWith('http://localhost:5173/', '_blank', 'noopener');
  });

  it('a link the core would refuse is refused at the toast’s click too', async () => {
    const { openUrl, app } = await inBrowser();
    const opened = vi.spyOn(window, 'open').mockReturnValue(null);
    activation(false);
    await openUrl('javascript:alert(1)');
    app.toasts[0].action!.onClick();
    await Promise.resolve();
    await Promise.resolve();
    expect(opened).not.toHaveBeenCalled();
    expect(app.toasts.map((t) => t.kind)).toEqual(['info', 'error']);
  });
});
