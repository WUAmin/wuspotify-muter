# AGENTS.md

## Scope

- macOS first. No other platforms unless asked.
- Isolate OS code: `src/platform/macos.rs`.
- No scattered `cfg`.
- Smallest correct change.

## Code

- Simple > clever. Stdlib > dependency.
- Small modules. Small funcs. One job each.
- Aim Cyclomatic complexity ≤10; prefer ≤5.
- Guard clauses; early returns; shallow nesting.
- Split branches only if named helper clarifies.
- No flag arguments for divergent behavior; use enum/separate func.
- `&str`/slices over owned types when possible.
- Avoid needless `clone`.
- `Result`; no `unwrap`/`panic` in production.
- `expect` only for proven invariant; explain it.
- Errors: typed, contextual, actionable. Never leak secrets.
- `pub` only when needed.
- `unsafe` only if unavoidable; add `// SAFETY:`.
- `rustfmt` is authoritative.
- Comments explain why, not what.

## Structure

- Thin `main.rs`; logic in `lib.rs`/modules.
- Add folders only for real responsibilities.
- Platform boundary for filesystem, processes, paths, OS APIs.
- Tests beside units; integration tests in `tests/`.

## Dependencies

- Check stdlib first.
- Add few, maintained, focused crates.
- Avoid overlapping crates/features.
- Commit `Cargo.lock` for apps.
- Review changes: `cargo tree`.

## Tests

- Test behavior, failures, edges.
- Deterministic. No network, home dir, order, or machine assumptions.
- Temp dirs for filesystem tests.
- Regression test bugs where useful.

Run before done:

```bash
cargo fmt --check
cargo check --all-targets
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## README structure

Keep short, task-first, macOS-only:

1. What it does — 1–2 lines.
2. Prerequisites — macOS, Rust/version, external requirements.
3. Install/build — copy-paste commands.
4. How to use — one safe, verified end-to-end example; expected output if stable.
5. Config — only if needed: path/env vars, defaults, precedence.
6. Help — command.
7. Limits — platform support, known constraints.

- Real commands only; verify on macOS.
- Use `<placeholders>`.
- Document destructive actions separately and visibly.
- Update README with behavior, flags, config, install changes.
- Delete stale text; no history/design diary.

## Workflow

1. Read `README.md`, `Cargo.toml`, relevant code.
2. Implement smallest clear fix.
3. Update tests/docs.
4. Run checks.
5. Report change, validation, limits.
