#!/usr/bin/env bash
# CLI + MCP proof for edit.select_lasso. Usage: lasso-cli.sh PROOF_DIR
set -euo pipefail
P=${1:?usage: lasso-cli.sh PROOF_DIR}
HERE=$(cd "$(dirname "$0")" && pwd)
export DRAWCRAFT_BIN=${DRAWCRAFT_BIN:-$(git -C "$HERE" rev-parse --show-toplevel)/target/debug/drawcraft}
export XDG_CONFIG_HOME="$P/xdg"
mkdir -p "$P"
F="$P/board.drawcraft"
"$HERE/seed-board.sh" "$F" > "$P/seed.json"
# Lasso around A (0..160) and B (220..380); C (440..600) is only partially inside.
PTS='[[-20,-20],[400,-20],[470,55],[400,130],[-20,130]]'
"$DRAWCRAFT_BIN" run edit.select_lasso --file "$F" --params "{\"points\":$PTS}" | tee "$P/select_lasso.json"
grep -q '"count": 2' "$P/select_lasso.json"
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
 "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"drawcraft_command\",\"arguments\":{\"id\":\"edit.select_lasso\",\"params\":{\"points\":$PTS}}}}" \
 '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"doc.info","params":{}}}}' \
 | timeout 20 "$DRAWCRAFT_BIN" mcp --file "$F" | tee "$P/mcp-session.jsonl"
grep -q '\\"selection\\":2' "$P/mcp-session.jsonl"
grep -q '\\"shapes\\":3' "$P/mcp-session.jsonl"
echo "PASS: edit.select_lasso selected 2 of 3 (C partially inside, excluded)" | tee "$P/RESULT.txt"
