# Inskea 

## Project at a glance

Inskea is an infinite-canvas drawing application written in Rust. The same Leptos 0.6 client runs as WebAssembly in a browser and inside a Tauri 2 desktop webview. The root crate contains the editor; `src-tauri/` is a separate native wrapper and command layer.

There is no Cargo workspace declaration. Run root-crate commands from the repository root and use `--manifest-path src-tauri/Cargo.toml` for the desktop crate.

## Repository layout

- `src/app.rs` — top-level Leptos component and shared application state/signals.
- `src/canvas/` — SVG canvas rendering, viewport transforms, pointer modes, text editing, grid, and selection handles.
- `src/model/` — serializable scene model, geometry, element types, styling, hit testing, and resize/rotation math.
- `src/edit_ops.rs` and `src/hotkeys.rs` — scene mutations, clipboard/group operations, undo/redo integration, and keyboard shortcuts.
- `src/ui/` — toolbar, dock/tool selection, settings, styling panels, reusable controls, and export UI.
- `src/skea.rs` — `.skea` serialization and migrations. The format is versioned; preserve compatibility when changing it.
- `src/tauri_bridge.rs` — browser-side bindings for Tauri dialogs, commands, file access, settings, and installed fonts.
- `src-tauri/src/lib.rs` — native Tauri commands and application setup. Keep command names and argument/return shapes synchronized with `src/tauri_bridge.rs`.
- `src-tauri/tauri.conf.json` — desktop build/dev commands, window configuration, asset permissions, file association, and bundle metadata.
- `input.css`, `output.css`, `Trunk.toml` — Tailwind input and the Trunk pre-build hook. `output.css` is generated.

## Development commands

Install Rust with the `wasm32-unknown-unknown` target, Node/npm, Trunk, and the Tauri CLI. Then install the CSS tooling once with `npm install`.

Browser development:

```sh
trunk serve
```

This runs the Tailwind hook from `Trunk.toml` and serves the WASM app at the configured Trunk URL. For a production-like browser build, use:

```sh
trunk build
```

Desktop development and packaging:

```sh
cargo tauri dev
cargo tauri build
```

The Tauri config runs `trunk serve`/`trunk build` automatically and expects the frontend output in `dist/`.

## Validation

Run the checks relevant to the files changed:

```sh
cargo fmt --all -- --check
cargo test
cargo check --manifest-path src-tauri/Cargo.toml
trunk build
```

Use `cargo fmt --all` to apply formatting. `cargo test` exercises the model, geometry, editing operations, and `.skea` round trips, but it does not exercise the browser DOM, SVG pointer events, or the native Tauri commands.

For UI, canvas, file, or desktop changes, also manually verify in both `trunk serve` and `cargo tauri dev` as applicable:

- draw/select/move/resize/rotate shapes and freehand paths;
- text creation, multiline editing, alignment, wrapping, caret movement, and undo/redo;
- keyboard shortcuts, grouping/layers, zoom/pan, and grid/settings behavior;
- save and reopen a `.skea` file, including an existing file when changing serialization;
- browser downloads versus native dialogs/file writes, SVG/PNG export, and image loading;
- first-run font installation and settings persistence in the desktop app.

## Implementation conventions

- Keep shared editor behavior in the root crate. Put OS/file-system-specific behavior behind `src/tauri_bridge.rs` and corresponding Tauri commands; preserve the browser fallback.
- Treat `Scene` and its element geometry as the source of truth. When changing transforms, hit testing, selection bounds, or rendering, check all of those paths together.
- Use Leptos signals and existing component patterns rather than introducing a second state mechanism. Keep mutation/undo snapshots at the same boundary as the existing edit operation.
- Add or update focused `#[cfg(test)]` tests for pure geometry, model, editing, and serialization changes. Prefer deterministic tests that do not require a browser or native window.
- When changing `.skea` serialization, increment and migrate `format_version` deliberately, retain readers for older versions, and add round-trip/backward-compatibility tests.
- Follow the existing Tailwind utility-class approach and shared style constants in `src/ui/styles.rs`; do not hand-edit generated `output.css`.
- Avoid broad refactors while changing interaction behavior. Canvas pointer coordinates, viewport zoom, selection handles, text layout, and SVG output are tightly coupled.

## Generated and local files

Do not commit build or local environment output: `target/`, `dist/`, `src-tauri/gen/`, `node_modules/`, or `output.css`. Both `Cargo.lock` files are tracked. `.env` and the local launcher files are also ignored.
