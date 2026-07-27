import { describe, expect, it } from "vitest";

import type { AssetRef, Overlay } from "../../contracts/types";
import {
  createImageOverlay,
  isOverlayVisible,
  moveOverlay,
  reorderOverlay,
  resizeOverlay,
} from "./overlay-model";

const asset: AssetRef = {
  relativePath: "assets/overlays/logo.png",
  sha256: "a".repeat(64),
  width: 800,
  height: 400,
  mimeType: "image/png",
  originalFilename: "logo.png",
};

describe("overlay model", () => {
  it("creates an aspect-correct centered image for the active trim", () => {
    const overlay = createImageOverlay(
      "00000000-0000-4000-8000-000000000000",
      asset,
      1_000,
      5_000,
      0,
    );

    expect(overlay.position).toEqual({
      x: 0.38,
      y: 0.46625,
      width: 0.24,
      height: 0.0675,
    });
    expect(isOverlayVisible(overlay, 999)).toBe(false);
    expect(isOverlayVisible(overlay, 1_000)).toBe(true);
    expect(isOverlayVisible(overlay, 5_000)).toBe(true);
  });

  it("clamps movement and preserves pixel aspect while resizing", () => {
    const position = {
      x: 0.38,
      y: 0.46625,
      width: 0.24,
      height: 0.0675,
    };
    expect(moveOverlay(position, 4, -4)).toEqual({
      ...position,
      x: 0.76,
      y: 0,
    });
    const resized = resizeOverlay(position, asset, 0.5);
    expect(resized).toEqual({
      x: 0.38,
      y: 0.46625,
      width: 0.5,
      height: 0.140625,
    });
  });

  it("moves overlays one z-order position at a time", () => {
    const overlays = [
      { id: "a", zIndex: 4 },
      { id: "b", zIndex: 9 },
      { id: "c", zIndex: 20 },
    ] as Overlay[];

    expect(reorderOverlay(overlays, "a", 1).map(({ id, zIndex }) => ({ id, zIndex }))).toEqual([
      { id: "b", zIndex: 0 },
      { id: "a", zIndex: 1 },
      { id: "c", zIndex: 2 },
    ]);
  });
});
