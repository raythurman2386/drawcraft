# Select first / last (z-order)

## Sub-features
- `edit.select_first`: bottommost unlocked shape by `(z, id)`; expands to its group.
- `edit.select_last`: topmost unlocked shape by `(z, id)`; expands to its group.
- Both return `{"count": N}`; empty page → selection cleared, count 0.

## How to get to it (user POV)
Command palette (Ctrl/Cmd+K) → filter "Select first" / "Select last". No menu item or default shortcut yet.

## Driving it with CLI harness
On the seed board: `drawcraft run edit.select_first --file F` → `{"count":1}`. To see *which*
shape, use MCP: `edit.select_first` → `edit.delete` → `file.save` and confirm shape-1 is gone
(or `edit.zorder {"action":"front"}` on shape-1 first, then `edit.select_last` selects it).

## Gotchas
- Locked shapes are skipped, so "first" may not be the literal bottom shape.
- Ties on `z` break by id string order (`shape-10` < `shape-2`).
- Palette path is not yet screenshot-proven; confirm the palette lists these ids before relying on it.
