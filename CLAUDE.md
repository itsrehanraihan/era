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

## MVP follows HTML/CSS (owner rule, D-080, 2026-11-22)
For the MVP, ERA is built **the HTML/CSS way**: CSS property names (every
spelling works: `font-size`, `fontSize`, `font_size`, `fontsize`), CSS
values and units, CSS shorthands, CSS keyword spellings, HTML element
names as aliases, and CSS-style states and media rules. Don't invent
ERA-specific syntax where CSS already has a form. Native output only (no
WebView); web-only CSS gets an ERA equivalent and a clear error.
The owner will reshape the syntax for **v2**. Every construct you add
must also be added to `docs/V2_SYNTAX_CHEATSHEET.md` (with an empty
"Your v2 syntax" cell), and its status flipped from 🧩 to ✅ there and in
`docs/CSS_SPEC.md`.

## Keep docs in sync (owner rule)
Every change updates the docs it affects, in the same commit:
- Syntax or grammar changes go in `docs/GRAMMAR.md`, plus a `DECISIONS.md`
  entry.
- New or changed diagnostics go in `docs/DIAGNOSTICS.md`.
- CLI behaviour or project layout goes in `docs/ERA_SPEC.md` and
  `README.md`.
- Skins and tokens go in `docs/SKIN_SPEC.md`. UI primitives go in
  `docs/UI_SPEC.md`.
- Milestone progress, deliverables and known limitations go in
  `MILESTONES.md`.

## Checks before every commit
```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```
When code generation, the preview host or the Android host changes,
also run the real Gradle smoke tests. They need Gradle, network access,
and `ANDROID_HOME` for the APK test:
```bash
cargo test -p era_cli -- --ignored
```

ERA Studio (`crates/era_studio`) has headless UI tests in the normal
run. To refresh its screenshot (needs a GPU, or lavapipe on Linux):
```bash
VK_ICD_FILENAMES=/opt/lavapipe/lvp_icd.json cargo test -p era_studio -- --ignored
```

## Layout
- `crates/`: Rust workspace crates (see MILESTONES.md for the planned
  split)
- `docs/`: specs
- `examples/`: ERA sample projects, also used as test fixtures
- `platforms/android/`: Android host template (from M5)
