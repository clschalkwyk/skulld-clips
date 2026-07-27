# Skull’d Clip Forge — Software Specification Pack

**Version:** 1.0  
**Prepared:** 2026-07-26  
**Stack:** Tauri 2, Rust, Svelte 5, TypeScript, FFmpeg/ffprobe  
**Product:** Local-first desktop utility for turning gameplay footage into branded vertical clips.

## Reading order

1. `00_MASTER_SPEC.md`
2. `docs/01_PRODUCT_AND_UX.md`
3. `docs/02_ARCHITECTURE.md`
4. `docs/03_MEDIA_PIPELINE.md`
5. `contracts/TAURI_COMMANDS.md`
6. `contracts/project.schema.json`
7. `quality/ACCEPTANCE_TESTS.feature`
8. `delivery/IMPLEMENTATION_PLAN.md`
9. `delivery/AGENT_PROMPT.md`

## MVP boundary

Included: one local source video, trim, locked 9:16 crop, static image overlays, rasterized caption overlays, project autosave, one export job, local MP4 output, progress and cancellation.

Excluded: accounts, cloud, AI, transcription, publishing APIs, multiple video tracks, transitions, keyframes, collaboration, mobile apps and general-purpose frontend shell access.

The project is meant to be useful and finishable. It is not meant to become Adobe Premiere after three enthusiastic evenings.

## Implementation status

The repository now contains the runnable M0–M5 implementation:

- local file import, safe ffprobe normalization, versioned projects, autosave,
  recents and relink;
- video preview, trim, locked 9:16 crop, image overlays and rasterized captions;
- one Rust-owned FFmpeg export with structured progress, clean cancellation,
  ffprobe verification and atomic publication;
- redacted rotating logs, explicit diagnostic ZIP creation and stale app-owned
  cache/partial cleanup;
- generated media fixtures, decoded-frame golden checks and an internal Windows
  package workflow.

Start with [DEVELOPMENT.md](DEVELOPMENT.md) and review the
[acceptance evidence](quality/ACCEPTANCE_STATUS.md). Public distribution remains
blocked on pinned/checksummed FFmpeg sidecars, release-specific licence records
and code signing; see
[release/KNOWN_LIMITATIONS.md](release/KNOWN_LIMITATIONS.md).
