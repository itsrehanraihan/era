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
    └── Logic/        reserved for M6 (state and events)
```

### File extensions

| Ext | Contents |
|---|---|
| `.era` | UI declarations: app, screen, layout, component |
| `.ers` | Skin declarations only |
| logic | **Undecided**; deferred to M6 (see DECISIONS Q-04) |

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
| `era preview` | M4 | Render the screens from the same UI IR (see ANDROID_BACKEND.md) |
| `era build android` | M5 | Generate Kotlin/Compose, run Gradle, produce a debug APK |

Exit codes: `0` success, `1` project has errors, `2` usage or environment error.
