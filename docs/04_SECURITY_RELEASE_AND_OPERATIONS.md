# Security, Privacy, Licensing and Release

## Threat model

Assets:

- source clips;
- project files;
- overlay artwork;
- outputs;
- filesystem permissions;
- update trust.

| Threat | Control |
|---|---|
| Malicious media decoder exploit | Patched pinned FFmpeg; non-elevated process |
| Frontend process execution | No shell capability; Rust fixed commands |
| Path traversal | Canonicalization and containment |
| Command injection | Argument arrays; rasterized text |
| Accidental overwrite | Explicit confirmation |
| Corrupt output reported as success | Partial file + ffprobe + atomic rename |
| Private path leakage | Logging redaction |
| Supply-chain compromise | Lockfiles, checksums, CI scanning |
| Malicious update | No updater before signing |

## Privacy

- no account;
- no media upload;
- no telemetry;
- no analytics;
- no network required;
- diagnostic bundle only on explicit action;
- diagnostic bundle excludes media, artwork and caption text.

## Process controls

- no shell;
- separate args;
- stdin closed;
- bounded output;
- ffprobe timeout;
- owned child handle;
- kill on app shutdown;
- never run elevated.

## FFmpeg distribution

Before public/commercial distribution:

1. choose and document LGPL-only vs GPL-enabled build;
2. pin exact source revision/configure flags;
3. archive corresponding source/build instructions;
4. include notices and licence texts;
5. include FFmpeg acknowledgement in app/about/download;
6. verify codec/patent implications by target market;
7. obtain legal review.

For internal development, a locally installed FFmpeg is acceptable. A random bundled binary is not a licensing plan.

## Code signing

- Windows unsigned internal builds are acceptable; public distribution needs signing to reduce trust warnings.
- macOS public distribution needs appropriate signing/notarization.
- Tauri updater requires signed update artifacts and is deferred.

## CI

1. checkout;
2. restore dependency caches;
3. locked frontend install;
4. lint/typecheck/test;
5. Rust fmt/clippy/test;
6. verify pinned FFmpeg;
7. media integration tests;
8. Tauri build;
9. checksums;
10. sign/publish only on protected release tags.

## Release manifest

Record:

- app version;
- commit;
- target triple;
- build timestamp;
- Rust/Node/package-manager versions;
- FFmpeg/ffprobe version;
- sidecar SHA-256;
- app artifact SHA-256;
- signing identity reference;
- project schema version.

## Logs

Release defaults:

- info level;
- 1 MiB per file;
- 5 files;
- bounded final FFmpeg stderr excerpt on failure;
- home path redaction.

## Startup recovery

- detect stale partials;
- clear dead job state;
- offer last project;
- avoid deleting a partial still owned by a process;
- cache cleanup is non-fatal.

## Support

Failure panel offers:

- readable error;
- code;
- safe detail;
- retry;
- reveal relevant folder;
- diagnostic bundle.

Users should not be sent spelunking through app-data directories as the primary support workflow.
