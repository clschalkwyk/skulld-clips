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
