import { describe, expect, it } from "vitest";

import type { MediaProbe } from "../../contracts/types";
import {
  approximateFrameStepMs,
  clampPlayhead,
  formatTimelineTime,
  setTrimIn,
  setTrimOut,
} from "./timeline";

describe("timeline utilities", () => {
  it("enforces integer millisecond trim bounds and the 250ms minimum", () => {
    expect(setTrimIn(9_900.9, 10_000, 15_000)).toBe(9_750);
    expect(setTrimIn(-2, 10_000, 15_000)).toBe(0);
    expect(setTrimOut(100, 1_000, 15_000)).toBe(1_250);
    expect(setTrimOut(20_000, 1_000, 15_000)).toBe(15_000);
    expect(clampPlayhead(20_000, 1_000, 15_000)).toBe(15_000);
  });

  it("derives an approximate frame step from normalized probe data", () => {
    const probe = {
      video: { avgFrameRate: 60, realFrameRate: 59.94 },
    } as MediaProbe;
    expect(approximateFrameStepMs(probe)).toBe(17);
    probe.video.avgFrameRate = null;
    probe.video.realFrameRate = null;
    expect(approximateFrameStepMs(probe)).toBe(33);
  });

  it("formats timeline time as mm:ss.mmm", () => {
    expect(formatTimelineTime(65_007)).toBe("01:05.007");
    expect(formatTimelineTime(-2)).toBe("00:00.000");
  });
});
