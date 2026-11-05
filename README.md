# DrawCraft

**An open-source, local-first infinite-canvas whiteboard and diagramming app.** Pure Rust, GPU-rendered with `wgpu` through `eframe`/`egui`, and drivable by humans, scripts, and agents through the same command system.

DrawCraft is clean-room: no copied code, assets, or algorithms from tldraw, godraw, Adobe, or other proprietary tools. See [AGENTS.md](AGENTS.md) for the rules that keep it that way.

## What works today (v0.3.0)

- **Infinite canvas** with pan, zoom, pinch, zoom-to-fit, grid, minimap, dark mode, and focus mode.
- **Tools:** select, lasso, hand, freehand draw, highlighter, eraser, laser, line, arrow, text, sticky note, image, frame, and 20 geometric shapes.
- **Editing:** click/shift/brush/lasso selection (groups are atomic; lasso is fully-contained), select first/last by z-order, move, resize, rotate, flip, duplicate, Alt-drag copy, grid/shape snapping (hold Ctrl/Cmd while dragging), group/ungroup, lock, z-order, align, distribute, copy/paste, undo/redo, multiple pages.
- **Styling:** solid/dashed/dotted/hand-drawn strokes; none/semi/solid/pattern/full fills; stroke width, opacity, font size; labels and text; shape/arrow styling panel.
- **Arrows:** bind endpoints to shapes and follow them, straight/bent/elbow routing, start/end heads, arrow labels.
- **Documents:** atomic `.drawcraft` JSON save, autosave + crash recovery, external-change detection (reload / keep mine), Open Recent with missing-file handling, session info.
- **Export:** SVG and PNG, whole page or selection.
- **Agent surface:** CLI, MCP stdio server, JSON document resource, command catalog, base64 previews.

Known gaps and the path to v1 are tracked in [ROADMAP.md](ROADMAP.md). The biggest remaining v1 items are line vertex editing, deeper frame membership UX, and `.tldr` interoperability.

## Design and theme

The UI follows the GrokDay/GrokNight visual language: a neutral gray base and a single magenta accent.

- **GrokDay** (default): `#eeeeee` base, `#f5f5f5` canvas, `#eaeaea` panels, `#262626` text, `#7D4BC6` accent.
- **GrokNight**: `#141414` base, `#0a0a0a` canvas, `#111111` panels, `#e1e1e1` text, `#bb9af7` accent.

Launch directly in Night with `cargo run -- --dark`, or toggle with **View → GrokNight** / the Day/Night control in the top bar. Icons are original line drawings rendered with egui primitives, so they do not depend on platform symbol fonts.

## Quick start


```sh
cargo build
cargo run                       # launch the desktop app
cargo run -- --help             # CLI help
```

The app opens as a native window on macOS, Windows, and Linux. `cargo run` is configured to build the app binary by default.

### CLI

```sh
# Create and save a document headlessly
cargo run -- run shape.add \
  --params '{"kind":"star","x":10,"y":20,"width":120,"height":100,"text":"Hello"}' \
  --file board.drawcraft --save

# Add a second page
cargo run -- run page.add --params '{"name":"Architecture"}' \
  --file board.drawcraft --save

# Export
cargo run -- export board.drawcraft --out board.svg
cargo run -- export board.drawcraft --out board.png --format png
```

`drawcraft catalog` lists every command and its parameters. `drawcraft run` accepts any command id.

### MCP (agents)

```sh
cargo run -- mcp --file board.drawcraft
```

The server is line-delimited JSON-RPC 2.0 over stdio. It exposes:

- `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`
- tools: `drawcraft_command`, `drawcraft_list_commands`, `drawcraft_document`, `drawcraft_open`, `drawcraft_save`, `drawcraft_export`, `drawcraft_render_preview`
- resources: `drawcraft://document`, `drawcraft://commands`

Every tool ultimately calls the same `Session::execute` path as the UI and CLI.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"shape.add","params":{"kind":"ellipse","x":0,"y":0,"width":80,"height":60}}}}
```

### Quality gate

```sh
cargo run -p xtask -- check
```

Runs formatting check, strict clippy (`-D warnings`), all tests, layer-rule validation, and command coverage.

Other tasks: `xtask fmt`, `clippy`, `test`, `build`, `wasm`, `bundle`, `layers`, `commands`, `run -- ...`.

### Releases

Push an annotated `vX.Y.Z` tag to cut a release:

```sh
git tag -a v0.3.0 -m "v0.3.0"
git push origin v0.3.0
```

`.github/workflows/release.yml` builds the desktop binary for Linux (x86_64), macOS (aarch64), and Windows (x86_64), packages each with the licenses, README, and changelog (`.tar.gz` on Unix, `.zip` on Windows), and publishes a GitHub release with `checksums.txt` and generated notes. Tags with a hyphen (`v0.3.0-rc.1`) are marked as prereleases.

## Architecture

Layered, testable, no window required below the UI:

```
geometry        pure math: Vec2, Rect, Transform, Camera, path simplification
  └─ document   records, styles, pages, validation, unknown-field preservation
       └─ command   static command registry + parameter metadata
            └─ editor   camera, selection, tools, history, arrow binding, dispatch
                 ├─ render   scene graph, SVG writer, PNG rasteriser (resvg)
                 ├─ format   .drawcraft JSON, atomic save, export
                 │    └─ session  editor + file lifecycle + recent files
                 │         ├─ mcp     JSON-RPC/MCP stdio server
                 │         └─ app     thin egui/eframe UI
                 └─ ... 
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for invariants and data flow, [FORMAT.md](FORMAT.md) for the native format, and [ROADMAP.md](ROADMAP.md) for the v1 plan.

## Design rules

- **Clean-room.** Public descriptions and original design only.
- **No panics in production paths.** `Result` and validation at every boundary; untrusted input is size-capped.
- **Everything user-visible is a command.** The UI, CLI, and MCP all use the same registry and dispatch path.
- **The engine is headless.** Camera, selection, history, and transforms are tested without a window.
- **Unknown data survives.** Newer document fields are preserved through `extra` maps.

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
