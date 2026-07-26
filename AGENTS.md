# AGENTS.md

## Purpose

This repository is the specification pack for **Skull’d Clip Forge**, a local-first
desktop utility that turns one gameplay video into one branded vertical MP4.

These instructions apply to the repository root and all future implementation
files below it.

## Current repository state

- The repository currently contains specifications, contracts, diagrams, examples,
  acceptance tests, and a delivery backlog.
- It does not yet contain a runnable Tauri/Svelte application, package manifest, or
  Cargo workspace.
- Do not claim that an app, build, test, or package command works until the
  corresponding implementation and configuration exist and the command has been
  run successfully.
- Preserve the specification pack as the source of truth while scaffolding the
  implementation into this repository.

## Read before changing code

Read only the material relevant to the task, starting with:

1. `00_MASTER_SPEC.md`
2. `contracts/project.schema.json`
3. `contracts/types.ts`
4. `contracts/TAURI_COMMANDS.md`
5. `docs/03_MEDIA_PIPELINE.md`
6. `quality/ACCEPTANCE_TESTS.feature`
7. `delivery/BACKLOG.csv`

For UI work, also read `docs/01_PRODUCT_AND_UX.md`. For native boundaries,
security, packaging, or release work, also read `docs/02_ARCHITECTURE.md` and
`docs/04_SECURITY_RELEASE_AND_OPERATIONS.md`.

If documents disagree, do not silently choose a convenient interpretation. Treat
the master spec and explicit JSON/TypeScript/Tauri contracts as authoritative,
record the conflict, and make the smallest coordinated correction.

## Product boundary

The MVP workflow is:

1. Import one local source video.
2. Select an in/out range.
3. Position a locked 9:16 crop.
4. Add rasterized caption and optional static image overlays.
5. Export one verified 1080x1920 MP4.

The MVP includes project autosave, reopen/relink, progress, cancellation, and local
diagnostics. It excludes accounts, cloud storage, analytics, AI, transcription,
publishing APIs, multiple video tracks, transitions, keyframes, collaboration,
mobile apps, plugins, and arbitrary frontend shell access.

Windows 11 x64 is the P0 release gate. macOS 13+ is P1. Linux is deferred.
Do not start roadmap work until MVP acceptance passes.

## Required stack and ownership

- Desktop shell: Tauri 2.
- Frontend: Svelte 5 with TypeScript and runes.
- Native core: Rust.
- Media tools: fixed FFmpeg and ffprobe binaries managed by Rust.
- Persistence: versioned JSON project folders; do not add a database without an
  approved, demonstrated need.

Svelte owns rendering, user interaction, local video preview, edit state, crop and
overlay manipulation, caption rasterization, typed command calls, and event
consumption.

Rust owns filesystem authorization, canonicalization, project persistence and
migration, asset import, ffprobe/FFmpeg execution, filter and argument construction,
job lifecycle, progress, cancellation, output verification, atomic rename, and
stable error mapping.

Only `src/services/tauri.ts` may call Tauri `invoke`. Tauri commands must remain thin;
put validation and behavior in domain/services, with pure media argument, filter,
coordinate, and parsing modules where possible.

Use the repository shape defined in `docs/02_ARCHITECTURE.md`. Do not introduce a
new framework, broad abstraction layer, or alternate architecture without explicit
approval.

## Non-negotiable invariants

- Never expose a general shell capability to the frontend.
- Never accept executable names, shell command strings, raw FFmpeg arguments, or
  filter fragments from the frontend.
- Spawn child processes with validated argument arrays. Close stdin, bound output,
  enforce timeouts where specified, and never run elevated.
- Source media remains external and is referenced, not copied or fully buffered.
- Imported image overlays are copied into the project and hashed.
- Caption text is rendered with bundled fonts to a transparent, content-addressed
  PNG; it is never interpolated into an FFmpeg filter expression.
- Project writes are versioned, validated, backed up, and atomically replaced.
- Crop coordinates are normalized against the display-oriented source. Preview and
  export use the same pure coordinate mapper.
- Store time as integer milliseconds. Enforce
  `0 <= inMs < outMs <= sourceDurationMs` and a minimum 250 ms duration.
- Enforce exactly one active export. Each export uses an immutable project snapshot.
- Write to a partial MP4, ffprobe it, verify the required streams/dimensions/duration,
  and only then atomically rename it to the final filename.
- Cancellation must terminate the process tree, clean the partial file, emit a
  terminal cancellation event, and leave the app ready for another export.
- Core import, edit, save, reopen, and export behavior must work offline.

The baseline output is MP4 with exactly 1080x1920 H.264-compatible video,
`yuv420p`, fast start, and AAC at 48 kHz when source audio exists. Variable-rate
sources produce a stable constant-rate output capped at 60 fps unless 30 or 60 is
explicitly selected.

## Contracts and errors

- Keep `contracts/project.schema.json`, `contracts/types.ts`, the Rust domain types,
  Tauri command payloads, event schemas, and example project synchronized.
- Validate at every trust boundary. JSON Schema does not replace Rust cross-field,
  path, source fingerprint, asset, or export validation.
- Preserve the stable `AppError` shape and codes from `contracts/types.ts`.
- Return actionable user-safe errors: what failed, stable code, safe detail,
  corrective action, and retryability where appropriate.
- Do not alter a public command/event name or payload casually. A contract change
  requires coordinated contract, implementation, test, traceability, and doc
  updates.
- Unregister frontend event listeners during store or window teardown.

## Security, privacy, and release rules

- Authorize only dialog-approved paths, validated project paths, project-contained
  assets, app-owned paths, and approved export destinations.
- Prevent project-relative escape through `..`, symlinks, or crafted JSON.
- Never log full private paths, caption text, media bytes, artwork, credentials, or
  unbounded child-process output.
- Diagnostic bundles are explicit user actions and must exclude source/output media,
  overlay artwork, and caption text.
- Do not add telemetry, analytics, accounts, uploads, or network dependencies.
- Development may use `SKCF_FFMPEG_PATH` and `SKCF_FFPROBE_PATH`. Production binaries
  must be pinned and checksummed with build and licensing records.
- Do not add an updater before signed update artifacts exist.
- Public distribution requires the signing and FFmpeg licensing work described in
  `docs/04_SECURITY_RELEASE_AND_OPERATIONS.md`; do not present an internal unsigned
  build as release-ready.

## Implementation workflow

- Work in dependency order from `delivery/BACKLOG.csv`; reference the relevant
  `SCF-*` item in implementation notes or commits.
- For the initial slice, follow `delivery/AGENT_PROMPT.md`: scaffold M0, implement
  safe probing/normalization, and display probe metadata through the typed Rust
  boundary.
- Make small, atomic changes. Do not combine scaffolding, contract redesign, media
  pipeline work, and UI polish in one change.
- Add no dependency unless it has clear value, is version-pinned through the chosen
  package ecosystem, and does not broaden permissions or the network surface.
- Prefer pure functions and fixture-driven tests for coordinate mapping, crop
  constraints, schema validation, probe normalization, FFmpeg graph/argument
  generation, progress parsing, and error sanitization.
- Preserve loading, empty, error, validation, unauthorized/denied, cancellation,
  and success states in touched user flows.
- Do not silently mutate production-like project data. Add sequential migrations,
  back up first, validate after every step, and fail safely on newer schemas.

## Verification

The only repository-wide checks available before application scaffolding are:

```sh
shasum -a 256 -c MANIFEST.sha256
jq empty spec-manifest.json contracts/project.schema.json \
  contracts/export-events.schema.json examples/example-project.skcf.json
```

`MANIFEST.sha256` covers the original specification-pack artifacts. Do not rewrite
it merely because implementation files are added.

After scaffolding, define and use committed package scripts for formatting, lint,
type checking, frontend unit/component tests, and browser-harness tests. Pin one
Node package manager and use its lockfile; do not invent package commands that are
not declared in the repository.

For Rust changes, run the applicable checks once a Cargo project exists:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Media pipeline changes require the relevant fixture/integration coverage from
`quality/TEST_PLAN.md`. Preview/export coordinate changes require golden-frame
coverage. Cancellation and process changes require platform-specific process-tree
tests, especially on Windows.

Before claiming completion, run the narrow relevant checks plus the broad gates
affected by the change. Report exact commands and results. If a platform, sidecar,
fixture, or signing check cannot run locally, state exactly what remains unverified.

## Version-control cadence

- After each coherent backlog section or implementation slice is complete and its
  relevant checks pass, stage only that slice, create a terse commit, and push the
  active branch. Do not wait until the entire MVP is complete before updating the
  remote repository.
- Keep unrelated changes in separate commits. Do not push known-broken intermediate
  states unless the user explicitly requests a clearly labelled work-in-progress
  checkpoint.
- Never force-push or rewrite shared history without explicit approval.
- Report the branch and commit hash after every successful push.

## Definition of done

A task is done only when:

- its spec and contract are satisfied;
- happy and unhappy paths are covered;
- stable errors and least-privilege capabilities are preserved;
- relevant TypeScript, Rust, schema, and media checks pass;
- affected docs, contracts, traceability, and examples are synchronized;
- no private paths, generated debug artifacts, fixtures without redistribution
  rights, or secrets are committed.

MVP completion additionally requires the acceptance suite, offline flow, fixture
matrix, golden frames, restart/relink, clean cancellation, Windows packaged smoke
test, checksums, notices, and known limitations required by
`delivery/DEFINITION_OF_DONE.md`.
