# Contributing to DrawCraft

Thanks for helping build a local-first, clean-room whiteboard.

## Before you start

- Read [AGENTS.md](AGENTS.md) and [ARCHITECTURE.md](ARCHITECTURE.md).
- DrawCraft does not accept copied code, assets, or algorithms from proprietary tools. Describe the public behaviour or standard algorithm you are implementing instead.
- Keep the layer rules: lower crates cannot depend on higher crates. `cargo run -p xtask -- layers` enforces this.

## Workflow

```sh
cargo run -p xtask -- check
```

All changes should keep the full gate green (format, strict clippy, tests, layers, command coverage).

For a feature:

1. Add a command id and registry entry.
2. Implement it in the editor or session.
3. Add tests.
4. Wire the UI.
5. Update `ROADMAP.md` / `CHANGELOG.md` if the scope changed.

## Commit style

Prefer focused commits with a short imperative subject, for example:

- `editor: remap arrow bindings when duplicating pages`
- `render: fill concave shapes with ear clipping`
- `app: add page management panel`

## License

By contributing, you agree your work is licensed under MIT OR Apache-2.0.
