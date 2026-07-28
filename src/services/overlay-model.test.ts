import { describe, expect, it } from "vitest";

import type { AssetRef, Overlay, StingAssetRef } from "../../contracts/types";
import {
  anchorOverlay,
  createImageOverlay,
  createStingOverlay,
  insertStingOverlayAtPlayhead,
  isOverlayVisible,
  moveOverlay,
  nudgeOverlay,
  reorderOverlay,
  resizeOverlay,
  setStingDuration,
  setStingPlaybackRate,
  setStingRepeat,
  stingPlaybackRate,
  stingRepeats,
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

  it("anchors overlays inside the output safe area and nudges in output pixels", () => {
    const position = {
      x: 0.38,
      y: 0.46625,
      width: 0.24,
      height: 0.0675,
    };

    expect(anchorOverlay(position, "top-left")).toEqual({
      ...position,
      x: 0.022222,
      y: 0.041667,
    });
    expect(anchorOverlay(position, "center")).toEqual({
      ...position,
      x: 0.38,
      y: 0.46625,
    });
    expect(anchorOverlay(position, "bottom-right")).toEqual({
      ...position,
      x: 0.737778,
      y: 0.890833,
    });
    expect(nudgeOverlay(position, 10, -10)).toEqual({
      ...position,
      x: 0.389259,
      y: 0.461042,
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

  it("creates a normal-speed Toasty-right sting and computes its entrance and exit", () => {
    const overlay = createStingOverlay(
      "00000000-0000-4000-8000-000000000001",
      stingAsset,
      1_000,
      10_000,
      2,
    );

    expect(overlay.startMs).toBe(3_000);
    expect(overlay.endMs).toBe(8_042);
    expect(overlay.playbackRate).toBe(1);
    expect(overlay.repeat).toBe(false);
    expect(overlay.position).toEqual({
      x: 0.57037,
      y: 0.729167,
      width: 0.407407,
      height: 0.229167,
    });
    expect(stingDisplayX(overlay, 3_000)).toBe(1);
    expect(stingDisplayX(overlay, 3_180)).toBeCloseTo(overlay.position.x);
    expect(stingDisplayX(overlay, 8_042)).toBe(1);
  });

  it("keeps legacy stings at 3x until the user changes them", () => {
    const legacy = createStingOverlay(
      "00000000-0000-4000-8000-000000000001",
      stingAsset,
      1_000,
      10_000,
      2,
    );
    delete legacy.playbackRate;
    delete legacy.repeat;

    expect(stingPlaybackRate(legacy)).toBe(3);
    expect(stingRepeats(legacy)).toBe(false);
  });

  it("changes speed, repeat mode, and bounded duration", () => {
    const initial = createStingOverlay(
      "00000000-0000-4000-8000-000000000001",
      stingAsset,
      1_000,
      20_000,
      2,
    );
    const faster = setStingPlaybackRate(initial, 2, 20_000);
    expect(faster.endMs - faster.startMs).toBe(2_521);

    const repeating = setStingRepeat(faster, true, 20_000);
    expect(repeating.repeat).toBe(true);
    expect(repeating.endMs - repeating.startMs).toBe(5_042);

    const extended = setStingDuration(repeating, 12_000, 20_000);
    expect(extended.endMs - extended.startMs).toBe(12_000);

    const once = setStingRepeat(extended, false, 20_000);
    expect(once.repeat).toBe(false);
    expect(once.endMs - once.startMs).toBe(2_521);
  });

  it("inserts a new sting instance at the playhead with the selected settings", () => {
    const source = setStingRepeat(
      setStingPlaybackRate(
        createStingOverlay(
          "00000000-0000-4000-8000-000000000001",
          stingAsset,
          1_000,
          20_000,
          2,
        ),
        2,
        20_000,
      ),
      true,
      20_000,
    );
    const inserted = insertStingOverlayAtPlayhead(
      source,
      "00000000-0000-4000-8000-000000000002",
      12_000,
      1_000,
      20_000,
      7,
    );

    expect(inserted.id).toBe("00000000-0000-4000-8000-000000000002");
    expect(inserted.startMs).toBe(12_000);
    expect(inserted.endMs - inserted.startMs).toBe(source.endMs - source.startMs);
    expect(inserted.playbackRate).toBe(2);
    expect(inserted.repeat).toBe(true);
    expect(inserted.position).toEqual(source.position);
    expect(inserted.position).not.toBe(source.position);
    expect(inserted.zIndex).toBe(7);
  });

  it("keeps an inserted sting inside the clip when the playhead is near the end", () => {
    const source = createStingOverlay(
      "00000000-0000-4000-8000-000000000001",
      stingAsset,
      1_000,
      10_000,
      2,
    );

    const inserted = insertStingOverlayAtPlayhead(
      source,
      "00000000-0000-4000-8000-000000000002",
      9_900,
      1_000,
      10_000,
      3,
    );

    expect(inserted.startMs).toBe(9_500);
    expect(inserted.endMs).toBe(10_000);
  });
});
