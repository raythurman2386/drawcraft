---
name: verify-drawcraft
description: Drive and prove DrawCraft behaviour (egui desktop app + headless `drawcraft run` CLI / MCP command runner). Use when you need evidence that a DrawCraft command or canvas gesture (selection, lasso, shapes, export) actually works, before claiming a feature is done or attaching proof to a PR.
---

# verify-drawcraft

DrawCraft has one rule (AGENTS.md): **everything user-visible is a command**. So the primary
agent drive path is the command runner (`drawcraft run <id>` for one command, `drawcraft mcp`
for a multi-command session). The egui GUI is the secondary surface; drive it only when the
proof is about a gesture (drag, tool shortcut, on-canvas feedback), on a private Xvfb display.

All paths below are relative to the repo root. `$P` is a proof dir you choose
(e.g. `/tmp/dc-proof-<task>`); evidence lives there and survives cleanup.

## Launch

```sh
cargo build -p drawcraft-app          # binary: target/debug/drawcraft (same as `cargo run --`)
export DRAWCRAFT_BIN=$PWD/target/debug/drawcraft
export XDG_CONFIG_HOME=$P/xdg         # isolate recent.json; CLI/GUI write it on every open/save
```

- CLI: no server. Each `run` is its own process with a fresh `Session`; ready = exit 0 + JSON on stdout.
  `--file X` opens X (creates it if missing); add `--save` to persist the mutation.
- GUI (gesture proofs only): `scripts/gui-lasso.sh $P` starts `Xvfb :99`, launches
  `drawcraft open <file>` with `WAYLAND_DISPLAY` unset (forces winit X11), waits 4 s.
  Ready = status bar reads `Opened <file>` in a screenshot. Never drive the user's real
  Wayland/Hyprland session.

## Doctor

Read-only "is this worth driving?":

```sh
git rev-parse --short HEAD && git status -sb | head -1
cargo build -p drawcraft-app 2>&1 | tail -1     # must say Finished
$DRAWCRAFT_BIN --version && $DRAWCRAFT_BIN catalog | grep -c .   # command count (>0)
```

Full gate (slow, required before push): `cargo run -p xtask -- check`
(fmt, clippy -D warnings, tests, layers, command coverage).

## Drive

Primary: CLI / MCP with real command ids from `drawcraft catalog`.

```sh
$DRAWCRAFT_BIN run shape.add --file $P/board.drawcraft --save \
  --params '{"kind":"rectangle","x":20,"y":20,"width":30,"height":30}'
$DRAWCRAFT_BIN run edit.select_lasso --file $P/board.drawcraft \
  --params '{"points":[[0,0],[100,0],[100,70],[0,70]]}'      # -> {"count": N}
```

Selection is **not persisted** in the file, so a chain that depends on selection
(select -> doc.info / edit.delete / edit.move) must run in ONE process via MCP:

```sh
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"edit.select_lasso","params":{"points":[[0,0],[100,0],[100,70],[0,70]]}}}}' \
 '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"drawcraft_command","arguments":{"id":"doc.info","params":{}}}}' \
 | $DRAWCRAFT_BIN mcp --file $P/board.drawcraft
```

Secondary: GUI via `scripts/xdrive.py` (ctypes XTest; no xdotool needed) on `DISPLAY=:99`:
`move X Y`, `focus` (XSetInputFocus on window under pointer — required, there is no WM),
`key q`, `dragpath x,y x,y ... [--hold]`, `up`. Screenshot with `import -window root out.png`.
Default canvas at 1400x900 with the seed board puts shapes A/B/C at screen x≈283–473 / 543–733 /
802–992, y≈387–517 (zoom 118%); re-screenshot first if the layout changed.

## Evidence

- CLI/MCP: save stdout JSON per step under `$P/` (`*.json`, `*.jsonl`), plus the seed `.drawcraft`.
- GUI: screenshots of the action in progress AND the resulting state (e.g. lasso path mid-drag, then
  inspector showing the selection). Keep full-size PNGs; make a downscaled composite for PRs.
- Standards: exercise real command ids / real input events, not internal setters; check the side
  effect (`doc.info` selection count, saved file shapes) not just a printed `ok`.

## Cleanup

Kill only PIDs you started (the scripts write `$P/*.pid`): `kill $(cat $P/app.pid) $(cat $P/xvfb.pid)`.
Never `pkill drawcraft` / `pkill Xvfb` — the user may have their own running. Remove `$P/xdg` if you
like; never delete `$P/*.json` or `$P/*.png` evidence.

## Helpers

| Script | Use |
|---|---|
| `scripts/seed-board.sh FILE` | Fresh board: A rect (0,0 160x110), B ellipse (220,0), C rect (440,0). |
| `scripts/lasso-cli.sh P` | Seeds `P/board.drawcraft`, runs `edit.select_lasso` via `run` and via MCP + `doc.info`, asserts count 2 / selection 2. Exit 0 = pass. |
| `scripts/gui-lasso.sh P` | Xvfb :99 + GUI, Q, freehand lasso around A+B, screenshots `P/lasso-drag.png`, `P/lasso-selected.png`, then kills its own PIDs. |
| `scripts/gui-transform.sh P` | Xvfb :99 + GUI, five transform cases (corner free/Shift, edge Alt, rotate free/Shift), mid-drag + after screenshots. |
| `scripts/xdrive.py` | XTest mouse/keyboard driver (`keydown`/`keyup` for held modifiers). |

All need `DRAWCRAFT_BIN` (default `target/debug/drawcraft`). GUI needs `Xvfb`, ImageMagick `import`, `libXtst`.

Feature map: [`features/README.md`](features/README.md).
