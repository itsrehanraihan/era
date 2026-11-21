# Desktop preview host

`era preview` copies these templates into `<project>/build/preview/` and
replaces `{{PACKAGE}}` with the app's Kotlin package (`<app id>.ui`). It
then adds the generated theme and screens, and runs Gradle.

| File | Role |
|---|---|
| `settings.gradle.kts`, `build.gradle.kts` | Compose Multiplatform Desktop build. Versions are pinned (DECISIONS D-042) |
| `EraAssets.kt` | Desktop implementations of `eraImage`, `eraIcon` and `eraFontFamily` (the platform host API) |
| `PreviewMain.kt` | Opens a window (screen list, dark mode toggle), or renders every screen to PNG without a display |

The phone canvas is 402 × 874 dp, rendered at 2×. That gives
804 × 1748 px PNGs, the same size as the design mockups.
