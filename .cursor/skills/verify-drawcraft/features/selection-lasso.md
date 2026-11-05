# Lasso selection

## Sub-features
- `edit.select_lasso {points:[[x,y],...], additive?}` — selects every unlocked shape whose world
  bounds are **fully contained** in the polygon; returns `{"count": N}` (selection size after).
- `Tool::Lasso` (rail button "Lasso", shortcut `Q`): freehand drag draws a purple path; release
  calls the same `lasso_select`. Shift at drag start = additive (group-level toggle).
- Hits expand to whole groups (non-additive) like click/marquee.

## How to get to it (user POV)
Press `Q` (or click **Lasso** in the left rail; tool hint chip shows "Lasso"), drag a loop around
shapes, release. Inspector switches to the selected shape(s) ("Rectangle +1").

## Driving it with CLI harness
```sh
.cursor/skills/verify-drawcraft/scripts/lasso-cli.sh $P     # exit 0 + $P/RESULT.txt = PASS
```
Manual: `drawcraft run edit.select_lasso --file F --params '{"points":[[-20,-20],[400,-20],[470,55],[400,130],[-20,130]]}'`
on the seed board → `{"count": 2}` (A + B; C partially inside is excluded). In MCP follow with
`doc.info` → `"selection":2`.
GUI: `scripts/gui-lasso.sh $P` → `lasso-drag.png` (path visible, encloses A + B and cuts into
C) + `lasso-selected.png` (A+B selection box, inspector "Rectangle +1"; C partially inside is not
selected).

## Gotchas
- Fewer than 3 points: clears the selection (non-additive) and returns the current count; not an error.
- Containment uses axis-aligned world bounds, so a rotated shape needs its whole AABB inside.
- Containment is true containment, not "4 corners inside": a concave lasso whose notch bites into
  a shape's bounds (all corners still inside) does **not** select it, because a lasso edge crosses
  a bounds edge. Touching a bounds edge also counts as not contained.
- Locked shapes are never lasso-selected.
- Selection is not saved to the file: `run edit.select_lasso` then a second `run` sees selection 0. Use MCP.
- GUI: no WM on Xvfb → call `xdrive.py focus` before `key q` or the shortcut is ignored.
