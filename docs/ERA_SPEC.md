# ERA Overview Spec (DRAFT)

ERA is a design-first language and toolchain for native apps. The
compiler and CLI are written in Rust. The MVP targets Android by
generating Kotlin and Jetpack Compose. ERA is **not** a WebView
framework, and in this MVP Rust does **not** render UI at runtime.

## Project layout

A minimal project needs only two files:

```text
my_app/
├── era.toml
└── src/app.era
```

The full convention:

```text
my_app/
├── era.toml
├── Skins/            *.ers  design tokens (auto-loaded)
├── Data/             *.json mock data lists (D-091): products.json -> Products
├── Assets/
│   ├── Images/       png, jpg, webp
│   ├── Icons/        svg (subset; see open questions)
│   ├── Fonts/        ttf, otf
│   └── Files/        bundled raw files
└── src/
    ├── app.era       entry point (required)
    ├── Screens/      *.era  screen declarations
    ├── Components/   *.era  component declarations
    ├── Layout/       *.era  layout declarations
    └── Logic/        *.er   stores: app state and actions (M6)
```

### File extensions

| Ext | Contents |
|---|---|
| `.era` | UI declarations: app, screen, layout, component |
| `.ers` | Skin declarations only |
| `.er` | Logic: `store` declarations only (D-056). Usually in `src/Logic/` |

## `era.toml`

```toml
[project]
name    = "showcase"           # package/folder name, snake_case
id      = "dev.era.showcase"   # reverse-DNS; Android applicationId
version = "0.1.0"

[era]
edition = "2026"               # language edition, for future syntax changes

[android]                      # optional; defaults shown
min_sdk = 24

[android.signing]              # optional; needed for `era build android --release`
keystore = "keys/upload.jks"   # relative to the project folder
alias    = "upload"
```

Passwords are never stored in `era.toml`. `--release` reads
`ERA_KEYSTORE_PASSWORD` (and `ERA_KEY_PASSWORD` if the key has its own
password). Create a key once with:

```bash
keytool -genkeypair -v -keystore keys/upload.jks -alias upload -keyalg RSA -keysize 2048 -validity 10000
```

Keep the keystore and its password safe and out of version control:
Google Play needs the same key for every update.

The display name shown to users lives in `app "..."` in `src/app.era`,
because it is part of the visual design. Identity and packaging live in
`era.toml`.

## CLI (target behavior)

| Command | Milestone | Behavior |
|---|---|---|
| `era new <name>` | M1 ✅ | Create a minimal project: `era.toml`, `src/app.era`, `.gitignore` |
| `era check [path] [--ir]` | M1–M3 ✅ | Validate the manifest, project layout, syntax, structure, skins, properties, references and assets, then build the UI IR. `--ir` prints the resolved tree |
| `era preview [path] [--png [dir]]` | M4 ✅ | Generate Compose code and run the desktop preview: a window, or PNGs of every screen. Needs JDK 17+ and Gradle 8.x (see ANDROID_BACKEND.md) |
| `era build android [path] [--release]` | M5 ✅, `--release` M9 ✅ | Generate Kotlin/Compose and the Android project in `build/android/`, run `gradle :app:assembleDebug`, and print the APK path (`build/android/app/build/outputs/apk/debug/app-debug.apk`) and `adb` commands. `--release` builds a signed, R8-optimised APK and an App Bundle (`app/build/outputs/apk/release/app-release.apk`, `app/build/outputs/bundle/release/app-release.aab`) using `[android.signing]` (D-086). Needs JDK 17+, Gradle 8.x and `ANDROID_HOME` |
| `era export android [path] [--out dir] [--force]` | M9 ✅ | Write a standalone Android Studio project (default `export/android/`) with the Gradle wrapper, a version catalog, an `app/` module and clean, editable Kotlin. A non-empty folder is refused (E3003); `--force` replaces only an earlier ERA export. Needs no Gradle; building it needs JDK 17+ and the Android SDK (D-084) |
| `era export ios` | later | Prints that SwiftUI output is planned (D-082) |

Exit codes: `0` success, `1` project has errors, `2` usage or environment error.
