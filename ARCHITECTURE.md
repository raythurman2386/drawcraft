# Architecture

DrawCraft is a layered Rust workspace. The rule that keeps it maintainable is simple: **lower layers never know about higher layers, and the engine never needs a window.**

```
drawcraft-geometry   pure math, serde only
      │
drawcraft-document   records, styles, pages, validation
      │
drawcraft-command    static command catalog + parameter metadata
      │
drawcraft-editor     camera, selection, tools, history, transforms, dispatch
      │
      ├── drawcraft-render   scene, SVG, PNG (resvg)
      ├── drawcraft-format   .drawcraft JSON, atomic save, export
      └── drawcraft-session  editor + files + recent list
                 ├── drawcraft-mcp   MCP stdio server
                 └── drawcraft-app   egui/eframe UI
```

`xtask` is outside the runtime layers and enforces the dependency graph.

## Data flow

```
human / CLI / MCP
       │
       ▼
  Session::execute(id, params) ──────────────┐
       │                                     │
       ├─ file.* / export.* / doc.*          │
       │      → format + atomic I/O          │
       │                                     │
       └─ editor.dispatch(id, params)        │
              │                              │
              ├─ validate params             │
              ├─ mutate Editor               │
              ├─ record undo snapshot        │
              └─ return JSON result          │
                                             ▼
                                   .drawcraft on disk
```

Rendering is pull-based: the UI asks `drawcraft-render` for a `Scene`, then draws it with egui. Export asks for the same `Scene` and writes SVG or rasterises it with resvg. There is no second geometry implementation for export.

## Undo

Undo is a bounded stack of whole-document snapshots (`History`). This is deliberately simple and preserves every invariant:

- `Editor::snapshot()` is called once before a logical edit.
- Interactive drags snapshot once at drag start, then update shapes directly to avoid one snapshot per frame.
- No-op commands do not create duplicate snapshots.
- Redo is cleared by any new edit.

Documents are plain data and cloning is cheap relative to interactive editing. If documents grow into the hundreds of thousands of shapes, the history layer can move to structural sharing behind the same API.

## Command system

The registry is the single source of truth:

- `drawcraft-command::Registry` has every id, label, category, scope, and typed parameter.
- `Editor::dispatch` handles editor commands.
- `Session::execute` handles file/export commands and delegates the rest.
- The UI calls the same ids; there are no UI-only document mutations except interactive drag previews, which are finalised through the same editor state.

Adding a command:

1. Add an id constant in `crates/command/src/lib.rs`.
2. Add a `CommandDef` in `crates/command/src/registry.rs`.
3. Handle it in `Editor::dispatch` (or `Session::execute` for I/O).
4. Add a test; the “every editor command is handled” test will catch omissions.
5. Wire a UI affordance only after the command exists.

## Geometry and coordinates

- World space is unbounded; `Vec2` and `Rect` are `f64`.
- A shape stores `pos`/`size` in local space and `rotation` in radians.
- `ShapeRecord::world_transform()` applies rotation and flips around the shape centre.
- Points for freehand/line/arrow records are absolute world points. Transformations update them consistently.
- The camera maps world ↔ viewport with `(0,0)` at the top-left of the canvas. Zoom is anchored at the pointer.

## Validation boundaries

Untrusted input enters at:

- `format::load` (8/16 MiB cap, UTF-8, JSON schema, `Document::validate`);
- MCP/CLI command parameters (typed `Params` helpers);
- image sources (size and dimension caps).

Everything below those boundaries can assume valid, finite data. The engine still avoids panics: no `unwrap`, no indexing without checks, no `panic!` in production paths.

## Rendering pipeline

1. `build_scene(page, selection)` walks shapes in z-order.
2. Each shape becomes a `SceneItem` with fill paths, stroke paths, optional text/image, clip rect, opacity, and hatch lines for pattern fills.
3. The UI converts scene paths to egui meshes:
   - fills use ear-clipping triangulation (`render::triangulate`) for concave shapes;
   - strokes use polylines, with dash splitting for the GPU path;
   - text uses egui’s galley layout with rotation around the shape centre;
   - images use a per-shape texture cache.
4. SVG export writes the same scene as `<path>`, `<text>`, `<image>`, `<clipPath>`, and `<pattern>`.
5. PNG export rasterises that SVG with `resvg` and system fonts.

## UI thinness

`crates/app` only:

- lays out panels and widgets;
- translates pointer gestures into editor operations;
- paints the scene and selection chrome;
- calls `Session::execute` for commands.

It does not own the document, undo history, arrow routing, validation, or export logic.

## Testing strategy

- **geometry:** camera invariants, rect algebra, point-in-polygon, simplification.
- **document:** validation, page operations, unknown-field preservation, reference remapping.
- **editor:** every command is dispatched; transforms; arrow binding; copy/paste/duplicate; pages.
- **render:** all geometric kinds produce paths; triangulation; SVG structure.
- **format:** JSON round-trip, unknown fields, oversized rejection, atomic save, SVG/PNG export.
- **session:** command flow, save/open/export round-trip.
- **mcp:** initialize, tools, notifications, parse errors, resource reads.

Run everything with `cargo run -p xtask -- check`.
