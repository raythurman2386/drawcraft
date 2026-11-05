# Changelog

All notable changes to DrawCraft are documented here.

## Unreleased

## [0.3.0] — 2026-11-05

Selection improvements (#3), placement and inline text editing (#4/#5), shapes rail (#6), transform modifiers (#7), and document lifecycle, autosave, recovery, external-change handling, and fuzz coverage (#8).

### Added

- Document lifecycle (Phase 4): debounced autosave with background worker, crash recovery (`session.recover_list` / `session.recover`), external-change detection (`session.external_check` / `session.reload` / `session.keep_mine`), atomic `format::write_atomic`, Open Recent missing-file handling + clear/remove, recovery and external-change prompts in the desktop UI.
- `format::from_json` fuzz target under `fuzz/` with seed corpus, stable corpus replay test, `xtask fuzz`, and CI nightly 60 s fuzz job.
- Shapes in the left tool rail with shortcuts (PR #6).
- Transform modifiers + live readout: Shift aspect lock, Alt centre scale, Shift 15° rotate snap (PR #7 / t2041u).
- Lasso selection tool (`Tool::Lasso`, shortcut Q) and `edit.select_lasso` command (fully-contained bounds: true containment, so a concave lasso that notches into a shape does not select it).
- `edit.select_first` / `edit.select_last` select bottommost / topmost unlocked shapes by z-order.
- One group-level shift rule for click, marquee, and lasso (groups are atomic toggle units).
- Marquee now expands to the full group, matching click selection.
- Project-local verification skill `.cursor/skills/verify-drawcraft/` (CLI/MCP + Xvfb GUI drive recipes, helper scripts, selection feature map) and GUI lasso proof at `docs/proof/lasso-gesture.png`.
- Inline canvas text editing for Text, Sticky, and Note shapes: double-click or use "Edit text" to edit directly on the canvas instead of a floating popup. Enter (or clicking away) commits, Shift+Enter inserts a newline in Sticky and Note, Escape cancels; unchanged text adds no undo entry.

### Fixed

- `edit.select` additive path now filters nonexistent ids and expands groups like the replace path.
- Line, polyline, and arrow placement no longer draws a box while dragging; the canvas preview shows the actual line shaft, and arrow previews draw a head at the pointer.
- Arrow placement now snaps each end to nearby shapes' anchors so the bound arrow matches the preview.
- Polyline is now a real multi-segment polyline instead of duplicating Line.
- Note shapes render with a folded corner instead of a plain rectangle.
- Sticky notes create with "Sticky" text instead of "Note".
- Axis-aligned Arrow/Line drags now commit (create gate uses drag length, not bounding-box width AND height); arrow preview and commit share one center-to-center endpoint resolver and skip linear snap targets.

## [0.2.0] — 2026-11-04

A full correctness and maintainability pass over the MVP. The document model and command surface are now treated as the foundation for v1 rather than a prototype.

### Added

- `drawcraft-session` crate: file lifecycle, recent documents, export commands, and a single `execute` entry point used by the UI, CLI, and MCP.
- Command registry with 56 commands, typed parameters, scopes, categories, and a coverage test.
- Proper MCP stdio server: notifications, parse errors, tools, resources, command catalog, base64 PNG previews, open/save/export tools.
- Faithful SVG export for paths, dashes, text, arrowheads, patterns, clips, and images.
- PNG export through `resvg` with system fonts, scale limits, and transparent background support.
- Ear-clipping triangulation so concave shapes fill correctly on the GPU.
- All 20 geometric shape paths, arrowhead geometry, hand-drawn wobble, and pattern hatching in the renderer.
- Multi-page UI with page list, rename, add/duplicate/delete.
- `edit.select` so agents and scripts can target shape ids without a pointer.
- Selection handles for resize and rotation, marquee selection, Alt-drag copy, snapping, grid, minimap, focus mode, and dark mode.
- Style/properties panel, arrow routing/head controls, text editor window, command palette, and keyboard shortcuts.
- Document validation: reference integrity, caps, finite geometry, style bounds.
- Unknown-field preservation at document, page, shape, and style levels.
- `ARCHITECTURE.md`, expanded `FORMAT.md`, rewritten `AGENTS.md`, `README.md`, and `ROADMAP.md`.
- GrokDay and GrokNight themes with the exact palette roles, a refined top bar, icon-based tool rail, inspector cards, dot grid, empty state, and optional minimap.
- `--dark` launch flag for GrokNight.
- `xtask` layering checks, command coverage, WASM core build, and bundle task.
- CI workflow.

### Fixed

- Replaced the prototype's placeholder unknown-command error with a typed error.
- Camera zoom now anchors the world point under the pointer.
- Corrected page-id and shape-id generation/deduplication.
- Arrow bindings and frame parents are remapped when duplicating pages or pasting.
- MCP no longer terminates on malformed input and no longer replies to notifications.
- Duplicate page now remaps bindings instead of producing invalid documents.
- The UI no longer draws every shape as a rectangle; it renders the shared scene graph.
- CLI `cargo run` works because the app is a default workspace member.
- Canvas painting now respects the central panel origin (scene no longer draws under the side panels).
- Opening a document defers zoom-to-fit until the real viewport is known.
- PNG text export uses the portable `sans-serif` family instead of the ambiguous `sans` family.
- Removed the prototype's placeholder error text and minified one-line modules.
- Fixed the left tool rail growing while the pointer moves: the rail and its rows now use fixed widths instead of a feedback loop through `ui.available_width`.
- Removed the always-visible page-name field in the inspector (now a Rename action), clipped the minimap viewport, and turned the minimap off by default.
- Selection dimensions are only shown while dragging.

## [0.1.0] — 2026-11-04

Initial prototype: workspace, minimal editor, commands, MCP, JSON format, basic egui canvas.
