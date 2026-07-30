# Media and Rendering Pipeline

## Probe

Representative ffprobe argument vector:

```text
-v error
-print_format json
-show_format
-show_streams
-show_chapters
-show_error
<input>
```

Controls:

- timeout;
- bounded JSON/stderr;
- exit-status validation;
- cancellation on app exit.

Video selection:

1. ignore attached pictures;
2. prefer default stream;
3. otherwise first usable video.

Audio selection:

1. prefer default;
2. otherwise first audio;
3. absence is valid.

Normalized output includes:

- duration/container/file size;
- selected stream indexes;
- raw/display dimensions;
- rotation 0/90/180/270;
- average and real frame rates;
- codecs;
- pixel/sample aspect;
- audio properties;
- warnings.

## Local clip discovery

The Diablo IV discovery profile samples the authorized source through a fixed
Rust-owned FFmpeg argument vector. It decodes two 320×180 RGB frames per second
to a bounded streaming reader; it does not write frames to the project, retain
media after analysis or expose raw FFmpeg inputs to Svelte.

Pure Rust classifiers inspect fixed normalized HUD regions for completion/title,
player-death and persistent wide boss-health-bar signatures. Adjacent positive
frames are grouped into timestamped candidates, short transients are rejected,
and at most 50 review suggestions are returned. Confidence is heuristic, not a
claim of semantic certainty. Each result retains the raw detected point or boss
interval. The default suggestion adds 15 seconds before and 5 seconds after
those anchors. Svelte may apply a validated 0–300 second before/after window to
the anchors without decoding the source again.

Analysis runs as one cancellable native job with structured progress. Timeout,
decode failure, cancellation and application exit terminate FFmpeg and leave the
project unchanged. A candidate modifies the timeline only after explicit user
selection.

## Coordinate model

Persist crop relative to display-oriented source:

```ts
type NormalizedRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};
```

Pixel conversion:

```text
x = round(normX * displayWidth)
y = round(normY * displayHeight)
w = round(normW * displayWidth)
h = round(normH * displayHeight)
```

Then:

- clamp;
- preserve 9:16 within tolerance;
- prefer even dimensions for H.264/yuv420p;
- log correction only at debug level.

All preview/export conversions use one pure mapper.

## Caption rasterization

Decision: render caption to transparent PNG in Svelte/webview using bundled fonts.

Algorithm:

1. Normalize style/text.
2. Wrap lines.
3. Render at output scale.
4. Export PNG.
5. Hash render-affecting properties + text.
6. Save `assets/captions/<sha256>.png`.
7. Reuse same hash.
8. Persist logical properties and generated asset reference.

Benefits:

- no filter escaping;
- cross-platform font control;
- preview parity;
- same overlay pipeline as images.

## FFmpeg graph

Stages:

1. orientation normalize;
2. video trim/timestamp reset;
3. audio trim/timestamp reset if audio exists;
4. crop;
5. scale 1080×1920;
6. sample aspect reset;
7. overlay each PNG in z-order;
8. key and composite the optional fixed-preset sting in z-order;
9. mix optional sting audio;
10. encode/mux.

Illustrative graph:

```text
[0:v]
  <orientation>,
  trim=start=<in>:end=<out>,
  setpts=PTS-STARTPTS,
  crop=<w>:<h>:<x>:<y>,
  scale=1080:1920:flags=lanczos,
  setsar=1
[base];

[base][1:v]
  overlay=<x>:<y>:enable='between(t,<start>,<end>)'
[v1];

[v1][2:v]
  overlay=<x>:<y>:enable='between(t,<start>,<end>)'
[vout]
```

Optional audio:

```text
[0:a]
  atrim=start=<in>:end=<out>,
  asetpts=PTS-STARTPTS
[aout]
```

Rust accepts validated values, not raw filter fragments.

### Constrained Skull’d sting

The `toasty-right` preset is generated entirely by Rust:

- input MP4 is project-owned, content-addressed and pre-probed;
- import creates a bounded 12 fps transparent PNG sprite for preview;
- video is played at the validated 1×, 2×, or 3× rate;
- a repeating input loops only long enough to fill its validated overlay window;
- `chromakey` uses the fixed captured-green profile;
- the keyed frame is scaled through the output-canvas coordinate mapper;
- x position enters from the right over 180 ms and exits over 120 ms;
- `eof_action=pass` and `repeatlast=0` prevent a frozen final frame;
- optional clip audio uses the matching validated tempo, is delayed to the
  overlay start, mixed with every other enabled sting through bounded source
  ducking and limited before AAC encoding.

Up to eight stings are accepted in a project. Repeating windows are limited to
60 seconds and the active clip. New stings default to 1×/once; missing fields in
legacy projects retain 3×/once behavior. No key colour, similarity, blend,
playback expression, filter expression or raw media argument crosses the
frontend boundary.

## Representative encoding arguments

```text
-hide_banner
-nostdin
-y
-i <source>
-loop 1 -framerate 1 -i <caption.png>
-loop 1 -framerate 1 -i <logo.png>
-filter_complex <generated>
-map [vout]
-map [aout]?
-c:v libx264
-preset medium
-crf 20
-profile:v high
-pix_fmt yuv420p
-r <derived>
-c:a aac
-b:a 192k
-ar 48000
-movflags +faststart
-progress pipe:1
-nostats
<output.partial.mp4>
```

Rules:

- process argument vector only;
- `-y` only after overwrite is approved;
- audio branch depends on source and verified sting probes;
- output duration is bounded;
- final filename does not exist before verification.

## Quality modes

| Mode | CRF | Preset |
|---|---:|---|
| Draft | 24 | veryfast |
| Balanced | 20 | medium |
| High | 18 | slow |

Balanced is default.

## Frame rate

- explicit 30 or 60 if selected;
- otherwise source average rounded sensibly and capped at 60;
- variable-rate sources become constant-rate output;
- tests verify duration/audio sync.

## Progress

Use:

```text
-progress pipe:1
-nostats
```

Parse keys such as:

- frame;
- fps;
- total_size;
- out_time_us/out_time_ms/out_time;
- speed;
- progress.

Percentage = encoded time / expected duration, clamped below 100% until verification and rename complete.

## Cancellation

1. mark cancelling;
2. request termination;
3. wait brief grace period;
4. force-kill process tree;
5. close readers;
6. remove partial output;
7. emit cancelled.

Windows/macOS process-tree handling is platform-tested.

## Output verification

ffprobe checks:

- file exists/nonzero;
- usable video;
- 1080×1920;
- expected codec/pixel format family;
- duration tolerance;
- expected audio presence;
- MP4-compatible container.

Only then atomically rename partial to final.

## Disk estimate

For CRF, bitrate is unknown. Use a conservative profile estimate and require:

```text
free >= max(estimated * 2, estimated + 1 GiB)
```

This is validation guidance, not an exact prediction.

## Golden frames

For fixture projects:

1. export;
2. extract frames at fixed times;
3. compare perceptually;
4. verify overlay/crop coordinates within tolerance;
5. verify ffprobe metadata.

This catches “preview looks right, export is wrong,” the classic multimedia practical joke.
