# Diagnostic Codes

Codes are stable once released. `E` is an error and makes `era check`
fail. `W` is a warning.

| Code | Meaning |
|---|---|
| E0001 | No `era.toml` found, or it cannot be read |
| E0101 | `era.toml` is not valid TOML, has a missing section or key, or has an unknown key |
| E0102 | Invalid project name (must be `snake_case`, at most 64 characters) |
| E0103 | Invalid project id (reverse-DNS, at least 2 segments, no reserved words) |
| E0104 | Invalid version (`MAJOR.MINOR.PATCH`) |
| E0105 | Unsupported language edition |
| E0106 | `android.min_sdk` is out of range (21–36) |
| E0110 | `era new`: invalid project name |
| E0111 | `era new`: the target folder exists and is not empty |
| E0112 | `era new`: a file system error |
| E0201 | Missing entry point `src/app.era` |
| E0207 | A source file is not valid UTF-8 |
| E0208 | A source file cannot be read |
| W0202 | A `.ers` skin file outside `Skins/` (ignored) |
| W0203 | A `.era` UI file inside `Skins/` (ignored) |
| W0204 | An unexpected file in `src/` or `Skins/` (ignored) |
| W0205 | *(removed in M6: `.er` logic files are now processed)* |
| W0206 | A folder has the wrong letter case (e.g. `skins` instead of `Skins`) |
| W0301 | An asset has the wrong file type for its category folder |
| W0302 | An asset sits directly in `Assets/`, not in a category folder |
| W0303 | An unknown folder inside `Assets/` |

## Syntax (M2): lexical

| Code | Meaning |
|---|---|
| E1001 | Unterminated string |
| E1002 | Unknown string escape or invalid `\u{...}` |
| E1003 | Invalid color (must be `#RGB`, `#RRGGBB` or `#RRGGBBAA`) |
| E1004 | Unexpected character (including non-ASCII outside strings) |
| E1005 | Malformed number (`1.`) |
| E1006 | Unknown unit after a number (`16qq`). Units: px dp pt rem em vw vh vmin vmax fr deg rad turn s ms |

## Syntax (M2): grammar

| Code | Meaning |
|---|---|
| E1101 | Expected X, found Y (general syntax error) |
| E1102 | Unclosed `{` (points at the opening brace) |
| E1103 | Two items on one line without a `;` (or `,`) |
| E1104 | A `screen`/`layout`/`component`/`skin`/`app` declared inside another block |
| E1105 | A block `{` used as a property value |
| E1106 | Unknown prop type (allowed: text, number, bool, color, image, icon) |
| E1107 | A reserved word used as a name |
| E1108 | `use skin "..."` (use `skin: Name` instead) |

## Structure (M2)

| Code | Meaning |
|---|---|
| E1201 | A form is not allowed in this place (e.g. `slot` in a screen, an element in `app`) |
| E1202 | No `app` declaration in `src/app.era` |
| E1203 | More than one `app` |
| E1204 | `app` declared outside `src/app.era` |
| E1206 | A screen, layout or component has no root element |
| E1207 | More than one root element |
| E1209 | A layout has no `slot` |
| E1210 | More than one `slot` |
| E1211 | `when` uses an undeclared variant |
| E1212 | `when` uses an option the variant doesn't have |
| E1213 | A prop, variant or option name is declared twice |
| E1214 | A variant's default is not one of its options |
| E1215 | Duplicate screen, layout or component name in the project |
| E1216 | Wrong file kind (`skin` in `.era`, or UI in `.ers`) |
| E1217 | A declaration name is not PascalCase |
| E1218 | A prop, variant or option name is not snake_case |

## Skins (M3)

| Code | Meaning |
|---|---|
| E2001 | Unknown skin section (with a suggestion) |
| E2002 | Unexpected value after a section name (only `colors dark` is allowed) |
| E2003 | A token is defined twice, possibly across files (cites the first) |
| E2004 | A token value has the wrong type or is out of range |
| E2005 | Unexpected item or key in a section or text style |
| E2006 | A token name is not snake_case |
| E2007 | A text style has no `size` |
| E2008 | A required color or the `body` text style is missing |
| E2009 | `colors dark` keys don't match `colors` |
| E2010 | No font files for the font family |

## Meaning (M3)

| Code | Meaning |
|---|---|
| E2101 | Unknown skin in `app` |
| E2102 | Unknown `app` property |
| E2103 | `skin`/`start` value is not a name |
| E2104 | The project has no screens |
| E2105 | Several screens and no `start` |
| E2106 | `start` names an unknown screen |
| E2110 | A layout used somewhere other than a screen's root |
| E2111 | A screen used as an element |
| E2112 | Unknown element (suggests primitives, or components and layouts) |
| E2113 | A value given to an element that takes none |
| E2114 | Unknown prop used by name (or unquoted text) |
| E2115 | An element's value has the wrong kind |
| E2116 | An element that needs a value has none |
| E2120 | Unknown property (with a suggestion) |
| E2121 | A property or prop is set twice |
| E2122 | Children inside an element that cannot have any |
| E2123 | `scroll` does not have exactly one child |
| E2130 | A prop's type doesn't fit where it is used |
| E2131 | A prop has the same name as a token it would hide |
| E2132 | Unknown skin token (suggests one, or lists them all) |
| E2133 | A value has the wrong type for a property or prop |
| E2140 | A component uses itself (directly or indirectly) |
| E2141 | A component given a value instead of named props |
| E2142 | Unknown variant option |
| E2143 | A component has no such prop or variant |
| E2144 | Children given to a component without a `slot` |
| E2145 | A required prop is missing |
| E2150 | An asset file does not exist |
| E2151 | An invalid asset path (absolute, `..`, backslashes) |
| E2152 | The wrong file type for `image` or `icon` |

## Android build (M5)

| Code | Meaning |
|---|---|
| E3001 | An SVG icon uses a feature the VectorDrawable converter doesn't support (transform, gradient, text, mask, unknown color...) |
| E3002 | The Android project cannot be written to disk |
| E3003 | `era export android`: the output folder is not empty. `--force` replaces it, but only when it holds an earlier ERA export (a `.era-export` marker); other folders are never deleted |

## Stores and interactions (M6)

| Code | Meaning |
|---|---|
| E1109 | Something other than `state`/`action` inside a `store` |
| E1110 | Unknown action step (use `set`, `toggle`, `add`) |
| E2160 | Bad `{...}` interpolation in text (unclosed, or not `Store.field`) |
| E2161 | Unknown store (with a suggestion) |
| E2162 | Unknown state, or an action used where state is needed |
| E2163 | Unknown action, or state used where an action is needed |
| E2164 | A value that is not an action in `on_press` |
| E2165 | `go` names an unknown screen |
| E2166 | State of the wrong type (`visible` needs bool, `bind` needs text) |
| E2170 | Unknown state type (`text`, `number`, `bool`) |
| E2171 | Initial value doesn't match the state type |
| E2172 | An action step names unknown state |
| E2173 | An action step has the wrong type (`toggle` non-bool, `add` non-number, `set` mismatch) |

## CSS values (M8)

| Code | Meaning |
|---|---|
| E2134 | A unit that the property doesn't take (`width: 45deg`, `gap: 2fr`) |
| E2135 | A CSS feature planned for a later phase (e.g. `dashed` borders, P3) |
