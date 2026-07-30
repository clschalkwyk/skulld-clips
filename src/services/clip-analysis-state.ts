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
