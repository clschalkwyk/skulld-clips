import { invoke } from "@tauri-apps/api/core";

import type { AppError, RuntimeInfo } from "../contracts/runtime";
import type { AppErrorCode } from "../../contracts/types";

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
