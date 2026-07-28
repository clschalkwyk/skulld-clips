# Canvas Dock — Fidelity Ledger

| Requirement | Planned evidence | Status |
| --- | --- | --- |
| Full-width editor removes wasteful side gutters | Native macOS bundle inspected at 1162×768 | Pass |
| Narrow rail replaces wide layer panel | Native bundle capture and accessibility tree | Pass |
| Preview remains the dominant workspace | Native bundle capture with a verified 1920×1080 source | Pass |
| Timeline sits below preview and dock, not below rail | Native bundle capture | Pass |
| Placement prioritizes anchors and size | Svelte check and native selected-sting inspection | Pass |
| Timing is expressed relative to clip in | Six timing helper tests and native inspection | Pass |
| Start/end can be set from playhead | Duration-preserving helper tests and native accessibility tree | Pass |
| Exact values remain available without dominating the dock | Collapsed Advanced controls in native accessibility tree | Pass |
| Existing dark visual system is preserved | Native macOS bundle capture | Intentional visual-system match |
| Media and project contracts remain unchanged | Full `npm run verify` gate | Pass |

## Accepted deviations

- Rail controls use text and typographic glyphs rather than an icon package.
- Existing source metadata remains below the preview.
- Inspector and rail collapse at constrained widths instead of shrinking the
  preview into an unusable canvas.
