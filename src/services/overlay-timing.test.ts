import { describe, expect, it } from "vitest";

import {
  formatRelativeSeconds,
  placeOverlayEndAtPlayhead,
  placeOverlayStartAtPlayhead,
  relativeOverlayTiming,
} from "./overlay-timing";

describe("overlay timing presentation", () => {
  it("describes overlay timing relative to the selected clip", () => {
    expect(relativeOverlayTiming(10_500, 11_000, 10_000, 20_000)).toEqual({
      startOffsetMs: 500,
      durationMs: 500,
      leftPercent: 5,
      widthPercent: 5,
    });
  });

  it("clips the visual range without hiding the real relative values", () => {
    expect(relativeOverlayTiming(9_500, 20_500, 10_000, 20_000)).toEqual({
      startOffsetMs: -500,
      durationMs: 11_000,
      leftPercent: 0,
      widthPercent: 100,
    });
  });

  it("formats seconds compactly while keeping useful precision", () => {
    expect(formatRelativeSeconds(0)).toBe("0.00s");
    expect(formatRelativeSeconds(500)).toBe("0.50s");
    expect(formatRelativeSeconds(1_250)).toBe("1.25s");
    expect(formatRelativeSeconds(333)).toBe("0.333s");
    expect(formatRelativeSeconds(-500)).toBe("−0.50s");
  });

  it("moves a minimum-duration overlay start to the playhead", () => {
    expect(
      placeOverlayStartAtPlayhead(
        12_000,
        10_000,
        10_500,
        10_000,
        20_000,
        500,
        1_680,
      ),
    ).toEqual({ startMs: 12_000, endMs: 12_500 });
  });

  it("moves an overlay end to the playhead while preserving duration", () => {
    expect(
      placeOverlayEndAtPlayhead(
        12_000,
        10_000,
        10_500,
        10_000,
        20_000,
        500,
        1_680,
      ),
    ).toEqual({ startMs: 11_500, endMs: 12_000 });
  });

  it("keeps playhead placement inside the active range", () => {
    expect(
      placeOverlayStartAtPlayhead(
        20_000,
        10_000,
        10_500,
        10_000,
        20_000,
        500,
      ),
    ).toEqual({ startMs: 19_500, endMs: 20_000 });
    expect(
      placeOverlayEndAtPlayhead(
        9_000,
        10_000,
        10_500,
        10_000,
        20_000,
        500,
      ),
    ).toEqual({ startMs: 10_000, endMs: 10_500 });
  });
});
