# Canvas Dock — Design Source of Truth

## Reference

- Accepted direction: **Canvas dock**, selected from the interactive editor
  directions reviewed in the Codex conversation on 2026-07-28.
- Role: implementation specification for the Milestone 6 editor layout.
- Target: the existing Svelte editor in `src/App.svelte` and
  `src/components/editor/`.

## Binding layout

- Use the full available application width; the editor must not retain the
  home-screen `1240px` maximum width.
- Keep the existing project header as a compact fixed row.
- Replace the wide layers sidebar with a narrow tool/layer rail.
- Give the local video preview the dominant share of the workspace.
- Keep the selected overlay inspector in a compact right-hand dock.
- Place the timeline directly below the preview and inspector. It must not extend
  underneath the layer rail.
- Keep the rail, preview, inspector, and timeline within the available viewport.
  The preview and inspector may scroll internally when required; the whole editor
  page must not become a long scrolling document.

## Binding interactions

- Selecting **Source** in the rail opens crop controls.
- Selecting an overlay opens its controls in the right dock.
- The primary placement controls are:
  - direct manipulation on the preview;
  - a nine-position anchor grid;
  - a size control;
  - reset/safe-corner placement.
- The primary timing controls describe the overlay relative to the selected clip:
  start offset after clip in and visible duration.
- Provide **Start at playhead** and **End at playhead** actions.
- Keep absolute coordinates, opacity, stacking, nudge distance, and absolute
  millisecond values available under collapsed advanced controls.
- SCF-060 originally preserved the fixed Toasty-right 3x behavior. SCF-061
  supersedes that product constraint with 1x/2x/3x and once/repeat controls
  while keeping the fixed key and entrance/exit preset.

## Responsive behavior

- Desktop target: 1440 x 900 and larger. Show rail, preview, inspector, and
  timeline simultaneously.
- Laptop target: approximately 1100 x 700. Keep the compact rail and preview;
  collapse the inspector when the available width can no longer support useful
  preview manipulation.
- Narrow target: 680px or less. Hide the rail and inspector, keep the preview and
  timeline usable, and retain keyboard shortcuts.

## Visual direction

- Preserve the existing Skull'd Clip Forge dark, square-edged visual language.
- Use the existing warm coral accent for primary actions and trim selection.
- Use the existing green accent for a selected overlay or confirmed anchor.
- Do not add a new icon library, decorative imagery, gradients, or animation.

## Intentional deviations from the mockup

- The implementation retains the product's existing typeface and color tokens
  instead of copying presentation-only mockup styling.
- Text labels and simple glyphs are used in the rail to avoid a new icon
  dependency.
- Existing expert controls remain available under an advanced disclosure instead
  of being deleted.
- Source metadata remains visible below the preview because it is verified local
  media information, not placeholder content.

## Resolved assumptions

- The compact dock applies to captions and image overlays as well as the sting so
  the editor has one consistent interaction model.
- Caption creation uses a rail-adjacent flyout because the narrow rail cannot
  contain a usable text field.
- There are no blocking design questions for this implementation slice.
