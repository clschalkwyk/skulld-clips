import { describe, expect, it, vi } from "vitest";

import type { RuntimeInfo } from "../contracts/runtime";
import {
  getRuntimeInfo,
  normalizeAppError,
  type InvokeFn,
} from "./tauri";

const runtimeInfo: RuntimeInfo = {
  appVersion: "0.1.0",
  projectSchemaVersion: 1,
  os: "macos",
  arch: "aarch64",
  ffmpegVersion: "8.1",
  ffprobeVersion: "8.1",
  bundledSidecars: false,
};

describe("getRuntimeInfo", () => {
  it("uses the single typed Tauri command boundary", async () => {
    const invokeCommand = vi.fn(async () => runtimeInfo) as InvokeFn;

    await expect(getRuntimeInfo(invokeCommand)).resolves.toEqual(runtimeInfo);
    expect(invokeCommand).toHaveBeenCalledWith("get_runtime_info");
  });

  it("normalizes a structured native error", async () => {
    const nativeError = {
      code: "E_FFPROBE_FAILED",
      message: "ffprobe is unavailable.",
      safeDetail: "Install ffprobe or configure the development override.",
      retryable: true,
    };
    const invokeCommand: InvokeFn = async () => {
      throw nativeError;
    };

    await expect(getRuntimeInfo(invokeCommand)).rejects.toEqual(nativeError);
  });
});

describe("normalizeAppError", () => {
  it("does not leak unknown rejection details", () => {
    expect(normalizeAppError(new Error("private/path/to/a/file"))).toEqual({
      code: "E_INTERNAL",
      message: "The native application returned an unexpected error.",
      safeDetail: null,
      retryable: true,
    });
  });
});
