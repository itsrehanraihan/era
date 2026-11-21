# Milestones

Work happens strictly in order. A milestone is done only when its exit
criteria are met and `cargo fmt --check`, `cargo check` and `cargo test`
all pass.

| # | Milestone | Status |
|---|---|---|
| M0 | Inspect, decide, document | **Done** (syntax approved 2026-11-21) |
| M1 | `era new`, `era.toml`, project discovery, `era check` (structure) | **Done; awaiting review** |
| M2 | Lexer, parser and AST with spans; parser tests | Not started |
| M3 | Skin tokens, semantic checks, UI IR, asset validation | Not started |
| M4 | Preview from the UI IR | Not started |
| M5 | Kotlin/Compose codegen, Android host project, debug APK | Not started |
| M6 | Button events, minimal state, navigation | Not started |

## M0 deliverables

- Rust workspace with an `era` CLI stub (`crates/era_cli`)
- `CLAUDE.md`, `DECISIONS.md`, `MILESTONES.md`
- `docs/GRAMMAR.md`, `UI_SPEC.md`, `SKIN_SPEC.md`, `ERA_SPEC.md`, `ANDROID_BACKEND.md`
- `examples/showcase/`: a draft example in the proposed syntax. It is
  **not yet checked by any tool**.

## Planned crate split (created only when needed)

| Crate | Introduced | Contents |
|---|---|---|
| `era_cli` | M0 | The `era` binary and argument handling |
| `era_project` | M1 | `era.toml`, project discovery, asset path resolution |
| `era_diagnostics` | M1 | Spans, error codes, terminal rendering |
| `era_syntax` | M2 | Lexer, parser, AST |
| `era_ir` | M3 | Skin tokens, semantic analysis, UI IR |
| `era_codegen_compose` | M4/M5 | UI IR → Kotlin/Compose (shared by preview and Android) |

The brief's separate `era_lexer`, `era_parser`, `era_ast`, `era_ui` and
`era_skins` crates are merged into `era_syntax` and `era_ir` until a real
boundary appears.

## M1 deliverables

- `era new <name>` writes `era.toml`, `src/app.era` and `.gitignore`. It
  validates the name and refuses to overwrite a folder that is not empty.
- `era.toml` is validated (name, id, version, edition, `android.min_sdk`),
  and every error is reported with its line and column.
- `era check [path]` finds the project root by walking up from the given
  folder. It requires `src/app.era`, sorts files into UI, skin, logic and
  asset files, and warns about misplaced files, unknown extensions and
  mis-cased folders.
- Asset path resolution (`era_project::assets::resolve_asset`) is ready for
  M3. It rejects absolute paths and `..`.
- Diagnostic codes are listed in `docs/DIAGNOSTICS.md`.

Known limitations:
- `era check` does not read `.era` or `.ers` contents beyond checking that
  they are valid UTF-8. Syntax checks arrive in M2.
- Output has no colour.
- `era new` takes a folder name; the `id` defaults to `dev.era.<name>`.

## Next smallest task (M2)

1. Create the `era_syntax` crate with a lexer (tokens, spans, newline
   handling) and lexer tests.
2. Add the parser for the uniform node grammar, with success and failure
   tests.
3. Make `era check` parse every `.era` and `.ers` file and report syntax
   errors with snippets.
