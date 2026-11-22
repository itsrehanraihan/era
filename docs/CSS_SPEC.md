# ERA × CSS: the CSS-familiar styling model

Status: **approved (D-080: the MVP follows HTML/CSS). Phase 1 is
implemented** (names, units, sizing, box spacing, positioning; see §15).
The owner's v2 input sheet is `docs/V2_SYNTAX_CHEATSHEET.md`.
It records the owner's 2026-11-21 direction: ERA should feel like
HTML5/CSS3. ERA should be easy to understand, take little code, be highly
customisable, and turn any UI into code. The sections marked **Decision**
need the owner's yes or no before implementation (DECISIONS D-071 to
D-079).

This replaces the brief's "avoid copying the entire CSS specification"
(scope change D-071). ERA still compiles to **native** Compose, never a
WebView. CSS features that only make sense in a web browser get an ERA
equivalent and a helpful error that points to it (§11).

---

## 1. What it looks like

```era
screen Checkout {
    column {
        padding: 24px 16px
        gap: 12px
        background: linear-gradient(180deg, #F7FAF7, #FFFFFF)

        text "Order summary" { fontSize: 28px; fontWeight: 600; letterSpacing: -0.4px }

        row {
            justifyContent: space-between
            alignItems: center
            text "Total" { color: muted }
            text "40 000 Ft" { fontWeight: bold; fontVariantNumeric: tabular-nums }
        }

        grid {
            gridTemplateColumns: repeat(3, 1fr)
            gap: 8px
            image "Images/a.png" { aspectRatio: 1; borderRadius: 12px; objectFit: cover }
            image "Images/b.png" { aspectRatio: 1; borderRadius: 12px; objectFit: cover }
            image "Images/c.png" { aspectRatio: 1; borderRadius: 12px; objectFit: cover }
        }

        button "Pay now" {
            width: 100%
            padding: 14px 24px
            border: none
            borderRadius: 999px
            background: linear-gradient(135deg, #00E6B5, #00B894)
            color: #143132
            boxShadow: 0 6px 16px rgba(0, 230, 181, 0.35)
            onPress: Checkout.pay then go Done
        }
    }
}
```

A web developer can read every line. The CSS values can be copied from
DevTools or Figma's "Copy as CSS".

---

## 2. Names (Decision D-072)

- **camelCase is canonical**, as in JavaScript and React styles:
  `fontSize`, `borderRadius`, `justifyContent`, `boxShadow`.
- **kebab-case is accepted when you paste.** `font-size` is read as
  `fontSize`, so blocks copied from a `.css` file work. `era fmt` rewrites
  them to camelCase.
- **ERA's current snake_case names** (`padding_x`, `text_color`,
  `on_press`, `max_lines`, `border_width`, ...) are still accepted for one
  release with a deprecation warning. `era fmt --fix` migrates them (see
  the table in §12).
- **Short names that are already CSS stay as they are:** `gap`,
  `padding`, `background`, `color`, `opacity`, `width`, `height`.
- **ERA-only conveniences** (not in CSS) are kept, in camelCase:
  `paddingX`, `paddingY`, `marginX`, `marginY`, `size` (icons),
  `textStyle` (a typography token), `onPress`, `visible`, `bind`.

## 3. Values (Decision D-073)

| CSS value | Example | ERA meaning |
|---|---|---|
| Number | `16`, `0.5` | A plain number. For lengths it means dp (unchanged) |
| `px` | `16px` | **= 16 dp.** A CSS px is a density-independent unit, like Android dp |
| `dp`, `pt` | `16dp`, `12pt` | dp; pt = 1.333 dp (for designs specified in points) |
| `%` | `50%` | A percentage of the parent's content box (width/height), or of the element's own size (`borderRadius: 50%`, `translate`) |
| `em`, `rem` | `1.5rem` | rem = the root font size (the skin's `body` size, 16 by default); em = the current text size |
| `vw`, `vh`, `vmin`, `vmax` | `100vh` | A percentage of the screen (window on desktop) |
| `fr` | `1fr 2fr` | A fraction of the free space, in grids and in row/column children |
| `auto`, `fit-content`, `min-content`, `max-content` | | Content-sized (see §4) |
| `deg`, `rad`, `turn` | `45deg` | Angles (transforms, gradients) |
| `s`, `ms` | `200ms` | Durations, **reserved**: animations are out of the MVP (§11) |
| Colours | `#0FA`, `#00FFAA80`, `rgb(0 255 170)`, `rgba(0,0,0,.1)`, `hsl(160 100% 50%)`, `red`, `transparent`, `currentColor` | All 148 CSS named colours, plus skin tokens (`primary`) |
| `var(--name)` | `var(--primary)` | The skin token `name` (copy-paste friendly) |
| Gradients | `linear-gradient(...)`, `radial-gradient(...)`, `conic-gradient(...)` | Native brushes |
| `url("...")` | `url("Images/bg.png")` | An asset path (validated like `image`) |
| `calc()`, `min()`, `max()`, `clamp()` | `calc(100% - 32px)` | **Phase 6.** Evaluated at build time when possible, at layout time otherwise |
| Keywords | `space-between`, `flex-start`, `sans-serif`, `ease-in-out` | Hyphenated keywords are allowed in values |

Shorthands work as in CSS:
- `padding` and `margin` take 1–4 values
- `border` / `borderTop…` take width, style and colour
- `borderRadius` takes 1–4 values
- `boxShadow` and `background` take several layers
- `flex` takes `1` or `1 1 0`
- `gap` takes row and column gaps
- `inset` takes 1–4 values
- `font` comes in phase 5

### Separators (Decision D-074, a breaking change)

CSS values contain commas (`rgba(0, 0, 0, .1)`, two shadows separated by
`,`). Today ERA uses `,` to separate items on one line. **Proposal:** use
`;` between items on one line, as CSS does, and keep `,` only inside
values. Newlines still separate items, so most files don't change.
`era fmt --fix` rewrites old one-liners such as `{ gap: md, padding: lg }`
to `{ gap: md; padding: lg }`.

## 4. Sizing: every mode (owner requirement)

`width` and `height` (and `minWidth`, `maxWidth`, `minHeight`,
`maxHeight`) accept:

| Value | Meaning | Native (Compose) |
|---|---|---|
| `120px` / `120` | Fixed size | `width(120.dp)` |
| `50%` | A share of the parent's content box | `fillMaxWidth(0.5f)` |
| `fill` / `100%` / `stretch` | Fill the parent; in a row or column along the main axis, share the free space | `fillMax…` / `weight(1f)` |
| `2fr` | In a row or column: take 2 shares of the free space (siblings with `1fr` take 1) | `weight(2f)` |
| `fit` / `fit-content` / `auto` | Hug the content (default) | wrap content |
| `min-content` / `max-content` | The narrowest / widest the content can be | `width(IntrinsicSize.Min/Max)` |
| `100vw` / `50vh` | A share of the screen | from the window size |
| `calc(100% - 32px)` | Mixed (phase 6) | custom layout |

Also included:
- `aspectRatio: 16 / 9` → `Modifier.aspectRatio`
- `flex: 1`, `flexGrow: 2` → weight
- `flexShrink` and `flexBasis`, partially (§6)
- `boxSizing` is always `border-box`, so `width` includes padding and
  border, matching how designers think

## 5. Box model

| CSS | ERA | Native |
|---|---|---|
| `padding`, `paddingTop/Right/Bottom/Left`, `paddingInline`, `paddingBlock` | ✅ | `padding` |
| `margin`, `marginTop/…`, `marginInline`, `marginBlock`, `margin: 0 auto` | ✅ **new** | outer padding; `auto` centres in the parent |
| `border`, `borderTop/Right/Bottom/Left`, `borderWidth`, `borderColor`, `borderStyle: solid / dashed / dotted / none` | ✅ per side, dashed included (fixes Q-10) | `border` / custom draw |
| `borderRadius`, `borderTopLeftRadius`, … (1–4 values, `%`) | ✅ | `RoundedCornerShape` |
| `outline`, `outlineOffset` | ✅ | custom draw |
| `boxShadow` (offset, blur, spread, colour, `inset`, several) | ✅ | Compose `dropShadow` / `innerShadow` (needs Compose 1.9, §13) |
| `opacity` | ✅ | `alpha` |
| `visibility: hidden` | ✅ keeps its space | `alpha(0)` |
| `display: none` | ✅ | not rendered (same as `visible: false`) |
| `overflow: hidden / visible / scroll / auto`, `overflowX/Y` | ✅ | `clip` / scroll |

## 6. Layout

ERA's layout elements stay (`column`, `row`, `stack`, `grid`, `scroll`),
and gain the CSS layout properties:

| CSS | ERA | Native |
|---|---|---|
| `display: flex` + `flexDirection: row / column / row-reverse / column-reverse` | ✅ any container can be a flexbox | `Row` / `Column` (reversed order) |
| `justifyContent: flex-start / center / flex-end / space-between / space-around / space-evenly` | ✅ | `Arrangement` |
| `alignItems` / `alignSelf: flex-start / center / flex-end / stretch / baseline` | ✅ | `Alignment` / `align` / baseline |
| `alignContent`, `flexWrap: wrap` | ✅ | `FlowRow` / `FlowColumn` |
| `gap`, `rowGap`, `columnGap` | ✅ | `spacedBy` |
| `order` | ✅ | reordered at build time |
| `display: grid`, `gridTemplateColumns: 1fr 2fr 120px auto`, `repeat(3, 1fr)`, `minmax()` | ✅ | ERA grid layout (runtime) |
| `gridTemplateRows`, `gridColumn: span 2`, `gridRow`, `gridArea`, `gridTemplateAreas` | ✅ (areas in phase 6) | ERA grid layout |
| `position: relative` + `top/right/bottom/left` | ✅ | `offset` |
| `position: absolute` + `inset` / `top/right/bottom/left` | ✅ relative to the nearest `relative`/`stack` ancestor | parent becomes a `Box` overlay plus `align` + `offset` |
| `position: fixed` | ✅ pinned to the screen (e.g. a floating button) | the screen-level overlay |
| `position: sticky` | ⏳ later (needs a lazy list) | `stickyHeader` |
| `zIndex` | ✅ | `zIndex` |
| `float`, `clear`, `display: inline / inline-block / table` | ❌ web-only → use `row` + `flexWrap: wrap`, or `grid` (§11) | |

This also fixes Q-09 (the raised "+" button, the tooltip pointer): it's
`position: absolute` with a negative `top`.

## 7. Backgrounds and colours

| CSS | ERA | Native |
|---|---|---|
| `background` / `backgroundColor` | ✅ colour, token, gradient, `url()`, several layers | `background(Brush)` |
| `linear-gradient(angle or direction, stops…)` | ✅ (fixes Q-07) | `Brush.linearGradient` |
| `radial-gradient(…)` | ✅ | `Brush.radialGradient` |
| `conic-gradient(…)` | ✅ | `Brush.sweepGradient` |
| `backgroundImage`, `backgroundSize: cover / contain / 50%`, `backgroundPosition`, `backgroundRepeat: no-repeat / repeat` | ✅ (repeat in phase 6) | `Image` behind the content |
| `color` | ✅ text and icon colour, and inherited by children like CSS (the one inherited property, besides font properties) | `LocalContentColor` |
| `accentColor`, `caretColor` | ✅ | inputs |

## 8. Typography

| CSS | ERA | Native |
|---|---|---|
| `fontFamily` (with fallbacks), `fontSize`, `fontWeight` (100–900 / `bold`), `fontStyle: italic` | ✅ italic fixes Q-10 | `TextStyle` |
| `lineHeight` (number = a multiple, or a length), `letterSpacing`, `wordSpacing` | ✅ (word spacing approximated) | `TextStyle` |
| `textAlign`, `textDecoration: underline / line-through` (+ colour, style), `textTransform: uppercase / lowercase / capitalize` | ✅ | `TextStyle` / done at build time |
| `textOverflow: ellipsis`, `whiteSpace: nowrap / pre`, `lineClamp` | ✅ | `maxLines`, `softWrap`, `overflow` |
| `textShadow` | ✅ (one shadow) | `TextStyle.shadow` |
| `fontVariantNumeric: tabular-nums`, `fontFeatureSettings` | ✅ | `fontFeatureSettings` |
| `userSelect: text` | ✅ selectable text | `SelectionContainer` |
| Inline spans (`<span>`, `<b>`, `<a>` inside a paragraph) | ✅ `span` elements inside `text` (fixes Q-08) | `AnnotatedString` |
| Inheritance | Font and colour properties inherit from parents, as in CSS | composition locals |

```era
text {
    span "Good morning, "
    span "Rehan" { color: accent }
    span "."
}
```

## 9. Effects and transforms

| CSS | ERA | Native |
|---|---|---|
| `transform: translate() rotate() scale()` (+ X/Y variants), `transformOrigin` | ✅ | `graphicsLayer` |
| `transform: skew()`, `matrix()` | ⏳ phase 6 | custom matrix |
| `filter: blur() brightness() contrast() grayscale() saturate() opacity() drop-shadow()` | ✅ (blur on Android 12+; other filters are colour matrices) | `blur` / `ColorFilter` |
| `backdropFilter: blur()` | ⏳ later (Android 12+ only, costly) | `RenderEffect` |
| `mixBlendMode` | ✅ common modes | `BlendMode` |
| `clipPath: circle() / ellipse() / inset() / polygon()` | ✅ | `clip(Shape)` |
| `objectFit` / `objectPosition` (images) | ✅ | `ContentScale` / `Alignment` |

## 10. Interaction states and responsiveness (Decision D-075, not answered yet)

Proposed, the CSS way but scoped to the element (no cascade):

```era
button "Save" {
    background: primary
    hover { background: #00CFA2 }        // desktop / mouse
    active { transform: scale(0.97) }    // while pressed (:active)
    focus { outline: 2px solid accent }  // keyboard focus (:focus-visible)
    disabled { opacity: 0.4 }            // when `disabled: Session.busy`
}

column {
    gridTemplateColumns: 1fr
    media (minWidth: 600px) { gridTemplateColumns: 1fr 1fr }   // @media
}
```

Also proposed: reusable **style classes** in skins
(`style card { … }` used with `class: card elevated`), as an alternative
to CSS selectors. There are no complex selectors or specificity rules,
which keeps results predictable on native. **The owner hasn't decided
this yet.**

## 11. HTML5 elements (Decision D-076)

ERA's element names stay. The proposal also accepts familiar **HTML
names as aliases**, so HTML-shaped thinking works:

| HTML | ERA | Notes |
|---|---|---|
| `div`, `section`, `header`, `footer`, `nav`, `main`, `article`, `aside` | `column` (with an accessibility role for the semantic ones) | `display: flex` row with `flexDirection: row` |
| `span`, `p`, `h1`…`h6`, `strong`, `em`, `small` | `text` / `span` with a default text style (`h1` = the `display` style, …) | |
| `img`, `picture`, `svg` | `image` / `icon` | |
| `button`, `a` | `button`, `link` (`href: go Screen` or an `https://` URL that opens the browser) | |
| `input type=text/email/password/number/tel/search/url` | `input { type: password }` | keyboard type and masking |
| `textarea` | `input { multiline: true }` / `textarea` | |
| `input type=checkbox / radio / range / date / time / color` | `checkbox`, `radio`, `switch`, `slider`, `datePicker`, `timePicker` | **new form controls** (phase 7) |
| `select` / `option` | `select` | dropdown |
| `ul`, `ol`, `li` | `list` (lazy, scrolls efficiently) | phase 7 |
| `hr` | `divider` | |
| `table`, `tr`, `td` | `grid` | |
| `video`, `audio`, `canvas`, `iframe` | ❌ not in the MVP | |

### Web-only CSS, and what ERA does instead

| CSS | Why not native | ERA instead |
|---|---|---|
| Selectors, cascade, specificity, `!important` | Unpredictable and slow to resolve on device | Styles live on elements, components and (proposed) classes. `!important` is ignored with a warning |
| `float`, `clear`, `display: inline / table` | Text-flow layout from print | `row` + `flexWrap`, `grid`, `span` |
| `transition`, `animation`, `@keyframes` | Animations are out of the MVP (brief) | Reserved; a clear error says it is planned |
| `cursor`, `@page`, `@print`, `columns` | Browser- or print-specific | `hover` state (desktop) |
| `@import`, `@font-face` | ERA loads skins and fonts automatically | `Skins/`, `Assets/Fonts/` |

## 12. Migration of current ERA names (owner decision D-072)

| Today | New canonical | Today | New canonical |
|---|---|---|---|
| `padding_x` / `padding_y` | `paddingX` / `paddingY` | `text_color` | `color` |
| `padding_top` … | `paddingTop` … | `max_lines` | `lineClamp` |
| `border_width` / `border_color` | `borderWidth` / `borderColor` | `line_height` (skin) | `lineHeight` |
| `radius` | `borderRadius` (`radius` kept as an alias) | `align` | `alignItems` (`align` kept) |
| `distribute` | `justifyContent` (`distribute` kept) | `elevation` | `boxShadow` (`elevation` kept as a token) |
| `style` (text) | `textStyle` | `on_press` | `onPress` |
| `fit: cover` (image) | `objectFit: cover` | `min_width` … | `minWidth` … |

Skin token names and your own prop names keep whatever case you choose.
Only built-in property names change.

## 13. Native platform impact

- Compose 1.9 adds `dropShadow`/`innerShadow` for real CSS box shadows.
  This means bumping Compose (BOM 2025.08+, Compose Multiplatform 1.9 for
  the preview), recorded as a toolchain decision at phase 4.
- Blur and backdrop filters need Android 12 (API 31). Below that, `blur`
  is skipped with a build warning that names the screen.
- ERA's own grid layout (for `fr`, `repeat`, `minmax`, spans) is a small
  custom Compose `Layout` in `EraRuntime.kt`. It's shared by the preview
  and Android.

## 14. Phases (each one tested, documented, previewed and APK-built before the next)

| Phase | Ships |
|---|---|
| **P1 Syntax** | camelCase names, kebab-case paste, `;` separator, hyphenated keywords, CSS function values in the parser, `era fmt` with `--fix` migration, deprecation warnings |
| **P2 Values and sizing** | px/dp/pt/%/rem/em/vw/vh/fr, `auto`/`fit-content`/`min/max-content`, every sizing mode in §4, `aspectRatio`, `flex` |
| **P3 Box model** | `margin` (incl. `auto`), per-side `padding`/`border`/`borderRadius`, `borderStyle` (dashed/dotted), `outline`, `opacity`, `visibility`, `display: none`, `overflow` |
| **P4 Paint** | All colour syntaxes, the 148 named colours, `var()`, linear/radial/conic gradients, background images, real `boxShadow` (Compose 1.9 bump), `filter`, `mixBlendMode`, `clipPath` |
| **P5 Text** | All typography in §8, italic, decorations, transforms, shadows, `span`, inheritance of colour and font |
| **P6 Layout** | Full flexbox (wrap, order, reverse, align-self, baseline), CSS grid (`fr`, `repeat`, `minmax`, spans, areas), `position` relative/absolute/fixed, `zIndex`, `transform`, `calc()` |
| **P7 HTML elements** | Aliases from §11, new form controls (checkbox, radio, switch, slider, select, textarea, password, date/time), `list`, `link` |
| **P8 States and responsive** | `hover`/`active`/`focus`/`disabled` blocks and `media (…)` rules (if D-075 is approved); style classes (if approved) |

Each phase also updates ERA Studio: colouring for the new syntax,
autocomplete for CSS names, and the Skin panel.

## 15. Implementation status

**Phase 1 (done, 2026-11-22):**

- **Syntax:**
  - every spelling of a name (kebab, camel, snake, lowercase)
  - CSS aliases: `border-radius`, `justify-content`, `align-items`,
    `text-align`, `object-fit`, `background-color`, `line-clamp`,
    `padding-left/right/inline/block`, `margin-*`, `row-gap`/`column-gap`,
    `flex-grow`
  - `;` separators, hyphenated keywords, `.5`, `!important` (ignored)
  - CSS units and function values in the parser
- **Values and sizing:**
  - `px`, `dp`, `pt`, `rem`, `em`, `%`, `fr`, `vw`, `vh`
  - `auto`, `fit-content`, `stretch`, `min-content`, `max-content`
  - `aspect-ratio` (`16 / 9`), `flex`, `align-self`
  - CSS keyword spellings (`flex-start`, `space-between`, …)
- **Box model:**
  - `margin` with 1–4 values and `auto` centring
  - `padding` shorthand with 1–4 values
  - `border: 1px solid colour`, `border: none`
  - `opacity` (also `%`), `visibility: hidden`, `display: none`
  - `overflow: hidden / clip / scroll / auto`
- **Layout:**
  - `position: relative / absolute / fixed` with `top/right/bottom/left/inset`
  - `z-index`
  - `flex-direction` (including reverse) and `flex-wrap: wrap` (FlowRow)
- **Verified** by IR and codegen tests, a rendered showcase screen
  (`Layout`), and Android APK builds.

**Phase 3 (done, 2026-11-22):** borders, corners and outline (D-087)

- `border-style: solid / dashed / dotted / none` (fixes the dashed part
  of Q-10)
- per-side `border-top/right/bottom/left`, `border-inline-*`,
  `border-block-*` (+ `-width`, `-color`, `-style`)
- `border-width` / `border-color` / `border-style` with 1–4 values
- `border-radius` with 1–4 values and `%`; `border-top-left-radius`, …,
  `border-start-start-radius`, …
- `outline` and `outline-offset` (drawn outside the element, never
  clipped, takes no space)
- **Verified** by IR and codegen tests and the showcase `Look` screen,
  rendered with the Compose preview.
- **Limits:** per-side borders draw square corners; `double` and 3-D
  styles and elliptical radii (`a / b`) are E2135.

**Next phases:**

| Phase | Covers |
|---|---|
| P4 | Colour functions, named colours, `var()`, gradients, `box-shadow`, filters (and the Compose 1.9 bump) |
| P5 | Typography properties and `span` |
| P6 | CSS grid templates, `calc()`, transforms, percentage offsets |
| P7 | HTML element aliases and form controls |
| P8 | `hover`/`active`/`focus`/`disabled` and `media` rules |
