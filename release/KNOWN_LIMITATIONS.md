# Known limitations — internal build

The current package is for internal evaluation, not public distribution.

- Windows 11 x64 is the P0 build and launch target. CI silently installs the
  internal package and exercises the installed binary's core export path; a
  complete human-driven editor click-through remains a release-candidate check.
- The internal debug package is unsigned.
- FFmpeg and ffprobe are not bundled. Both must already be available on `PATH`,
  or absolute development overrides must be configured before launch.
- macOS 13+ remains P1 and public macOS signing/notarization is not configured.
- Linux packaging is deferred.
- Public release is blocked on a pinned, checksummed FFmpeg distribution,
  licence/source records, codec/legal review, and code signing.
- There is no updater, required account, cloud storage, telemetry, media upload,
  AI, transcription, or publishing integration.
- The optional read-only YouTube performance workspace requires a configured
  Google OAuth desktop client and authorized channel. The Rust OAuth/query
  boundary and local states are implemented, but real packaged
  Windows/macOS browser callback, OS credential-store, quota and owner-metric
  smokes remain unverified in this internal build. Public enablement also
  requires the applicable Google OAuth consent/verification and privacy-policy
  work.
- The editor supports one source video, one active export, static raster
  overlays, one fixed-preset Skull’d sting, and a fixed 1080×1920 H.264/AAC
  output. It does not support general video tracks, arbitrary keys or keyframes.

Core editing and export run without network access once the local media tools are
available.
