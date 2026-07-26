# Coding Agent Prompt

You are implementing **Skull’d Clip Forge**.

Read first:

1. `00_MASTER_SPEC.md`
2. `contracts/project.schema.json`
3. `contracts/types.ts`
4. `contracts/TAURI_COMMANDS.md`
5. `docs/03_MEDIA_PIPELINE.md`
6. `quality/ACCEPTANCE_TESTS.feature`
7. `delivery/BACKLOG.csv`

## Non-negotiable

- Tauri 2, Svelte 5, TypeScript and Rust.
- Rust owns files, paths, ffprobe, FFmpeg, jobs and cancellation.
- No frontend general shell permission.
- Child-process argument arrays, never command strings.
- Source media referenced, not copied.
- Atomic versioned project JSON.
- Caption text rasterized to PNG.
- One source and one active export.
- Partial output, ffprobe verification, then atomic final rename.
- No network/accounts/analytics/cloud/publishing.

## Method

- Work backlog dependency order.
- Small commits.
- Tests with pure modules.
- Thin commands, domain/services contain logic.
- Do not add dependencies without clear value.
- Preserve stable errors/contracts.
- Do not start mascot animation, AI or publishing before MVP acceptance.

## First slice

Implement SCF-001 through SCF-005, SCF-010 and SCF-011 plus a home screen that selects a file and displays normalized probe metadata.

Prove:

- app launches;
- file selected;
- ffprobe runs through Rust;
- typed JSON returned;
- no arbitrary shell exposed;
- valid/malformed probe tests pass.
