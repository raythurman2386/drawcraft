# CLAUDE.md

Read [AGENTS.md](AGENTS.md) first. It contains the clean-room rules, layered architecture, command-system contract, run instructions, and quality gates.

Quick reminders:

- Everything user-visible is a command in `crates/command`, executed by `Editor::dispatch` or `Session::execute`.
- The engine and editor are headless and tested without a window.
- Never panic or unwrap in production paths; return `Result`.
- Run `cargo run -p xtask -- check` before calling work done.
- Do not copy code, assets, or algorithms from tldraw, godraw, Adobe, or other proprietary tools.
