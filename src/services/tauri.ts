import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { AppError, RuntimeInfo } from "../contracts/runtime";
import type {
  AppErrorCode,
  AssetRef,
  ClipAnalysisEvent,
  CreateProjectResult,
  ExportEvent,
  ExportRequest,
  ExportValidation,
  LoadProjectResult,
  MediaProbe,
  ProjectV1,
  RecentProject,
  RelinkSourceResult,
  SaveProjectResult,
  StingAssetRef,
  YouTubeConnectionStatus,
  YouTubeProjectPerformance,
  YouTubeVideoCandidate,
} from "../../contracts/types";

export interface StartExportResponse {
  jobId: string;
  acceptedAt: string;
}

export interface CancelExportResponse {
  accepted: boolean;
}

export interface StartClipAnalysisResponse {
  jobId: string;
  acceptedAt: string;
}

export interface CancelClipAnalysisResponse {
  accepted: boolean;
}

export interface DiagnosticBundleResponse {
  path: string;
  sizeBytes: number;
}

export interface RevealResponse {
  opened: boolean;
}

export type InvokeFn = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

const APP_ERROR_CODES: ReadonlySet<AppErrorCode> = new Set([
  "E_INVALID_ARGUMENT",
  "E_MEDIA_UNSUPPORTED",
  "E_SOURCE_MISSING",
  "E_SOURCE_CHANGED",
  "E_PROJECT_SCHEMA",
  "E_ASSET_MISSING",
  "E_DESTINATION_DENIED",
  "E_OUTPUT_EXISTS",
  "E_DISK_SPACE",
  "E_FFPROBE_FAILED",
  "E_FFMPEG_FAILED",
  "E_EXPORT_ACTIVE",
  "E_EXPORT_NOT_FOUND",
  "E_EXPORT_CANCELLED",
  "E_ANALYSIS_ACTIVE",
  "E_ANALYSIS_NOT_FOUND",
  "E_ANALYSIS_FAILED",
  "E_INTEGRATION_UNAVAILABLE",
  "E_AUTH_REQUIRED",
  "E_NETWORK",
  "E_YOUTUBE_API",
  "E_IO",
  "E_INTERNAL",
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isAppErrorCode(value: unknown): value is AppErrorCode {
  return typeof value === "string" && APP_ERROR_CODES.has(value as AppErrorCode);
}

export function normalizeAppError(error: unknown): AppError {
  if (
    isRecord(error) &&
    isAppErrorCode(error.code) &&
    typeof error.message === "string" &&
    (typeof error.safeDetail === "string" || error.safeDetail === null) &&
    typeof error.retryable === "boolean"
  ) {
    return {
      code: error.code,
      message: error.message,
      safeDetail: error.safeDetail,
      retryable: error.retryable,
    };
  }

  return {
    code: "E_INTERNAL",
    message: "The native application returned an unexpected error.",
    safeDetail: null,
    retryable: true,
  };
}

export async function getRuntimeInfo(
  invokeCommand: InvokeFn = invoke,
): Promise<RuntimeInfo> {
  try {
    return await invokeCommand<RuntimeInfo>("get_runtime_info");
  } catch (error) {
    throw normalizeAppError(error);
  }
}

async function invokeNative<T>(
  command: string,
  args?: Record<string, unknown>,
  invokeCommand: InvokeFn = invoke,
): Promise<T> {
  try {
    return await invokeCommand<T>(command, args);
  } catch (error) {
    throw normalizeAppError(error);
  }
}

export function selectMediaFile(
  invokeCommand: InvokeFn = invoke,
): Promise<string | null> {
  return invokeNative("select_media_file", undefined, invokeCommand);
}

export function selectProjectFile(
  invokeCommand: InvokeFn = invoke,
): Promise<string | null> {
  return invokeNative("select_project_file", undefined, invokeCommand);
}

export function selectOverlayFile(
  invokeCommand: InvokeFn = invoke,
): Promise<string | null> {
  return invokeNative("select_overlay_file", undefined, invokeCommand);
}

export function selectStingFile(
  invokeCommand: InvokeFn = invoke,
): Promise<string | null> {
  return invokeNative("select_sting_file", undefined, invokeCommand);
}

export function probeMedia(
  path: string,
  invokeCommand: InvokeFn = invoke,
): Promise<MediaProbe> {
  return invokeNative("probe_media", { path }, invokeCommand);
}

export function createProject(
  sourcePath: string,
  projectName?: string,
  invokeCommand: InvokeFn = invoke,
): Promise<CreateProjectResult> {
  return invokeNative(
    "create_project",
    { sourcePath, projectName, projectsRoot: null },
    invokeCommand,
  );
}

export function loadProject(
  projectPath: string,
  invokeCommand: InvokeFn = invoke,
): Promise<LoadProjectResult> {
  return invokeNative("load_project", { projectPath }, invokeCommand);
}

export function saveProject(
  projectPath: string,
  project: ProjectV1,
  invokeCommand: InvokeFn = invoke,
): Promise<SaveProjectResult> {
  return invokeNative("save_project", { projectPath, project }, invokeCommand);
}

export function relinkSource(
  projectPath: string,
  replacementPath: string,
  acceptFingerprintMismatch: boolean,
  invokeCommand: InvokeFn = invoke,
): Promise<RelinkSourceResult> {
  return invokeNative(
    "relink_source",
    { projectPath, replacementPath, acceptFingerprintMismatch },
    invokeCommand,
  );
}

export function listRecentProjects(
  invokeCommand: InvokeFn = invoke,
): Promise<RecentProject[]> {
  return invokeNative("list_recent_projects", undefined, invokeCommand);
}

export function removeRecentProject(
  projectPath: string,
  invokeCommand: InvokeFn = invoke,
): Promise<void> {
  return invokeNative("remove_recent_project", { projectPath }, invokeCommand);
}

export function importOverlayAsset(
  projectPath: string,
  sourceAssetPath: string,
  invokeCommand: InvokeFn = invoke,
): Promise<AssetRef> {
  return invokeNative(
    "import_overlay_asset",
    { projectPath, sourceAssetPath },
    invokeCommand,
  );
}

export function importStingAsset(
  projectPath: string,
  sourceAssetPath: string,
  invokeCommand: InvokeFn = invoke,
): Promise<StingAssetRef> {
  return invokeNative(
    "import_sting_asset",
    { projectPath, sourceAssetPath },
    invokeCommand,
  );
}

export function writeCaptionAsset(
  projectPath: string,
  contentHash: string,
  pngBytesBase64: string,
  width: number,
  height: number,
  invokeCommand: InvokeFn = invoke,
): Promise<AssetRef> {
  return invokeNative(
    "write_caption_asset",
    { projectPath, contentHash, pngBytesBase64, width, height },
    invokeCommand,
  );
}

export function selectExportDestination(
  suggestedName: string,
  invokeCommand: InvokeFn = invoke,
): Promise<string | null> {
  return invokeNative(
    "select_export_destination",
    { suggestedName },
    invokeCommand,
  );
}

export function validateExport(
  request: ExportRequest,
  invokeCommand: InvokeFn = invoke,
): Promise<ExportValidation> {
  return invokeNative("validate_export", { request }, invokeCommand);
}

export function startExport(
  request: ExportRequest,
  invokeCommand: InvokeFn = invoke,
): Promise<StartExportResponse> {
  return invokeNative("start_export", { request }, invokeCommand);
}

export function cancelExport(
  jobId: string,
  invokeCommand: InvokeFn = invoke,
): Promise<CancelExportResponse> {
  return invokeNative("cancel_export", { jobId }, invokeCommand);
}

export function startClipAnalysis(
  sourcePath: string,
  invokeCommand: InvokeFn = invoke,
): Promise<StartClipAnalysisResponse> {
  return invokeNative(
    "start_clip_analysis",
    { sourcePath },
    invokeCommand,
  );
}

export function cancelClipAnalysis(
  jobId: string,
  invokeCommand: InvokeFn = invoke,
): Promise<CancelClipAnalysisResponse> {
  return invokeNative("cancel_clip_analysis", { jobId }, invokeCommand);
}

export function selectDiagnosticDestination(
  suggestedName: string,
  invokeCommand: InvokeFn = invoke,
): Promise<string | null> {
  return invokeNative(
    "select_diagnostic_destination",
    { suggestedName },
    invokeCommand,
  );
}

export function createDiagnosticBundle(
  destinationZipPath: string,
  projectPath?: string,
  invokeCommand: InvokeFn = invoke,
): Promise<DiagnosticBundleResponse> {
  return invokeNative(
    "create_diagnostic_bundle",
    { destinationZipPath, projectPath: projectPath ?? null },
    invokeCommand,
  );
}

export function revealInFolder(
  path: string,
  invokeCommand: InvokeFn = invoke,
): Promise<RevealResponse> {
  return invokeNative("reveal_in_folder", { path }, invokeCommand);
}

export function getYouTubeConnectionStatus(
  invokeCommand: InvokeFn = invoke,
): Promise<YouTubeConnectionStatus> {
  return invokeNative(
    "get_youtube_connection_status",
    undefined,
    invokeCommand,
  );
}

export function connectYouTubeChannel(
  invokeCommand: InvokeFn = invoke,
): Promise<YouTubeConnectionStatus> {
  return invokeNative("connect_youtube_channel", undefined, invokeCommand);
}

export function disconnectYouTubeChannel(
  invokeCommand: InvokeFn = invoke,
): Promise<YouTubeConnectionStatus> {
  return invokeNative("disconnect_youtube_channel", undefined, invokeCommand);
}

export function listRecentYouTubeUploads(
  invokeCommand: InvokeFn = invoke,
): Promise<YouTubeVideoCandidate[]> {
  return invokeNative(
    "list_recent_youtube_uploads",
    undefined,
    invokeCommand,
  );
}

export function linkProjectToYouTubeVideo(
  projectId: string,
  projectName: string,
  videoIdOrUrl: string,
  invokeCommand: InvokeFn = invoke,
): Promise<YouTubeProjectPerformance> {
  return invokeNative(
    "link_project_to_youtube_video",
    { projectId, projectName, videoIdOrUrl },
    invokeCommand,
  );
}

export function listYouTubePerformance(
  invokeCommand: InvokeFn = invoke,
): Promise<YouTubeProjectPerformance[]> {
  return invokeNative("list_youtube_performance", undefined, invokeCommand);
}

export function syncYouTubePerformance(
  projectId?: string,
  invokeCommand: InvokeFn = invoke,
): Promise<YouTubeProjectPerformance[]> {
  return invokeNative(
    "sync_youtube_performance",
    { projectId: projectId ?? null },
    invokeCommand,
  );
}

export async function listenForExportEvents(
  onEvent: (event: ExportEvent) => void,
): Promise<UnlistenFn> {
  const eventNames: ExportEvent["event"][] = [
    "export://progress",
    "export://completed",
    "export://failed",
    "export://cancelled",
  ];
  const unlisteners = await Promise.all(
    eventNames.map((eventName) =>
      listen<ExportEvent>(eventName, ({ payload }) => onEvent(payload)),
    ),
  );
  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}

export async function listenForClipAnalysisEvents(
  onEvent: (event: ClipAnalysisEvent) => void,
): Promise<UnlistenFn> {
  const eventNames: ClipAnalysisEvent["event"][] = [
    "clip-analysis://progress",
    "clip-analysis://completed",
    "clip-analysis://failed",
    "clip-analysis://cancelled",
  ];
  const unlisteners = await Promise.all(
    eventNames.map((eventName) =>
      listen<ClipAnalysisEvent>(eventName, ({ payload }) => onEvent(payload)),
    ),
  );
  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}

export function listenForFileDrops(
  onDrop: (paths: string[]) => void,
  onActiveChange: (active: boolean) => void,
): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent(({ payload }) => {
    switch (payload.type) {
      case "enter":
        onActiveChange(true);
        break;
      case "drop":
        onActiveChange(false);
        onDrop(payload.paths);
        break;
      case "leave":
        onActiveChange(false);
        break;
      case "over":
        break;
    }
  });
}

export function mediaPreviewUrl(path: string): string {
  return convertFileSrc(path);
}

export function projectAssetPath(
  projectPath: string,
  relativePath: string,
): string {
  const lastSeparator = Math.max(
    projectPath.lastIndexOf("/"),
    projectPath.lastIndexOf("\\"),
  );
  if (lastSeparator < 0) {
    throw new Error("Project path does not contain a parent folder");
  }
  const separator = projectPath.includes("\\") ? "\\" : "/";
  const relative = relativePath.replaceAll(/[\\/]/g, separator);
  return `${projectPath.slice(0, lastSeparator)}${separator}${relative}`;
}

export function projectAssetPreviewUrl(
  projectPath: string,
  relativePath: string,
): string {
  return convertFileSrc(projectAssetPath(projectPath, relativePath));
}
