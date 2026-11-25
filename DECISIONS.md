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

## Accepted in M6 (owner approved "stores + actions" on 2026-11-21)

- **D-056 Stores and actions.** Logic lives in `.er` files under `src/`
  (conventionally `src/Logic/`), as `store Name { ... }` blocks:
  - `state name: text|number|bool [= value]` declares a piece of state.
    Defaults are `""`, `0` and `false`.
  - `action name { ... }` contains steps: `set x = value`, `set x = other`,
    `toggle x` (bool) and `add x n` (number).
  - The UI only refers to stores, never the other way round. Logic
    cannot navigate (D-063).
- **D-057 Actions are chained with `then`, not `,`.** The approved preview
  showed `on_press: Session.verify, go Home`, but `,` already separates
  items, so that line would parse as two items. It is written
  `on_press: Session.verify then go Home`.
- **D-058 Text interpolation:** `"Code: {Session.code}"` shows state in
  any text (element text, text props, placeholders). Use `{{` and `}}` for
  literal braces. Numbers show without a trailing `.0`.
- **D-059 New properties.** `on_press` (tap) and `visible` (a bool literal
  or bool state) apply to every element. `bind` is for `input` only and
  takes text state.
- **D-060 `action` prop type.** `prop tap: action?` lets a component
  receive taps from outside (`IconTile { tap: back }`). This keeps D-031:
  outside style properties are still not allowed.
- **D-061 Navigation is a stack.** `go Screen` pushes a screen and `back`
  pops one. The app starts at `start`. Android's system back gesture pops
  the stack, and on the first screen it leaves the app.
- **D-062 Store state is app-wide and lives as long as the app.** Each
  store becomes a Kotlin `object` with Compose state, and numbers are
  `Double`. Persistence (saving across restarts) is not in the MVP.
- **D-063 Logic never refers to screens.** `go` and `back` are only
  allowed in the UI, which keeps `.er` files screen-independent (brief
  §4).
- **D-064 Desktop preview test hook.** A `script=` line in
  `era-preview.properties` (`tap x,y; back`) renders `step-N.png` after
  each step. The smoke tests use it to verify taps and navigation
  headlessly.
- **D-065 Scope change: the ERA editor.** The owner asked (2026-11-21)
  for a GUI editor written in Rust, which the brief had listed as out of
  scope. The approved first version is a code editor with live preview,
  built with egui/eframe for macOS (Apple Silicon). It is planned as M7,
  and owner-assisted testing on the owner's Mac was approved.

## Accepted in M7 (ERA Studio)

- **D-066 Studio stack.** Studio uses eframe/egui 0.37 (native windows,
  Metal on macOS via wgpu), `rfd` for the native folder picker and
  `image` (PNG only) for preview textures. `egui_kittest` is used for
  headless UI tests and screenshots, as a dev-dependency only.
- **D-067 One shared check pipeline.** `era_ir::pipeline::check_sources_with`
  is used by both `era check` and Studio. Studio passes unsaved editor
  text through it, so problems update as you type, in-process, with no
  subprocess.
- **D-068 Studio runs the `era` CLI for Render and Build APK**, in a
  background thread, with output streamed to the Output tab. The binary
  is found through `ERA_BIN`, then an `era` next to `era-studio`, then
  `PATH`.
- **D-069 Studio's interface text uses only characters the bundled egui
  fonts contain** (no ▶ ✖ ⚠ ⌘). The UI tests run with egui's
  missing-glyph check set to panic, which caught this. User text never
  panics; it falls back to a box.
- **D-070 Every Studio widget has an accessible name** (for VoiceOver on
  macOS). The UI tests enforce this.

## M8: CSS-familiar styling (see docs/CSS_SPEC.md)

- **D-071 (Accepted, owner 2026-11-21) Scope change: ERA adopts
  HTML5/CSS3 familiarity.** Every CSS property and value that has a native
  equivalent is supported, with CSS names and values. Web-only parts get
  ERA equivalents and clear errors. This replaces the brief's "avoid
  copying the CSS specification". The output stays native (no WebView).
- **D-072 (Accepted: camelCase) Property names.** Canonical names are
  camelCase (`fontSize`). **Proposed:** pasted kebab-case
  (`font-size`) is also read, and current snake_case names are
  deprecated with an `era fmt --fix` migration (CSS_SPEC §12).
- **D-073 (Accepted: full CSS values) Values.** Units, functions and
  shorthands, with `px` = dp (CSS_SPEC §3). Every sizing mode is required
  by the owner: fixed, %, fill, fit/auto, fr, min/max-content, vw/vh,
  aspect-ratio (§4).
- **D-074 (Accepted, not breaking, see D-080) `;` separates items on one line**, as in
  CSS, so `,` can appear inside values (`rgba(0, 0, 0, .1)`, several
  shadows). Newlines still separate items. `era fmt --fix` migrates.
- **D-075 (Accepted, D-080) Interaction states and
  responsiveness:** `hover`/`active`/`focus`/`disabled` blocks, `media
  (minWidth: …)` rules, and optional style classes in skins. No CSS
  selectors or cascade.
- **D-076 (Accepted, D-080) HTML element names as aliases** (`div`, `p`, `h1`,
  `img`, `a`, …) plus new form controls (checkbox, radio, switch, slider,
  select, textarea, date/time) and `list` (§11).
- **D-077 (Proposed) Inheritance:** only `color` and font properties
  inherit from parent elements, as in CSS. Nothing else does.
- **D-078 (Proposed) Native limits:** real box shadows need Compose 1.9
  (a toolchain bump at phase P4). `blur` needs Android 12+ and is skipped
  below that with a warning. `position: sticky`, `skew`, `backdropFilter`
  and `calc()` with mixed units come later.
- **D-079 (Accepted, owner 2026-11-21) Rollout:** spec first
  (`docs/CSS_SPEC.md`), then phases P1–P8, each tested and shipped.

## Accepted 2026-11-22: the MVP follows HTML/CSS

- **D-080 (owner) The MVP is built the HTML/CSS way. The owner will
  reshape the syntax, patterns and grammar for v2.** This settles the
  open M8 decisions:
  - **D-072:** every spelling works (kebab, camel, snake, lowercase).
  - **D-073:** full CSS values; plain numbers still work.
  - **D-074:** `;` separates items as in CSS, and `,` keeps working, so
    there is no breaking change.
  - **D-075:** states and media rules are accepted.
  - **D-076:** HTML element names are accepted as aliases.

  `docs/V2_SYNTAX_CHEATSHEET.md` is the owner's input sheet for v2.
- **D-081 Phase 1 semantics** (implemented):
  - `px` = dp; `pt` = 4/3 dp; `rem`/`em` = 16 dp.
  - `%` is relative to the parent's content box.
  - `fr` and `flex` become `weight`.
  - `vw`/`vh` use the screen size (`LocalEraScreenSize`).
  - `display: none` removes the element. `flex-direction` turns a column
    into a row (or back) and reverses the children if asked.
  - `position: absolute` is placed against the parent's **padding box**,
    in an overlay that doesn't change the parent's size, and isn't
    squeezed by it. `position: fixed` is pinned to the screen.
    `position: relative` only offsets the element.
  - Shadows never clip content (only `overflow: hidden` does).
  - A row with `justify-content: space-between/around/evenly` and no
    width, inside a column, takes the full width, like a block-level flex
    row.
  - A comma followed by a value-like token continues a CSS list value
    (`0 1px red, 0 2px blue`); otherwise it separates items as before.

## Accepted 2026-11-22: production-ready output (owner)

- **D-082 (owner) Scope change: iOS output is SwiftUI.** "Even if I write
  HTML/CSS, the built app should compile to production-ready standard
  Swift/Android code." The iOS backend will generate SwiftUI. It is
  accepted but scheduled later: it needs Xcode on the owner's Mac. Until
  then `era export ios` prints that it is not available yet. This lifts
  the brief's "no iOS" rule for this backend only.
- **D-083 (owner) Order of work:** production Android first (M9), then
  the remaining CSS phases (M8 P3–P8), then real `.html`/`.css` input and
  the SwiftUI backend.
- **D-084 `era export android` (M9).** It writes a standalone Android
  Studio project that the team owns and edits by hand:
  - **Standard layout,** shared with `era build android`: an `app/`
    module, `gradle/libs.versions.toml` (version catalog), and the
    Gradle wrapper pinned to 8.14.3 (`gradlew`, `gradlew.bat`,
    `gradle/wrapper/`). The wrapper jar is the stock one, produced by
    `gradle wrapper`, and committed in `platforms/android/`.
  - **Clean code:** in export mode the "do not edit" headers become
    `Generated by ERA (era export). This is ordinary Kotlin: edit it
    freely.`, and internal IR ids (`@Home/0/2`) are left out of
    comments. Component origins stay as plain comments (`from HeroCard`).
  - **Safety:** the default output is `<project>/export/android`. A
    non-empty folder is refused (E3003). `--force` replaces only a
    folder holding the `.era-export` marker written by an earlier
    export, so a folder of someone else's files is never deleted.
  - An export is a one-way snapshot. Re-exporting overwrites hand edits,
    so the team should commit the export to its own repository.
  - The debug APK path is now
    `build/android/app/build/outputs/apk/debug/app-debug.apk`.

- **D-085 App launcher icon** (resolves Q-12). `app` takes
  `icon: "Images/app_icon.png"` (PNG, JPEG or WebP; 512x512 or larger is
  best) and an optional `icon-background` (a colour or skin colour,
  default white), like a web app manifest's `icons` and
  `background_color`. ERA writes legacy `mipmap-*/ic_launcher.png` icons
  for every density and an adaptive icon for Android 8+, with the image
  fitted into the middle 72dp of the 108dp layer so launcher masks don't
  cut it. The icon is not copied into the app's runtime assets.
- **D-086 Release builds.** `era build android --release` builds a
  signed, R8-shrunk release APK and an App Bundle (`.aab`, for Google
  Play). The keystore is named in `era.toml`
  (`[android.signing] keystore`, `alias`). Passwords are never written to
  disk by ERA: they come from `ERA_KEYSTORE_PASSWORD` (and
  `ERA_KEY_PASSWORD` if the key has its own). Exported projects read
  `keystore.properties` or the same variables. `.gitignore` excludes
  keys and `keystore.properties`. Without signing settings, `--release`
  fails with E3004 and shows the `keytool` command.

## M8 phase 3: borders, corners, outline

- **D-087 Borders the CSS way.** Every element takes `border-style`
  (`solid`, `dashed`, `dotted`, `none`), per-side borders
  (`border-top/right/bottom/left` and `-width`/`-color`/`-style`, also
  `inline`/`block` spellings), `border-width`/`-color`/`-style` with 1–4
  values, `border-radius` with 1–4 values or `%`, the per-corner radius
  properties, `outline` and `outline-offset`. Left/right map to ERA's
  logical start/end, so layouts flip in right-to-left languages. A later
  all-sides value replaces earlier per-side values (CSS cascade).
  Rendering: solid all-round borders use Compose `border`; dashed and
  dotted ones, per-side borders and outlines use small helpers in a new
  generated file, `EraDraw.kt`. Limits: per-side borders draw square
  corners; `double`/`groove`/`ridge`/`inset`/`outset` and elliptical
  radii (`a / b`) are E2135.

## M8 phase 4a: colours and gradients

- **D-088 CSS colours and gradients.** Colour properties take the 148
  CSS named colours and `transparent` (a skin token of the same name
  wins), `rgb()`/`rgba()`/`hsl()`/`hsla()` in comma or space syntax, and
  `var(--name)`, which reads a skin colour (`--on-primary` is
  `on_primary`; the lexer accepts `--name` words for this).
  `background` also takes `linear-gradient()` (angles in deg, turn, rad
  or grad; `to` a side or a corner, using CSS's size-dependent corner
  angle), `radial-gradient()` (a circle from the centre to the farthest
  corner; `ellipse` is drawn as that circle for now), colour stops with
  `%` positions filled in by the CSS rules, and comma-separated layers,
  top first. Rendering: `ShaderBrush` helpers in `EraDraw.kt`. Limits:
  `conic-gradient`, `repeating-*` and `url()` are E2135; a bare colour
  name cannot be the last layer after a comma, because a word after a
  comma starts a new item in ERA (use `#hex` or `rgb()` there).

## M8 phase 4b: shadows and filters

- **D-089 `box-shadow` and `filter` without a Compose upgrade.** Real CSS
  box shadows (x, y, blur, spread, colour, `inset`, several layers,
  `none`; no colour = the text colour, like `currentColor`) are drawn by
  `eraBoxShadow`/`eraInsetShadow` in `EraDraw.kt`: blurred copies of the
  element's shape, never drawn under the element itself, as in CSS. The
  blur comes from a small per-host hook, `eraBlurPaint` in
  `EraAssets.kt` (Android `BlurMaskFilter`, Skia `MaskFilter` in the
  preview). That is the technique Compose 1.9's `dropShadow` uses, so
  ERA keeps its pinned versions (D-052): moving to Compose 1.9 would
  need SDK platform 36 and a new desktop toolchain. Any `box-shadow`
  replaces the skin `elevation` shadow. `filter` supports `blur()`
  (Compose `blur`, Android 12+) and the colour filters `brightness`,
  `contrast`, `grayscale`, `saturate`, `sepia`, `invert`, `opacity` and
  `hue-rotate`, combined at build time into one colour matrix (Filter
  Effects spec). Limits: blurred shadows need Android 9+ (older versions
  draw a sharp shadow); `drop-shadow()` and `url()` filters are E2135.

## Accepted 2026-11-22: data, step 1 (owner)

- **D-090 (owner) Saved state: `persist state`.** "1 and 2 agreed":
  ERA stays a frontend tool and connects to backends later. Step 1 is
  `persist state name: type = value` in a store (the owner chose the
  keyword form). The value is restored at launch and saved on every
  change through the host hooks `eraLoad`/`eraSave` in `EraAssets.kt`
  (Android: private SharedPreferences, key `Store.field`; preview: in
  memory). No network is involved.
- **D-091 (owner) Mock data lists: `for p in Products { ... }`.** Step 2:
  every `Data/<name>.json` (a JSON array of flat objects) becomes a list
  named in PascalCase, bundled into the app as Kotlin data classes
  (`EraData.kt`). `for p in Products { ... }` (the owner chose `for …
  in`) repeats elements in place; `p.field` works like `Store.field`, and
  in `image`/`icon` every item's path is checked and packaged. Field
  types come from the JSON; mixed types, nested values and bad names are
  E2180. New dependency: `serde_json` in `era_ir`. Later steps (real
  APIs, forms) need network access and a separate owner decision. Also:
  a rounded `image` is now clipped to its corners, like a CSS `img`.

## Accepted 2026-11-25: ERA Studio becomes a visual builder (owner)

- **D-092 (owner) A Framer-style visual app builder, in ERA Studio on
  the Mac.** A canvas with every screen, layers and properties panels,
  direct manipulation (drag, resize, inline text), an insert panel and
  publishing, all kept in sync with the `.era` source both ways. It
  extends the M7 exception to the "no IDE" rule. Order (owner): first
  close the language gaps (data items in components, P5 text styling,
  P4c effects), then the builder in steps: live Rust canvas, inspect and
  edit panels, direct manipulation, design-system tools. ERA's own look,
  not a copy of Framer's interface.
- **D-093 Data items in components.** A component's `image`/`icon` prop
  takes a data item field (`photo: d.photo`, checked like `image
  d.photo`), and a `bool` prop takes live bool state (`hot: d.hot` or
  `Session.verified`), which works where a condition is read
  (`visible: hot`). A `number` prop cannot take a field yet; pass it to a
  text prop (`price: "${d.price}"`).

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
- **Q-04 (Resolved in M6: `.er`, see D-056)** **Logic file extension.**
- **Q-05 (Resolved and implemented in M4: Compose Multiplatform Desktop, plus `--png`)** **Preview technology.** The proposal is Compose Multiplatform
  Desktop running the same generated code, with a `--png` headless mode.
  The alternatives are a Rust-native renderer or an HTML mock, but both
  would drift from Android. Compose Multiplatform could also be an iOS
  path later; a SwiftUI backend would be the more "native" iOS option.
- **Q-07 (Resolved 2026-11-22, D-088)** **Gradients.** The E-Renty hero cards and buttons use gradients.
  The demo uses solid colors for now. A future skin token type could be
  `gradient`.
- **Q-08 Mixed styles within one sentence.** "Good morning, **Rehan**."
  and "Didn't get a code? **Resend**" are written as a `row` of `text`
  elements, which does not wrap like a sentence. Rich text spans would
  fix this.
- **Q-09 Overlap and offset positioning.** The raised "+" button in the
  bottom bar and the chart tooltip's pointer need offsets. The demo
  approximates both.
- **Q-10 (Partly resolved: dashed lines, D-087; italic is P5)** **Dashed lines and italic text.** The chart grid lines are
  dashed, and the wordmark is italic. The demo uses the logo as an image.
- **Q-11 (Partly resolved, D-050)** **System bars and safe areas.** The status bar and home indicator
  in the mockups are drawn by the OS. Backends must add safe-area insets.
- **Q-12 (Resolved 2026-11-22, D-085)** **App launcher icon.** The APK uses the system default icon. An
  `app` property such as `icon: "Images/app_icon.png"` could generate
  adaptive icons.
- **Q-06 (Resolved 2026-11-21: the owner installed the E-Renty APK on an Android phone, and it works, including the M6 flow)** **Emulator testing.** The dev
  container has no KVM, so no emulator. A JVM screenshot tool was tried
  (Paparazzi): 1.3.5's layout engine is too old for Android 15 APIs, and
  2.0-alpha needs a newer AGP than D-052. On-device install and launch
  must be done on the owner's machine or phone. Original question:
  **Emulator testing.** This cloud container probably lacks KVM. The
  "install and launch" step of M5 may need the owner's machine or a
  physical device.
