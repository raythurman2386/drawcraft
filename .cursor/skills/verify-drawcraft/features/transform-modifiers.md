# Transform modifiers + live readout

## Sub-features
- Eight resize handles + rotation knob on the selection (Select tool). Without modifiers they behave as before.
- **Shift** while dragging a resize handle: aspect ratio locked. Corners follow whichever axis moved
  further; edges scale the other axis by the same factor, centred on the perpendicular centre line.
- **Alt** while resizing: scale about the selection centre (opposite edge moves symmetrically).
  Shift+Alt = uniform scale about the centre.
- **Shift** while rotating: angle snaps to 15° increments (relative to drag start).
- Live numeric readout above the selection while transforming: `W × H  θ°`, mirrored in the
  status bar as `Resize …` / `Rotate …`.
- Modifiers are read every frame (press/release mid-drag works). One undo snapshot per drag.

Pure math lives in `crates/app/src/transform.rs` (`resize_scale`, `rotate_angle`, `readout`) with
unit tests. Headless `edit.resize` / `edit.rotate` are unchanged (no new params).

## How to get to it (user POV)
Select a shape (`V`, click). Drag a corner handle; hold Shift to keep proportions, Alt to grow from the
centre. Drag the knob above the selection to rotate; hold Shift for 15° steps. Watch the label above
the selection and the status bar.

## Driving it
Unit: `cargo test -p drawcraft-app transform`.

GUI (Xvfb :99, `WAYLAND_DISPLAY` unset, `scripts/xdrive.py` with `dragpath ... --hold`, `keydown`/
`keyup` for Shift_L / Alt_L): seed board, click rect A, drag SE corner with/without Shift, drag E edge
with Alt, drag the rotation knob with/without Shift. Screenshot mid-drag (`--hold`) to capture the readout.

Headless round-trip (unchanged semantics):
```sh
$DRAWCRAFT_BIN run edit.resize --file $P/board.drawcraft --params '{...}'
$DRAWCRAFT_BIN run edit.rotate --file $P/board.drawcraft --params '{...}'
```

## Gotchas
- Rotation snap is relative to the angle at drag start, not absolute shape rotation.
- Readout size is the axis-aligned selection bounds (so it grows while rotating).
- Out of scope: vertex editing, visible snap guides, undo coalesce, `edit.duplicate_drag`.
