# Canvas Dock — Development Handoff

## Scope

Implement backlog item **SCF-060** as a focused Milestone 6 editor usability
slice. This is a layout and control-presentation change only.

## Implementation order

1. Move the timeline into the editor grid and remove the editor max-width.
2. Convert `LayerPanel.svelte` to the compact rail without changing add/select
   callbacks.
3. Add relative timing presentation helpers with unit coverage.
4. Recompose `OverlayInspector.svelte` around anchor, size, relative timing, and
   playhead actions.
5. Update responsive grid rules and compact timeline spacing.
6. Synchronize UX, acceptance, backlog, delivery status, and manifest hashes.

## Do not change

- Project schema or contracts.
- Tauri command/event names or payloads.
- Rust media pipeline or FFmpeg argument construction.
- Fixed Toasty-right speed and keying behavior.
- Autosave, relink, export, cancellation, or diagnostics behavior.

## Verification

Run:

```sh
npm run check
npm test
npm run verify
npm run tauri build -- --debug --bundles app
```

Then inspect the native macOS bundle at a desktop-sized window:

- no document-level scrolling;
- compact rail, dominant preview, and compact inspector visible together;
- timeline visible below the preview without extending under the rail;
- source and overlay selection work;
- relative timing labels update after setting start/end at playhead;
- advanced exact controls remain discoverable;
- caption flyout does not clip behind the preview.

## Rollback boundary

The change is isolated to Svelte editor composition, CSS, a pure timing helper,
tests, and documentation. No project migration or native rollback is required.
