# ERA Grammar — Visual MVP (DRAFT, awaiting approval)

Status: **proposal**. Nothing here is implemented yet. Changes from the
starter brief are listed in `DECISIONS.md` (D-007 onward) and need approval.

## 1. Design idea: one node shape, many meanings

ERA source is built from a single, uniform shape, the **node**:

```era
name argument {
    property: value
    child_node ...
}
```

The **syntax** stays small and uniform. The **meaning** of a node (whether
`gap` is valid, whether `text` takes a string) is checked by the semantic
pass, which knows each context: skin, screen, component and so on. This is
the same split HCL and KDL use. It keeps the parser small, makes error
recovery easier, and lets us add primitives later without touching the
grammar.

Only a handful of forms are special: top-level declarations, `prop`,
`variant`, `when` and `slot`.

## 2. Lexical structure

| Token | Form | Examples |
|---|---|---|
| Comment | `//` to end of line | `// hero section` |
| Identifier | `[A-Za-z_][A-Za-z0-9_]*` | `column`, `primary`, `AppButton` |
| String | `"..."`, escapes `\" \\ \n \t \u{1F600}` | `"Hello"` |
| Number | `123`, `1.5` (no sign in MVP; no units) | `16`, `1.25` |
| Color | `#RGB`, `#RRGGBB`, `#RRGGBBAA` | `#FF6B35` |
| Punctuation | `{ } : , \| = ?` | |
| Newline | significant as a separator (see below) | |

**Keywords:** `app skin screen layout component prop variant when is slot true false none`

**Naming convention (enforced by `era check`):**
- `PascalCase`: user declarations (screens, components, layouts, skins).
- `snake_case`: built-in primitives, properties, tokens, props, variants.

That is how a reader tells `AppButton` (yours) from `button` (built-in).

**Separators:** items inside a block are separated by a newline or a comma.
Blank lines and trailing commas are allowed. There are no semicolons.

**Units:** plain numbers are density-independent pixels (dp). Font sizes
and line heights are scalable pixels (sp). Units are never written in
source.

## 3. Grammar (EBNF)

```ebnf
file          = { SEP } { declaration { SEP } } EOF ;
SEP           = NEWLINE | "," ;

declaration   = app_decl | skin_decl | screen_decl | layout_decl | component_decl ;
app_decl      = "app"       STRING block ;
skin_decl     = "skin"      PASCAL_IDENT block ;
screen_decl   = "screen"    PASCAL_IDENT block ;
layout_decl   = "layout"    PASCAL_IDENT block ;
component_decl= "component" PASCAL_IDENT block ;

block         = "{" { SEP } [ item { SEP { SEP } item } { SEP } ] "}" ;

item          = property
              | prop_decl
              | variant_decl
              | when_block
              | "slot"
              | node ;

property      = IDENT ":" value ;                   (* IDENT followed by ':' *)
node          = IDENT [ argument ] [ block ] ;      (* IDENT NOT followed by ':' *)
argument      = STRING | NUMBER | COLOR | IDENT ;    (* same line as the node name *)

prop_decl     = "prop" IDENT ":" prop_type [ "?" ] [ "=" value ] ;
prop_type     = "text" | "number" | "bool" | "color" | "image" | "icon" ;

variant_decl  = "variant" IDENT ":" IDENT { "|" IDENT } [ "=" IDENT ] ;

when_block    = "when" IDENT "is" IDENT block ;     (* property overrides only *)

value         = STRING | NUMBER | COLOR | IDENT | "true" | "false" | "none" ;
```

The parser is LL(2): one token of lookahead past an identifier decides
between `property` (`ident :`) and `node`.

## 4. Where each form may appear

| Context | Properties | Nodes | `prop`/`variant` | `when` | `slot` |
|---|---|---|---|---|---|
| `app` | yes (`skin`, `start`) | no | no | no | no |
| `skin` | no | token sections only | no | no | no |
| `screen` | yes (`title`) | exactly one root | no | no | no |
| `layout` | no | exactly one root | `prop` only | no | exactly one, inside root |
| `component` | no | exactly one root | yes | inside nodes | optional, at most one |
| a node | yes, per primitive | if the primitive is a container | no | inside component nodes | inside layout/component nodes |

## 5. Name resolution

- All files under `src/` share **one project namespace**. There are no
  imports. Duplicate names are an error that points at both locations.
- Skins in `Skins/*.ers` are loaded automatically and referenced **by
  name** (`skin: Studio`). They are not referenced by path.
- A bare identifier used as a value is resolved **by the property's
  type**:
  - `background: primary` looks in the skin's `colors`
  - `gap: md` looks in `spacing`
  - `radius: lg` looks in `radius`
  - `style: heading` looks in `typography`
  - `elevation: sm` looks in `elevation`

  Because each property has exactly one value type, the lookup is
  unambiguous.
- Inside a component or layout, a bare identifier first matches that
  declaration's own `prop`s. A prop whose name is also a token name in a
  category the component uses is an error, so shadowing cannot happen
  silently.
- Asset paths are strings relative to the project's `Assets/` folder, for
  example `"Images/hero.png"`. They are checked to exist during `era check`.

## 6. Diagnostics contract

Every AST node carries a `Span { file, start, end }`. Errors print as:

```text
error[E0201]: unknown property `gapp` on `column`
  --> src/Screens/Home.era:12:9
   |
12 |         gapp: md
   |         ^^^^ did you mean `gap`?
```

## 7. Explicitly not in the grammar (MVP)

There are no expressions, operators, string interpolation, loops,
functions, generics, imports, modules or inheritance. Events and state
(`on_press`, `state`) are reserved for M6 and will be specified then.
