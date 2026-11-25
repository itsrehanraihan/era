//! `era.toml` schema and validation.

use era_diagnostics::Diagnostic;
use serde::Deserialize;
use toml::Spanned;

use crate::names;

/// Language editions this compiler understands.
pub const SUPPORTED_EDITIONS: &[&str] = &["2026"];
pub const DEFAULT_MIN_SDK: u32 = 24;
/// Jetpack Compose requires API 21 or newer.
pub const MIN_SDK_RANGE: std::ops::RangeInclusive<u32> = 21..=36;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub id: String,
    pub version: String,
    pub edition: String,
    pub android: AndroidSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndroidSettings {
    pub min_sdk: u32,
    /// `[android.signing]`: the release keystore (D-086). Passwords are never
    /// stored here; they come from environment variables.
    pub signing: Option<Signing>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signing {
    /// Keystore path, relative to the project folder (or absolute).
    pub keystore: String,
    /// Key alias inside the keystore.
    pub alias: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    project: RawProject,
    era: RawEra,
    android: Option<RawAndroid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProject {
    name: Spanned<String>,
    id: Spanned<String>,
    version: Spanned<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEra {
    edition: Spanned<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAndroid {
    min_sdk: Option<Spanned<i64>>,
    signing: Option<RawSigning>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSigning {
    keystore: Spanned<String>,
    alias: Spanned<String>,
}

/// Parse and validate `era.toml`. `path` is only used for diagnostics.
/// Returns every validation error found, not just the first.
pub fn parse_manifest(src: &str, path: &str) -> Result<Manifest, Vec<Diagnostic>> {
    let raw: RawManifest = match toml::from_str(src) {
        Ok(raw) => raw,
        Err(e) => {
            let mut d = Diagnostic::error("E0101", format!("invalid {path}: {}", e.message()));
            d = match e.span() {
                Some(span) => d.at(path, span),
                None => d.in_file(path),
            };
            return Err(vec![
                d.with_help("see docs/ERA_SPEC.md for the era.toml format"),
            ]);
        }
    };

    let mut errors = Vec::new();
    let mut check =
        |field: &Spanned<String>, code, what: &str, rule: fn(&str) -> Result<(), String>| {
            if let Err(msg) = rule(field.get_ref()) {
                errors.push(
                    Diagnostic::error(code, format!("invalid {what} `{}`: {msg}", field.get_ref()))
                        .at(path, field.span()),
                );
            }
        };
    check(
        &raw.project.name,
        "E0102",
        "project name",
        names::check_project_name,
    );
    check(&raw.project.id, "E0103", "project id", names::check_app_id);
    check(
        &raw.project.version,
        "E0104",
        "version",
        names::check_version,
    );

    let edition = &raw.era.edition;
    if !SUPPORTED_EDITIONS.contains(&edition.get_ref().as_str()) {
        errors.push(
            Diagnostic::error(
                "E0105",
                format!("unsupported edition `{}`", edition.get_ref()),
            )
            .at(path, edition.span())
            .with_help(format!(
                "supported editions: {}",
                SUPPORTED_EDITIONS.join(", ")
            )),
        );
    }

    let mut min_sdk = DEFAULT_MIN_SDK;
    let (raw_sdk, raw_signing) = match raw.android {
        Some(a) => (a.min_sdk, a.signing),
        None => (None, None),
    };
    let mut signing = None;
    if let Some(sg) = raw_signing {
        for (field, what) in [(&sg.keystore, "keystore"), (&sg.alias, "alias")] {
            if field.get_ref().trim().is_empty() {
                errors.push(
                    Diagnostic::error("E0107", format!("android.signing.{what} is empty"))
                        .at(path, field.span())
                        .with_help("see docs/ERA_SPEC.md (release builds)"),
                );
            }
        }
        signing = Some(Signing {
            keystore: sg.keystore.into_inner(),
            alias: sg.alias.into_inner(),
        });
    }
    if let Some(sdk) = raw_sdk {
        let value = *sdk.get_ref();
        match u32::try_from(value) {
            Ok(v) if MIN_SDK_RANGE.contains(&v) => min_sdk = v,
            _ => errors.push(
                Diagnostic::error(
                    "E0106",
                    format!("android.min_sdk `{value}` is out of range"),
                )
                .at(path, sdk.span())
                .with_help(format!(
                    "use a value from {} to {}",
                    MIN_SDK_RANGE.start(),
                    MIN_SDK_RANGE.end()
                )),
            ),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Manifest {
        name: raw.project.name.into_inner(),
        id: raw.project.id.into_inner(),
        version: raw.project.version.into_inner(),
        edition: raw.era.edition.into_inner(),
        android: AndroidSettings { min_sdk, signing },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
[project]
name = "showcase"
id = "dev.era.showcase"
version = "0.1.0"

[era]
edition = "2026"
"#;

    fn errs(src: &str) -> Vec<Diagnostic> {
        parse_manifest(src, "era.toml").expect_err("expected errors")
    }

    #[test]
    fn parses_valid_manifest_with_defaults() {
        let m = parse_manifest(VALID, "era.toml").unwrap();
        assert_eq!(m.name, "showcase");
        assert_eq!(m.id, "dev.era.showcase");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.edition, "2026");
        assert_eq!(m.android.min_sdk, DEFAULT_MIN_SDK);
    }

    #[test]
    fn reads_min_sdk() {
        let src = format!("{VALID}\n[android]\nmin_sdk = 26\n");
        assert_eq!(
            parse_manifest(&src, "era.toml").unwrap().android.min_sdk,
            26
        );
    }

    #[test]
    fn reads_signing() {
        let src = format!(
            "{VALID}\n[android.signing]\nkeystore = \"keys/upload.jks\"\nalias = \"upload\"\n"
        );
        let s = parse_manifest(&src, "era.toml")
            .unwrap()
            .android
            .signing
            .unwrap();
        assert_eq!(s.keystore, "keys/upload.jks");
        assert_eq!(s.alias, "upload");
        let bad = format!("{VALID}\n[android.signing]\nkeystore = \"\"\nalias = \"a\"\n");
        assert_eq!(errs(&bad)[0].code, "E0107");
    }

    #[test]
    fn syntax_error_has_location() {
        let e = errs("[project\nname = 1");
        assert_eq!(e[0].code, "E0101");
        assert!(e[0].location.as_ref().unwrap().span.is_some());
    }

    #[test]
    fn missing_section_is_reported() {
        let e = errs("[project]\nname = \"a\"\nid = \"a.b\"\nversion = \"0.1.0\"\n");
        assert_eq!(e[0].code, "E0101");
        assert!(e[0].message.contains("era"), "{}", e[0].message);
    }

    #[test]
    fn unknown_key_is_rejected() {
        let src = VALID.replace("version", "colour = \"red\"\nversion");
        let e = errs(&src);
        assert_eq!(e[0].code, "E0101");
        assert!(e[0].message.contains("colour"), "{}", e[0].message);
    }

    #[test]
    fn reports_all_field_errors_with_spans() {
        let src = VALID
            .replace("\"showcase\"", "\"Show-case\"")
            .replace("dev.era.showcase", "showcase")
            .replace("0.1.0", "1.0")
            .replace("2026", "1999");
        let e = errs(&src);
        let codes: Vec<_> = e.iter().map(|d| d.code).collect();
        assert_eq!(codes, ["E0102", "E0103", "E0104", "E0105"]);
        let name_span = e[0].location.as_ref().unwrap().span.clone().unwrap();
        assert_eq!(&src[name_span], "\"Show-case\"");
    }

    #[test]
    fn min_sdk_out_of_range() {
        for bad in ["20", "99", "-1"] {
            let src = format!("{VALID}\n[android]\nmin_sdk = {bad}\n");
            assert_eq!(errs(&src)[0].code, "E0106", "{bad}");
        }
    }
}
