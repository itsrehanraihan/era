# Decisions Log

Status values: **Accepted** (from the brief, or approved by the owner),
**Proposed** (awaiting owner approval), **Open** (needs input).

## Accepted (from the starter brief)

- **D-001** The compiler and CLI are written in Rust.
- **D-002** The MVP targets Android only. iOS is out of scope for the MVP,
  but the UI IR must stay platform-independent so an iOS backend can be
  added later.
- **D-003** The Android backend generates Kotlin and Jetpack Compose. It
  never uses a WebView.
- **D-004** The pipeline is: lexer → parser → AST (with spans) → semantic
  checks → UI IR → codegen → Gradle.
- **D-005** The project folders are `Skins/ Assets/ src/{Screens,Components,Layout,Logic}`.
  Only `era.toml` and `src/app.era` are required.
- **D-006** Start with a small crate workspace. Split crates only when
  there is a real boundary.

## Accepted syntax refinements (approved by the owner on 2026-11-21)

- **D-007 One uniform node grammar.** Everything is `name [arg] { property: value, child }`,
  and meaning is checked semantically. *Why:* a tiny parser, good error
  recovery, and new primitives need no grammar changes.
- **D-008 Skins are referenced by name, not path.** `skin: Studio` replaces
  `use skin "../Skins/theme.ers"`, and `Skins/*.ers` loads automatically.
  *Why:* the brief requires project-relative paths, and the example's `../`
  path was relative to the file. Names also survive moving files.
- **D-009 `skin Studio` uses an identifier, not a string.** *Why:* it is
  referenced from code, like `screen Home`. Only `app "..."` keeps a string,
  because it is the user-visible display name.
- **D-010 Colors are literals: `#FF6B35`, not `"#FF6B35"`.** *Why:* the
  lexer can type and validate them, which gives better errors.
- **D-011 Typography uses named text styles** (`heading { size: 28, weight: bold }`)
  instead of flat keys (`heading_size`, `heading_weight`). *Why:* this
  allows `text "x" { style: heading }` and avoids a combinatorial explosion
  of keys.
- **D-012 Screens are top-level declarations**, not nested inside `app {}`.
  `app` sets `skin` and `start`. *Why:* there is one way to declare a
  screen whether it lives in `app.era` or `Screens/Home.era`.
- **D-013 Token lookup is by property type.** `background: primary` finds a
  color, and `gap: md` finds a spacing value. No `$`, `@` or
  `colors.primary` prefix is needed. *Why:* the source reads cleanly for
  beginners, and the lookup is unambiguous because each property has one
  type.
- **D-014 Layouts use the same syntax as components** (`MainLayout { ... }`
  plus `slot`), restricted semantically to screen roots.
- **D-015 Variants use `when <variant> is <value> { overrides }`.** The
  overrides change properties only, never structure.
- **D-016 Asset paths are relative to `Assets/`** (`image "Images/hero.png"`)
  and are validated by `era check`.
- **D-017 A required color `on_primary` is added** to the brief's 10
  color tokens. *Why:* buttons need a guaranteed readable label color on
  `primary`.
- **D-018 `card` lays out its children vertically** with skin defaults. The
  brief's `card { column { ... } }` still works.
- **D-019 Separators are newlines or commas**, so `{ gap: md, padding: lg }`
  works on one line. There are no semicolons.

## Accepted in M1

- **D-020 (Proposed) Skin is optional.** If `app` has no `skin:`, a built-in
  neutral default skin applies. *Why:* the brief says a project may contain
  only `era.toml` and `src/app.era`. Until M3 defines the default skin, the
  `era new` template avoids tokens and uses plain numbers. **Needs owner
  approval before M3.**
- **D-021 Dependencies.** `toml` and `serde` parse `era.toml` with byte
  spans, so errors show line and column. `tempfile` is a test-only
  dependency. Diagnostics rendering is written by hand, with no extra
  crate.
- **D-022 Crates for M1.** The crates are `era_diagnostics`, `era_project`
  and `era_cli`.
- **D-023 Exit codes.** `0` means success (warnings allowed). `1` means the
  project has errors. `2` means a usage or environment error, such as no
  project found or a bad `era new` name.
- **D-024 Folder conventions produce warnings, not errors.** A misplaced or
  mis-cased folder or file is a warning. A missing `src/app.era`, an
  invalid `era.toml` or a non-UTF-8 source file is an error.

## Open questions

- **Q-01 (Resolved)** The proprietary notice is accepted. The repo is still public, at the owner's choice. Original question: **Closed source vs. a public repo.** `itsrehanraihan/era` is
  currently **public** on GitHub. The owner wants ERA to be closed source.
  Should the repo be made private before anything else is pushed? Which
  license or notice should be used (a proprietary "All rights reserved"
  notice is assumed for now)?
- **Q-02 (Resolved: named props only for the MVP)** **Positional props for components.** Should `AppButton "Go"` be
  allowed, in addition to `AppButton { label: "Go" }`? The MVP proposal
  supports named props only.
- **Q-03 Icon format.** SVG needs conversion to an Android VectorDrawable,
  either with a restricted SVG subset converted in Rust or by bundling an
  icon set. This must be decided before M5.
- **Q-04 Logic file extension.** The brief uses `.er` for logic files.
  This will be decided at M6, together with the state/event syntax.
- **Q-05 (Resolved: Compose Multiplatform Desktop, plus `--png`)** **Preview technology.** The proposal is Compose Multiplatform
  Desktop running the same generated code, with a `--png` headless mode.
  The alternatives are a Rust-native renderer or an HTML mock, but both
  would drift from Android. Compose Multiplatform could also be an iOS
  path later; a SwiftUI backend would be the more "native" iOS option.
- **Q-06 Emulator testing.** This cloud container probably lacks KVM. The
  "install and launch" step of M5 may need the owner's machine or a
  physical device.
