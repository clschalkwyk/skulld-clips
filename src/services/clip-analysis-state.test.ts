import { describe, expect, it } from "vitest";

import type { ClipAnalysisEvent, ClipCandidate } from "../../contracts/types";
import {
  applyMomentExtractionWindow,
  clipAnalysisIsActive,
  createClipAnalysisState,
  reduceClipAnalysisEvent,
  validateMomentExtractionWindow,
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
          detectedStartMs: 25_000,
          detectedEndMs: 25_000,
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

  it("recalculates point and interval candidates without rescanning", () => {
    const pointCandidate: ClipCandidate = {
      id: "candidate-1",
      kind: "death",
      eventMs: 25_000,
      detectedStartMs: 25_000,
      detectedEndMs: 25_000,
      suggestedInMs: 10_000,
      suggestedOutMs: 30_000,
      confidence: 0.84,
      evidence: [],
    };
    expect(
      applyMomentExtractionWindow(pointCandidate, 100_000, {
        momentExtractStartTimeMs: 15_000,
        momentExtractEndTimeMs: 5_000,
      }),
    ).toMatchObject({
      suggestedInMs: 10_000,
      suggestedOutMs: 30_000,
    });

    const bossCandidate: ClipCandidate = {
      ...pointCandidate,
      id: "candidate-2",
      kind: "bossEncounter",
      eventMs: 45_000,
      detectedStartMs: 40_000,
      detectedEndMs: 50_000,
    };
    expect(
      applyMomentExtractionWindow(bossCandidate, 100_000, {
        momentExtractStartTimeMs: 15_000,
        momentExtractEndTimeMs: 5_000,
      }),
    ).toMatchObject({
      suggestedInMs: 25_000,
      suggestedOutMs: 55_000,
    });
  });

  it("clamps extraction ranges and rejects invalid settings", () => {
    const candidate: ClipCandidate = {
      id: "candidate-1",
      kind: "completion",
      eventMs: 5_000,
      detectedStartMs: 5_000,
      detectedEndMs: 5_000,
      suggestedInMs: 0,
      suggestedOutMs: 10_000,
      confidence: 0.9,
      evidence: [],
    };
    expect(
      applyMomentExtractionWindow(candidate, 8_000, {
        momentExtractStartTimeMs: 15_000,
        momentExtractEndTimeMs: 5_000,
      }),
    ).toMatchObject({
      suggestedInMs: 0,
      suggestedOutMs: 8_000,
    });
    expect(
      applyMomentExtractionWindow(candidate, 8_000, {
        momentExtractStartTimeMs: 250,
        momentExtractEndTimeMs: 0,
      }),
    ).toMatchObject({
      suggestedInMs: 4_750,
      suggestedOutMs: 5_000,
    });
    expect(
      applyMomentExtractionWindow(
        {
          ...candidate,
          eventMs: 0,
          detectedStartMs: 0,
          detectedEndMs: 0,
        },
        8_000,
        {
          momentExtractStartTimeMs: 250,
          momentExtractEndTimeMs: 0,
        },
      ),
    ).toMatchObject({
      suggestedInMs: 0,
      suggestedOutMs: 250,
    });
    expect(
      validateMomentExtractionWindow({
        momentExtractStartTimeMs: 0,
        momentExtractEndTimeMs: 0,
      }),
    ).not.toBeNull();
    expect(
      validateMomentExtractionWindow({
        momentExtractStartTimeMs: -1,
        momentExtractEndTimeMs: 5_000,
      }),
    ).not.toBeNull();
    expect(
      validateMomentExtractionWindow({
        momentExtractStartTimeMs: Number.NaN,
        momentExtractEndTimeMs: 5_000,
      }),
    ).not.toBeNull();
    expect(
      validateMomentExtractionWindow({
        momentExtractStartTimeMs: 300_001,
        momentExtractEndTimeMs: 5_000,
      }),
    ).not.toBeNull();
    expect(
      validateMomentExtractionWindow({
        momentExtractStartTimeMs: 15_000,
        momentExtractEndTimeMs: 5_000,
      }),
    ).toBeNull();
  });
});
