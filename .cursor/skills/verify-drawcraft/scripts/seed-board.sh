#!/usr/bin/env bash
# Seed a fresh 3-shape board for selection proofs. Usage: seed-board.sh FILE
set -euo pipefail
F=${1:?usage: seed-board.sh FILE}
BIN=${DRAWCRAFT_BIN:-$(git rev-parse --show-toplevel)/target/debug/drawcraft}
rm -f "$F"
"$BIN" run shape.add --file "$F" --save --params '{"kind":"rectangle","x":0,"y":0,"width":160,"height":110}'
"$BIN" run shape.add --file "$F" --save --params '{"kind":"ellipse","x":220,"y":0,"width":160,"height":110}'
"$BIN" run shape.add --file "$F" --save --params '{"kind":"rectangle","x":440,"y":0,"width":160,"height":110}'
