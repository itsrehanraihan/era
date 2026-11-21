# ERA Quick Start Guide

This guide takes you from nothing to a checked ERA design project in about
ten minutes.

> **What works today (M4):**
> - creating projects
> - writing skins, components, layouts and screens
> - validating everything with `era check` (helpful errors, "did you
>   mean" suggestions)
> - **previewing** the real native UI with `era preview`, as a window or
>   PNG screenshots
>
> - **building an Android APK** with `era build android`
>
> **Not yet:** taps, state and navigation (M6). The app opens on its start
> screen.

---

## 1. Install the prerequisites

You need **Rust 1.88 or newer** and **Git**. For `era preview` (step 10)
you also need **JDK 17+** and **Gradle 8.x**. For `era build android`
(step 11) you also need the **Android SDK**. None of these are needed
for `era check`.

```bash
rustc --version
```

If Rust is missing, install it from <https://rustup.rs>, then run:

```bash
rustup update
```

For previews, check Java and Gradle:

```bash
java -version
```

```bash
gradle --version
```

Install them from <https://adoptium.net> and
<https://gradle.org/install/> if they're missing. If Gradle isn't on your
`PATH`, point ERA at it with `ERA_GRADLE=/path/to/gradle`.

## 2. Get ERA and build the `era` command

```bash
git clone https://github.com/itsrehanraihan/era.git
```

```bash
cd era
```

```bash
cargo install --path crates/era_cli
```

This compiles ERA and puts the `era` command on your `PATH` (usually in
`~/.cargo/bin`). Check that it works:

```bash
era --version
```

You should see `era 0.0.1`.

> To use ERA without installing it, run `cargo build --release` and call
> `./target/release/era` instead of `era`.

## 3. Create your first project

```bash
era new my_app
```

```bash
cd my_app
```

`era new` creates three files:

```text
my_app/
├── era.toml        project name, id and version
├── .gitignore
└── src/
    └── app.era     your app and its first screen
```

## 4. Check it

```bash
era check
```

```text
Checked `my_app` (dev.era.my_app): 1 UI file(s), 0 skin file(s), 0 asset(s), 0 warning(s)
Declarations: 1 app, 1 screen, 0 layouts, 0 components, 0 skins
UI IR: 1 screen(s), 3 element(s), built-in default skin, 0 asset reference(s), start screen `Home`
```

Run `era check` after every change. It reports the file, line and column
of every problem, and suggests a fix when it can.

## 5. Read `src/app.era`

```era
app "My App" {
    start: Home
}

screen Home {
    title: "Home"

    column {
        padding: 24
        gap: 8
        text "Hello from ERA"
        text "Edit src/app.era to start designing."
    }
}
```

Everything in ERA has the same shape:

```era
name [value] {
    property: value
    child ...
}
```

- `app` sets the display name and the first screen.
- A `screen` has exactly one root element. Here it is a `column`.
- Lowercase names (`column`, `text`) are built-in elements.
  PascalCase names (`Home`, `AppButton`) are things you declare.
- Put one item per line, or separate items with `,`.

## 6. Give the app a look: add a Skin

A skin is your design system: colors, spacing, corner radii and text
styles. Create `Skins/theme.ers`:

```era
skin Studio {
    colors {
        background: #FFFFFF
        surface:    #F7F7F7
        text:       #171717
        muted:      #737373
        border:     #E8E8E8
        primary:    #FF6B35
        on_primary: #FFFFFF
        secondary:  #2B2D42
        success:    #2E9E5B
        warning:    #F2A93B
        danger:     #E5484D
    }
    spacing    { xs: 4, sm: 8, md: 16, lg: 24 }
    radius     { sm: 8, md: 14, pill: 999 }
    typography {
        heading { size: 28, weight: bold }
        body    { size: 16 }
    }
}
```

Then select it in `src/app.era`, and use **token names** instead of
numbers:

```era
app "My App" {
    skin: Studio
    start: Home
}

screen Home {
    title: "Home"

    column {
        padding: lg
        gap: md
        text "Hello from ERA" { style: heading }
        text "Designed with tokens" { color: muted }
    }
}
```

ERA looks each token up by the property's type: `padding: lg` finds a
spacing value, `color: muted` finds a color, and `style: heading` finds a
text style.

The 11 colors shown and the `body` text style are required. You can add
any extra tokens you like. A skin may also be split across several
`.ers` files.

## 7. Make a reusable component

Create `src/Components/AppButton.era`:

```era
component AppButton {
    prop label: text
    variant tone: solid | outline = solid

    button label {
        radius: pill
        when tone is outline {
            background: none
            border_width: 1
            border_color: primary
            text_color: primary
        }
    }
}
```

Use it in the screen:

```era
        AppButton { label: "Get started" }
        AppButton { label: "Learn more", tone: outline }
```

- A `prop` is a value the caller passes in. Props without a default are
  required.
- A `variant` is a controlled visual alternative. `when` changes
  properties for one option.
- A component can declare a `slot` to receive child elements.

## 8. Add an image

Put a file at `Assets/Images/hero.png` and use its path relative to
`Assets/`:

```era
        image "Images/hero.png" { width: 120, height: 120, radius: md, clip: true }
```

`era check` confirms the file exists and has a supported type: png, jpg
or webp for `image`, and svg for `icon`.

## 9. See what ERA understood

```bash
era check --ir
```

This prints the fully resolved UI tree, with every token, default and
component expanded:

```text
screen Home title="Home" background=background(#FFFFFF)
  column {gap=md(16) padding=lg(24)} @Home
    text "Hello from ERA" {color=text(#171717) style=heading} @Home/0
```

The preview and the Android build (M5) are both generated from this
same tree.

## 10. Preview your screens

Open a desktop window that shows every screen, with a dark-mode toggle
when your skin defines `colors dark`:

```bash
era preview
```

On a machine without a screen (a server or CI), render PNG screenshots
instead:

```bash
era preview --png
```

```text
era-preview: wrote …/my_app/build/preview/png/Home.png
Preview images are in …/my_app/build/preview/png
```

- The first run downloads Kotlin and Compose through Gradle, which takes
  a minute or two. Later runs take a few seconds.
- Each screen is rendered on a 402 × 874 phone canvas at 2×
  (804 × 1748 px).
- The preview runs **real native Compose code** generated from your ERA
  files, the same code the Android app will use. Look in
  `build/preview/src/` if you're curious.
- Run `era preview` again after every change to see the update. There is
  no hot reload yet.

## 11. Build the Android app

You need the **Android SDK** (platform 35 and build-tools). The easiest
way to get it is to install Android Studio. Point ERA at the SDK:

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
```

(On macOS it's usually `~/Library/Android/sdk`.) Then build:

```bash
era build android
```

```text
Built …/my_app/build/android/build/outputs/apk/debug/era-android-debug.apk (…)
```

Install it on a phone with USB debugging enabled, or on an emulator:

```bash
adb install -r build/android/build/outputs/apk/debug/era-android-debug.apk
```

The app opens on your `start` screen. To check another screen before
navigation exists, open it by name:

```bash
adb shell am start -n dev.era.my_app/dev.era.my_app.ui.MainActivity --es era.screen Home
```

The first build downloads the Android build tools, which takes a few
minutes. SVG icons are converted to Android vector drawables. If an icon
uses something unsupported, such as a transform or a gradient, ERA
explains what to change.

## 12. Explore the E-Renty demo

The repository includes a full four-screen demo built from real
mockups: Splash, Home, Menu and Verify code.

```bash
era check ../era/examples/erenty --ir
```

```bash
era preview ../era/examples/erenty --png
```

(Adjust the path to wherever you cloned `era`.) The demo is a good place
to see layouts, slots, variants, badges, a bar chart and a bottom
navigation bar written in ERA.

## 13. When something is wrong

Errors point at the exact spot and usually suggest a fix:

```text
error[E2120]: `column` has no property `gapp`
  --> src/app.era:11:9
   |
11 |         gapp: md
   |         ^^^^
   = help: did you mean `gap`?
```

Every code is explained in `docs/DIAGNOSTICS.md`.

| You see | Usually means |
|---|---|
| `E1006 unexpected px after number` | ERA numbers have no units; write `16`, not `16px` |
| `E1103 expected a new line or ,` | Two properties on one line need a `,` between them |
| `E1207 exactly one root element` | Wrap the screen's elements in a `column` |
| `E2132 unknown … token` | The skin doesn't define that token; check the suggestion |
| `E2145 missing required prop` | Pass the prop: `AppButton { label: "..." }` |
| `gradle was not found` (preview) | Install Gradle 8.x or set `ERA_GRADLE` |
| `no display found` (preview) | Use `era preview --png` |
| `Android SDK not found` | Install the SDK and set `ANDROID_HOME` |
| `E3001 cannot convert icon` | Simplify the SVG: flatten transforms, outline text, no gradients |

## 14. Syntax cheat sheet

| Want | Write |
|---|---|
| Vertical / horizontal layout | `column { }` / `row { }` |
| Overlap elements | `stack { align: center }` |
| Grid | `grid { columns: 3, gap: md }` |
| Scrolling | `scroll { column { ... } }` (exactly one child) |
| Surface with a shadow | `card { }` |
| Text | `text "Hi" { style: heading, color: muted }` |
| Button | `button "Go"`, or your own `AppButton { label: "Go" }` |
| Image / icon | `image "Images/a.png"` / `icon "Icons/a.svg" { size: 24 }` |
| Divider / flexible space | `divider` / `spacer` |
| Size | `width: fill`, `width: fit`, `width: 120` |
| Spacing | `padding`, `padding_x`, `padding_y`, `padding_top`, `gap` |
| Alignment | `align: center`, `distribute: between` |
| No color / no shadow | `background: none`, `elevation: none` |
| Comment | `// like this` |

## 15. Learn more

| Document | What's inside |
|---|---|
| `docs/GRAMMAR.md` | The complete syntax |
| `docs/UI_SPEC.md` | Every element, property and default |
| `docs/SKIN_SPEC.md` | Skins, tokens, fonts and the default skin |
| `docs/DIAGNOSTICS.md` | Every error code |
| `docs/ANDROID_BACKEND.md` | How ERA turns your design into native Compose code |
| `MILESTONES.md` | What's done and what's next |
| `DECISIONS.md` | Why ERA works the way it does |
