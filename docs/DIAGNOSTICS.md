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
