# Compose Backend (preview: M4 ✅, Android APK: M5 ✅ built, on-device check pending)

## Strategy

```text
UI IR ──► era_codegen_compose ──► Kotlin + Compose (foundation only)
                                    ├─► desktop preview host  (M4, Compose Multiplatform)
                                    └─► Android host          (M5, Jetpack Compose)
```

The **same generated files** compile for both hosts, so the preview and
the Android app cannot drift apart. The generated code uses only Compose
`foundation` and `ui`, not Material, so the skin fully controls the look
(D-047).

## Generated files (`crates/era_codegen_compose`)

The package is `<app id>.ui`, for example `dev.era.erenty.ui` (D-045).

| File | Contents |
|---|---|
| `EraTheme.kt` | `EraColors` with light and dark palettes, `EraSpacing`, `EraRadius`, `EraBorder`, `EraElevation` (dp), `EraType` text styles, `EraFont`, `EraTheme(dark) { }` |
| `EraRuntime.kt` | `EraScreen`, `EraInput` (a text field that keeps its own text until M6) |
| `EraScreens.kt` | `EraAppName`, `EraStartScreen`, and the `EraScreens` registry |
| `screens/<Name>Screen.kt` | One `@Composable fun <Name>Screen()` per screen |

Token references stay symbolic: `c.primary`, `EraSpacing.md`,
`EraType.heading`. Literal values become `Color(0xAARRGGBB)` and `16.dp`.
Each element is preceded by a comment with its IR id and origin, so
generated code can be traced back to ERA source. Output is
deterministic, and both examples are snapshot-tested.

## Platform host API (D-044)

Each host implements three functions in the generated package:

| Function | Desktop preview | Android |
|---|---|---|
| `@Composable eraImage(path): Painter` | `loadImageBitmap` from `Assets/` | `BitmapFactory` from the APK's `assets/` (D-053) |
| `@Composable eraIcon(path): Painter` | `loadSvgPainter` from `Assets/` | `painterResource` of the converted VectorDrawable (D-051) |
| `eraFontFamily(files): FontFamily` | `Font(File, weight)`, with the weight taken from the file name | `Font(path, assets, weight)` |

## Element mapping

| ERA | Compose |
|---|---|
| `column`, `card` | `Column` (`card` adds its skin defaults: surface, radius, padding, shadow) |
| `row` | `Row` |
| `stack` | `Box(contentAlignment = …)` |
| `grid` | A `Column` of `Row`s, with each cell `weight(1f)` and the last row padded |
| `scroll` | `Box(Modifier.verticalScroll / horizontalScroll)` |
| `text` | `BasicText` with an `EraType` style copy (color, align, maxLines + ellipsis) |
| `button` | `Box(contentAlignment = Center) { BasicText }`. No click handling until M6 |
| `input` | `EraInput` (`BasicTextField` with a placeholder) |
| `image`, `icon` | `Image(eraImage/eraIcon, contentScale / ColorFilter.tint)` |
| `divider` | A `Box` with full width, the thickness as height, and the color as background |
| `spacer` | `Spacer(weight(1f))` in a row or column, or `Spacer(size)` |

## Layout rules

- **`fill` (D-041).** Along a row or column's main axis, `fill` becomes
  `weight(1f)`, so siblings share the free space equally. Across the
  cross axis it becomes `fillMaxWidth/Height()`.
- **`fill` along a scrolling axis is ignored (D-048).** That space is
  infinite, and Compose would crash on a weight there.
- **`align: stretch`** gives children without an explicit size
  `fillMax…` on the cross axis.
- **`distribute: start/center/end` with `gap`** becomes
  `Arrangement.spacedBy(gap, alignment)`. `between/around/evenly` use
  `SpaceBetween/Around/Evenly`, and `gap` is ignored for them.
- **Modifier order:** size → shadow → clip → background → border →
  safe-area insets (screen root only, D-050) → scroll → padding.
- **The screen root fills the screen** unless it is sized (D-043).

## Desktop preview (`era preview`, M4)

`platforms/desktop-preview/` is copied into `<project>/build/preview/`,
with the generated files added. Gradle then runs it.

| Pin (D-042) | Version |
|---|---|
| Kotlin (+ compose compiler plugin) | 2.1.21 |
| Compose Multiplatform | 1.8.2 |
| Gradle | 8.x on `PATH`, or `ERA_GRADLE` |
| JDK | 17+ (tested with 21) |

- `era preview --png [dir]` renders every screen at 402 × 874 dp, 2×
  (804 × 1748 px), without a display. When the skin has `colors dark`,
  it also renders `<Screen>-dark.png`.
- `era preview` opens a window with a screen list and a dark-mode
  toggle. It needs a display.
- Only changed files are rewritten, so Gradle's up-to-date checks work.
  Stale screens are deleted.

## Android (`era build android`, M5)

`platforms/android/` is copied into `<project>/build/android/`, together
with the generated Compose files and the Android resources. Gradle then
runs `assembleDebug`.

| Pin (D-052) | Version |
|---|---|
| Android Gradle Plugin | 8.9.3 |
| compileSdk / targetSdk | 35 |
| minSdk | `[android] min_sdk` in `era.toml` (default 24) |
| Compose | BOM 2025.05.01 (`foundation`, `ui`) |
| activity-compose | 1.10.1 |
| Kotlin | 2.1.21, JVM target 17 |
| SDK location | `ANDROID_HOME` or `ANDROID_SDK_ROOT`, written to `local.properties` |

- **Assets (D-053):** images and fonts go to `src/main/assets/<path>`.
  SVG icons go to `res/drawable/era_icon_*.xml` (D-051), and
  `EraIconIds.kt` maps ERA paths to resource ids.
- **App:** `MainActivity` runs edge to edge and picks light or dark from
  the system (when the skin has dark colors). It shows `EraStartScreen`,
  or the screen named by the `era.screen` intent extra (D-054). The app
  label comes from `app "..."`. The version code is
  `MAJOR*10000 + MINOR*100 + PATCH`.
- **Output:** `build/android/build/outputs/apk/debug/era-android-debug.apk`,
  signed with the debug key (D-055).
- **Verification here:** APK contents and signature (`aapt2`,
  `apksigner`). There is no emulator (no KVM), and a JVM screenshot tool
  (Paparazzi) was tried and was incompatible with these versions (Q-06).

## Toolchain status (dev container, 2026-11-21)

| Tool | Status |
|---|---|
| Rust 1.97, JDK 21, Gradle 8.14.3 | ✅ |
| Maven Central, Google Maven, Gradle plugin portal | ✅ reachable |
| Compose Desktop headless rendering | ✅ verified (`era preview --png`) |
| Android SDK (cmdline-tools, platform 35, build-tools 35.0.0, platform-tools) | ✅ installed at `/opt/android-sdk` in the dev container (not persistent) |
| Android emulator | ❌ no KVM in the container |
