# Third-party notices

This repository builds Skull’d Clip Forge with open-source software. Exact
dependency versions are recorded in `package-lock.json` and
`src-tauri/Cargo.lock`.

## Application dependencies

- Tauri and its Rust crates: Apache-2.0 or MIT, depending on the crate.
- Svelte, Vite, TypeScript and Vitest: MIT.
- Inter font files from `@fontsource/inter`: SIL Open Font License 1.1.
- Rust crates: licences are declared by each locked crate and must be audited
  before public distribution.

## FFmpeg and ffprobe

Internal development and CI use separately installed FFmpeg and ffprobe. They
are not bundled by this repository.

Before any public or commercial distribution, choose an LGPL-only or GPL-enabled
FFmpeg build, pin its exact source revision and configure flags, archive the
corresponding source/build instructions, include its licence text and
acknowledgement, assess codec patent implications for each target market, and
obtain legal review.

This notice is an engineering inventory, not legal advice or a substitute for a
release-specific licence audit.
