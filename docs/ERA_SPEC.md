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
```

The display name shown to users lives in `app "..."` in `src/app.era`,
because it is part of the visual design. Identity and packaging live in
`era.toml`.

## CLI (target behavior)

| Command | Milestone | Behavior |
|---|---|---|
| `era new <name>` | M1 ✅ | Create a minimal project: `era.toml`, `src/app.era`, `.gitignore` |
| `era check [path] [--ir]` | M1–M3 ✅ | Validate the manifest, project layout, syntax, structure, skins, properties, references and assets, then build the UI IR. `--ir` prints the resolved tree |
| `era preview [path] [--png [dir]]` | M4 ✅ | Generate Compose code and run the desktop preview: a window, or PNGs of every screen. Needs JDK 17+ and Gradle 8.x (see ANDROID_BACKEND.md) |
| `era build android [path]` | M5 ✅ | Generate Kotlin/Compose and the Android project, run `gradle assembleDebug`, and print the APK path and `adb` commands. Needs JDK 17+, Gradle 8.x and `ANDROID_HOME` |

Exit codes: `0` success, `1` project has errors, `2` usage or environment error.
