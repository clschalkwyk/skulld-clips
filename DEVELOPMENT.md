# Development

## Prerequisites

- Node.js 22.12 or newer
- Rust 1.77.2 or newer
- npm
- FFmpeg and ffprobe available on `PATH` for development

Optional development overrides:

```sh
SKCF_FFMPEG_PATH=/absolute/path/to/ffmpeg
SKCF_FFPROBE_PATH=/absolute/path/to/ffprobe
```

The overrides are ignored by release builds. Production packaging will use pinned,
checksummed sidecars only.

## Install and verify

```sh
npm ci
npm run verify
```

Run the desktop application:

```sh
npm run tauri dev
```

The frontend dev server binds only to `127.0.0.1:1420`.

## Milestone 0 boundary

Milestone 0 implements:

- Tauri 2, Svelte 5, TypeScript, Vite, npm, and Cargo scaffolding;
- strict frontend checks and unit tests;
- minimal Tauri capabilities without shell or opener permissions;
- the stable structured `AppError` contract;
- a single typed frontend invoke adapter;
- Rust-owned FFmpeg/ffprobe path and version checks;
- the `get_runtime_info` command and readiness UI;
- Windows build and launch-smoke CI.

Media probing, project creation, dialogs, editing, and export are intentionally
deferred to their dependency-ordered backlog items.
