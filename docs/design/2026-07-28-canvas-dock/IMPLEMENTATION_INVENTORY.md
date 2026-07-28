# Canvas Dock — Implementation Inventory

## Page structure

| Area | Existing owner | Required change |
| --- | --- | --- |
| Editor shell | `src/App.svelte`, `src/styles.css` | Remove editor max-width and keep the shell viewport-bound |
| Layer rail | `LayerPanel.svelte` | Compact source, add actions, and overlay selection into a narrow rail |
| Preview | `PreviewStage.svelte` | Preserve behavior; allow it to consume the reclaimed width |
| Inspector dock | `OverlayInspector.svelte`, `CropInspector.svelte` | Compact overlay primary controls; retain crop inspector |
| Timeline | `Timeline.svelte` | Move inside the editor grid below preview and dock |
| Source metadata | `src/App.svelte` | Preserve directly below preview |

## Interaction inventory

| Interaction | Contract |
| --- | --- |
| Select source | Set `selectedOverlayId` to `null` and expose crop controls |
| Select overlay | Preserve current ID-based selection |
| Add image/sting/caption | Preserve existing project mutation functions |
| Caption creation | Open a rail-adjacent flyout; retain validation and render status |
| Place overlay | Direct preview drag, anchor grid, reset/safe corner, and size |
| Fine adjustment | Keep nudge and exact normalized values under advanced controls |
| Set overlay timing | Present relative offset and duration; preserve integer milliseconds |
| Set start/end from playhead | Clamp through existing overlay timing constraints |
| Trim clip | Preserve timeline inputs, range sliders, shortcuts, and 250ms minimum |
| Export | Preserve current export sheet and native command boundary |

## Accessibility inventory

- Rail controls use visible labels and descriptive `aria-label` text.
- Selected source and overlay items expose `aria-pressed`.
- Anchor buttons retain labels for all nine positions.
- Advanced controls use native `details` and `summary`.
- Time controls retain number inputs for keyboard users.
- Existing focus, disabled, error, and live status behavior remains in place.

## Data and native boundaries

- No project schema, Tauri command, Rust, FFmpeg, or persistence change.
- Overlay timing remains integer milliseconds in the project contract.
- Overlay coordinates remain normalized and continue through the existing pure
  coordinate mapper.
- The fixed sting speed, keying, duration ceiling, and audio flag remain unchanged.
