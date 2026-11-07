# Pen/touch edit cancellation

Input preference: `view.set_touch_draw` (`touch_draw: boolean`), View → Draw with touch.
With it on, a finger edits with the tool. A second finger switches to navigation and cancels the live edit. Escape also cancels through the same checkpoint boundary. Cancellation restores the exact pre-gesture document and both history stacks, regardless of snapshot deduplication, multiple eraser deletions, or history-limit eviction. Normal committed move/erase and undo/redo behavior is unchanged (eraser retains its existing per-deletion undo granularity).

## Automated event proof

`cargo test -p drawcraft-app --lib input_cancel_tests` feeds synthetic `egui::Event::Touch` and pointer events through the actual canvas. It covers three erased shapes + second-finger cancel with preexisting undo and redo; duplicate-snapshot move cancel; move cancel with redo; bounded-history eviction; normal move and erase commit/undo/redo. Both depths AND every reachable document in the existing undo/redo stacks are compared. These are event integration tests, not physical touchscreen tests. Retain the original `canvas::touch_tests`, `state` touch tracking/latching tests, and editor history tests (`cargo test -p drawcraft-app -p drawcraft-editor -p drawcraft-command`). Full gate: `cargo run -p xtask -- check`.

## Native runtime proof (sanctioned xdrive.py / private Xvfb)

```sh
cargo build -p drawcraft-app
DRAWCRAFT_BIN=$PWD/target/debug/drawcraft python3 .cursor/skills/verify-drawcraft/scripts/gui-input-cancel.py /tmp/fresh-input-cancel-proof
```

Requires Xvfb, ImageMagick import, libX11, libXtst, Python 3. Uses private `:194` (override `DRAWCRAFT_DISPLAY`), disposable documents/XDG config; terminates only its own processes. Creates four shapes through native pointer input; undo leaves 3 undo/1 redo; erases all three mid-gesture; Escape restores the document; actual Ctrl+Z/Y and saved JSON verify the entire existing history trajectory. Also exercises committed eraser, move cancellation, and normal move with undo/redo. Full real before/live/after framebuffer captures and JSON are retained. No diagnostic mutation API or production input injection is used.

## Limitations (do not overclaim)

Xvfb/XTest supplies core mouse/keyboard, not physical pen or multi-touch; the native proof uses Escape to reach the shared rollback, whereas second-finger takeover is verified only with synthetic egui events. No physical touchscreen, palm rejection, pressure, native-Wayland pen, or hardware timing is proven. A resting finger after a pen stroke can still latch navigation until all contacts lift. Cancellation does not implement hardware palm rejection. No Alt-copy finding is asserted or expanded here.
