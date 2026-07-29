# Implementation status

## Milestone coverage

| Milestone | Backlog | Implemented verification |
|---|---|---|
| M0 | SCF-001–005 | Tauri/Svelte/Rust scaffold, typed invoke boundary, minimal capability and Windows launch workflow |
| M1 | SCF-010–015 | bounded probe, normalization fixtures, versioned validation, atomic save/backup, fingerprint, recents and relink |
| M2 | SCF-020–025 | local preview, trim/shortcuts, shared coordinate mapper, locked crop and persisted edit state |
| M3 | SCF-030–034 | hashed project-owned image assets, overlay transform/timing/z-order and content-addressed raster captions |
| M4 | SCF-040–046 | validation, pure graph/args, one process registry, progress, process-tree cancellation, partial verification and atomic publication |
| M5 | SCF-050–055 | generated fixture/golden suite, redacted logs, diagnostic ZIP exclusions, stale cleanup and installed Windows export/package smoke |
| M6 | SCF-056–063 | up to eight bounded project-owned MP4 stings, Rust-owned green key with 1×/2×/3× once/repeat motion and multi-sting audio mix, transparent looping sprite preview, visible selection, safe-area anchors, pixel nudging, full-width canvas dock, compact layer rail, direct seconds-based timing with cycle presets and one-click playhead insertion, viewport-contained editor panels and decoded-frame/audio integration |
| M7 | SCF-070 | opt-in read-only YouTube OAuth, OS credential storage, exact project/video ownership links, bounded cached owner scorecards and daily history behind a Rust-only network boundary |

## Local gates

```sh
npm run verify
SKCF_RUN_MEDIA_INTEGRATION=1 cargo test \
  --manifest-path src-tauri/Cargo.toml --all
shasum -a 256 -c MANIFEST.sha256
jq empty spec-manifest.json contracts/project.schema.json \
  contracts/export-events.schema.json examples/example-project.skcf.json \
  examples/example-sting-project.skcf.json \
  release/RELEASE_MANIFEST.schema.json
```

## Release boundary

The generated Windows artifact is an unsigned internal debug package. Public
distribution is not complete until pinned/checksummed FFmpeg sidecars, matching
source and licence records, legal review, signing and platform release smokes are
complete. Those limits are recorded in `release/KNOWN_LIMITATIONS.md`.

The YouTube integration is implemented but not live-verified in an authorized
packaged build. Public enablement additionally requires Google OAuth consent
configuration/verification and Windows/macOS browser-callback plus credential
store smokes.
