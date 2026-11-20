# Milestones

Work happens strictly in order. A milestone is done only when its exit
criteria are met and `cargo fmt --check`, `cargo check` and `cargo test`
all pass.

| # | Milestone | Status |
|---|---|---|
| M0 | Inspect, decide, document | **Done; awaiting owner approval of syntax (DECISIONS D-007…D-019, Q-01…Q-06)** |
| M1 | `era new`, `era.toml`, project discovery, `era check` (structure) | Not started |
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

## Next smallest task (M1, after approval)

1. Implement `era new <name>` to write `era.toml` and `src/app.era`.
2. Add the `era.toml` schema and validation, with errors for missing or
   invalid keys.
3. Implement `era check` to find the project root, require `src/app.era`
   and list the source files.
