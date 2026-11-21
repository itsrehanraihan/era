//! Validation rules for project names and application IDs.

/// Kotlin hard keywords. They cannot be used as package segments without
/// escaping, and the application ID becomes the generated Kotlin package.
const KOTLIN_HARD_KEYWORDS: &[&str] = &[
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
    "while",
];

pub const MAX_NAME_LEN: usize = 64;

/// A project name is snake_case: `[a-z][a-z0-9_]*`, at most 64 characters.
pub fn check_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("project name is empty".into());
    }
    if name.len() > MAX_NAME_LEN {
        return Err(format!(
            "project name is longer than {MAX_NAME_LEN} characters"
        ));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return Err("project name must start with a lowercase letter (a-z)".into());
    }
    if let Some(bad) = chars.find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_'))
    {
        return Err(format!(
            "project name may only contain a-z, 0-9 and `_` (found `{bad}`)"
        ));
    }
    Ok(())
}

/// Best-effort suggestion for an invalid name: `My-App` -> `my_app`.
pub fn suggest_project_name(name: &str) -> Option<String> {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    let s = s.trim_matches('_').to_string();
    let s = if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("app_{s}")
    } else {
        s
    };
    (check_project_name(&s).is_ok() && s != name).then_some(s)
}

/// Application ID in reverse-DNS form: at least two `.`-separated segments,
/// each `[A-Za-z][A-Za-z0-9_]*` and not a Kotlin keyword.
pub fn check_app_id(id: &str) -> Result<(), String> {
    let segments: Vec<&str> = id.split('.').collect();
    if segments.len() < 2 {
        return Err("application id needs at least two segments, like `dev.era.my_app`".into());
    }
    for seg in segments {
        if seg.is_empty() {
            return Err("application id has an empty segment".into());
        }
        if !seg.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return Err(format!("segment `{seg}` must start with a letter"));
        }
        if let Some(bad) = seg
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || *c == '_'))
        {
            return Err(format!(
                "segment `{seg}` may only contain letters, digits and `_` (found `{bad}`)"
            ));
        }
        if KOTLIN_HARD_KEYWORDS.contains(&seg) {
            return Err(format!("segment `{seg}` is a reserved word"));
        }
    }
    Ok(())
}

/// `MAJOR.MINOR.PATCH`, digits only.
pub fn check_version(v: &str) -> Result<(), String> {
    let parts: Vec<&str> = v.split('.').collect();
    let ok = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if ok {
        Ok(())
    } else {
        Err("version must look like `MAJOR.MINOR.PATCH`, e.g. `0.1.0`".into())
    }
}

/// `my_app` -> `My App`, used as the default display name.
pub fn display_name(name: &str) -> String {
    name.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        for n in ["a", "my_app", "app2", "showcase"] {
            assert!(check_project_name(n).is_ok(), "{n}");
        }
    }

    #[test]
    fn invalid_names() {
        for n in [
            "",
            "My_app",
            "2app",
            "my-app",
            "_app",
            "my app",
            &"a".repeat(65),
        ] {
            assert!(check_project_name(n).is_err(), "{n}");
        }
    }

    #[test]
    fn suggestions() {
        assert_eq!(suggest_project_name("My-App").as_deref(), Some("my_app"));
        assert_eq!(suggest_project_name("2cool").as_deref(), Some("app_2cool"));
        assert_eq!(suggest_project_name("--"), None);
    }

    #[test]
    fn app_ids() {
        assert!(check_app_id("dev.era.showcase").is_ok());
        assert!(check_app_id("com.Example.App_1").is_ok());
        for bad in [
            "showcase",
            "dev..x",
            "dev.1x",
            "dev.my-app",
            "dev.era.fun",
            ".dev.x",
        ] {
            assert!(check_app_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn versions() {
        assert!(check_version("0.1.0").is_ok());
        for bad in ["1.0", "1.0.0-beta", "a.b.c", "1..0", ""] {
            assert!(check_version(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn display_names() {
        assert_eq!(display_name("my_app"), "My App");
        assert_eq!(display_name("showcase"), "Showcase");
    }
}
