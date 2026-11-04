# AGENTS.md — DrawCraft

This file is the operating manual for coding agents working on DrawCraft. Read it before changing code.

## Clean-room rules

- **Do not copy** code, assets, algorithms, names, or visual designs from tldraw, godraw, Adobe, Figma, or any proprietary tool.
- Base behaviour on **public descriptions** and **original design**. When in doubt, write the simplest original implementation and document the reasoning.
- Do not add a dependency that exists mainly to reproduce a proprietary feature or asset.
- Keep the dual license (MIT OR Apache-2.0) and do not introduce copyleft runtime dependencies without discussion.

## Project shape

```
crates/geometry   pure math; serde only; no I/O
crates/document   document records, styles, validation; depends on geometry
crates/command    static command registry; no UI/editor dependency
crates/editor     headless engine; depends on command, document, geometry
crates/render     scene, SVG, PNG; depends on document, geometry
crates/format     .drawcraft format + export; depends on doc/render
crates/session    editor + files + recent list; depends on command/doc/editor/format/render
crates/mcp        MCP stdio server; depends on command/format/session
crates/app        egui/eframe UI; depends on everything above
xtask             quality/build/packaging tasks; no runtime layering
```

The dependency graph is enforced by `cargo run -p xtask -- layers`. Do not create cycles or reach from a lower layer into a higher one.

## The one rule for features

**Everything user-visible is a command.**

- Registry: `crates/command/src/registry.rs`
- IDs: `crates/command/src/lib.rs` (`ids::*`)
- Editor execution: `crates/editor/src/commands.rs`
- File/export execution: `crates/session/src/lib.rs`
- UI: `crates/app/src/*` calls `Session::execute` (or editor methods that the command also exposes)

Adding a command checklist:

1. Add an `ids::*` constant.
2. Add a `CommandDef` with category, scope, and typed params.
3. Handle it in `Editor::dispatch` or `Session::execute`.
4. Add/extend a unit test.
5. The test `every_editor_command_is_handled_by_dispatch` must keep passing.
6. Add UI only after the command works headlessly.

Tool gestures may mutate shapes directly during a drag for performance, but they must snapshot once at drag start and finish through editor state (`refresh_arrows`, selection, dirty). Prefer adding an editor method over duplicating transform logic in the UI.

## Coding standards

- Edition 2024, Rust 1.95+. Workspace clippy denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`.
- Production code returns `Result`; no panics on user input. Tests may use `unwrap`/`panic` (crate-level `cfg_attr(test, allow(...))`).
- Cap untrusted allocations. The document loader is limited to 16 MiB; images to 8192×8192 / 32 MiB; export scale to 8× and 64 MiB total pixels.
- Validate at boundaries and keep the engine total. Coordinates must be finite; sizes non-negative.
- Preserve unknown JSON fields (`extra` maps) at document, page, shape, and style level.
- Keep the UI thin. Do not move document logic into `crates/app`.
- Prefer small modules and file-level documentation over one giant file. `cargo fmt` is mandatory.

## Running and driving

```sh
cargo run                         # launch the desktop app
cargo run -- --help
cargo run -- catalog              # list commands
cargo run -- run shape.add --params '{"kind":"rectangle","x":0,"y":0,"width":100,"height":60}'
cargo run -- export board.drawcraft --out board.svg
cargo run -- mcp --file board.drawcraft
```

MCP is line-delimited JSON-RPC over stdio. Useful methods: `initialize`, `tools/list`, `tools/call`, `resources/read`. The universal tool is `drawcraft_command` with `{ "id": "...", "params": { ... } }`.

The same flow is available over the local session API used by the UI, so an agent script that works through MCP should produce the same document a human would.

## Quality gates

```sh
cargo run -p xtask -- check
```

This runs:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `xtask layers` (dependency rules)
5. `xtask commands` (registry coverage test)

Other tasks: `xtask fmt`, `clippy`, `test`, `build`, `wasm`, `bundle`, `layers`, `commands`, `run -- ...`.

## Where things live

- Camera and coordinate math: `crates/geometry/src/camera.rs`
- Transforms and shape bounds: `crates/document/src/shape.rs`
- Validation: `crates/document/src/document.rs`
- Arrow binding/routing: `Editor::refresh_arrows` in `crates/editor/src/editor.rs`
- Undo: `crates/editor/src/history.rs`
- Scene building and shape paths: `crates/render/src/{scene,shapes}.rs`
- SVG/PNG: `crates/render/src/{svg,png}.rs`
- Atomic save and export: `crates/format/src/lib.rs`
- File/session commands: `crates/session/src/lib.rs`
- MCP protocol: `crates/mcp/src/lib.rs`
- Canvas gestures: `crates/app/src/canvas.rs`
- Panels and style controls: `crates/app/src/panels.rs`
- Theme and palette: `crates/app/src/theme.rs`
- Original line icons: `crates/app/src/icons.rs`

## Current limitations (do not “fix” by guessing)

- Line/polyline vertex dragging is not implemented yet; points are stored and routed but not edited by handles.
- Frame membership (`parent_id`) clips when set, but the UI does not yet move shapes into frames automatically.
- Autosave, crash recovery, and external-change detection are roadmap items, not implemented.
- `.tldr` interoperability is not implemented. Keep the format native and clean-room.
- The browser/WASM UI entry point is not wired yet; the core crates build for `wasm32-unknown-unknown`.

When a task touches one of these, update `ROADMAP.md` and the tests together.
