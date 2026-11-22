// In a browser, the system notifications and the chime of escouade-server (`notify`, `chime`), which has neither: the
// desktop app's host shows and plays its own, its window never gets these events.
import { api } from './ipc';

/** The tab is shown and has the focus: the desktop's `window_attended`. */
const attended = () => document.visibilityState === 'visible' && document.hasFocus();

/** The permission asked at the user's first gesture while `wanted` (the system notifications are on): a browser asks
 *  for none without one. Asked again at the next gesture until it is answered (a key may be no gesture for it). */
export function askOnFirstGesture(wanted: () => boolean) {
  if (typeof Notification === 'undefined' || Notification.permission !== 'default') return;
  let asking = false;
  const ask = () => {
    if (asking || !wanted()) return;
    asking = true;
    void Notification.requestPermission()
      .then((answer) => {
        if (answer === 'default') return;
        removeEventListener('pointerdown', ask, true);
        removeEventListener('keydown', ask, true);
      })
      .catch(() => {})
      .finally(() => (asking = false));
  };
  addEventListener('pointerdown', ask, true);
  addEventListener('keydown', ask, true);
}

/** Shown by the browser, its click as the desktop's: the tab brought back, the server told. False when the browser shows
 *  none (the tab attended, no permission), for the page to say it itself. */
export function notify(id: number, title: string, body: string): boolean {
  if (attended() || typeof Notification === 'undefined' || Notification.permission !== 'granted') return false;
  // One per alert, whatever the tabs open: the system keeps one of a tag.
  const shown = new Notification(title, { body, tag: `escouade-${id}` });
  shown.onclick = () => {
    window.focus();
    shown.close();
    api.notificationClick(id).catch(() => {});
  };
  return true;
}

let audio: AudioContext | null = null;

/** The desktop's chime (notify.rs): 880 Hz then 1318.5 Hz, 0.13 s apart, each fading out in 0.65 s. */
export function chime() {
  if (typeof AudioContext === 'undefined') return;
  audio ??= new AudioContext();
  const ctx = audio;
  // Suspended until a gesture of the user: then silent, as the browser wants. Played only once it runs: notes set on a
  // clock that stands still would all ring together at the first gesture.
  ctx
    .resume()
    .then(() => ctx.state === 'running' && play(ctx))
    .catch(() => {});
}

function play(ctx: AudioContext) {
  [880, 1318.5].forEach((freq, i) => {
    const at = ctx.currentTime + i * 0.13;
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.value = freq;
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(0.28, at + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.0004, at + 0.6);
    osc.connect(gain).connect(ctx.destination);
    osc.start(at);
    osc.stop(at + 0.65);
  });
}
