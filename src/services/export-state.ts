import type {
  AppError,
  ExportEvent,
  ExportValidation,
} from "../../contracts/types";

export type ExportStatus =
  | "idle"
  | "validating"
  | "starting"
  | "running"
  | "completed"
  | "cancelled"
  | "error";

export interface ExportState {
  status: ExportStatus;
  jobId: string | null;
  validation: ExportValidation | null;
  progress: number;
  phase: string | null;
  encodedMs: number;
  totalMs: number;
  fps: number | null;
  speed: number | null;
  outputBytes: number | null;
  outputPath: string | null;
  error: AppError | null;
  cancelRequested: boolean;
}

export function createExportState(): ExportState {
  return {
    status: "idle",
    jobId: null,
    validation: null,
    progress: 0,
    phase: null,
    encodedMs: 0,
    totalMs: 0,
    fps: null,
    speed: null,
    outputBytes: null,
    outputPath: null,
    error: null,
    cancelRequested: false,
  };
}

export function reduceExportEvent(
  state: ExportState,
  event: ExportEvent,
): ExportState {
  if (
    state.jobId !== null &&
    state.jobId !== event.jobId
  ) {
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
    case "export://progress":
      return {
        ...state,
        status: "running",
        jobId: event.jobId,
        progress: event.progress,
        phase: event.phase,
        encodedMs: event.encodedMs,
        totalMs: event.totalMs,
        fps: event.fps,
        speed: event.speed,
        outputBytes: event.outputBytes,
        error: null,
      };
    case "export://completed":
      return {
        ...state,
        status: "completed",
        jobId: event.jobId,
        progress: 1,
        phase: null,
        encodedMs: event.durationMs,
        totalMs: event.durationMs,
        outputBytes: event.sizeBytes,
        outputPath: event.outputPath,
        error: null,
        cancelRequested: false,
      };
    case "export://failed":
      return {
        ...state,
        status: "error",
        jobId: event.jobId,
        phase: null,
        error: event.error,
        cancelRequested: false,
      };
    case "export://cancelled":
      return {
        ...state,
        status: "cancelled",
        jobId: event.jobId,
        phase: null,
        cancelRequested: false,
      };
  }
}

export function exportIsActive(state: ExportState): boolean {
  return state.status === "starting" || state.status === "running";
}
