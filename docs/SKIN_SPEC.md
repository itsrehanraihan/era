# ERA Skins — Design Tokens

Status: approved. Implemented in M3 (`crates/era_ir/src/skin.rs`).

A **Skin** is a named set of typed design tokens. Skins live in
`Skins/*.ers`, and the app selects one with `skin: Name`.

## Example

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

    colors dark {          // P1: overrides for dark mode; same keys required
        background: #0F0F0F
        surface:    #1A1A1A
        text:       #F5F5F5
        // ...
    }

    spacing   { xs: 4, sm: 8, md: 16, lg: 24, xl: 32 }
    radius    { sm: 8, md: 14, lg: 22, pill: 999 }
    border    { thin: 1, thick: 2 }
    elevation { none: 0, sm: 1, md: 4, lg: 12 }

    typography {
        font: "Inter"      // resolved from Assets/Fonts/, or "system"
        display { size: 40, weight: bold,     line_height: 46, tracking: -0.5 }
        heading { size: 28, weight: bold,     line_height: 34 }
        title   { size: 20, weight: semibold, line_height: 26 }
        body    { size: 16, weight: regular,  line_height: 24 }
        caption { size: 13, weight: medium,   line_height: 18, tracking: 0.2 }
    }
}
```

## Token categories

| Section | Value type | Required keys | Notes |
|---|---|---|---|
| `colors` | color literal | `background surface text muted border primary on_primary secondary success warning danger` | You may add extra keys |
| `colors dark` | color literal | the same keys as `colors` | P1 |
| `spacing` | number ≥ 0 (dp) | none | Used by `padding*`, `gap` |
| `radius` | number ≥ 0 (dp) | none | `pill` is conventionally 999 |
| `border` | number ≥ 0 (dp) | none | Used by `border_width` |
| `elevation` | number ≥ 0 (dp) | none | Mapped to native shadow or tonal elevation |
| `typography` | `font` plus named text styles | `body` | Every style needs `size`; it may set `weight`, `line_height`, `tracking` |

`weight` accepts `thin light regular medium semibold bold black` or a
number from 100 to 900 in steps of 100.

## Splitting a skin across files

Several `.ers` files may each declare `skin Studio { ... }`. Their sections
are merged, which allows `colors.ers` and `typography.ers` to live
separately. Defining the same token twice is an error that cites both
locations.

## Fonts (D-032)

`font: "Inter"` needs at least one file named `Inter.ttf` or
`Inter-*.ttf` (or `.otf`) in `Assets/Fonts/`. For example,
`Inter-Regular.ttf` and `Inter-Bold.ttf` both count. `font: "system"`,
or no `font` at all, uses the platform font.

## Built-in default skin (D-020)

The default skin is used when `app` has no `skin:`. It defines:

| Category | Tokens |
|---|---|
| colors (light) | background #FFFFFF, surface #F5F5F5, text #1A1A1A, muted #6B6B6B, border #E0E0E0, primary #3D5AFE, on_primary #FFFFFF, secondary #37474F, success #2E7D32, warning #ED6C02, danger #D32F2F |
| colors dark | background #121212, surface #1E1E1E, text #F2F2F2, muted #A0A0A0, border #333333, primary #8C9EFF, on_primary #000000, secondary #B0BEC5, success #81C784, warning #FFB74D, danger #E57373 |
| spacing | xs 4, sm 8, md 16, lg 24, xl 32 |
| radius | sm 6, md 12, lg 20, pill 999 |
| border | thin 1, thick 2 |
| elevation | none 0, sm 1, md 4, lg 12 |
| typography | font `system`; display 36/700, heading 26/700, title 20/600, body 16/400, caption 13/500 |

## Validation performed by `era check` (M3)

- Color literal format is valid.
- Required keys are present.
- Light and dark key sets match.
- Numbers are within range.
- Font files exist.
- Every token referenced from UI code exists in the right category, with a
  "did you mean" suggestion when it does not.
- Token names are snake_case.
- Skins that the app does not use are validated too.
- A token written with an invalid value is reported once. It is not
  reported again as "missing".

Error codes E2001–E2010 are listed in `docs/DIAGNOSTICS.md`.
