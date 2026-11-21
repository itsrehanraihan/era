# CLAUDE.md — working rules for ERA

ERA is a closed-source, design-first language and toolchain for native
apps. The compiler is written in Rust. The MVP targets Android through
generated Kotlin and Jetpack Compose.

## Read first
- `MILESTONES.md`: the current milestone. Work only on that milestone.
- `DECISIONS.md`: accepted, proposed and open decisions. Never change
  language syntax silently; record a proposal there and ask the owner.
- `docs/GRAMMAR.md`, `docs/UI_SPEC.md`, `docs/SKIN_SPEC.md`: the language
  definition.

## Hard rules
- Do not use a WebView. Do not let Android types into the grammar, AST or
  UI IR.
- Every AST node keeps a source span. Diagnostics show the file, line and
  column.
- Generated output must be deterministic.
- Do not add dependencies without a one-line reason in the commit or in
  DECISIONS.md.
- Never claim a feature works unless it built and was tested. Report the
  actual command output.
- Do not start out-of-scope work: iOS, generics, macros, plugins,
  animations, networking, or an IDE.

## Checks before every commit
```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

## Layout
- `crates/`: Rust workspace crates (see MILESTONES.md for the planned
  split)
- `docs/`: specs
- `examples/`: ERA sample projects, also used as test fixtures
- `platforms/android/`: Android host template (from M5)
