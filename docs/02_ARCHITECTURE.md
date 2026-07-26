# System Architecture

## Layers

1. Svelte presentation/edit state.
2. Typed Tauri command/event bridge.
3. Rust domain and application services.
4. FFmpeg/ffprobe and local filesystem.

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

Does not own:

- arbitrary file access;
- process spawning;
- command construction;
- path authorization;
- export lifecycle.

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

## Concurrency

- webview state is single-threaded;
- Tauri commands are asynchronous;
- probing/export occur outside UI thread;
- one active export;
- saves can occur during preview;
- running export uses an immutable snapshot;
- later edits do not mutate the active job.

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

Never log full source paths, caption text, media data or unbounded command output.

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
