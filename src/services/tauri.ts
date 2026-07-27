import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { UnlistenFn } from "@tauri-apps/api/event";

import type { AppError, RuntimeInfo } from "../contracts/runtime";
import type {
  AppErrorCode,
  AssetRef,
  CreateProjectResult,
  LoadProjectResult,
  MediaProbe,
  ProjectV1,
  RecentProject,
  RelinkSourceResult,
  SaveProjectResult,
} from "../../contracts/types";

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
