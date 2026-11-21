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

## Scripted taps (test aid, D-064)

Add a line to `era-preview.properties`, then run `gradle run`:

```properties
script=tap 201,437; tap 42,42; back
```

The script starts at the start screen and applies each step: `tap x,y`
in dp on the 402 × 874 canvas, or `back`. After each step it saves
`step-N.png` and prints `era-preview: step-N screen=<current>`.
