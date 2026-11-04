# DrawCraft — roadmap to v1

**Vision:** a local-first, native, clean-room infinite-canvas whiteboard that feels as direct as tldraw but is built to be extended and driven by agents. v1 means the product is trustworthy: it does not lose work, renders what it exports, and every action is available through the same command surface as the UI.

**Current release:** 0.2.0 (foundation + interactive vertical slice).

---

## Definition of done for v1

v1 ships when all **P0** items below are complete and the quality gate is green:

- [ ] create/edit/save/load a multi-page document without data loss;
- [ ] all tools in the product list work with pointer + keyboard;
- [ ] styles and exports visually agree;
- [ ] undo/redo covers every document mutation, including pages and transforms;
- [ ] autosave + crash recovery + external-change detection are tested;
- [ ] an agent can complete a full scenario through MCP alone in CI;
- [ ] desktop bundles build for macOS, Windows, Linux; WASM core builds;
- [ ] no panics in production paths; fuzz corpus for parsers is clean;
- [ ] docs (`README`, `FORMAT`, `ARCHITECTURE`, `AGENTS`) match the code.

---

## Status at 0.2.0

| Area | State | Notes |
| --- | --- | --- |
| Workspace / layers | ✅ | 10 crates, enforced by `xtask layers`; strict clippy; 70+ tests |
| Document model | ✅ | records, styles, pages, validation, unknown-field preservation |
| Command surface | ✅ | 56 commands; dispatch coverage test; CLI + MCP + UI share it |
| Rendering | ✅ | shared scene; all 20 geometric shapes; text, images, arrowheads, patterns |
| SVG export | ✅ | paths, dashes, text, clips, arrowheads, images |
| PNG export | ✅ | resvg rasterisation, scale + transparency limits |
| Tools | ✅ | select, hand, draw, highlighter, eraser, laser, line, polyline, arrow, text, sticky, image, frame, shapes |
| Editing | 🟡 | move/resize/rotate/flip, duplicate, Alt-drag, snapping, group, lock, z-order, align, distribute, undo/redo, pages |
| Arrows | 🟡 | binding + straight/bent/elbow + heads; vertex editing not yet |
| Documents | 🟡 | save/open/recent; autosave/recovery/external-change pending |
| View | ✅ | pan, zoom, pinch, fit, minimap, dark, focus, grid |
| WASM UI | 🟡 | core crates build; browser entry not wired |
| `.tldr` interop | ❌ | planned for 1.1; format remains native and clean-room |

Legend: ✅ done for the current scope · 🟡 partial · ❌ not started.

---

## Phase 1 — finish the editing core (P0)

The interaction model must be complete before adding more surface area.

- [ ] **Selection**: marquee already works; add lasso selection; make shift-click group behaviour consistent; selection order commands (first/last).
- [ ] **Transform**: finish edge handles, aspect-ratio lock (Shift), proportional resize from centre (Alt), rotation snapping (Shift 15°), numeric transform readout.
- [ ] **Vertex editing**: drag points/vertices on line, polyline, arrow, and freehand records; add/remove vertices; convert corner ↔ smooth.
- [ ] **Snapping**: visible guides (not just silent offset), distance labels, snap to shape edges/centres/grid; toggle per document.
- [ ] **Alt-drag copy**: verify it copies groups, arrows, and frame membership; add a command (`edit.duplicate_drag`) so agents can reproduce it.
- [ ] **Undo semantics**: coalesce continuous style changes (colour picker/slider) into one step; make page switches non-undoable; test deep history.
- [ ] **Keyboard**: document shortcuts, focus handling, and command palette parity.

**Exit:** a 50-action scripted editing scenario passes through commands; UI smoke test covers every gesture.

## Phase 2 — frames, groups, and pages (P0)

- [ ] **Frames**: drawing a frame around selected shapes parents them; moving a frame moves children; resizing clips; “send to frame” command.
- [ ] **Groups**: nested groups; selecting one member selects the group; group bounds; ungroup restores membership.
- [ ] **Pages**: reorder by drag; duplicate with remapped ids (done); page thumbnails; page-level background/colour.
- [ ] **Layering**: layer panel with lock/visibility and drag reorder.

**Exit:** frame/group/page scenarios have headless tests and UI affordances.

## Phase 3 — text, images, and styling polish (P0/P1)

- [ ] **Text**: inline editing on canvas (not only the floating window); caret, selection, wrap, alignment, vertical alignment; font family picker using system fonts.
- [ ] **Images**: drop/paste images; embed as data URLs for portability; crop; aspect lock; broken-source placeholder.
- [ ] **Styles**: custom colour picker with recent swatches; pattern density; hand-drawn intensity; per-shape vs per-selection editing.
- [ ] **Sticky notes**: auto-resize to text; palette; stacking.
- [ ] **Accessibility**: keyboard navigation of canvas objects, screen-reader labels, visible focus.

## Phase 4 — document lifecycle (P0)

- [ ] **Autosave**: debounce, atomic write (temp + rename), configurable interval.
- [ ] **Crash recovery**: recover the last autosave on launch, with a prompt and diff-free restore.
- [ ] **External change detection**: poll mtime/hash; prompt to reload/overwrite; never silently clobber.
- [ ] **Open Recent**: already in session; add UI, clear list, missing-file handling.
- [ ] **Packed format decision**: keep single-file JSON with embedded images or move to a zip container (`.drawcraft` with manifest/assets). Decide before 1.0 and document migration.
- [ ] **fuzz**: `format::from_json` and image decoding; corpus + CI job.

**Exit:** kill -9 during editing restores work; external edit is detected; fuzz corpus clean.

## Phase 5 — export and interoperability (P0/P1)

- [ ] **PNG parity**: ensure screen and raster agree for every shape/style; add golden-image tests.
- [ ] **SVG parity**: golden-file tests for each shape/arrow/dash/fill combination.
- [ ] **Copy as SVG/PNG** to clipboard.
- [ ] **Export options**: scale, padding, background, current page vs all pages.
- [ ] **`.tldr` (P1)**: clean-room reader/writer from public format descriptions; preserve unknown records; round-trip fixtures. Do not copy code or assets.
- [ ] **PDF/print (P2)**: via SVG if cheap.

## Phase 6 — performance and rendering (P0 core / P1 polish)

- [ ] **Scene cache**: invalidate per-shape on document revision; avoid rebuilding the whole scene each frame.
- [ ] **Spatial index**: grid or BVH for hit-testing and marquee on 10k+ shapes.
- [ ] **Render batching**: merge strokes/fills where possible; measure with `xtask bench`.
- [ ] **Parallel export**: rasterise large pages on a worker; keep UI responsive.
- [ ] **Budget tests**: 1k/10k shape documents at target frame time.

## Phase 7 — agent surface hardening (P0 — differentiator)

- [x] Universal command tool, command catalog, document resource, export, preview.
- [x] Notifications are silent; parse errors are JSON-RPC errors; sizes capped.
- [ ] **JSON control channel over localhost**: line-delimited request/response with a version handshake, bound to loopback only, documented.
- [ ] **CI agent scenario**: a script that creates a document, adds shapes/arrows, groups/aligns, exports SVG+PNG, saves, reopens, and asserts the result — through MCP only.
- [ ] **`tools/list` / registry parity test** in CI.
- [ ] **Screenshots**: `drawcraft_render_preview` is in place; add deterministic fixtures.

## Phase 8 — release engineering (P0)

- [ ] **WASM UI**: wire `eframe::WebRunner` (or a small canvas integration) behind a `wasm` feature; no hand-written JS except the required boot glue.
- [ ] **Bundles**: macOS `.app`, Windows `.exe`/installer, Linux AppImage/tarball; file association for `.drawcraft`.
- [ ] **CI**: format, clippy, tests, layers, commands, wasm core, fuzz corpus, benches.
- [ ] **Licensing/NOTICE**: verify dependency licenses; keep dual MIT/Apache-2.0.
- [ ] **Security**: loopback-only control channel, path validation, no arbitrary command execution from documents.

## Phase 9 — visual polish (P1)

- [ ] Icon set (original vector icons), consistent spacing and typography.
- [ ] Hover/active/drag motion; selection glow; cursor set.
- [ ] Toolbar grouping and overflow; contextual properties.
- [ ] Onboarding canvas with a sample board.

---

## Explicitly out of scope for v1

Real-time collaboration/presence, cloud sync/accounts, a plugin marketplace, iPad-specific Pencil APIs beyond generic touch, and third-party shape libraries. These are 1.x candidates.

## Ground rules (also in AGENTS.md)

- Clean-room: public descriptions + original design only.
- No panics in production paths; `Result` at boundaries; cap untrusted input.
- Everything user-visible is a command shared by UI, CLI, and MCP.
- The engine stays headless and testable; the UI stays thin.
- Unknown data is preserved.
