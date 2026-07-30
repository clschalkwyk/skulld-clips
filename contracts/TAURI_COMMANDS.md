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
select_sting_file(): string | null
select_export_destination({ suggestedName: string }): string | null
select_diagnostic_destination({ suggestedName: string }): string | null
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

## `import_sting_asset`

```ts
input: { projectPath: string; sourceAssetPath: string }
output: StingAssetRef
```

Accept a user-selected square MP4 up to 50 MiB and 10 seconds, probe it through the
Rust-owned media boundary, copy it into `assets/stings`, and record its content
hash, dimensions, duration, and audio presence. Rust also creates a bounded
transparent PNG preview sprite with the same fixed chroma-key profile. The
frontend may select the fixed `toasty-right` preset, a validated 1×/2×/3× rate,
once/repeat mode, bounded timing, and whether verified clip audio is included.
It may not supply a chroma-key expression, playback-rate expression, filter
fragment, or FFmpeg argument. A project may reference a content-addressed
import from more than one sting overlay, subject to project validation.

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

## Local clip discovery

```ts
start_clip_analysis({
  sourcePath: string;
}): {
  jobId: UUID;
  acceptedAt: string;
}

cancel_clip_analysis({
  jobId: UUID;
}): {
  accepted: boolean;
}
```

Rust revalidates the authorized source, probes its selected video stream and
constructs one fixed FFmpeg frame-sampling command. The frontend cannot supply a
profile, executable, frame filter, threshold or raw argument. One analysis runs
at a time; results are suggestions and do not mutate the project.

## Optional YouTube performance

These commands exist only for the read-only post-MVP performance workspace.
Rust owns OAuth, credentials, fixed endpoints, response bounds, channel
ownership checks and local persistence. No token or generic HTTP request is
exposed to the frontend.

```ts
get_youtube_connection_status(): YouTubeConnectionStatus
connect_youtube_channel(): YouTubeConnectionStatus
disconnect_youtube_channel(): YouTubeConnectionStatus
list_recent_youtube_uploads(): YouTubeVideoCandidate[]
list_youtube_performance(): YouTubeProjectPerformance[]
```

`connect_youtube_channel` opens the system browser, validates OAuth state and
PKCE through a bounded loopback callback, stores the refresh token in the OS
credential store, and loads the authorized channel. Disconnect removes both the
credential and cached local performance data.

`YouTubeConnectionStatus.connectionPhase` is one of `disconnected`,
`awaitingBrowser`, `exchangingToken`, `loadingChannel`, `connected`, or
`failed`. `get_youtube_connection_status` remains non-blocking while the
serialized connect operation runs so the frontend can poll this phase without
receiving tokens or generic network capability.

```ts
link_project_to_youtube_video({
  projectId: UUID;
  projectName: string;
  videoIdOrUrl: string;
}): YouTubeProjectPerformance
```

The native service accepts a supported YouTube URL or video ID, loads the video
through the fixed Data API endpoint, and rejects it unless its channel ID equals
the connected channel. It never guesses a link from filename, title, or time.

```ts
sync_youtube_performance({
  projectId?: UUID | null;
}): YouTubeProjectPerformance[]
```

With `projectId`, refresh exactly one linked video. Without it, refresh all
linked videos, up to the local 100-link bound. Each snapshot contains an
aggregate scorecard and daily rows through the last complete requested day.
New uploads with no report rows return a valid zero/pending snapshot.

## Events

### `export://progress`

At most 10/second, at least 1/second when FFmpeg supplies progress.

### `export://completed`

Only after verification and final rename.

### `export://failed`

Bounded sanitized detail.

### `export://cancelled`

Only after termination and cleanup attempt.

### `clip-analysis://progress`

```ts
{
  event: "clip-analysis://progress";
  jobId: UUID;
  progress: number;
  analyzedMs: number;
  totalMs: number;
}
```

### `clip-analysis://completed`

Returns at most 50 `ClipCandidate` records with kind, event time, raw detected
start/end anchors, a default suggested range, confidence and bounded evidence
labels. Point events use the same detected start/end time; boss encounters retain
their grouped detection interval. Svelte may recalculate the suggested range from
those anchors when the user changes the extraction offsets; no rescan is required.

### `clip-analysis://failed`

Returns a stable sanitized `AppError` without media paths, decoded frames or
FFmpeg output.

### `clip-analysis://cancelled`

Emitted after the frame sampler is terminated. The project remains unchanged.

Frontend listeners must be unregistered on store/window teardown.
