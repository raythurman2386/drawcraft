# Document lifecycle (autosave, recovery, external change, Open Recent)

## What it is

Phase 4 data safety: debounced autosave, crash recovery prompt, external-change
prompt (never silently clobber), and Open Recent with missing-file handling.

## Command ids

| Id | Role |
|---|---|
| `session.autosave_status` | enabled, interval_ms, path, pending |
| `session.autosave_set` | `{enabled?, interval_ms?}` (min 1000 ms) |
| `session.autosave_now` | synchronous write; returns `{path, revision}` |
| `session.recover_list` | `{candidates:[...]}` |
| `session.recover` | `{action:"restore"|"discard", autosave?}` |
| `session.external_check` | `{changed, path, disk_mtime}` |
| `session.reload` | `{force?}` — dirty without force errors |
| `session.keep_mine` | accept disk fingerprint; next save overwrites |
| `doc.recent` | `{recent, entries:[{path,exists}]}` |
| `doc.recent_clear` / `doc.recent_remove` | manage list |
| `file.save` / `file.save_as` | optional `force` to overwrite after external change |

## GUI entry

- File → Autosave checkbox; File → Open recent (missing + Remove + Clear recent).
- Launch recovery modal: **Restore** / **Discard**.
- External-change modal: **Reload** / **Keep mine** (also on save when disk drifted).
- Idle windows keep ticking via `request_repaint_after` from `Session::tick`.

## Drive

Always isolate config:

```sh
export XDG_CONFIG_HOME=$P/xdg
export DRAWCRAFT_BIN=$PWD/target/debug/drawcraft
```

CLI/MCP (primary): `scripts/lifecycle-cli.sh $P` — edit via MCP, `session.autosave_now`,
SIGKILL, relaunch, recover, assert. Multi-step state must use `drawcraft mcp` (not
per-process `drawcraft run`).

GUI: `scripts/gui-recovery.sh $P` and `scripts/gui-external.sh $P` on private Xvfb `:99`.
Short intervals: `DRAWCRAFT_AUTOSAVE_INTERVAL_MS=1000`.

Fuzz smoke: `cargo run -p xtask -- fuzz 30` (needs nightly + cargo-fuzz).

## Gotchas

- Tests/scripts must set `XDG_CONFIG_HOME`; otherwise recent/autosave write to the real config dir.
- Own autosaves write a different file and never trip the external watcher.
- Equal-content autosaves are discarded silently (stale worker after save).
- `drawcraft run` is one process per command — autosave/poll state does not persist across runs.
