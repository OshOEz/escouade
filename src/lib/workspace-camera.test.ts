import { describe, expect, it } from 'vitest';
import { altitude, clampZoom, fit, pan, zoomAt } from './workspace-camera';

describe('the camera of the workspace', () => {
  it('clamps the zoom', () => {
    expect(clampZoom(0.1)).toBe(0.3);
    expect(clampZoom(9)).toBe(2.5);
  });

  it('keeps the point under the pointer fixed', () => {
    const c = zoomAt({ x: 0, y: 0, zoom: 1 }, 2, 100, 50);
    expect(c).toEqual({ x: -100, y: -50, zoom: 2 });
    expect((100 - c.x) / c.zoom).toBe(100);
  });

  it('does not move when the zoom is already at its bound', () => {
    expect(zoomAt({ x: 10, y: 20, zoom: 2.5 }, 2, 0, 0)).toEqual({ x: 10, y: 20, zoom: 2.5 });
  });

  it('pans', () => {
    expect(pan({ x: 1, y: 2, zoom: 1.5 }, 10, -5)).toEqual({ x: 11, y: -3, zoom: 1.5 });
  });

  it('fits a world in the view, centred', () => {
    const c = fit({ w: 1000, h: 500 }, { w: 1064, h: 800 });
    expect(c.zoom).toBe(1);
    expect(c.x).toBe(32);
    expect(c.y).toBe(150);
  });

  it('fits within the zoom bounds, even in a tiny view', () => {
    expect(fit({ w: 5000, h: 5000 }, { w: 200, h: 100 }).zoom).toBe(0.3);
    expect(fit({ w: 10, h: 10 }, { w: 2000, h: 2000 }).zoom).toBe(2.5);
  });

  it('centres an empty world', () => {
    expect(fit({ w: 0, h: 0 }, { w: 800, h: 600 })).toEqual({ x: 400, y: 300, zoom: 1 });
  });

  it('tells the altitude', () => {
    expect(altitude(0.44)).toBe('far');
    expect(altitude(0.45)).toBe('near');
  });
});
