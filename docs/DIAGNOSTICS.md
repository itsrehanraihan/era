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
| W0205 | Files in `src/Logic/` are not processed yet (M6) |
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
| E1005 | Malformed number (`1.`, `.5`) |
| E1006 | Unit after a number (`16px`). ERA numbers have no units |

## Syntax (M2): grammar

| Code | Meaning |
|---|---|
| E1101 | Expected X, found Y (general syntax error) |
| E1102 | Unclosed `{` (points at the opening brace) |
| E1103 | Two items on one line without a `,` |
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
