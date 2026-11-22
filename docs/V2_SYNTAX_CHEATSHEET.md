# ERA v2 syntax cheatsheet: your input sheet

**Why this exists.** For the MVP, ERA follows HTML and CSS directly
(owner decision D-080). The owner will gradually reshape the syntax,
patterns and grammar into ERA's own style for v2. This sheet lists
**every construct the MVP has today**, so you can write your v2 form
next to each one.

How to use it:
1. Fill in **Your v2 syntax** (leave it empty to keep the MVP form).
2. Add notes such as "drop this", "rename", or "merge with …".
3. Commit the file. The next session turns your column into a v2 grammar
   proposal, plus `era fmt --to-v2` to migrate existing code
   automatically.

Legend: ✅ works today · 🧩 planned in the CSS phases (docs/CSS_SPEC.md)

---

## 1. Files and declarations

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Project manifest | `era.toml` | `[project] name = "shop"` | |
| UI file | `.era` | `src/Screens/Home.era` | |
| Skin (theme) file | `.ers` | `Skins/theme.ers` | |
| Logic file | `.er` | `src/Logic/session.er` | |
| App entry ✅ | `app "Name" { … }` | `app "Shop" { skin: Studio; start: Home }` | |
| Screen ✅ | `screen Name { … }` | `screen Home { title: "Home"; column { … } }` | |
| Reusable component ✅ | `component Name { … }` | `component Card { prop title: text … }` | |
| Page layout ✅ | `layout Name { … slot … }` | `layout Page { column { slot } }` | |
| Skin ✅ | `skin Name { … }` | `skin Studio { colors { … } }` | |
| Store (state + logic) ✅ | `store Name { … }` | `store Cart { state count: number }` | |
| Comment ✅ | `// …` | `// header` | |

## 2. Elements

| Concept | MVP today | HTML equivalent | Your v2 syntax |
|---|---|---|---|
| Vertical stack ✅ | `column { }` | `<div style="display:flex;flex-direction:column">` | |
| Horizontal stack ✅ | `row { }` | `<div style="display:flex">` | |
| Overlap layers ✅ | `stack { }` | `position: relative` container | |
| Grid ✅ | `grid { columns: 3 }` | `display: grid` | |
| Scroll area ✅ | `scroll { }` | `overflow: auto` | |
| Card / surface ✅ | `card { }` | `<div class="card">` | |
| Text ✅ | `text "Hi"` | `<p>`, `<span>` | |
| Image ✅ | `image "Images/a.png"` | `<img src>` | |
| Icon ✅ | `icon "Icons/a.svg"` | `<svg>` | |
| Button ✅ | `button "Go"` | `<button>` | |
| Text field ✅ | `input { placeholder: "Email" }` | `<input>` | |
| Divider ✅ | `divider` | `<hr>` | |
| Spacer ✅ | `spacer` | `flex: 1` filler | |
| Insert slot ✅ | `slot` | `<slot>` / children | |
| Use a component ✅ | `AppButton { label: "Go" }` | custom element | |
| HTML names as aliases 🧩 | `div`, `p`, `h1`…`h6`, `img`, `a`, `ul/li` | | |
| Form controls 🧩 | `checkbox`, `radio`, `switch`, `slider`, `select`, `textarea`, date/time | `<input type=…>` | |
| Rich text spans 🧩 | `text { span "a"; span "b" { color: red } }` | `<span>` | |

## 3. Writing properties

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Property ✅ | `name: value` | `padding: 16px` | |
| Every spelling works ✅ | kebab / camel / snake / lowercase | `font-size` = `fontSize` = `font_size` = `fontsize` | |
| Several on one line ✅ | `;` (CSS), and `,` (still accepted) | `{ gap: 8px; padding: 16px }` | |
| Element text argument ✅ | after the element name | `text "Hello"` | |
| Block of children ✅ | `{ … }` | `column { text "a" }` | |
| `!important` ✅ (ignored) | | `gap: 8px !important` | |

## 4. Values and units

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Plain number = dp ✅ | number | `16` | |
| Pixels (= dp) ✅ | `px` | `16px` | |
| Density pixels ✅ | `dp` | `16dp` | |
| Points ✅ | `pt` (= 4/3 dp) | `12pt` | |
| Root / font size ✅ | `rem`, `em` (16) | `1.5rem` | |
| Percent of parent ✅ | `%` | `width: 50%` | |
| Fraction of free space ✅ | `fr` | `width: 2fr` | |
| Screen share ✅ | `vw`, `vh` | `height: 100vh` | |
| Fill / fit ✅ | `fill`, `fit`, `auto`, `stretch` | `width: fill` | |
| Intrinsic ✅ | `min-content`, `max-content` | `width: max-content` | |
| Ratio ✅ | `a / b` | `aspect-ratio: 16 / 9` | |
| Hex colour ✅ | `#RGB`, `#RRGGBB`, `#RRGGBBAA` | `#FF6B35` | |
| Skin token ✅ | bare name | `background: primary` | |
| No value ✅ | `none` | `background: none` | |
| Bool ✅ | `true`, `false` | `clip: true` | |
| Text ✅ | `"…"` | `"Hello"` | |
| Show state in text ✅ | `{Store.field}` | `"Hi {User.name}"` | |
| Literal brace ✅ | `{{` `}}` | `"{{x}}"` | |
| CSS colour functions 🧩 | `rgb()`, `rgba()`, `hsl()`, named colours | `rgba(0, 0, 0, .1)` | |
| Token variable 🧩 | `var(--name)` | `var(--primary)` | |
| Gradients 🧩 | `linear-/radial-/conic-gradient()` | `linear-gradient(135deg, red, blue)` | |
| Image url 🧩 | `url("…")` | `url("Images/bg.png")` | |
| Math 🧩 | `calc()`, `min()`, `max()`, `clamp()` | `calc(100% - 32px)` | |
| Angles / durations | `deg`, `rad`, `turn` / `s`, `ms` | `45deg` (durations reserved) | |

## 5. Sizing and spacing

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Width / height ✅ | `width`, `height` | `width: 50%` | |
| Min / max size ✅ | `min-width`, `max-width`, `min-height`, `max-height` | `max-width: 480px` | |
| Aspect ratio ✅ | `aspect-ratio` | `aspect-ratio: 1` | |
| Flex grow ✅ | `flex`, `flex-grow` | `flex: 1` | |
| Padding ✅ | `padding` (1–4 values), `padding-top/right/bottom/left`, `padding-x/-y`, `padding-inline/-block` | `padding: 8px 16px` | |
| Margin ✅ | `margin` (1–4 values, `auto`), the same side names | `margin: 0 auto` | |
| Gap ✅ | `gap`, `row-gap`, `column-gap` | `gap: 12px` | |

## 6. Layout

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Main-axis distribution ✅ | `justify-content` (`distribute`) | `justify-content: space-between` | |
| Cross-axis alignment ✅ | `align-items` (`align`) | `align-items: center` | |
| Self alignment ✅ | `align-self` | `align-self: flex-end` | |
| Direction ✅ | `flex-direction` | `flex-direction: row-reverse` | |
| Wrap ✅ | `flex-wrap` | `flex-wrap: wrap` | |
| Hide ✅ | `display: none`, `visible: false`, `visibility: hidden` | | |
| Position ✅ | `position: relative / absolute / fixed` | `position: absolute` | |
| Offsets ✅ | `top`, `right`, `bottom`, `left`, `inset` | `top: -8px; right: -8px` | |
| Stacking ✅ | `z-index` | `z-index: 2` | |
| Overflow ✅ | `overflow: hidden / scroll / auto` | `overflow: hidden` | |
| Stack alignment ✅ | `align: center` (9 positions) | `align: bottom_end` | |
| Grid columns ✅ / 🧩 | `columns: 3` ✅; `grid-template-columns: 1fr 2fr` 🧩 | | |
| Grid spans 🧩 | `grid-column: span 2` | | |
| Sticky 🧩 | `position: sticky` | | |

## 7. Look (colour, border, shadow, effects)

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Background ✅ | `background`, `background-color` | `background: surface` | |
| Border ✅ | `border` shorthand, `border-width`, `border-color` | `border: 1px solid border` | |
| Rounded corners ✅ | `border-radius` (`radius`) | `border-radius: 12px` | |
| Shadow (elevation) ✅ | `elevation` | `elevation: md` | |
| Opacity ✅ | `opacity` | `opacity: .5` | |
| Clip ✅ | `clip: true`, `overflow: hidden` | | |
| Dashed / dotted borders 🧩 | `border-style` | `border: 1px dashed red` | |
| Per-side borders and corners 🧩 | `border-top`, `border-top-left-radius` | | |
| Real CSS box-shadow 🧩 | `box-shadow` | `0 4px 12px rgba(0,0,0,.1)` | |
| Gradients 🧩 | `background: linear-gradient(…)` | | |
| Transforms 🧩 | `transform` | `rotate(45deg) scale(1.1)` | |
| Filters 🧩 | `filter`, `mix-blend-mode`, `clip-path` | `blur(8px)` | |

## 8. Text

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Text style token ✅ | `style` / `text-style` | `style: heading` | |
| Colour ✅ | `color` | `color: muted` | |
| Alignment ✅ | `text-align` (`align`) | `text-align: center` | |
| Line limit ✅ | `line-clamp` (`max-lines`) | `line-clamp: 2` | |
| Font size, weight, family 🧩 | `font-size`, `font-weight`, `font-family` | `font-size: 18px` | |
| Italic, decoration, transform 🧩 | `font-style`, `text-decoration`, `text-transform` | | |
| Line height, letter spacing 🧩 | `line-height`, `letter-spacing` | | |
| Text shadow, nowrap, ellipsis 🧩 | `text-shadow`, `white-space`, `text-overflow` | | |

## 9. Images and icons

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Image fit ✅ | `object-fit` (`fit`) | `object-fit: cover` | |
| Icon size and tint ✅ | `size`, `color` | `icon "Icons/x.svg" { size: 24px }` | |
| Asset paths ✅ | relative to `Assets/` | `"Images/hero.png"` | |

## 10. Interaction and state

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Tap ✅ | `on-press` / `onClick` | `on-press: go Home` | |
| Navigate ✅ | `go Screen`, `back` | `go Checkout` | |
| Run logic ✅ | `Store.action` | `Cart.add` | |
| Chain ✅ | `then` | `Cart.add then go Cart` | |
| Show / hide by state ✅ | `visible: Store.flag` | `visible: Session.verified` | |
| Two-way input ✅ | `bind: Store.text` | `bind: Login.email` | |
| State ✅ | `state name: type = value` | `state count: number = 0` | |
| Action ✅ | `action name { … }` | `action reset { set count = 0 }` | |
| Steps ✅ | `set`, `toggle`, `add` | `add count 1` | |
| States 🧩 | `hover`, `active`, `focus`, `disabled` blocks | `active { opacity: .8 }` | |
| Responsive 🧩 | `media (min-width: 600px) { }` | | |

## 11. Components

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Prop ✅ | `prop name: type[?] [= default]` | `prop label: text` | |
| Prop types ✅ | `text number bool color image icon action` | `prop tap: action?` | |
| Variant ✅ | `variant name: a \| b = a` | `variant tone: solid \| ghost = solid` | |
| Variant styling ✅ | `when variant is option { }` | `when tone is ghost { background: none }` | |

## 12. Skins (design tokens)

| Concept | MVP today | Example | Your v2 syntax |
|---|---|---|---|
| Colours ✅ | `colors { name: #hex }` | `primary: #FF6B35` | |
| Dark colours ✅ | `colors dark { }` | | |
| Spacing / radius / border / elevation ✅ | `spacing { md: 16 }` … | | |
| Typography ✅ | `typography { font: "Inter"; body { size: 16 } }` | | |

---

*Last updated with M8 phase 1 (2026-11-22). Each CSS phase adds rows or
flips 🧩 to ✅.*
