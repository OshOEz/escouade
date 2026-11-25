// The camera of the « Workspace » (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md, « La toile »): where
// the canvas is seen from and how close. Pure, so the view only applies it.

/** The point of the screen where the world's origin is drawn, and the scale it is drawn at. */
export type Camera = { x: number; y: number; zoom: number };

export const ZOOM_MIN = 0.3;
export const ZOOM_MAX = 2.5;
/** One click of « + » or « − ». */
export const ZOOM_STEP = 1.25;
/** The step of the dotted background, at a zoom of 1 (px). */
export const DOT = 18;

/** Below this zoom the agents are discs seen from afar; above it, their compass. */
export const FAR_BELOW = 0.45;
export type Altitude = 'far' | 'near';

export const clampZoom = (z: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z));

/** Zooms by `factor` around the point (px, py) of the screen, which stays where it is. */
export function zoomAt(cam: Camera, factor: number, px: number, py: number): Camera {
  const zoom = clampZoom(cam.zoom * factor);
  const k = zoom / cam.zoom;
  return { x: px - (px - cam.x) * k, y: py - (py - cam.y) * k, zoom };
}

export const pan = (cam: Camera, dx: number, dy: number): Camera => ({ x: cam.x + dx, y: cam.y + dy, zoom: cam.zoom });

/** The camera that shows the whole world in the view, centred, with `margin` around it. */
export function fit(world: { w: number; h: number }, view: { w: number; h: number }, margin = 32): Camera {
  if (!world.w || !world.h) return { x: view.w / 2, y: view.h / 2, zoom: 1 };
  const zoom = clampZoom(Math.min((view.w - 2 * margin) / world.w, (view.h - 2 * margin) / world.h));
  return { x: (view.w - world.w * zoom) / 2, y: (view.h - world.h * zoom) / 2, zoom };
}

export const altitude = (zoom: number): Altitude => (zoom < FAR_BELOW ? 'far' : 'near');
