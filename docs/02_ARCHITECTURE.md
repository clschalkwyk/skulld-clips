# System Architecture

## Layers

1. Svelte presentation/edit state.
2. Typed Tauri command/event bridge.
3. Rust domain and application services.
4. FFmpeg/ffprobe, local filesystem and fixed opt-in YouTube APIs.

## Responsibilities

### Svelte

Owns:

- rendering;
- user input;
- local video preview;
- crop/overlay manipulation;
- caption rasterization;
- typed command invocation;
- event consumption.
- pure local YouTube publishing-copy generation from creator-confirmed project
  context.

Does not own:

- arbitrary file access;
- process spawning;
- command construction;
- path authorization;
- export lifecycle.
- OAuth tokens, generic HTTP, API URLs or analytics query construction.

### Rust commands

Thin adapters that:

- deserialize inputs;
- invoke services;
- map domain errors;
- serialize outputs;
- emit events.

### Project service

- create/load/save;
- atomic writes;
- schema migration;
- fingerprint checks;
- asset imports;
- bounded sting MP4 import/probe;
- cache cleanup.

### Probe service

- ffprobe process;
- 20-second default timeout;
- bounded output;
- JSON normalization;
- stream selection;
- orientation metadata.

### Export service

- immutable snapshot;
- validation;
- filter/argument construction;
- process registry;
- progress;
- cancellation;
- partial output;
- verification;
- atomic rename.

### Clip analysis service

- one cancellable analysis job outside the UI thread;
- authorized source and native-selected video stream;
- fixed FFmpeg sampling arguments at two 320×180 RGB frames per second;
- streaming frame classification with no retained frame cache;
- pure Diablo IV HUD/title heuristics and bounded candidate grouping;
- progress/completed/failed/cancelled events;
- no frontend thresholds, filter fragments, executable names or automatic
  project mutation.

### YouTube performance service

- optional OAuth desktop-client configuration;
- authorization code + PKCE + state through a bounded loopback callback;
- refresh token in OS credential storage and access token in memory;
- fixed Google OAuth, YouTube Data and YouTube Analytics endpoints;
- 20-second HTTP timeout and two-MiB response cap;
- connected-channel and video ownership validation;
- versioned atomic app-local link/snapshot persistence;
- no media upload, publishing action or frontend HTTP capability.

### YouTube post generator

- pure TypeScript with no Tauri command or network request;
- consumes project name, first caption, applied moment kind and creator-edited
  metadata only;
- never claims to inspect, transcribe or semantically understand raw media;
- enforces YouTube title/description limits and bounded hashtag output;
- stores the working brief and editable output only in the current Svelte
  session.

## Concurrency

- webview state is single-threaded;
- Tauri commands are asynchronous;
- probing/export occur outside UI thread;
- one active export;
- saves can occur during preview;
- running export uses an immutable snapshot;
- later edits do not mutate the active job.
- YouTube operations are serialized outside the UI thread;
- performance refresh never blocks local edit/save/export operations.
- clip analysis is single-job, cancellable and independent from the immutable
  export registry;
- application exit waits for active export and analysis samplers to terminate.

## Process registry

```rust
job_id -> {
    project_id,
    child_handle,
    destination,
    started_at,
    cancellation_state,
    export_state
}
```

Protected by a mutex. Never exposed directly to frontend.

## Persistence algorithm

1. Serialize and validate.
2. Write `project.skcf.json.tmp`.
3. Flush/sync as appropriate.
4. Copy current valid file to `.bak`.
5. Atomically replace final project file.
6. Update save state.

A failed save preserves the previous valid file.

## Path authorization

Allowed paths are:

- returned by a native dialog in current session;
- inside a known project;
- existing source/project paths loaded from valid project data;
- app-owned data/cache/log paths;
- user-approved export destination.

Canonicalize where possible. Project-relative assets may not escape project root through `..`, symlink or crafted JSON.

Each optional sting is a constrained asset type rather than a general video
track. A project may contain up to eight. Rust fixes the chroma key,
entrance/exit motion and bounded audio mix graph, and turns the validated
1×/2×/3× and once/repeat values into FFmpeg arguments. Svelte may edit only
those project fields, placement, timing, opacity and audio inclusion.

## Sidecar layout

```text
src-tauri/binaries/
  ffmpeg-x86_64-pc-windows-msvc.exe
  ffprobe-x86_64-pc-windows-msvc.exe
  ffmpeg-aarch64-apple-darwin
  ffprobe-aarch64-apple-darwin
  ffmpeg-x86_64-apple-darwin
  ffprobe-x86_64-apple-darwin
```

Development overrides:

```text
SKCF_FFMPEG_PATH
SKCF_FFPROBE_PATH
```

Production uses pinned binaries with checksums and license/build records.

## Capabilities

The main window receives only required commands/plugins. It does not receive general shell spawn/execute permission.

The YouTube integration also receives no frontend HTTP permission. Rust launches
only its constructed Google OAuth URL in the system browser and accepts only the
fixed typed performance commands.

Future updater or secondary windows receive separate capabilities.

## Project migration

Sequential only:

```text
v1 -> v2 -> v3
```

Rules:

- backup first;
- never skip;
- validate after each migration;
- newer unknown schema fails safely;
- failed migration opens recovery guidance;
- source media is untouched.

## Logging

Structured fields:

- timestamp;
- level;
- component;
- event code;
- project/job ID;
- sanitized detail;
- duration.

Never log full source paths, caption text, media or sampled frame data, OAuth values, channel or
video metadata, performance metrics, or unbounded command/API output.

## Repository shape

```text
src/
  components/{home,editor,export,common}
  state/
  services/
  contracts/
src-tauri/src/
  commands/
  domain/
  services/
  ffmpeg/
  migrations/
  security/
fixtures/
tests/
scripts/
```

Only `src/services/tauri.ts` invokes Tauri. Rust filter/argument builders remain pure.
