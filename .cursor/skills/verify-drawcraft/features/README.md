# DrawCraft feature map (verify-drawcraft)

One file per user-facing feature: what it is, how a user reaches it, how to drive it with the
CLI/MCP harness (primary) or the Xvfb GUI helper (gestures), and gotchas. Extend this map when a
PR adds or changes a command; command ids come from `drawcraft catalog`.

| Feature | File | Command ids | GUI entry |
|---|---|---|---|
| Lasso selection | [selection-lasso.md](selection-lasso.md) | `edit.select_lasso` | Tool rail **Lasso**, key `Q` |
| Shift = group-level toggle | [selection-shift-groups.md](selection-shift-groups.md) | `edit.select` (`additive`), `edit.select_lasso` (`additive`), `edit.group` | Shift+click / Shift+marquee / Shift+lasso |
| Select first / last | [selection-first-last.md](selection-first-last.md) | `edit.select_first`, `edit.select_last` | Command palette (Ctrl/Cmd+K) |
| Tool rail geometric shapes | [tool-rail-shapes.md](tool-rail-shapes.md) | `shape.add` (via tool) | Rail **SHAPES**, keys `R`/`O`/`3`… |
| Transform modifiers + readout | [transform-modifiers.md](transform-modifiers.md) | `edit.resize`, `edit.rotate` (headless, unchanged) | Select handles: Shift aspect / Alt centre / Shift 15° rotate |

Proven so far: `edit.select_lasso` CLI+MCP (`scripts/lasso-cli.sh`) and the GUI lasso gesture
(`scripts/gui-lasso.sh`) on 2026-11-04. Others are mapped but not yet driven end to end.
