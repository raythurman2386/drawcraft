#!/usr/bin/env bash
set -euo pipefail
P=$1
BIN=${2:-${DRAWCRAFT_BIN:-$PWD/target/debug/drawcraft}}
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=${DRAWCRAFT_ROOT:-$(cd "$HERE/../../.." && pwd)}
mkdir -p "$P"
export DRAWCRAFT_BIN="$BIN" DRAWCRAFT_ROOT="$ROOT" DRAWCRAFT_PROOF="$P" XDG_RUNTIME_DIR="$P/xdg-runtime"
mkdir -p "$XDG_RUNTIME_DIR";chmod 700 "$XDG_RUNTIME_DIR"
export DISPLAY=${DRAWCRAFT_DISPLAY:-:198}
if [[ -S "/tmp/.X11-unix/X${DISPLAY#:}" ]]; then
    printf 'Refusing to use existing display $DISPLAY\n' >&2
    exit 2
fi
Xvfb "$DISPLAY" -screen 0 1600x1000x24 -nolisten tcp >"$P/xvfb.log" 2>&1 &
XVFB=$!
cleanup() {
    if [[ -f "$P/app.pid" ]]; then kill "$(cat "$P/app.pid")" 2>/dev/null || true; fi
    if [[ -f "$P/case.pid" ]]; then kill "$(cat "$P/case.pid")" 2>/dev/null || true; fi
    kill "$XVFB" 2>/dev/null || true
}
trap cleanup EXIT
sleep 0.5
printf '%s\n' "$XVFB" > "$P/xvfb.pid"
python3 -u "$HERE/gui-sidebars.py"
