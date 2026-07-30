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
- typed YouTube command names/payloads and stable integration errors.
- clip-analysis command names, event reducer and candidate application bounds.
- YouTube post brief seeding, machine-name rejection, validation, title
  uniqueness/limits, primary-phrase placement, description limits and hashtag
  bounds.
- typed AI provider status/key/model/generation commands and stable provider
  authentication/API errors.

## Component tests

- drop states;
- trim controls;
- crop inspector/stage synchronization;
- overlay selection/timing;
- bounded multi-sting selection, 1×/2×/3× timing, repeat, audio and duration;
- export validation;
- cancellation UI;
- relink flow.
- YouTube unavailable, disconnected, linking, pending, error and scorecard states.
- clip-discovery idle, progress, cancelled, empty, error, review and apply states.
- YouTube post empty, invalid brief, generated, edited, SEO-check, copy success
  and clipboard-unavailable states.
- Local/OpenAI/OpenRouter source switching, masked key entry, saved/removed
  status, model loading/empty/error states, generation progress and local
  fallback.

## Browser-harness E2E

Run Svelte against a mock Tauri adapter for fast workflow tests:

- home → import;
- editor interactions;
- export UI;
- errors;
- recent projects.
- editor → YouTube post → generate → choose title → edit → copy without a
  connected channel.
- editor → YouTube post → save provider key → load model → generate → switch
  provider/local, using a mock native adapter with no real key.

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
- OAuth URL PKCE/state/read-only scopes;
- supported YouTube URL/video-ID parsing;
- returned-header analytics metric mapping;
- oversized API response rejection;
- versioned YouTube catalog bounds and atomic persistence.
- AI API-key shape validation and per-provider credential entry selection;
- OpenAI/OpenRouter text-model catalog filtering and bounded response parsing;
- provider structured-output parsing, title/description/hashtag normalization
  and sanitized authentication/rate-limit/provider failures;
- fixed frame-sampler arguments;
- completion/death/boss region classifier fixtures;
- persistence grouping, confidence and suggested-range bounds;
- point/interval extraction anchors, default 15s/5s offsets, custom offsets and
  source-bound clamping;
- rejection of negative, non-numeric, over-five-minute and sub-250ms extraction
  windows;
- analysis reservation, cancellation, timeout and sanitized process failures.

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

Run the opt-in clip-discovery smoke against an authorized local gameplay source:

```sh
SKCF_RUN_CLIP_ANALYSIS_INTEGRATION=1 \
SKCF_CLIP_ANALYSIS_SOURCE=/absolute/path/to/gameplay.mp4 \
cargo test --manifest-path src-tauri/Cargo.toml \
  services::clip_analysis::tests::analyzes_real_media_source_when_enabled -- --nocapture
```

The smoke test reports only candidate counts; it does not persist frame bytes or
print the source path.

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
- OAuth timeout/state mismatch/denial;
- unavailable/locked OS credential store;
- expired/revoked refresh token;
- wrong-channel video;
- YouTube 401/403/429/5xx and malformed/oversized responses;
- newly published video with no report rows.
- invalid/revoked OpenAI and OpenRouter keys;
- empty/incompatible AI model catalogs;
- AI provider 400/401/403/429/5xx, timeout and malformed/oversized responses;
- locked credential store during AI key save/load/remove;
- source decode failure during analysis;
- oversized recording analysis timeout;
- app close and user cancellation during frame sampling;
- transient red HUD elements that must not become boss candidates.
- invalid or source-edge extraction windows that must not apply a zero-length
  timeline range.

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
- configured-channel OAuth, exact-link, analytics refresh, disconnect-clear and
  packaged Windows/macOS credential-store smoke when YouTube is enabled.
- user-authorized save/list/generate/remove packaged smoke for OpenAI and
  OpenRouter, with keys/prompts/responses excluded from captured artifacts.
- configured Diablo IV source scan on Windows and macOS with reviewed
  completion/death/boss reference moments, live extraction-window recalculation
  and no retained raw frames.
