# Tool rail geometric shapes

## Sub-features
- Left tool rail **SHAPES** section: compact 2-column buttons for every `ShapeKind::GEOMETRIC`
  kind except `Note` (Note stays under **CONTENT**). Declared GEOMETRIC order (Rectangle first),
  not alphabetical.
- Selecting a shape tool (click or shortcut) uses the existing `Tool` / `set_tool` path; drawing
  still goes through the canvas gesture → shape add command flow.
- Top bar menus are File / Edit / View / Help only (no Shapes… menu).

## How to get to it (user POV)
Open DrawCraft. In the left rail, scroll past CONNECT to **SHAPES**. Click **Rectangle** (or press
`R`). Status bar shows "Rectangle"; drag on the canvas to place one. Other shortcuts: Ellipse `O`,
Triangle `3`, Diamond `4`, … (see `Tool::shortcut`).

## Driving it with CLI harness
Shape creation is still command-first:
```sh
$DRAWCRAFT_BIN run shape.add --file $P/board.drawcraft --save \
  --params '{"kind":"rectangle","x":20,"y":20,"width":120,"height":80}'
```
GUI chrome proof (Xvfb :99, `WAYLAND_DISPLAY` unset, `scripts/xdrive.py`):
1. Launch `drawcraft` on a fresh/seed board.
2. Screenshot: top bar has no "Shapes…"; rail shows **SHAPES**.
3. `xdrive.py focus` then `key r` — Rectangle tool selected (accent fill on the compact button).
4. Drag on canvas to draw one rectangle; note status / shape count.

## Gotchas
- `Tool::Note` is intentionally omitted from SHAPES; it already appears under CONTENT.
- `Tool::Polyline` is still absent from chrome (out of scope for the shapes-rail move).
- Soft park: Sticky double-click-when-already-selected behaviour is unchanged by this chrome move.
- No WM on Xvfb → call `xdrive.py focus` before shortcut keys.
