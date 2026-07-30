# Product and UX Specification

## Product statement

Skull’d Clip Forge is a focused desktop utility for creators who need to turn one raw gameplay clip into one branded vertical output quickly. It competes on workflow speed, not editing depth.

## User

Primary user:

- captures gameplay from PS5, OBS or Twitch;
- publishes short vertical clips;
- wants recognisable branding;
- does not want to open a heavyweight editor for every clip.

Job to be done:

> When I find a good gameplay moment, I want to trim, crop and brand it quickly so I can publish it as a vertical clip.

## Product principles

1. Local first.
2. One screen for editing.
3. Non-destructive metadata edits.
4. Safe defaults before codec jargon.
5. No fake cloud.
6. Export truth beats preview theatre.
7. No scope growth before the happy path is solid.

## Screens

### Home

Components:

- product title/mark;
- full-screen drop zone;
- **Open clip**;
- **Open project**;
- recent projects;
- settings/about.

Drop states:

| State | Message |
|---|---|
| Idle | Drop a gameplay clip here |
| Valid drag | Release to open clip |
| Invalid drag | That is not a supported video file |
| Probing | Reading clip details… |
| Failed | Could not read this clip. View details |

Recent project card:

- name;
- source filename;
- last opened;
- duration;
- thumbnail if available;
- source missing/changed status;
- open/reveal/remove-from-recents actions.

### Editor

```text
┌────────────────────────────────────────────────────────────────────┐
│ Back | Project name | Saved/Saving/Error | Export                  │
├────────┬───────────────────────────────────────────┬───────────────┤
│ Layer  │ Preview stage                             │ Compact dock  │
│ rail   │ 9:16 crop + overlays                      │ Crop/overlay  │
│        │                                           │ controls      │
│        ├───────────────────────────────────────────┴───────────────┤
│        │ Transport | trim | playhead | overlay visibility bars     │
└────────┴───────────────────────────────────────────────────────────┘
```

Minimum practical window: 1100×700. Below that, collapse the inspector to a drawer.

At or above the minimum window, the editor uses the full available application
width and is contained within the viewport. The layer rail remains narrow, the
preview receives the dominant workspace, and the timeline begins after the rail
below the preview and dock. The preview and inspector handle their own overflow;
the editor page does not become a long scrolling document.

### Export sheet

Default:

- preset;
- filename;
- destination;
- output summary;
- estimated size;
- overwrite warning;
- primary export action.

Advanced:

- quality mode;
- CRF;
- encoder preset;
- frame-rate mode;
- audio bitrate.

Active export:

- phase;
- percentage;
- encoded/total duration;
- speed/fps if available;
- elapsed time;
- cancel.

### Channel performance

This is a separate opt-in post-MVP workspace, available from Home and Editor. It
does not sit in the export happy path.

Disconnected/configuration states:

- integration unavailable in this build;
- no channel connected;
- browser authorization active;
- authorization denied/expired;
- credential store unavailable;
- network/API failure with retry guidance.

Connected state:

- connected channel title;
- explicit disconnect-and-clear confirmation;
- current project link state;
- recent channel uploads with title and publish date;
- YouTube URL/video-ID fallback;
- manual refresh for one project or all linked projects;
- scorecards for engaged views, total views, watch time, average duration,
  average percentage viewed, interactions and net subscribers;
- an accessible latest-fourteen-days table;
- valid pending state when YouTube has not published report rows.

The user always confirms the exact project/video link. The UI does not imply
that matching titles or filenames proves provenance. Scorecards identify the
last requested complete date and state that YouTube data may be delayed.

### Find clip moments

The Editor exposes a separate **Find moments** workspace for the current source.
The first analysis profile is intentionally Diablo IV-specific and scans locally
for:

- completion/title treatments with a bright emblem and gold title signature;
- player-death treatments with a wide pale title, red treatment and darkened
  gameplay field;
- boss encounters with a persistent health bar substantially wider than normal
  enemy health bars near the top HUD.

The panel shows progress and supports cancellation. Results include event time,
suggested clip range, confidence and safe evidence labels. The user can review
the source frame before choosing **Use suggested range**. Detection never
silently changes the timeline, and empty/error/cancelled states preserve the
current edit.

## Preview/crop interaction

The preview stage uses the display-oriented source aspect ratio and fits within
the available canvas row without forcing document-level scrolling. The video is
letterboxed only when the stage and decoded frame ratios differ. The crop frame
represents output.

Interactions:

- drag inside crop: pan;
- resize handles: zoom while retaining 9:16;
- wheel/trackpad: zoom around cursor;
- double-click: reset to maximum centered crop;
- arrows: nudge;
- Shift+arrows: larger nudge.

Persist six-decimal normalized coordinates. On export, convert to display-oriented source pixels, clamp and adjust to encoder-compatible dimensions.

## Timeline

This is not a full NLE.

It includes:

- duration ruler;
- in/out selection;
- playhead;
- overlay visibility bars;
- current time and range duration.

Storage: integer milliseconds.  
Display: `mm:ss.mmm`.  
Variable-frame-rate source editing remains time-based.

The selected overlay dock presents timing relative to the active clip: offset
after the in point and visible duration. Start/end-at-playhead actions are
primary; exact source-relative millisecond values remain available under
advanced controls.

## Overlay editing

Common properties:

- name;
- position;
- dimensions;
- opacity;
- start/end;
- z-index;
- delete;
- bring forward/send backward.

Image properties:

- replace asset;
- lock aspect;
- reset position.

Skull’d sting properties:

- insert a new sting at the current timeline playhead, reusing the selected sting
  asset and settings when available and clamping only when fewer than 500 ms remain;
- replace MP4;
- constrained `toasty-right` motion preset;
- choose 1×, 2×, or 3× playback;
- play once or repeat to fill the selected duration;
- duplicate a sting without importing the same asset again;
- include verified clip audio;
- reset to the bottom-right safe placement;
- choose one of nine safe-area anchors;
- adjust size as the primary numeric placement control;
- edit the start offset after clip in and duration directly in seconds;
- set the start or end from the current playhead;
- see the selected-speed animation length, cycle count, and playhead position;
- apply full-animation, two-loop, or fill-remaining timing presets;
- nudge by 1, 8, or 24 output pixels;
- adjust left and top under advanced controls;
- reveal exact normalized placement and timing values only on demand.

Up to eight stings are supported. A repeating sting is limited to 60 seconds and
the active clip range. Rust generates a bounded transparent PNG sprite with the
fixed green-screen key for local preview; the frontend advances or loops that
sprite at the selected playback rate. Export remains authoritative and is
covered by golden-frame tolerance. Selecting a sting moves the playhead to a
stable visible frame when the current playhead is outside its settled interval,
so placement changes are immediately visible.

Caption properties:

- text;
- bundled font;
- size;
- weight;
- alignment;
- line spacing;
- max width;
- fill;
- outline;
- optional background/padding;
- style preset.

Caption rendering is debounced by 250 ms and content-addressed by hash.

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Space | Play/pause |
| I | Set in |
| O | Set out |
| Left/Right | Approximate frame step |
| Shift+Left/Right | Seek one second |
| Home/End | Seek to in/out |
| Delete/Backspace | Delete selected overlay |
| Ctrl/Cmd+S | Save |
| Ctrl/Cmd+E | Export |
| Esc | Close modal/deselect |
| Ctrl/Cmd+Z | Undo when implemented |
| Ctrl/Cmd+Shift+Z | Redo when implemented |

Shortcuts do not trigger while typing, except Escape.

## Accessibility

- keyboard reachable controls;
- visible focus;
- slider semantics for timeline handles;
- numeric alternatives to drag;
- WCAG AA contrast;
- reduced-motion support;
- errors not colour-only;
- accessible button labels.
- semantic metric tables and an equivalent non-visual reading order;
- focus moves into the performance dialog and Escape closes it.

## Error UX

Errors include:

- what failed;
- stable error code;
- safe detail;
- corrective action;
- retry where appropriate;
- diagnostic bundle option for export failures.

No indefinite spinner after a child process exits.
