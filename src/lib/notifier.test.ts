// The system notifications and the chime of escouade-server, in a browser.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend } from '../test/ipc';
import { askOnFirstGesture, chime, notify } from './notifier';

class FakeNotification {
  static permission: NotificationPermission = 'granted';
  static requestPermission = vi.fn(async () => 'granted' as NotificationPermission);
  static shown: FakeNotification[] = [];
  onclick: (() => void) | null = null;
  close = vi.fn();
  constructor(
    public title: string,
    public options: NotificationOptions,
  ) {
    FakeNotification.shown.push(this);
  }
}

let focused = false;
beforeEach(() => {
  FakeNotification.permission = 'granted';
  FakeNotification.requestPermission.mockClear();
  FakeNotification.shown = [];
  vi.stubGlobal('Notification', FakeNotification);
  focused = false;
  vi.spyOn(document, 'hasFocus').mockImplementation(() => focused);
});
afterEach(() => vi.unstubAllGlobals());

describe('notifier', () => {
  it('shows a notification out of sight; its click brings the tab back and tells the server once', async () => {
    const backend = fakeBackend();
    const focus = vi.spyOn(window, 'focus').mockImplementation(() => {});
    expect(notify(7, 'demo', 'Tour fini')).toBe(true);
    const [n] = FakeNotification.shown;
    // One per alert whatever the tabs open: the system keeps one of a tag.
    expect(n).toMatchObject({ title: 'demo', options: { body: 'Tour fini', tag: 'escouade-7' } });
    n.onclick!();
    expect(focus).toHaveBeenCalled();
    expect(n.close).toHaveBeenCalled();
    await vi.waitFor(() => expect(backend.called('notification_click').map((c) => c.args)).toEqual([{ id: 7 }]));
  });

  it('shows none while the tab is attended, or without the permission: the page says it instead', () => {
    focused = true;
    expect(notify(1, 'demo', 'x')).toBe(false);
    focused = false;
    FakeNotification.permission = 'denied';
    expect(notify(2, 'demo', 'x')).toBe(false);
    vi.stubGlobal('Notification', undefined);
    expect(notify(3, 'demo', 'x')).toBe(false);
    expect(FakeNotification.shown).toEqual([]);
  });

  it('asks the permission at the first gesture, once, and not when it was answered', async () => {
    FakeNotification.permission = 'default';
    askOnFirstGesture(() => true);
    expect(FakeNotification.requestPermission).not.toHaveBeenCalled();
    window.dispatchEvent(new Event('pointerdown'));
    window.dispatchEvent(new Event('keydown'));
    expect(FakeNotification.requestPermission).toHaveBeenCalledTimes(1);
    await Promise.resolve();
    window.dispatchEvent(new Event('pointerdown'));
    expect(FakeNotification.requestPermission).toHaveBeenCalledTimes(1);
    FakeNotification.permission = 'denied';
    askOnFirstGesture(() => true);
    window.dispatchEvent(new Event('pointerdown'));
    expect(FakeNotification.requestPermission).toHaveBeenCalledTimes(1);
  });

  it('asks nothing while the notifications are off, and again at the next gesture when it was not answered', async () => {
    FakeNotification.permission = 'default';
    let on = false;
    askOnFirstGesture(() => on);
    window.dispatchEvent(new Event('pointerdown'));
    expect(FakeNotification.requestPermission).not.toHaveBeenCalled();
    on = true;
    // A key that is no gesture for the browser: the request stays unanswered.
    FakeNotification.requestPermission.mockResolvedValueOnce('default');
    window.dispatchEvent(new Event('keydown'));
    expect(FakeNotification.requestPermission).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => {
      window.dispatchEvent(new Event('pointerdown'));
      expect(FakeNotification.requestPermission).toHaveBeenCalledTimes(2);
    });
    await new Promise((r) => setTimeout(r));
    window.dispatchEvent(new Event('pointerdown'));
    expect(FakeNotification.requestPermission).toHaveBeenCalledTimes(2);
  });

  it('plays the two notes of the desktop chime', async () => {
    const notes: number[] = [];
    const param = () => ({ value: 0, setValueAtTime: vi.fn(), linearRampToValueAtTime: vi.fn(), exponentialRampToValueAtTime: vi.fn() });
    class FakeAudio {
      currentTime = 0;
      destination = {};
      state = 'running';
      resume = vi.fn(async () => {});
      createOscillator() {
        const frequency = param();
        return {
          frequency,
          connect: (n: unknown) => n,
          start: () => notes.push(frequency.value),
          stop: vi.fn(),
        };
      }
      createGain() {
        return { gain: param(), connect: (n: unknown) => n };
      }
    }
    vi.stubGlobal('AudioContext', FakeAudio);
    chime();
    await vi.waitFor(() => expect(notes).toEqual([880, 1318.5]));
  });

  it('plays nothing while the browser holds the sound back: the chimes of before do not ring all at once', async () => {
    const notes: number[] = [];
    const param = () => ({ value: 0, setValueAtTime: vi.fn(), linearRampToValueAtTime: vi.fn(), exponentialRampToValueAtTime: vi.fn() });
    class HeldAudio {
      currentTime = 0;
      destination = {};
      state = 'suspended';
      // No gesture yet: the browser leaves it suspended.
      resume = vi.fn(async () => {});
      createOscillator() {
        const frequency = param();
        return { frequency, connect: (n: unknown) => n, start: () => notes.push(frequency.value), stop: vi.fn() };
      }
      createGain() {
        return { gain: param(), connect: (n: unknown) => n };
      }
    }
    vi.stubGlobal('AudioContext', HeldAudio);
    // A page of its own: the sound of the test before is another.
    vi.resetModules();
    const { chime: held } = await import('./notifier');
    held();
    held();
    await new Promise((r) => setTimeout(r));
    expect(notes).toEqual([]);
  });
});
