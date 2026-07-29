# Implementation Plan

## Milestone 0 — Scaffold and boundary (4h)

Deliver:

- Tauri 2 + Svelte 5 TypeScript project;
- lint/typecheck/test;
- minimal capabilities;
- stable `AppError`;
- typed bridge;
- FFmpeg/ffprobe path resolution;
- runtime info.

Exit:

- Windows dev app launches;
- ffprobe version is obtained through Rust;
- frontend has no arbitrary shell capability.

## Milestone 1 — Probe and projects (5h)

Deliver:

- file dialog/drop;
- ffprobe wrapper/normalizer;
- schema v1;
- project folder;
- weak fingerprint;
- atomic save/autosave;
- recents;
- source status.

Exit:

- fixture clips import;
- restart restores project;
- missing/changed source detected.

## Milestone 2 — Trim/crop editor (7h)

Deliver:

- local video element;
- playhead/range;
- keyboard transport;
- pure coordinate mapper;
- crop constraint solver;
- interactive crop;
- persistence.

Exit:

- crop never escapes source;
- landscape/portrait/rotation unit coverage;
- trim acceptance.

## Milestone 3 — Overlays (5h)

Deliver:

- image import;
- stage move/resize;
- timing/z-order;
- caption editor;
- PNG rendering/hash.

Exit:

- overlays persist;
- caption asset reuse;
- no text in FFmpeg expression.

## Milestone 4 — Export (7h)

Deliver:

- validation;
- graph/argument builder;
- process registry;
- progress events;
- cancellation;
- partial output;
- verification;
- rename/reveal.

Exit:

- happy, silent and cancellation scenarios pass;
- no final on failure;
- dimensions/duration verified.

## Milestone 5 — Hardening/internal package (5h)

Deliver:

- fixture suite;
- golden frames;
- logs/redaction;
- diagnostics;
- stale partial/cache cleanup;
- Windows package;
- third-party notices/release manifest.

Exit:

- acceptance suite passes;
- offline;
- checksums recorded;
- limitations documented.

## Milestone 6 — Constrained Skull’d sting (2h)

Deliver:

- bounded square MP4 import and project-owned copy;
- fixed green-key preview sprite;
- validated 1×, 2×, or 3× right-edge entrance and exit;
- once or bounded repeat playback;
- up to eight independently timed sting overlays;
- optional bounded sting audio mix;
- contract, decoded-frame, audio and no-freeze verification.

Exit:

- multiple stings can be imported or duplicated, previewed, saved, reopened and exported;
- arbitrary video tracks, key settings and FFmpeg inputs remain unavailable.

## Milestone 7 — Opt-in YouTube performance (5h)

Deliver:

- native OAuth desktop flow with PKCE, state and loopback timeout;
- refresh token in the OS credential store;
- exact project-to-owned-video linking through recent uploads or URL;
- fixed aggregate and daily owner-analytics queries;
- bounded atomic local snapshot storage;
- connected, unconfigured, unauthorized, empty, pending, error and success UI.

Exit:

- an authorized channel can link a project to one of its videos and retrieve the
  defined scorecard without exposing tokens or generic network access;
- disconnect clears credentials and cached account/performance data;
- the core editor/export continues to pass offline acceptance.

## Weekend cut (12–16h)

Include only:

- open clip;
- probe;
- trim;
- crop;
- one caption;
- one image;
- export using local/dev FFmpeg.

Defer:

- recents;
- relink;
- diagnostics;
- public installers;
- macOS;
- updater;
- advanced quality.

This is the correct palate-cleanser cut. Public distribution is a separate project phase, because installers, signing and media licensing are where innocent weekends go to die.
