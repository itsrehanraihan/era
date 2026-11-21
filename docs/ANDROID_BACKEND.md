# Android Backend (PLAN, nothing implemented)

## Strategy

UI IR → generated Kotlin and Jetpack Compose → Gradle → debug APK.

- Generated code goes to `build/android/`. It is never edited by hand and
  is regenerated deterministically, with sorted output and no timestamps.
- A small host project template (Gradle files, `MainActivity`, manifest)
  lives in `platforms/android/` and is copied into the build directory.
- Skin tokens become a generated `EraTheme` object. Generated UI code
  refers to tokens by name (`EraTheme.colors.primary`), so the token
  structure survives into the native code.
- Assets are copied to `res/drawable*`, `res/font` and `assets/`, with
  names sanitized to Android resource rules. Every renaming is recorded so
  diagnostics can point back to the original ERA path.

## Planned primitive mapping

| ERA | Compose |
|---|---|
| `column` / `row` | `Column` / `Row` with `Arrangement` and `Alignment` |
| `stack` | `Box` |
| `grid` | `LazyVerticalGrid`, or a `Row` per chunk for small grids (decide in M5) |
| `scroll` | `Modifier.verticalScroll` / `horizontalScroll` |
| `card` | `Surface` with shape, elevation and color |
| `text` | `Text` with a `TextStyle` from the typography tokens |
| `image` | `Image(painterResource)` with `ContentScale` |
| `icon` | `Icon(painterResource)` from a generated VectorDrawable |
| `button` | `Button`, or a styled `Surface` plus `clickable`, so skins fully control the look |
| `input` | `BasicTextField` with a skin-styled decoration |
| `divider` / `spacer` | `HorizontalDivider` / `Spacer` |

## Preview proposal (M4, see DECISIONS Q-05)

Use the **same generated Compose code** on Compose Multiplatform
**Desktop**:

- `era preview` opens a desktop window.
- `era preview --png out/` renders each screen to PNG without a display.
  This also works in CI and in this cloud container.

Preview and Android share the codegen, so they cannot drift apart. The
cost is a heavier first Gradle download.

## Toolchain status (checked 2026-11-20 in the dev container)

| Tool | Status |
|---|---|
| Rust / Cargo | 1.97.0 ✅ |
| Java | OpenJDK 21 ✅ |
| Gradle | 8.14.3 ✅ (with Kotlin 2.0.21 for build scripts) |
| Android SDK, platform-tools, build-tools | ❌ missing (download hosts are reachable, install in M5) |
| `kotlinc` | ❌ missing (not needed, since Gradle supplies Kotlin) |
| Emulator / device | ❌ none. The emulator needs KVM; launch testing may require the user's machine |
