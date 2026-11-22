# Android host

`era build android` copies these templates into `<project>/build/android/`,
and `era export android` into a standalone folder (default
`<project>/export/android/`). Both fill in the placeholders:

| Placeholder | Value |
|---|---|
| `{{APP_ID}}` | `[project] id` from `era.toml` (namespace and applicationId) |
| `{{PACKAGE}}` | The generated Kotlin package, `<app id>.ui` |
| `{{PROJECT_NAME}}` | `[project] name` (Gradle root project name) |
| `{{MIN_SDK}}` | `[android] min_sdk` (default 24) |
| `{{VERSION_CODE}}` | `MAJOR*10000 + MINOR*100 + PATCH` |
| `{{VERSION_NAME}}` | `[project] version` |

It then adds the generated Compose code (the same files as the desktop
preview) and the Android resources: VectorDrawables converted from the SVG
icons, the icon id table and the app name. Images and fonts are copied to
`app/src/main/assets/`. The build then runs `gradle :app:assembleDebug`;
an export adds the wrapper scripts, `.gitignore`, a `README.md` and a
`.era-export` marker instead (D-084).

| File | Role |
|---|---|
| `settings.gradle.kts`, `build.gradle.kts`, `gradle.properties` | Root project with one `:app` module |
| `gradle/libs.versions.toml` | Version catalog. Versions are pinned (DECISIONS D-052) |
| `gradle/wrapper/`, `gradlew`, `gradlew.bat` | Stock Gradle 8.14.3 wrapper (from `gradle wrapper`); written by `era export` only |
| `gitignore.txt` | Written as `.gitignore` by `era export` |
| `app/build.gradle.kts` | The Android app module |
| `app/AndroidManifest.xml` | Launcher activity, app label, platform theme. No AppCompat or Material |
| `app/MainActivity.kt` | Edge-to-edge window, light/dark from the system, runs the app from the start screen |
| `app/EraAssets.kt` | Android `eraImage` (from `assets/`), `eraIcon` (VectorDrawable) and `eraFontFamily` (from `assets/`) |

You can open any screen directly for testing:

```bash
adb shell am start -n <app id>/<app id>.ui.MainActivity --es era.screen Home
```
