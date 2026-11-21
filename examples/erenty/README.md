# E-Renty: the first ERA demo

This is a four-screen electric-bike rental app, written in ERA from the
owner's mockups.

| Screen | File | What it shows |
|---|---|---|
| Splash | `src/Screens/Splash.era` | A full-bleed brand color and a centered logo |
| Home | `src/Screens/Home.era` | Greeting, horizontal alert cards, rental card with status pill and buttons, distance bar chart with tooltip, bottom navigation |
| Menu | `src/Screens/Menu.era` | Profile card, illustrated shortcut grid with a badge, settings list with dividers, sign out |
| Verify code | `src/Screens/VerifyCode.era` | Illustration, code boxes (focused and idle variants), resend line, disabled button |

Check it from the repository root:

```bash
era check examples/erenty --ir
```

## Structure

- `Skins/`: the `Renty` skin. Colors were sampled from the mockups, and
  there are 13 text styles.
- `src/Layout/Page.era`: a scrolling page with a back button and an
  optional background `tint`.
- `src/Components/`:
  - `AppButton` (solid, outline and disabled variants)
  - `IconTile`, `MenuTile` (with a badge slot), `Badge`
  - `SettingsRow`, `HeroCard` (green and grey variants), `StatusPill`
  - `Bar` and `Tooltip` (the chart)
  - `NavItem` (idle and active variants)
  - `OtpGroup` (idle and focused variants)

## Assets

- `Assets/Images/*.png`: the logo, the 3D illustrations and the bike
  photo were **cropped from the mockup screenshots**, so they are low
  resolution. Replace them with the original exports, keeping the same
  file names.
- `Assets/Icons/*.svg`: simple line icons drawn for this demo.

## Known differences from the mockups

These come from features ERA doesn't have yet (see `DECISIONS.md`):

- Gradients (hero cards, the "Show pickup QR" button) are solid colors
  (Q-07).
- Mixed-style sentences ("Good morning, **Rehan**.") are a row of text
  elements (Q-08).
- The raised "+" button and the tooltip's pointer are approximated (Q-09).
- The dashed chart grid lines are omitted (Q-10).
- The status bar and home indicator belong to the OS (Q-11).
- Screens don't navigate yet. Navigation and button actions arrive in M6.
- Nothing can be rendered yet. Preview arrives in M4 and Android in M5.
