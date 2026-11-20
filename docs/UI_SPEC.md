# ERA UI Model (DRAFT, awaiting approval)

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
| `width`, `height` | `fill`, `fit`, or a number |
| `min_width max_width min_height max_height` | number |
| `padding padding_x padding_y padding_top padding_bottom padding_start padding_end` | spacing token or number |
| `background` | color token, color literal, or `none` |
| `border_width` | border token or number |
| `border_color` | color token or literal |
| `radius` | radius token or number |
| `elevation` | elevation token or number |
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

## 5. UI IR (M3, platform-independent)

The UI IR is the contract between the frontend and every backend:
Compose for preview and Android now, possibly SwiftUI later. It holds:

- Fully resolved, typed values. Tokens are resolved, but each resolved
  value keeps its token name so backends can emit theme references.
- Components expanded, or kept as reusable IR functions (decided in M3).
- Stable node IDs derived from file plus path, never from memory
  addresses.
- No Android types.

It is serializable, so it can be snapshot-tested, and ordered
deterministically.
