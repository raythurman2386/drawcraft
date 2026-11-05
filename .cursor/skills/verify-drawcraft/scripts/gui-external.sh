#!/usr/bin/env bash
# GUI external-change prompt proof on private Xvfb. Usage: gui-external.sh PROOF_DIR
set -euo pipefail
P=${1:?usage: gui-external.sh PROOF_DIR}
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$HERE" rev-parse --show-toplevel)
export DRAWCRAFT_BIN=${DRAWCRAFT_BIN:-$ROOT/target/debug/drawcraft}
export XDG_CONFIG_HOME="$P/xdg"
D=${XVFB_DISPLAY:-:99}
mkdir -p "$P/xdg"
cargo build -q -p drawcraft-app

F="$P/external.drawcraft"
"$DRAWCRAFT_BIN" run shape.add --file "$F" --save \
  --params '{"kind":"rectangle","x":0,"y":0,"width":160,"height":110}' >/dev/null

Xvfb "$D" -screen 0 1400x900x24 -nolisten tcp >"$P/xvfb.log" 2>&1 &
echo $! >"$P/xvfb.pid"
cleanup() { kill "$(cat "$P/app.pid" 2>/dev/null)" "$(cat "$P/xvfb.pid")" 2>/dev/null || true; }
trap cleanup EXIT
sleep 1
export DISPLAY="$D"

env -u WAYLAND_DISPLAY DISPLAY="$D" "$DRAWCRAFT_BIN" open "$F" >"$P/app.log" 2>&1 &
echo $! >"$P/app.pid"
sleep 4
"$HERE/xdrive.py" move 700 760; "$HERE/xdrive.py" focus >/dev/null

# External rewrite while the GUI holds the file.
"$DRAWCRAFT_BIN" run shape.add --file "$F" --save \
  --params '{"kind":"ellipse","x":200,"y":0,"width":80,"height":80}' >"$P/ext-rewrite.json"
touch -d 'now + 5 seconds' "$F" 2>/dev/null || touch "$F"

# Wait for poll (>2s) + repaint.
sleep 4
import -window root "$P/external-prompt.png"

# Click Reload (left action in centered modal).
"$HERE/xdrive.py" move 660 470
"$HERE/xdrive.py" click
sleep 1
import -window root "$P/external-after-reload.png"

# Trigger keep-mine path: rewrite again, wait, click Keep mine.
"$DRAWCRAFT_BIN" run shape.add --file "$F" --save \
  --params '{"kind":"triangle","x":40,"y":40,"width":40,"height":40}' >"$P/ext-rewrite2.json"
touch -d 'now + 10 seconds' "$F" 2>/dev/null || touch "$F"
sleep 4
import -window root "$P/external-prompt-2.png"
"$HERE/xdrive.py" move 780 470
"$HERE/xdrive.py" click
sleep 1
import -window root "$P/external-after-keep.png"

mkdir -p "$ROOT/docs/proof/t2099u"
cp -f "$P/external-prompt.png" "$ROOT/docs/proof/t2099u/external-prompt.png" 2>/dev/null || true
cp -f "$P/external-after-reload.png" "$ROOT/docs/proof/t2099u/external-after-reload.png" 2>/dev/null || true
cp -f "$P/external-after-keep.png" "$ROOT/docs/proof/t2099u/external-after-keep.png" 2>/dev/null || true
echo "screenshots in $P (external-prompt.png)"
