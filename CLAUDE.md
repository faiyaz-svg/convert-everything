# Convert Everything

A free, open-source Windows desktop app that converts files between formats on the user's own computer. Tauri 2 (Rust) back end, Svelte 5 + TypeScript front end, and bundled open-source engines that run as separate programs.

## Hard rules

- **The project has a cap of 50 git commits in total.** Run `git rev-list --count HEAD` before every commit. Group related work into one commit. Never squash or rewrite pushed history to get under the cap.
- Everything stays free: no paid services, signing or dependencies.
- No co-author or "Generated with" lines in commits.
- UI text follows `STYLE.md`. The look follows `DESIGN.md` (copied from the Math and CS Foundations textbook sites).

## Layout

- `src-tauri/src/core/`: conversion core, no Tauri types, so tests can drive it.
  - `formats.rs` every format; `routes.rs` which conversions exist, which engine runs each, and their options; `detect.rs` file type detection; `run.rs` runs engine programs (hidden window, progress, cancel, log); `convert/` one module per engine.
- `src-tauri/src/app/`: Tauri commands, the job queue (`jobs.rs`), settings and history (`store.rs`), the Explorer menu (`explorer.rs`).
- `src/`: the Svelte UI. `lib/state.svelte.ts` holds app state and actions; `lib/api.ts` calls Rust, or `lib/mock.ts` in a plain browser.
- `scripts/fetch-engines.ps1` + `scripts/engines.lock.json`: download, check, unpack and trim the engines into `src-tauri/engines/` (gitignored).

## Commands

- `npm run engines` downloads the engines (about 1.8 GB unpacked). Needed once before `tauri dev` or tests.
- `npm run dev` runs the UI in a browser with the mock back end. Add `?demo=1` to the URL for sample files.
- `npm run tauri dev` runs the real app.
- `npm run test:rust` runs unit tests. `npm run test:convert` runs every conversion route through the real engines (results in `tests/out`). Set `CE_ONLY=pdf` to test only routes that mention a format.
- `npm run release` builds the NSIS installer with the engines included.
- After changing `routes.rs`, refresh the browser mock: `cargo test --manifest-path src-tauri/Cargo.toml write_registry_for_ui_preview -- --ignored`.

## Adding a conversion

1. Add the format to `formats.rs` if it is new.
2. Add or extend a rule in `routes.rs` (order matters: the first rule that matches wins) and its options in `build_options`.
3. Handle it in the engine module under `convert/`.
4. Run `npm run test:convert` with `CE_ONLY=<format>`.
