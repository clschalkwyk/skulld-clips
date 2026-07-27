# Tauri Command and Event Contract

## Rules

- Async JSON-serializable commands.
- Stable `AppError`.
- Backend validates every request.
- No executable names, shell commands or raw FFmpeg args from frontend.
- Export receives an immutable project snapshot.

## Native file selection

```ts
select_media_file(): string | null
select_overlay_file(): string | null
select_export_destination({ suggestedName: string }): string | null
select_project_file(): string | null
select_projects_folder(): string | null
```

Selection happens in Rust. Returned paths are authorized for the matching
follow-up command. Native drag-and-drop paths are authorized by the Rust webview
event handler before the frontend receives the drop event.

## `probe_media`

```ts
input:  { path: string }
output: MediaProbe
```

Validate authorized file, timeout and bounded output.

## `create_project`

```ts
input: {
  sourcePath: string;
  projectName?: string;
  projectsRoot?: string;
}
output: {
  projectPath: string;
  project: ProjectV1;
}
```

Probe source, create folder, calculate fingerprint, initialize centered crop, atomic save and add recent project.

## `load_project`

```ts
input: { projectPath: string }
output: {
  projectPath: string;
  project: ProjectV1;
  sourceStatus: "ok" | "missing" | "changed";
  migrationApplied: boolean;
}
```

## `save_project`

```ts
input: { projectPath: string; project: ProjectV1 }
output: { savedAt: string; projectSha256: string }
```

Backend checks cross-field invariants not fully expressible in JSON Schema.

## `relink_source`

```ts
input: {
  projectPath: string;
  replacementPath: string;
  acceptFingerprintMismatch: boolean;
}
output: {
  project: ProjectV1;
  fingerprintMatched: boolean;
}
```

## Recent projects

```ts
list_recent_projects(): RecentProject[]
remove_recent_project({ projectPath: string }): void
```

The Rust-owned Tauri store retains at most 20 entries. Listing refreshes the
source status for readable projects and authorizes those stored project paths for
reopening.

## `import_overlay_asset`

```ts
input: { projectPath: string; sourceAssetPath: string }
output: AssetRef
```

Copy into project and record hash/dimensions/MIME.

## `write_caption_asset`

```ts
input: {
  projectPath: string;
  contentHash: string;
  pngBytesBase64: string;
  width: number;
  height: number;
}
output: AssetRef
```

Initial payload cap: 10 MiB. Replace with temp-file/binary IPC only if real usage requires it.

## `validate_export`

```ts
input: ExportRequest
output: ExportValidation
```

Checks source, project, trim, crop, overlays, destination, overwrite, free space and settings.

## `start_export`

```ts
input: ExportRequest
output: { jobId: string; acceptedAt: string }
```

Server-side validation repeats. One active job.

## `cancel_export`

```ts
input: { jobId: string }
output: { accepted: boolean }
```

Terminal cancellation is delivered by event after process/cleanup.

## `reveal_in_folder`

```ts
input: { path: string }
output: { opened: boolean }
```

Path must exist and be authorized by project/output context.

## `create_diagnostic_bundle`

```ts
input: { destinationZipPath: string; projectPath?: string }
output: { path: string; sizeBytes: number }
```

## `get_runtime_info`

```ts
input: {}
output: {
  appVersion: string;
  projectSchemaVersion: number;
  os: string;
  arch: string;
  ffmpegVersion: string;
  ffprobeVersion: string;
  bundledSidecars: boolean;
}
```

## Events

### `export://progress`

At most 10/second, at least 1/second when FFmpeg supplies progress.

### `export://completed`

Only after verification and final rename.

### `export://failed`

Bounded sanitized detail.

### `export://cancelled`

Only after termination and cleanup attempt.

Frontend listeners must be unregistered on store/window teardown.
