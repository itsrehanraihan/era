# ERA

**Build beautiful native applications with the simplicity of visual web
development and the engineering foundation of Rust.**

ERA is a design-first language and toolchain for native apps. The
compiler is written in Rust and generates native Android UI (Kotlin and
Jetpack Compose), with no WebView.

> **Status: M5, APK builds.** `era new`, `era check`, `era preview` and
> `era build android` work. ERA validates the whole design project,
> builds a platform-independent UI IR, and generates native Jetpack
> Compose code from it. That code runs in a desktop preview (window or
> PNGs) and is packaged as an Android debug APK. The APK has not yet
> been checked on a device. Interactions and navigation (M6) are next.

![E-Renty demo rendered by `era preview --png`](docs/images/erenty-preview.png)

**New here? Start with [Quick_start_guide.md](Quick_start_guide.md).**

Examples:
- `examples/erenty/`: the first demo, a four-screen rental app (Splash,
  Home, Menu, Verify code) built from real mockups.
- `examples/showcase/`: a compact tour of the design system, also used
  as a test fixture.

See `MILESTONES.md`, `DECISIONS.md` and `docs/`.

Copyright © 2026 the ERA authors. All rights reserved. Proprietary and
confidential.
