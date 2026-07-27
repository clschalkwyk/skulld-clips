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

`npm run verify:spec` is the cross-platform specification checksum and contract
JSON gate used by CI.

Run the generated FFmpeg fixture, golden-frame and cancellation integrations:

```sh
SKCF_RUN_MEDIA_INTEGRATION=1 cargo test \
  --manifest-path src-tauri/Cargo.toml --all
```

Run the desktop application:

```sh
npm run tauri dev
```

The frontend dev server binds only to `127.0.0.1:1420`.

## Implemented baseline

Milestones M0–M5 implement:

- the least-privilege Tauri/Svelte/Rust boundary and stable typed contracts;
- probing, versioned projects, autosave, recents and relink;
- preview, trim, crop, image overlays and rasterized captions;
- safe FFmpeg argument/filter construction, progress, process-tree cancellation,
  output verification and atomic publication;
- rotating redacted logs, explicit diagnostic ZIPs and startup cleanup;
- generated media fixtures, decoded-frame golden checks and Windows package CI.

The Windows CI package is an unsigned internal debug build and requires FFmpeg
and ffprobe on `PATH`. It is not a public release. See `release/` for notices,
limitations and the generated release-manifest schema.
