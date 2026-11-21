# ERA UI Model

Status: approved. The property tables, defaults and component rules are
implemented in M3 (`crates/era_ir/src/schema.rs` and `resolve.rs`).

## 1. Declarations

| Keyword | Lives in | Purpose |
|---|---|---|
| `app "Display Name"` | `src/app.era` (exactly one) | Entry point: `skin`, `start` screen |
| `screen Name` | `src/Screens/` (or `app.era`) | One full screen: `title` plus one root node |
| `layout Name` | `src/Layout/` | Reusable screen frame with one `slot`; only usable as a screen's root |
| `component Name` | `src/Components/` | Reusable piece with `prop`s, `variant`s and an optional `slot` |

Folder placement is a **convention**, which `era check` warns about. It is
not a hard error, so a minimal project can keep everything in `app.era`.

## 2. Primitives

Layout containers:

| Node | Argument | Children | Key properties |
|---|---|---|---|
| `column` | none | many | `gap align distribute` |
| `row` | none | many | `gap align distribute` |
| `stack` | none | many (overlaid) | `align` (9-point: `top_start` … `center` … `bottom_end`) |
| `grid` | none | many | `columns gap` |
| `scroll` | none | exactly one | `direction: vertical \| horizontal` |
| `card` | none | many (vertical) | Like `column`, with skin defaults: `surface` color, `md` radius and padding, `sm` elevation |

Content:

| Node | Argument | Key properties |
|---|---|---|
| `text` | string or text prop | `style color align max_lines` |
| `image` | asset path or image prop | `fit: cover \| contain \| fill` |
| `icon` | asset path or icon prop | `size color` |
| `button` | label string or text prop | `style background text_color` (events in M6) |
| `input` | none | `placeholder label` (holds local text state only until M6) |
| `divider` | none | `color thickness` |
| `spacer` | none | no `size` fills the free space; `size: md` gives a fixed gap |

Common properties for every node (as applicable):

| Property | Values |
|---|---|
| `width`, `height` | `fill`, `fit`, or a number. In a row or column, `fill` along the main axis shares the free space with other `fill` siblings (D-041) |
| `min_width max_width min_height max_height` | number |
| `padding padding_x padding_y padding_top padding_bottom padding_start padding_end` | spacing token, number, or `none` (0, D-039) |
| `background` | color token, color literal, or `none` |
| `border_width` | border token, number, or `none` |
| `border_color` | color token or literal |
| `radius` | radius token, number, or `none` |
| `elevation` | elevation token, number, or `none` (flat) |
| `clip` | `true` / `false` |
| `align` (containers) | `start center end stretch` (cross axis) |
| `distribute` (row/column) | `start center end between around evenly` (main axis) |

Each property has a **single value type**, and that type drives token
lookup (see GRAMMAR.md §5).

## 3. Components, props and variants

```era
component AppButton {
    prop label: text
    variant size: small | medium | large = medium
    variant tone: solid | outline | ghost = solid

    button label {
        background: primary
        text_color: on_primary
        radius: pill
        padding_x: lg
        padding_y: sm

        when size is small { padding_x: md, padding_y: xs }
        when size is large { padding_x: xl, padding_y: md }
        when tone is outline {
            background: none
            border_width: thin
            border_color: primary
            text_color: primary
        }
        when tone is ghost { background: none, text_color: primary }
    }
}
```

Usage looks like a primitive. Props and variants are set as properties:

```era
AppButton { label: "Get started", tone: outline, size: large }
```

Rules:
- Props without a default are required, and a missing one is an error.
- `when` only **overrides properties** of the node it sits in. It cannot
  add or remove children. This keeps variants as visual alternatives, as
  the brief asks.
- If a component declares `slot`, nodes in the usage block fill it.
- There is no inheritance and no component-in-component variants.
- A component is configured only through its props and variants. Style
  properties such as `padding` cannot be set on `AppButton { }` from
  outside (D-031).
- Props are named only. `AppButton "Go"` is an error (Q-02).
- An optional prop (`prop badge: icon?`) with no value leaves the
  property unset and leaves out any `image` or `icon` that shows it
  (D-037).
- A component may not use itself, directly or through other components.
- Every component is checked even if no screen uses it.

## 4. Layouts and screens

```era
layout MainLayout {
    prop heading: text
    column {
        background: background
        padding: lg
        gap: md
        text heading { style: heading }
        slot
    }
}

screen Home {
    title: "Home"
    MainLayout {
        heading: "Design starts here"
        text "A native interface built with ERA" { color: muted }
    }
}
```

A layout is used with exactly the same syntax as a component. The
difference is semantic: a layout may only be a screen's root node, and it
must contain one `slot`.

## 4a. Interactions (M6)

```era
// src/Logic/session.er
store Session {
    state code: text
    state verified: bool = false
    action verify { set verified = true }
}

// src/Screens/Verify.era
screen Verify {
    column {
        input { bind: Session.code, placeholder: "Code" }
        text "You typed {Session.code}"
        button "Verify" { on_press: Session.verify then go Home }
        text "Verified!" { visible: Session.verified }
        button "Back" { on_press: back }
    }
}
```

| Property | On | Value |
|---|---|---|
| `on_press` | every element | `go Screen`, `back`, `Store.action`, an `action` prop, or a chain joined with `then`. `none` does nothing |
| `visible` | every element | `true`, `false`, bool state (`Session.verified`) or a bool prop. A hidden element takes no space |
| `bind` | `input` | Text state that the field reads and writes |

Component props may have the type `action`, for example
`prop tap: action?` with `button label { on_press: tap }`. A text prop
can receive interpolated text or state (`label: "Hi {Session.name}"`).

## 4b. Defaults (D-030)

These are applied in the IR when a property is not set. A token name in
parentheses falls back to the number shown if the skin has no such
token.

| Element | Defaults |
|---|---|
| `text` | `style: body`, `color: text` |
| `button` | `style: body`, `background: primary`, `text_color: on_primary`, `radius: md (12)`, `padding_x: md (16)`, `padding_y: sm (8)` |
| `card` | `background: surface`, `radius: md (12)`, `padding: md (16)`, `elevation: sm (1)` |
| `input` | `style: body`, `text_color: text`, `background: surface`, `border_width: thin (1)`, `border_color: border`, `radius: sm (6)`, `padding: sm (8)` |
| `divider` | `color: border`, `thickness: 1` |
| `icon` | `size: 24`, `color: text` |
| `image` | `fit: cover` |
| `scroll` | `direction: vertical` |
| `grid` | `columns: 2` |
| screen | background is the skin's `background` color; the root element fills the screen unless sized (D-043) |

## 5. UI IR (M3, platform-independent)

The UI IR is the contract between the frontend and every backend:
Compose for preview and Android now, possibly SwiftUI later. It holds:

- Fully resolved, typed values. Tokens are resolved, but each resolved
  value keeps its token name so backends can emit theme references.
- Components and layouts are expanded inline (D-029). Each element lists
  its `origin` chain.
- Stable node IDs are paths: `Home/0/2` is a child-index path,
  `Home/3:AppButton` is a component expansion, and `Home/slot/1` is slot
  content.
- No Android types.

It is ordered deterministically. `era check --ir` prints it as text,
and `crates/era_ir/tests/snapshots/showcase.ir` is the snapshot of the
showcase.
