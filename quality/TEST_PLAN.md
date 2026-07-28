# Test Plan

## Frontend unit tests

- coordinate mapping;
- 9:16 crop solver;
- timeline validation;
- caption wrapping/hash input;
- shortcut guards;
- progress reducer;
- autosave state;
- error mapping.

## Component tests

- drop states;
- trim controls;
- crop inspector/stage synchronization;
- overlay selection/timing;
- bounded multi-sting selection, 1×/2×/3× timing, repeat, audio and duration;
- export validation;
- cancellation UI;
- relink flow.

## Browser-harness E2E

Run Svelte against a mock Tauri adapter for fast workflow tests:

- home → import;
- editor interactions;
- export UI;
- errors;
- recent projects.

## Rust unit tests

- path containment;
- project cross-field validation;
- migrations;
- fingerprint;
- ffprobe JSON parsing;
- stream selection;
- orientation normalization;
- filter graph;
- argument vector;
- progress parser;
- error sanitization.

## Rust media integration tests

Use pinned FFmpeg/ffprobe fixtures:

| Fixture | Purpose |
|---|---|
| landscape H.264/AAC | baseline |
| HEVC MOV | decoder/container variation |
| silent H.264 | optional audio |
| rotated MOV | orientation metadata |
| portrait source | crop edge |
| VFR gameplay | rate/audio sync |
| Unicode filename | path handling |
| corrupt header | probe failure |
| attached-cover MKV | stream selection |
| 250 ms clip | minimum duration |

Fixtures must be generated or redistributable.

## Golden media tests

For fixture projects:

- export;
- ffprobe metadata;
- extract frames at fixed times;
- perceptual comparison;
- crop/overlay bounding tolerance;
- duration/audio tolerance.

## Failure injection

- read-only project;
- destination unavailable;
- disk full;
- FFmpeg non-zero;
- ffprobe hang;
- app closes during export;
- missing overlay;
- oversized/overlong/malformed sting;
- malformed JSON;
- newer schema;
- source replacement.

## CI gates

Pull request:

- formatting;
- lint;
- typecheck;
- frontend tests;
- Rust fmt/clippy/tests;
- schema parse;
- contract drift;
- fast media integration.

Release candidate:

- full fixture matrix;
- golden tests;
- packaged smoke test;
- sidecar checksum;
- licence notices;
- signing checks if public.
