# MVP acceptance status

This maps the executable checks to `quality/ACCEPTANCE_TESTS.feature`. It does
not turn an unsigned internal package into a public release.

| Scenario | Status | Automated evidence |
|---|---|---|
| Happy path with caption and logo | Covered | real H.264/AAC export, two raster overlays, decoded-frame position/color assertions, stream/dimension/duration verification |
| Silent source | Covered | generated silent source, two-overlay golden export and verified absence of audio |
| Cancel | Covered | live FFmpeg process-tree termination and partial cleanup, registry and cancellation UI state tests |
| Invalid trim | Covered | shared timeline and Rust project cross-field validation reject invalid or sub-250 ms ranges before argument generation |
| Existing output | Covered | non-overwrite publication preserves existing bytes; overwrite is explicit in validation/UI |
| Reopen project | Covered | atomic save/backup/load tests and exact persisted edit-state serialization |
| Relink moved source | Covered | generated media is moved, reopened as missing, relinked by matching fingerprint, and trim/crop are preserved |
| Changed source | Covered | changed-content detection and explicit mismatch acceptance boundary |
| Special-character paths | Covered | argument-vector tests and real exports use spaces, apostrophes and Unicode without a shell |
| Rotated source | Covered | generated rotation metadata is normalized against display dimensions and exported through the orientation/crop graph |
| Variable-frame-rate source | Covered | generated VFR/AAC media is normalized to verified CFR and audio/video duration drift stays within one frame plus 20 ms |
| Offline operation | Waived for physical-network smoke | core workflow has no network commands, capabilities, accounts, uploads or telemetry; repeat the packaged click-through with networking disabled before public distribution |

## Platform evidence

- macOS: local frontend/Rust gates, generated-media matrix, golden frames,
  cancellation, internal export smoke and debug application bundle.
- Windows 11 x64: GitHub Actions installs the NSIS package silently, runs a real
  export through the installed debug binary, verifies the MP4, launches the
  normal UI process, records artifact hashes and uploads the internal package.
- Human-driven installed editor click-through remains a release-candidate check
  and is recorded in `release/KNOWN_LIMITATIONS.md`.
