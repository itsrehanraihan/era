# Milestones

Work happens strictly in order. A milestone is done only when its exit
criteria are met and `cargo fmt --check`, `cargo check` and `cargo test`
all pass.

| # | Milestone | Status |
|---|---|---|
| M0 | Inspect, decide, document | **Done** (syntax approved 2026-11-21) |
| M1 | `era new`, `era.toml`, project discovery, `era check` (structure) | **Done** |
| M2 | Lexer, parser and AST with spans; parser tests | **Done** |
| M3 | Skin tokens, semantic checks, UI IR, asset validation | **Done** |
| M4 | Preview from the UI IR | **Done; awaiting review** |
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

## First demo: E-Renty (after M3)

`examples/erenty/` contains four screens built from the owner's mockups:
Splash, Home, Menu and Verify code. It has 4 screens, 11 components,
1 layout, 21 assets and 187 UI elements, and `era check` passes with no
errors. Its IR is snapshot-tested (`crates/era_ir/tests/snapshots/erenty.ir`).

Building it found and fixed two problems (D-039 and an error-message
article). It also raised open questions Q-07 to Q-11 (gradients, rich
text, overlap, dashed lines/italics, safe areas) and the proposal D-041
(`fill` along the main axis), which M4 needs.

`Quick_start_guide.md` walks a new user from install to a checked
project. Every step was run on a fresh install.

## M4 deliverables

- The `era_codegen_compose` crate turns the UI IR into Kotlin + Compose
  (foundation only). It generates the theme (light and dark palettes,
  dimension tokens, text styles, fonts), a runtime, a screen registry, and
  one composable per screen. Output is deterministic and
  snapshot-tested.
- `platforms/desktop-preview/`: the Compose Multiplatform Desktop host,
  with the asset host API (D-044), a PNG renderer and a preview window.
- `era preview [path]` opens a window. `era preview [path] --png [dir]`
  renders every screen (and dark variants) to 804 × 1748 PNGs without a
  display.
- The **E-Renty demo renders**: `docs/images/erenty-preview.png`. It was
  compared side by side with the mockups. Fixes found while doing this
  are D-043 (the root fills the screen) and the demo tuning, and the
  Inter font is now bundled (D-049).
- Tests: codegen unit tests, Kotlin snapshots for both examples, and CLI
  tests (argument errors, refusing invalid projects, the display check,
  a missing Gradle, stale-file removal). There is also a real Gradle smoke
  test, `cargo test -p era_cli -- --ignored`, which passed in the dev
  container.

Known limitations:
- The preview **window** could not be tested here, because the container
  has no display. Only the `--png` path was run.
- Buttons have no click handling, and nothing navigates (M6).
- `divider` is horizontal only. `gap` is ignored with
  `distribute: between/around/evenly`.
- Font weights are guessed from file names (`Inter-SemiBold.ttf` → 600).
- The first `era preview` downloads Kotlin and Compose (about a minute
  in the dev container). No Gradle wrapper is shipped, so Gradle 8.x must
  be installed.
- Remaining visual gaps against the mockups are the open questions
  Q-07 to Q-11 (gradients, rich text, overlap, dashed lines, safe areas).

## Next smallest task (M5)

1. Install the Android SDK (command-line tools, platform 35, build-tools)
   in the dev environment. Decide the AGP version and the min/target SDK.
2. Add the `platforms/android/` host: `MainActivity` showing
   `EraStartScreen`, plus `eraImage`/`eraIcon`/`eraFontFamily` for
   Android resources.
3. Copy assets into `res/` with sanitized names. Decide Q-03 (SVG to
   VectorDrawable).
4. Add `era build android`, which builds a debug APK, with a smoke test.
   Install and launch on an emulator or device (Q-06).
