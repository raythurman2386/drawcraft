# Shift = group-level toggle (click / marquee / lasso)

## Sub-features
- One rule for all three gestures: with Shift, each **atomic unit** (a whole group, or an ungrouped
  shape) hit by the gesture toggles — if every member is selected it is removed, else all members are added.
- Non-additive marquee now expands hits to the full group (same as click).
- `edit.select {ids, additive:true}` filters nonexistent ids and expands groups.

## How to get to it (user POV)
Select tool (`V`): Shift+click a grouped shape, or Shift+drag a marquee; Lasso tool (`Q`):
hold Shift when starting the drag. Group via Edit → Group (Ctrl/Cmd+G).

## Driving it with CLI harness
Single MCP session on the seed board (`scripts/seed-board.sh F`; ids shape-1..3):
1. `edit.select {"ids":["shape-1","shape-2"]}` → `edit.group`
2. `edit.select_none`, then `edit.select {"id":"shape-1"}` → `doc.info` selection 2 (group expanded)
3. `edit.select {"id":"shape-3","additive":true}` → selection 3
4. `edit.select_lasso {"points":<around A only>,"additive":true}` → group toggles off → selection 1
Assert each `doc.info` `selection` value.

## Gotchas
- A marquee/lasso that only touches part of a group still toggles the whole group with Shift.
- Additive with an id that does not exist is silently dropped (no error).
- Not yet driven end to end by a script; add `scripts/shift-groups.sh` when you prove it.
