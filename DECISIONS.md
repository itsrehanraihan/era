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

- **D-020 (Accepted 2026-11-21) Skin is optional.** If `app` has no `skin:`, a built-in
  neutral default skin applies. *Why:* the brief says a project may contain
  only `era.toml` and `src/app.era`. Until M3 defines the default skin, the
  `era new` template avoids tokens and uses plain numbers. The default
  skin's values are listed in `docs/SKIN_SPEC.md`.
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

## Accepted in M2

- **D-025 Negative numbers.** `-0.5` is a number literal, needed for
  `tracking`. This fixes an inconsistency in the M0 draft, which said
  "no sign" while its own skin example used `-0.5`.
- **D-026 Keywords as property names.** A keyword followed by `:` is a
  property or token name (`skin: Studio`, `none: 0`). This is required by
  the approved syntax itself.
- **D-027 Structure is checked in M2; meaning is checked in M3.** M2 checks
  where forms may appear, root and slot counts, variant and `when`
  consistency, naming conventions, the single `app` and duplicate names.
  M3 checks token lookup, property names and types, and asset existence.
- **D-028 One `era_syntax` crate** holds the lexer, parser, AST and
  structural checks. It has no new external dependencies.

## Accepted in M3

- **D-029 Components and layouts are expanded inline in the UI IR.** Each
  expanded element records its `origin` chain, such as
  `<Palette>Swatch>`. *Why:* this gives one simple tree for every backend,
  and resolved values never depend on call sites. Revisit if generated
  code grows too large.
- **D-030 Defaults are resolved in the IR, not in backends.** Examples are
  `text` style `body`, color `text`, and the `card` surface. Preview and
  Android therefore cannot disagree. See the table in `docs/UI_SPEC.md`.
- **D-031 Components are configured only through their props and
  variants.** Style properties cannot be set on a component from outside
  (E2143). *Why:* variants are the controlled way to change a component's
  look, as the brief asks.
- **D-032 Font resolution.** `font: "Inter"` needs at least one file in
  `Assets/Fonts/` named `Inter.ttf` or `Inter-*.ttf` (or `.otf`).
  `"system"` uses the platform font.
- **D-033 Asset types come from the element.** `image` takes
  png/jpg/jpeg/webp and `icon` takes svg, whatever folder the file is in.
  The icon conversion question (Q-03) is still open for M5.
- **D-034 `start` is optional when there is exactly one screen.**
- **D-035 Every component and layout is checked on its own as well**
  (validation mode, with placeholder props and every `when` block
  checked). Unused components are therefore still checked, and an error is
  reported once, not once per use.
- **D-036 Semantic analysis runs only when syntax and structure are
  error-free**, so a broken file does not cause a flood of follow-on
  errors.
- **D-037 An optional prop with no value is "absent".** A property set
  from it is left unset. An `image` or `icon` showing it is left out. A
  `text` showing it is empty.
- **D-038 A `scroll` whose single `slot` is filled with several elements**
  wraps them in a `column` automatically.

## Accepted during the E-Renty demo (after M3)

- **D-039 `none` on a spacing, radius, border or elevation property means
  0.** Examples: `elevation: none`, `gap: none`. *Why:* `none` is a
  keyword, so the M0 example's `elevation { none: 0 }` token could never
  be referenced. That token has been removed from the default skin and
  the examples.
- **D-040 The first demo is E-Renty** (`examples/erenty/`), built from
  the owner's mockups. The showcase stays as a compact test fixture.
  Brand illustrations in the demo were cropped from the mockups and
  should be replaced with original exports.

## Accepted in M4

- **D-041 (Accepted 2026-11-21) `fill` along a row or column's main axis
  shares the remaining space.** For example, three `MenuTile`s with `width: fill` in a `row`
  get equal widths (Compose `weight(1f)`). Across the cross axis, `fill`
  takes the full size. The E-Renty demo relies on this.

- **D-042 Pinned preview toolchain.** Kotlin 2.1.21 (with the compose
  compiler plugin) and Compose Multiplatform 1.8.2. Gradle 8.x comes from
  `PATH` or `ERA_GRADLE`, and the JDK must be 17+. *Why:* these versions
  are known to work together and are on Maven Central. No Gradle wrapper
  is shipped yet.
- **D-043 A screen's root element fills the screen** unless it sets
  `width` or `height`. This is applied in the IR. *Why:* found while
  rendering E-Renty, where a page's tint stopped at its content.
- **D-044 Platform host API.** Generated code loads assets only through
  `eraImage(path)`, `eraIcon(path)` and `eraFontFamily(files)`, which
  each platform implements. The generated UI code is therefore identical
  for the preview and Android.
- **D-045 Generated package `<app id>.ui`.** Files are `EraTheme.kt`,
  `EraRuntime.kt`, `EraScreens.kt` and `screens/<Name>Screen.kt`, plus a
  header saying "Generated by ERA. Do not edit".
- **D-046 Preview canvas.** 402 × 874 dp at 2× (804 × 1748 px), matching
  the mockups. Dark renders are added when the skin has `colors dark`.
- **D-047 Compose foundation only, no Material.** Buttons, cards and
  inputs are built from `Box`, `Column` and `BasicText`, so the skin fully
  controls their look.
- **D-048 `fill` along a scrolling axis is ignored**, because that space
  is infinite and Compose would crash on a weight there.
- **D-049 The E-Renty demo bundles the Inter font** (SIL Open Font
  License 1.1; the license is in `Assets/Files/`). This also exercises
  the custom-font path.

## Accepted in M5

- **D-050 Safe areas.** A screen's root element draws its background
  behind the system bars, and its content is padded clear of them
  (`safeDrawingPadding()`). The Android app runs edge to edge, which
  Android 15 requires. On the desktop preview the insets are zero.
- **D-051 SVG icons are converted to Android VectorDrawables in Rust**, at
  build time. The supported subset is `path`, `circle`, `ellipse`, `rect`,
  `line`, `polyline`, `polygon` and untransformed `g`, with
  fill/stroke/opacity/linecap/linejoin/fill-rule, also via `style`.
  Unsupported features (transforms, gradients, text, masks, unknown colors)
  are build errors (E3001) that name the feature. This needs one new
  dependency: `roxmltree`, a small pure-Rust XML reader.
- **D-052 Pinned Android toolchain.** AGP 8.9.3, compile/target SDK 35,
  min SDK from `era.toml` (default 24), Compose BOM 2025.05.01 (foundation
  and ui only), activity-compose 1.10.1, JVM target 17. The platform theme
  is `Theme.Material.Light.NoActionBar`, with no AppCompat or Material.
- **D-053 Images and fonts are packaged in the APK's `assets/` folder at
  their ERA paths**, not in `res/`, so no file renaming is needed. Only
  icons become resources (`era_icon_<path>`), looked up through a generated
  `EraIconIds` table.
- **D-054 Debug screen selection.** Until navigation exists (M6), the
  intent extra `era.screen` opens any screen:
  `adb shell am start -n <id>/<id>.ui.MainActivity --es era.screen Home`.
- **D-055 `era build android` builds a debug APK** signed with the
  standard debug key. Release signing is out of scope for the MVP.

## Open questions

- **Q-01 (Resolved)** The proprietary notice is accepted. The repo is still public, at the owner's choice. Original question: **Closed source vs. a public repo.** `itsrehanraihan/era` is
  currently **public** on GitHub. The owner wants ERA to be closed source.
  Should the repo be made private before anything else is pushed? Which
  license or notice should be used (a proprietary "All rights reserved"
  notice is assumed for now)?
- **Q-02 (Resolved: named props only for the MVP)** **Positional props for components.** Should `AppButton "Go"` be
  allowed, in addition to `AppButton { label: "Go" }`? The MVP proposal
  supports named props only.
- **Q-03 (Resolved in M5, D-051)** **Icon format.** SVG needs conversion to an Android VectorDrawable,
  either with a restricted SVG subset converted in Rust or by bundling an
  icon set. This must be decided before M5.
- **Q-04 Logic file extension.** The brief uses `.er` for logic files.
  This will be decided at M6, together with the state/event syntax.
- **Q-05 (Resolved and implemented in M4: Compose Multiplatform Desktop, plus `--png`)** **Preview technology.** The proposal is Compose Multiplatform
  Desktop running the same generated code, with a `--png` headless mode.
  The alternatives are a Rust-native renderer or an HTML mock, but both
  would drift from Android. Compose Multiplatform could also be an iOS
  path later; a SwiftUI backend would be the more "native" iOS option.
- **Q-07 Gradients.** The E-Renty hero cards and buttons use gradients.
  The demo uses solid colors for now. A future skin token type could be
  `gradient`.
- **Q-08 Mixed styles within one sentence.** "Good morning, **Rehan**."
  and "Didn't get a code? **Resend**" are written as a `row` of `text`
  elements, which does not wrap like a sentence. Rich text spans would
  fix this.
- **Q-09 Overlap and offset positioning.** The raised "+" button in the
  bottom bar and the chart tooltip's pointer need offsets. The demo
  approximates both.
- **Q-10 Dashed lines and italic text.** The chart grid lines are
  dashed, and the wordmark is italic. The demo uses the logo as an image.
- **Q-11 (Partly resolved, D-050)** **System bars and safe areas.** The status bar and home indicator
  in the mockups are drawn by the OS. Backends must add safe-area insets.
- **Q-12 App launcher icon.** The APK uses the system default icon. An
  `app` property such as `icon: "Images/app_icon.png"` could generate
  adaptive icons.
- **Q-06 (Blocked in this environment) Emulator testing.** The dev
  container has no KVM, so no emulator. A JVM screenshot tool was tried
  (Paparazzi): 1.3.5's layout engine is too old for Android 15 APIs, and
  2.0-alpha needs a newer AGP than D-052. On-device install and launch
  must be done on the owner's machine or phone. Original question:
  **Emulator testing.** This cloud container probably lacks KVM. The
  "install and launch" step of M5 may need the owner's machine or a
  physical device.
