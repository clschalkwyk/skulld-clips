import { describe, expect, it } from "vitest";

import type { AssetRef, Overlay, StingAssetRef } from "../../contracts/types";
import {
  createImageOverlay,
  createStingOverlay,
  isOverlayVisible,
  moveOverlay,
  reorderOverlay,
  resizeOverlay,
  stingDisplayX,
} from "./overlay-model";

const asset: AssetRef = {
  relativePath: "assets/overlays/logo.png",
  sha256: "a".repeat(64),
  width: 800,
  height: 400,
  mimeType: "image/png",
  originalFilename: "logo.png",
};
const stingAsset: StingAssetRef = {
  ...asset,
  relativePath: "assets/stings/sting.mp4",
  mimeType: "video/mp4",
  originalFilename: "skulld-sting.mp4",
  width: 832,
  height: 832,
  durationMs: 5_042,
  hasAudio: true,
  preview: {
    relativePath: "assets/stings/sting.preview.png",
    sha256: "b".repeat(64),
    width: 960,
    height: 768,
    frameWidth: 192,
    frameHeight: 192,
    columns: 5,
    rows: 4,
    frameCount: 20,
    framesPerSecond: 12,
  },
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

  it("creates the fixed Toasty-right sting and computes its entrance and exit", () => {
    const overlay = createStingOverlay(
      "00000000-0000-4000-8000-000000000001",
      stingAsset,
      1_000,
      10_000,
      2,
    );

    expect(overlay.startMs).toBe(3_000);
    expect(overlay.endMs).toBe(4_680);
    expect(overlay.position).toEqual({
      x: 0.57037,
      y: 0.729167,
      width: 0.407407,
      height: 0.229167,
    });
    expect(stingDisplayX(overlay, 3_000)).toBe(1);
    expect(stingDisplayX(overlay, 3_180)).toBeCloseTo(overlay.position.x);
    expect(stingDisplayX(overlay, 4_680)).toBe(1);
  });
});
