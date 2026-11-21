# Milestones

Work happens strictly in order. A milestone is done only when its exit
criteria are met and `cargo fmt --check`, `cargo check` and `cargo test`
all pass.

| # | Milestone | Status |
|---|---|---|
| M0 | Inspect, decide, document | **Done** (syntax approved 2026-11-21) |
| M1 | `era new`, `era.toml`, project discovery, `era check` (structure) | **Done** |
| M2 | Lexer, parser and AST with spans; parser tests | **Done** |
| M3 | Skin tokens, semantic checks, UI IR, asset validation | **Done; awaiting review** |
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

## M2 deliverables

- The `era_syntax` crate:
  - a lexer producing typed tokens (strings with escapes, `#RRGGBB(AA)`
    colors, signed numbers) with byte spans
  - a recovering recursive-descent parser for the uniform node grammar
  - an AST in which every node keeps its source span
  - structural checks (D-027)
- An asset reference collector (`image`/`icon` literal paths), ready for
  M3 validation.
- `era check` now parses every `.era` and `.ers` file, runs the
  structural checks, and prints a declaration summary.
- Friendly errors for the brief's original draft syntax (`use skin`, a
  screen nested in `app`).
- Diagnostic codes E1001–E1218 are documented in `docs/DIAGNOSTICS.md`.

Known limitations:
- Property names, token names and value types are not validated yet. For
  example, `gapp: md` passes M2 and is caught in M3.
- `start: Home` is not yet checked against the declared screens (M3).
- Asset files referenced in source are not yet checked to exist (M3).
- If `era.toml` is invalid, source files are not parsed in that run. Fix
  the manifest first.

## M3 deliverables

- The `era_ir` crate:
  - typed skins merged across files, with validation of sections, value
    types, required colors, the `body` style, light/dark parity, weights
    and font files
  - the built-in default skin (D-020)
  - a property table for every primitive (`schema.rs`)
  - token lookup by property type, with "did you mean" suggestions
  - component and layout expansion with props, variants (`when`) and
    slots
  - asset existence and type checks
  - the platform-independent UI IR (`ir.rs`), in which every element has
    stable ids and an origin chain, and default values are resolved
- `era check` builds the IR and prints a summary. `era check --ir` prints
  the full resolved tree.
- The showcase gains image and icon examples (`Assets/Images/hero.png`,
  `Assets/Icons/star.svg`).
- A snapshot test of the showcase IR, regenerated with
  `UPDATE_SNAPSHOTS=1 cargo test`, plus a determinism test.
- Diagnostic codes E2001–E2152 are documented.

Known limitations:
- Dark mode is parsed and validated, but which mode applies, and when,
  is for the backends (P1).
- SVG icons are only checked to exist and to have the right extension.
  Converting them to native vectors is Q-03, due in M5.
- Navigation, events and state are not part of the IR yet (M6).
- Text styles are not checked against the fonts' actual weights.

## Next smallest task (M4)

1. Decide the generated Kotlin package layout and pin the
   Compose/Kotlin versions (record them in DECISIONS).
2. Create the `era_codegen_compose` crate: UI IR → Kotlin/Compose
   source, with deterministic output and snapshot tests.
3. Add a Compose Desktop preview host (Gradle) and `era preview`, with a
   `--png` headless mode that renders each screen to an image.
