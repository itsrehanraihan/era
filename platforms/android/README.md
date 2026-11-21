# Android host

`era build android` copies these templates into `<project>/build/android/`
and fills in the placeholders:

| Placeholder | Value |
|---|---|
| `{{APP_ID}}` | `[project] id` from `era.toml` (namespace and applicationId) |
| `{{PACKAGE}}` | The generated Kotlin package, `<app id>.ui` |
| `{{MIN_SDK}}` | `[android] min_sdk` (default 24) |
| `{{VERSION_CODE}}` | `MAJOR*10000 + MINOR*100 + PATCH` |
| `{{VERSION_NAME}}` | `[project] version` |

It then adds the generated Compose code (the same files as the desktop
preview) and the Android resources: VectorDrawables converted from the SVG
icons, the icon id table and the app name. Images and fonts are copied to
`src/main/assets/`. Finally it runs `gradle assembleDebug`.

| File | Role |
|---|---|
| `settings.gradle.kts`, `build.gradle.kts`, `gradle.properties` | A single-module Android app. Versions are pinned (DECISIONS D-052) |
| `AndroidManifest.xml` | Launcher activity, app label, platform theme. No AppCompat or Material |
| `MainActivity.kt` | Edge-to-edge window, light/dark from the system, shows the start screen |
| `EraAssets.kt` | Android `eraImage` (from `assets/`), `eraIcon` (VectorDrawable) and `eraFontFamily` (from `assets/`) |

Until navigation exists (M6), you can open any screen for testing:

```bash
adb shell am start -n <app id>/<app id>.ui.MainActivity --es era.screen Home
```
