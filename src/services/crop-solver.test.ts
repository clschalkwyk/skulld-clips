import { describe, expect, it } from "vitest";

import {
  centeredMaximumCrop,
  cropZoom,
  isValidLockedCrop,
  moveCrop,
  nudgeCrop,
  resetCrop,
  setCropZoom,
  zoomCrop,
} from "./crop-solver";

describe("locked 9:16 crop solver", () => {
  it("creates maximum centered crops for landscape, portrait, and rotated display sizes", () => {
    expect(centeredMaximumCrop({ width: 1920, height: 1080 })).toEqual({
      x: 0.341797,
      y: 0,
      width: 0.316406,
      height: 1,
    });
    expect(centeredMaximumCrop({ width: 720, height: 1280 })).toEqual({
      x: 0,
      y: 0,
      width: 1,
      height: 1,
    });
    expect(centeredMaximumCrop({ width: 1080, height: 1920 })).toEqual({
      x: 0,
      y: 0,
      width: 1,
      height: 1,
    });
  });

  it("clamps pan, zoom, and pixel nudges inside the source", () => {
    const source = { width: 1920, height: 1080 };
    const initial = centeredMaximumCrop(source);
    const zoomed = zoomCrop(initial, 0.5, { x: 0.5, y: 0.5 }, source);
    const moved = moveCrop(zoomed, { x: 10, y: -10 });
    const nudged = nudgeCrop(moved, source, 10, 10);

    expect(zoomed).toEqual({
      x: 0.420898,
      y: 0.25,
      width: 0.158203,
      height: 0.5,
    });
    expect(moved.x + moved.width).toBeLessThanOrEqual(1.000001);
    expect(moved.y).toBe(0);
    expect(isValidLockedCrop(nudged, source)).toBe(true);
  });

  it("round-trips zoom controls and reset", () => {
    const source = { width: 1920, height: 1080 };
    const initial = centeredMaximumCrop(source);
    const zoomed = setCropZoom(initial, 4, source);

    expect(cropZoom(zoomed, source)).toBe(4);
    expect(resetCrop(source)).toEqual(initial);
    expect(isValidLockedCrop(zoomed, source)).toBe(true);
  });
});
