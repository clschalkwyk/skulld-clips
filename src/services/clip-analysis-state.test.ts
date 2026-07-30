import { describe, expect, it } from "vitest";

import type { ClipAnalysisEvent } from "../../contracts/types";
import {
  clipAnalysisIsActive,
  createClipAnalysisState,
  reduceClipAnalysisEvent,
} from "./clip-analysis-state";

describe("clip analysis state", () => {
  it("accepts progress before the start command resolves", () => {
    const state = {
      ...createClipAnalysisState(),
      status: "starting" as const,
    };
    const event: ClipAnalysisEvent = {
      event: "clip-analysis://progress",
      jobId: "analysis-1",
      progress: 0.4,
      analyzedMs: 40_000,
      totalMs: 100_000,
    };

    const next = reduceClipAnalysisEvent(state, event);

    expect(next).toMatchObject({
      status: "running",
      jobId: "analysis-1",
      progress: 0.4,
    });
    expect(clipAnalysisIsActive(next)).toBe(true);
  });

  it("records candidates only for the current job", () => {
    const state = {
      ...createClipAnalysisState(),
      status: "running" as const,
      jobId: "analysis-1",
    };
    const stale: ClipAnalysisEvent = {
      event: "clip-analysis://completed",
      jobId: "analysis-old",
      candidates: [],
    };
    expect(reduceClipAnalysisEvent(state, stale)).toBe(state);

    const completed: ClipAnalysisEvent = {
      event: "clip-analysis://completed",
      jobId: "analysis-1",
      candidates: [
        {
          id: "candidate-1",
          kind: "death",
          eventMs: 25_000,
          suggestedInMs: 10_000,
          suggestedOutMs: 30_000,
          confidence: 0.84,
          evidence: ["Wide pale title over red death treatment"],
        },
      ],
    };
    expect(reduceClipAnalysisEvent(state, completed)).toMatchObject({
      status: "completed",
      progress: 1,
      candidates: completed.candidates,
    });
  });
});
