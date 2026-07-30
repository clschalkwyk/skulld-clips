import type {
  AppError,
  ClipAnalysisEvent,
  ClipCandidate,
} from "../../contracts/types";

export type ClipAnalysisStatus =
  | "idle"
  | "starting"
  | "running"
  | "completed"
  | "cancelled"
  | "error";

export interface ClipAnalysisState {
  status: ClipAnalysisStatus;
  jobId: string | null;
  progress: number;
  analyzedMs: number;
  totalMs: number;
  candidates: ClipCandidate[];
  error: AppError | null;
  cancelRequested: boolean;
}

export interface MomentExtractionWindow {
  momentExtractStartTimeMs: number;
  momentExtractEndTimeMs: number;
}

export const DEFAULT_MOMENT_EXTRACTION_WINDOW: MomentExtractionWindow = {
  momentExtractStartTimeMs: 15_000,
  momentExtractEndTimeMs: 5_000,
};

export const MAX_MOMENT_EXTRACTION_OFFSET_MS = 300_000;

export function validateMomentExtractionWindow(
  window: MomentExtractionWindow,
): string | null {
  const { momentExtractStartTimeMs, momentExtractEndTimeMs } = window;
  if (
    !Number.isFinite(momentExtractStartTimeMs) ||
    !Number.isFinite(momentExtractEndTimeMs)
  ) {
    return "Enter a valid number of seconds for both extraction offsets.";
  }
  if (momentExtractStartTimeMs < 0 || momentExtractEndTimeMs < 0) {
    return "Extraction offsets cannot be negative.";
  }
  if (
    momentExtractStartTimeMs > MAX_MOMENT_EXTRACTION_OFFSET_MS ||
    momentExtractEndTimeMs > MAX_MOMENT_EXTRACTION_OFFSET_MS
  ) {
    return "Each extraction offset must be five minutes or less.";
  }
  if (momentExtractStartTimeMs + momentExtractEndTimeMs < 250) {
    return "The extraction window must be at least 0.25 seconds.";
  }
  return null;
}

export function applyMomentExtractionWindow(
  candidate: ClipCandidate,
  sourceDurationMs: number,
  window: MomentExtractionWindow,
): ClipCandidate {
  const momentExtractStartTimeMs = Math.round(
    window.momentExtractStartTimeMs,
  );
  const momentExtractEndTimeMs = Math.round(window.momentExtractEndTimeMs);
  let suggestedInMs = Math.max(
    0,
    candidate.detectedStartMs - momentExtractStartTimeMs,
  );
  let suggestedOutMs = Math.min(
    sourceDurationMs,
    candidate.detectedEndMs + momentExtractEndTimeMs,
  );
  if (suggestedOutMs - suggestedInMs < 250) {
    suggestedOutMs = Math.min(sourceDurationMs, suggestedInMs + 250);
    suggestedInMs = Math.max(
      0,
      Math.min(suggestedInMs, suggestedOutMs - 250),
    );
  }
  return {
    ...candidate,
    suggestedInMs,
    suggestedOutMs,
  };
}

export function createClipAnalysisState(): ClipAnalysisState {
  return {
    status: "idle",
    jobId: null,
    progress: 0,
    analyzedMs: 0,
    totalMs: 0,
    candidates: [],
    error: null,
    cancelRequested: false,
  };
}

export function reduceClipAnalysisEvent(
  state: ClipAnalysisState,
  event: ClipAnalysisEvent,
): ClipAnalysisState {
  if (state.jobId !== null && state.jobId !== event.jobId) {
    return state;
  }
  if (
    state.jobId === null &&
    state.status !== "starting" &&
    state.status !== "running"
  ) {
    return state;
  }

  switch (event.event) {
    case "clip-analysis://progress":
      return {
        ...state,
        status: "running",
        jobId: event.jobId,
        progress: event.progress,
        analyzedMs: event.analyzedMs,
        totalMs: event.totalMs,
        error: null,
      };
    case "clip-analysis://completed":
      return {
        ...state,
        status: "completed",
        jobId: event.jobId,
        progress: 1,
        analyzedMs: state.totalMs,
        candidates: event.candidates,
        error: null,
        cancelRequested: false,
      };
    case "clip-analysis://failed":
      return {
        ...state,
        status: "error",
        jobId: event.jobId,
        error: event.error,
        cancelRequested: false,
      };
    case "clip-analysis://cancelled":
      return {
        ...state,
        status: "cancelled",
        jobId: event.jobId,
        cancelRequested: false,
      };
  }
}

export function clipAnalysisIsActive(state: ClipAnalysisState): boolean {
  return state.status === "starting" || state.status === "running";
}
