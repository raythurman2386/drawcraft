#!/usr/bin/env bash
# GUI lasso gesture proof on a private Xvfb display. Usage: gui-lasso.sh PROOF_DIR
# The path encloses A + B and cuts into C (screen x 802-992 at 1400x900), so C is
# only partially inside and must stay unselected.
set -euo pipefail
P=${1:?usage: gui-lasso.sh PROOF_DIR}
HERE=$(cd "$(dirname "$0")" && pwd)
export DRAWCRAFT_BIN=${DRAWCRAFT_BIN:-$(git -C "$HERE" rev-parse --show-toplevel)/target/debug/drawcraft}
export XDG_CONFIG_HOME="$P/xdg"
D=${XVFB_DISPLAY:-:99}
mkdir -p "$P"
F="$P/gui.drawcraft"
"$HERE/seed-board.sh" "$F" > /dev/null
Xvfb "$D" -screen 0 1400x900x24 -nolisten tcp > "$P/xvfb.log" 2>&1 & echo $! > "$P/xvfb.pid"
cleanup() { kill "$(cat "$P/app.pid" 2>/dev/null)" "$(cat "$P/xvfb.pid")" 2>/dev/null || true; }
trap cleanup EXIT
sleep 1
env -u WAYLAND_DISPLAY DISPLAY="$D" "$DRAWCRAFT_BIN" open "$F" > "$P/app.log" 2>&1 & echo $! > "$P/app.pid"
sleep 4
export DISPLAY="$D"
import -window root "$P/gui-opened.png"
"$HERE/xdrive.py" move 700 760
"$HERE/xdrive.py" focus
"$HERE/xdrive.py" key q
sleep 1
"$HERE/xdrive.py" dragpath --hold 250,340 420,320 600,325 770,330 880,340 900,450 880,560 770,575 600,590 420,585 245,565 230,450 250,350
sleep 1; import -window root "$P/lasso-drag.png"
"$HERE/xdrive.py" up
sleep 1; import -window root "$P/lasso-selected.png"
echo "screenshots: $P/gui-opened.png $P/lasso-drag.png $P/lasso-selected.png"
