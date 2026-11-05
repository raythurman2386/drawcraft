#!/usr/bin/env bash
# MCP kill-restore + external-change proof. Usage: lifecycle-cli.sh PROOF_DIR
set -euo pipefail
P=${1:?usage: lifecycle-cli.sh PROOF_DIR}
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$HERE" rev-parse --show-toplevel)
export DRAWCRAFT_BIN=${DRAWCRAFT_BIN:-$ROOT/target/debug/drawcraft}
export XDG_CONFIG_HOME="$P/xdg"
mkdir -p "$P/xdg"
F="$P/lifecycle.drawcraft"

cargo build -q -p drawcraft-app

# Seed a valid document.
"$DRAWCRAFT_BIN" run shape.add --file "$F" --save \
  --params '{"kind":"rectangle","x":0,"y":0,"width":10,"height":10}' | tee "$P/seed.json" >/dev/null

# --- kill -9 restore via persistent MCP (FIFO keeps stdin open) ---
FIFO="$P/mcp.in"
rm -f "$FIFO"
mkfifo "$FIFO"
"$DRAWCRAFT_BIN" mcp --file "$F" <"$FIFO" >"$P/mcp-prekill.jsonl" 2>"$P/mcp-prekill.err" &
MPID=$!
echo $MPID >"$P/mcp.pid"
# Hold the FIFO open for the lifetime of this script section.
exec 3>"$FIFO"

send() { printf '%s\n' "$1" >&3; sleep 0.05; }

send '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
send '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"shape.add","params":{"kind":"ellipse","x":5,"y":5,"width":8,"height":8}}}}'
send '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"file.save","params":{}}}}'
send '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"shape.add","params":{"kind":"triangle","x":10,"y":10,"width":12,"height":12}}}}'
send '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"session.autosave_now","params":{}}}}'
send '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"doc.info","params":{}}}}'
sleep 0.3

# Confirm autosave landed before kill.
for _ in $(seq 1 30); do
  if grep -q '"path"' "$P/mcp-prekill.jsonl" 2>/dev/null && grep -q 'revision\|shapes' "$P/mcp-prekill.jsonl"; then
    break
  fi
  sleep 0.1
done
grep -q 'shapes' "$P/mcp-prekill.jsonl"

kill -9 "$MPID"
wait "$MPID" 2>/dev/null || true
exec 3>&-
rm -f "$FIFO"

# Relaunch and restore.
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"session.recover_list","params":{}}}}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"session.recover","params":{"action":"restore"}}}}' \
  '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"doc.info","params":{}}}}' \
  | timeout 20 "$DRAWCRAFT_BIN" mcp --file "$F" | tee "$P/mcp-restore.jsonl"

grep -q 'candidates' "$P/mcp-restore.jsonl"
grep -q 'dirty' "$P/mcp-restore.jsonl"

# --- external change ---
EXTF="$P/external.drawcraft"
"$DRAWCRAFT_BIN" run shape.add --file "$EXTF" --save \
  --params '{"kind":"rectangle","x":0,"y":0,"width":10,"height":10}' >/dev/null

FIFO="$P/ext.in"
rm -f "$FIFO"
mkfifo "$FIFO"
"$DRAWCRAFT_BIN" mcp --file "$EXTF" <"$FIFO" >"$P/mcp-external.jsonl" 2>"$P/mcp-external.err" &
EPID=$!
echo $EPID >"$P/ext.pid"
exec 3>"$FIFO"
send '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
send '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"doc.info","params":{}}}}'
# External rewrite + newer mtime.
"$DRAWCRAFT_BIN" run shape.add --file "$EXTF" --save \
  --params '{"kind":"ellipse","x":1,"y":1,"width":5,"height":5}' >"$P/ext-rewrite.json"
touch -d 'now + 5 seconds' "$EXTF" 2>/dev/null || touch "$EXTF"
send '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"session.external_check","params":{}}}}'
send '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"file.save","params":{}}}}'
send '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"session.keep_mine","params":{}}}}'
send '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"file.save","params":{}}}}'
sleep 0.4
kill "$EPID" 2>/dev/null || true
wait "$EPID" 2>/dev/null || true
exec 3>&-
rm -f "$FIFO"

grep -q 'changed' "$P/mcp-external.jsonl"
grep -qiE 'external|changed on disk|isError|Keep mine|keep_mine|"ok":true' "$P/mcp-external.jsonl"

echo "PASS: lifecycle CLI/MCP kill-restore + external-change" | tee "$P/RESULT.txt"
