#!/usr/bin/env bash
# GUI transform-modifier proof on a private Xvfb display. Usage: gui-transform.sh PROOF_DIR
# Each case: fresh seed board, click rect A (screen ~283-473 x 387-517), drag a handle
# with optional modifier held, screenshot mid-drag (readout) and after release.
# Handle hit-tests use the pointer when egui reports drag start (~6 px past press),
# so each path opens with a short slow segment that stays inside the 7 px handle radius.
set -euo pipefail
P=${1:?usage: gui-transform.sh PROOF_DIR}
HERE=$(cd "$(dirname "$0")" && pwd)
export DRAWCRAFT_BIN=${DRAWCRAFT_BIN:-$(git -C "$HERE" rev-parse --show-toplevel)/target/debug/drawcraft}
export XDG_CONFIG_HOME="$P/xdg"
D=${XVFB_DISPLAY:-:99}
mkdir -p "$P"
Xvfb "$D" -screen 0 1400x900x24 -nolisten tcp > "$P/xvfb.log" 2>&1 & echo $! > "$P/xvfb.pid"
cleanup() { kill "$(cat "$P/app.pid" 2>/dev/null)" "$(cat "$P/xvfb.pid")" 2>/dev/null || true; }
trap cleanup EXIT
sleep 1
export DISPLAY="$D"
run_case() { # name modifier(or -) path...
  local name=$1 mod=$2; shift 2
  local F="$P/$name.drawcraft"
  "$HERE/seed-board.sh" "$F" > /dev/null
  env -u WAYLAND_DISPLAY DISPLAY="$D" "$DRAWCRAFT_BIN" open "$F" > "$P/app-$name.log" 2>&1 & echo $! > "$P/app.pid"
  sleep 4
  "$HERE/xdrive.py" move 700 760; "$HERE/xdrive.py" focus > /dev/null
  "$HERE/xdrive.py" key v
  "$HERE/xdrive.py" drag 378,452 378,452; sleep 0.5
  [ "$mod" != - ] && "$HERE/xdrive.py" keydown "$mod"
  "$HERE/xdrive.py" dragpath --hold "$@"
  sleep 0.8; import -window root "$P/$name-drag.png"
  "$HERE/xdrive.py" up
  [ "$mod" != - ] && "$HERE/xdrive.py" keyup "$mod"
  sleep 0.8; import -window root "$P/$name-after.png"
  kill "$(cat "$P/app.pid")"; sleep 0.5
}
run_case corner-free  -       473,517 479,517 600,560
run_case corner-shift Shift_L 473,517 479,517 600,560
run_case edge-alt     Alt_L   473,452 479,452 560,452
run_case rotate-free  -       378,365 470,400
run_case rotate-shift Shift_L 378,365 470,400
echo "screenshots in $P"
