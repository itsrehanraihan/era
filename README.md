# ERA

**Build beautiful native applications with the simplicity of visual web
development and the engineering foundation of Rust.**

ERA is a design-first language and toolchain for native apps. The
compiler is written in Rust and generates native Android UI (Kotlin and
Jetpack Compose), with no WebView.

> **Status: M3 complete.** `era new` and `era check` work. `era check`
> validates the whole design project (manifest, layout, syntax, skins,
> properties, components and assets) and builds a platform-independent
> UI IR (`era check --ir` prints it). Preview (M4) and Android builds (M5)
> are not implemented yet.

**New here? Start with [Quick_start_guide.md](Quick_start_guide.md).**

Examples:
- `examples/erenty/`: the first demo, a four-screen rental app (Splash,
  Home, Menu, Verify code) built from real mockups.
- `examples/showcase/`: a compact tour of the design system, also used
  as a test fixture.

See `MILESTONES.md`, `DECISIONS.md` and `docs/`.

Copyright © 2026 the ERA authors. All rights reserved. Proprietary and
confidential.
