import { describe, expect, it, vi } from "vitest";

import type { RuntimeInfo } from "../contracts/runtime";
import {
  cancelExport,
  createProject,
  getRuntimeInfo,
  importOverlayAsset,
  loadProject,
  normalizeAppError,
  projectAssetPath,
  saveProject,
  selectExportDestination,
  startExport,
  validateExport,
  writeCaptionAsset,
  type InvokeFn,
} from "./tauri";
import type { ExportRequest, ProjectV1 } from "../../contracts/types";

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

describe("project commands", () => {
  it("uses camel-case payloads through the single invoke boundary", async () => {
    const invokeCommand = vi.fn(async () => ({})) as InvokeFn;
    const project = {} as ProjectV1;

    await createProject("/clips/source.mp4", "Boss fight", invokeCommand);
    await loadProject("/projects/id/project.skcf.json", invokeCommand);
    await saveProject("/projects/id/project.skcf.json", project, invokeCommand);

    expect(invokeCommand).toHaveBeenNthCalledWith(1, "create_project", {
      sourcePath: "/clips/source.mp4",
      projectName: "Boss fight",
      projectsRoot: null,
    });
    expect(invokeCommand).toHaveBeenNthCalledWith(2, "load_project", {
      projectPath: "/projects/id/project.skcf.json",
    });
    expect(invokeCommand).toHaveBeenNthCalledWith(3, "save_project", {
      projectPath: "/projects/id/project.skcf.json",
      project,
    });
  });

  it("uses typed asset payloads and derives cross-platform preview paths", async () => {
    const invokeCommand = vi.fn(async () => ({})) as InvokeFn;

    await importOverlayAsset(
      "/projects/id/project.skcf.json",
      "/art/logo.png",
      invokeCommand,
    );
    await writeCaptionAsset(
      "/projects/id/project.skcf.json",
      "a".repeat(64),
      "cG5n",
      400,
      120,
      invokeCommand,
    );

    expect(invokeCommand).toHaveBeenNthCalledWith(1, "import_overlay_asset", {
      projectPath: "/projects/id/project.skcf.json",
      sourceAssetPath: "/art/logo.png",
    });
    expect(invokeCommand).toHaveBeenNthCalledWith(2, "write_caption_asset", {
      projectPath: "/projects/id/project.skcf.json",
      contentHash: "a".repeat(64),
      pngBytesBase64: "cG5n",
      width: 400,
      height: 120,
    });
    expect(
      projectAssetPath(
        "C:\\Projects\\id\\project.skcf.json",
        "assets/captions/hash.png",
      ),
    ).toBe("C:\\Projects\\id\\assets\\captions\\hash.png");
  });

  it("keeps export commands behind typed request payloads", async () => {
    const invokeCommand = vi.fn(async () => ({})) as InvokeFn;
    const request = {
      projectPath: "/projects/id/project.skcf.json",
    } as ExportRequest;

    await selectExportDestination("Boss fight.mp4", invokeCommand);
    await validateExport(request, invokeCommand);
    await startExport(request, invokeCommand);
    await cancelExport("job-1", invokeCommand);

    expect(invokeCommand).toHaveBeenNthCalledWith(
      1,
      "select_export_destination",
      { suggestedName: "Boss fight.mp4" },
    );
    expect(invokeCommand).toHaveBeenNthCalledWith(2, "validate_export", {
      request,
    });
    expect(invokeCommand).toHaveBeenNthCalledWith(3, "start_export", {
      request,
    });
    expect(invokeCommand).toHaveBeenNthCalledWith(4, "cancel_export", {
      jobId: "job-1",
    });
  });
});
