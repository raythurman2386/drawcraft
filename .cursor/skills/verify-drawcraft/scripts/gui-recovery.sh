#!/usr/bin/env bash
# GUI recovery-prompt proof on private Xvfb. Usage: gui-recovery.sh PROOF_DIR
# Stages a real autosave via MCP (same XDG) so the relaunch modal is deterministic.
set -euo pipefail
P=${1:?usage: gui-recovery.sh PROOF_DIR}
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$HERE" rev-parse --show-toplevel)
export DRAWCRAFT_BIN=${DRAWCRAFT_BIN:-$ROOT/target/debug/drawcraft}
export XDG_CONFIG_HOME="$P/xdg"
D=${XVFB_DISPLAY:-:99}
mkdir -p "$P/xdg"
cargo build -q -p drawcraft-app

F="$P/recovery.drawcraft"
"$DRAWCRAFT_BIN" run shape.add --file "$F" --save \
  --params '{"kind":"rectangle","x":0,"y":0,"width":160,"height":110}' >/dev/null

# Dirty autosave via MCP, then SIGKILL so the candidate remains.
FIFO="$P/mcp.in"
rm -f "$FIFO"
mkfifo "$FIFO"
"$DRAWCRAFT_BIN" mcp --file "$F" <"$FIFO" >"$P/mcp-prekill.jsonl" 2>"$P/mcp-prekill.err" &
MPID=$!
echo $MPID >"$P/mcp.pid"
exec 3>"$FIFO"
send() { printf '%s\n' "$1" >&3; sleep 0.05; }
send '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
send '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"shape.add","params":{"kind":"ellipse","x":40,"y":40,"width":90,"height":70}}}}'
send '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"session.autosave_now","params":{}}}}'
send '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"session.recover_list","params":{}}}}'
sleep 0.3
grep -q 'candidates' "$P/mcp-prekill.jsonl"
kill -9 "$MPID"
wait "$MPID" 2>/dev/null || true
exec 3>&-
rm -f "$FIFO"

Xvfb "$D" -screen 0 1400x900x24 -nolisten tcp >"$P/xvfb.log" 2>&1 &
echo $! >"$P/xvfb.pid"
cleanup() { kill "$(cat "$P/app.pid" 2>/dev/null)" "$(cat "$P/xvfb.pid")" 2>/dev/null || true; }
trap cleanup EXIT
sleep 1
export DISPLAY="$D"

env -u WAYLAND_DISPLAY DISPLAY="$D" XDG_CONFIG_HOME="$P/xdg" \
  "$DRAWCRAFT_BIN" open "$F" >"$P/app2.log" 2>&1 &
echo $! >"$P/app.pid"
sleep 4
"$HERE/xdrive.py" move 700 450; "$HERE/xdrive.py" focus >/dev/null
import -window root "$P/recovery-prompt.png"
# Click Restore (left action in centered modal).
"$HERE/xdrive.py" move 700 470
"$HERE/xdrive.py" click
sleep 1
import -window root "$P/recovery-after.png"
mkdir -p "$ROOT/docs/proof/t2099u"
cp -f "$P/recovery-prompt.png" "$ROOT/docs/proof/t2099u/recovery-prompt.png" 2>/dev/null || true
cp -f "$P/recovery-after.png" "$ROOT/docs/proof/t2099u/recovery-after.png" 2>/dev/null || true
echo "screenshots in $P (recovery-prompt.png)"
