# Architecture Decisions

## ADR-001 — Tauri 2 + Svelte 5

**Accepted.**

Use Tauri 2, Rust and Svelte 5.

Benefits:

- useful Rust/native learning;
- fast UI iteration;
- controlled process/filesystem boundary;
- cross-platform path.

Costs:

- sidecar packaging;
- webview differences;
- signing/updater work;
- Rust learning overhead.

Rejected:

- Electron: easier but less novel and broader runtime surface.
- pure web: poor controlled local FFmpeg process story.
- native Rust UI: slower visual iteration.

## ADR-002 — FFmpeg/ffprobe sidecars

**Accepted.**

Rust exclusively manages fixed media binaries.

Rules:

- no shell;
- separate args;
- progress;
- process registry;
- cancellation;
- partial output;
- verification;
- checksummed production binaries.

## ADR-003 — JSON project folders

**Accepted.**

One versioned JSON file plus assets/cache per project. No database until real query/scale requirements exist.

## ADR-004 — Rasterized captions

**Accepted.**

Render captions with bundled fonts into transparent PNGs. Avoid drawtext escaping/font lookup and improve preview/export parity.

## ADR-005 — CPU encoding default

**Accepted for MVP.**

Use libx264 for predictable output. Detect NVENC/QSV/VideoToolbox later after correctness and fixtures are stable.
