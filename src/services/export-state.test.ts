import { describe, expect, it } from "vitest";

import type { ExportEvent } from "../../contracts/types";
import {
  createExportState,
  exportIsActive,
  reduceExportEvent,
} from "./export-state";

describe("export state", () => {
  it("accepts an early progress event while start is pending", () => {
    const state = { ...createExportState(), status: "starting" as const };
    const event: ExportEvent = {
      event: "export://progress",
      jobId: "job-1",
      phase: "encoding",
      progress: 0.42,
      encodedMs: 4_200,
      totalMs: 10_000,
      fps: 58,
      speed: 1.3,
      outputBytes: 8_000,
    };

    const next = reduceExportEvent(state, event);

    expect(next).toMatchObject({
      status: "running",
      jobId: "job-1",
      progress: 0.42,
      phase: "encoding",
    });
    expect(exportIsActive(next)).toBe(true);
  });

  it("ignores stale events from a different job", () => {
    const state = {
      ...createExportState(),
      status: "running" as const,
      jobId: "current-job",
    };
    const stale: ExportEvent = {
      event: "export://cancelled",
      jobId: "old-job",
    };

    expect(reduceExportEvent(state, stale)).toBe(state);
  });

  it("records verified completion as the terminal state", () => {
    const state = {
      ...createExportState(),
      status: "running" as const,
      jobId: "job-1",
    };
    const completed: ExportEvent = {
      event: "export://completed",
      jobId: "job-1",
      outputPath: "/exports/clip.mp4",
      durationMs: 15_000,
      sizeBytes: 2_400_000,
    };

    expect(reduceExportEvent(state, completed)).toMatchObject({
      status: "completed",
      progress: 1,
      outputPath: "/exports/clip.mp4",
      outputBytes: 2_400_000,
    });
  });
});
