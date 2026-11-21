# ERA

**Build beautiful native applications with the simplicity of visual web
development and the engineering foundation of Rust.**

ERA is a design-first language and toolchain for native apps. The
compiler is written in Rust and generates native Android UI (Kotlin and
Jetpack Compose), with no WebView.

> **Status: M7.** `era new`, `era check`, `era preview` and
> `era build android` work, and **ERA Studio** (`era-studio`) is the
> desktop editor, with live problems as you type, a real Compose preview
> and one-click APK builds. Apps support taps, state and navigation, and
> the APK has been verified on an Android phone.

![ERA Studio](docs/images/era-studio.png)

![E-Renty demo rendered by `era preview --png`](docs/images/erenty-preview.png)

**New here? Start with [Quick_start_guide.md](Quick_start_guide.md).** Prefer an editor? See [docs/STUDIO.md](docs/STUDIO.md).

Examples:
- `examples/erenty/`: the first demo, a four-screen rental app (Splash,
  Home, Menu, Verify code) built from real mockups.
- `examples/showcase/`: a compact tour of the design system, also used
  as a test fixture.

Coming next: CSS-familiar styling, with `fontSize: 18px` and `boxShadow: 0 4px 12px rgba(0,0,0,.1)` written as on the web (`docs/CSS_SPEC.md`, under review).

See `MILESTONES.md`, `DECISIONS.md` and `docs/`.

Copyright © 2026 the ERA authors. All rights reserved. Proprietary and
confidential.
