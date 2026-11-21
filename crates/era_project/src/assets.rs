//! Asset path resolution. ERA source refers to assets with paths relative to
//! the project's `Assets/` folder, e.g. `"Images/hero.png"`.

use std::path::{Component, Path, PathBuf};

pub const ASSETS_DIR: &str = "Assets";

/// Asset category folders and the file extensions each accepts.
/// `Files/` accepts anything.
pub const CATEGORIES: &[(&str, &[&str])] = &[
    ("Images", &["png", "jpg", "jpeg", "webp"]),
    ("Icons", &["svg"]),
    ("Fonts", &["ttf", "otf"]),
    ("Files", &[]),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    Empty,
    /// Absolute paths, `..`, `.` or backslashes.
    NotRelative,
    NotFound(PathBuf),
    NotAFile(PathBuf),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::Empty => write!(f, "asset path is empty"),
            AssetError::NotRelative => write!(
                f,
                "asset paths must be relative to `{ASSETS_DIR}/`, use `/` and must not contain `.` or `..`"
            ),
            AssetError::NotFound(p) => write!(f, "asset `{}` does not exist", p.display()),
            AssetError::NotAFile(p) => write!(f, "asset `{}` is not a file", p.display()),
        }
    }
}

/// Resolve `reference` (as written in ERA source) to a file inside
/// `<root>/Assets/`. The path may not escape the folder.
pub fn resolve_asset(root: &Path, reference: &str) -> Result<PathBuf, AssetError> {
    if reference.is_empty() {
        return Err(AssetError::Empty);
    }
    if reference.contains('\\') {
        return Err(AssetError::NotRelative);
    }
    let rel = Path::new(reference);
    if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(AssetError::NotRelative);
    }
    let full = root.join(ASSETS_DIR).join(rel);
    match std::fs::metadata(&full) {
        Ok(m) if m.is_file() => Ok(full),
        Ok(_) => Err(AssetError::NotAFile(PathBuf::from(reference))),
        Err(_) => Err(AssetError::NotFound(PathBuf::from(reference))),
    }
}

/// Allowed extensions for an asset category folder, or `None` for an
/// unknown folder. An empty slice means any extension is accepted.
pub fn category_extensions(category: &str) -> Option<&'static [&'static str]> {
    CATEGORIES
        .iter()
        .find(|(name, _)| *name == category)
        .map(|(_, exts)| *exts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Assets/Images/Nested")).unwrap();
        std::fs::write(dir.path().join("Assets/Images/hero.png"), b"png").unwrap();
        std::fs::write(dir.path().join("Assets/Images/Nested/a.png"), b"png").unwrap();
        std::fs::write(dir.path().join("secret.txt"), b"x").unwrap();
        dir
    }

    #[test]
    fn resolves_existing_assets() {
        let p = project();
        let r = resolve_asset(p.path(), "Images/hero.png").unwrap();
        assert_eq!(r, p.path().join("Assets/Images/hero.png"));
        assert!(resolve_asset(p.path(), "Images/Nested/a.png").is_ok());
    }

    #[test]
    fn rejects_missing_and_directories() {
        let p = project();
        assert!(matches!(
            resolve_asset(p.path(), "Images/nope.png"),
            Err(AssetError::NotFound(_))
        ));
        assert!(matches!(
            resolve_asset(p.path(), "Images"),
            Err(AssetError::NotAFile(_))
        ));
    }

    #[test]
    fn rejects_paths_that_escape_or_are_not_plain() {
        let p = project();
        for bad in [
            "../secret.txt",
            "Images/../../secret.txt",
            "/etc/passwd",
            "./Images/hero.png",
            "Images\\hero.png",
        ] {
            assert_eq!(
                resolve_asset(p.path(), bad),
                Err(AssetError::NotRelative),
                "{bad}"
            );
        }
        assert_eq!(resolve_asset(p.path(), ""), Err(AssetError::Empty));
    }

    #[test]
    fn categories() {
        assert_eq!(category_extensions("Icons"), Some(&["svg"][..]));
        assert_eq!(category_extensions("Files"), Some(&[][..]));
        assert_eq!(category_extensions("images"), None);
    }
}
