//! Find a project root and inventory its files.
//!
//! Folder placement is a convention: unexpected files produce warnings, not
//! errors, so a minimal project with only `era.toml` and `src/app.era` is
//! valid. Results are sorted so output is deterministic.

use std::fs;
use std::path::{Path, PathBuf};

use era_diagnostics::Diagnostic;

use crate::assets::{self, ASSETS_DIR};
use crate::manifest::{Manifest, parse_manifest};
use crate::{ENTRY_FILE, MANIFEST_FILE};

pub const SRC_DIR: &str = "src";
pub const SKINS_DIR: &str = "Skins";
pub const SRC_FOLDERS: &[&str] = &["Screens", "Components", "Layout", "Logic"];
const LOGIC_DIR: &str = "Logic";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    /// `.era` file under `src/`.
    Ui,
    /// `.ers` file under `Skins/`.
    Skin,
    /// File under `src/Logic/` (not processed until M6).
    Logic,
    /// File under `Assets/<category>/`.
    Asset { category: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    /// Project-relative path with `/` separators, used in diagnostics.
    pub rel_path: String,
    pub abs_path: PathBuf,
    pub kind: FileKind,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    /// Sorted by `rel_path`.
    pub files: Vec<ProjectFile>,
}

impl Project {
    pub fn files_of<'a>(
        &'a self,
        pred: impl Fn(&FileKind) -> bool + 'a,
    ) -> impl Iterator<Item = &'a ProjectFile> + 'a {
        self.files.iter().filter(move |f| pred(&f.kind))
    }

    pub fn count(&self, pred: impl Fn(&FileKind) -> bool) -> usize {
        self.files.iter().filter(|f| pred(&f.kind)).count()
    }
}

/// Walk up from `start` to the nearest directory containing `era.toml`.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join(MANIFEST_FILE).is_file())
        .map(Path::to_path_buf)
}

/// Load the project at `root`. Always returns every diagnostic found; the
/// project itself is `None` if the manifest is unusable.
pub fn load_project(root: &Path) -> (Option<Project>, Vec<Diagnostic>) {
    let mut diags = Vec::new();

    let manifest = match fs::read_to_string(root.join(MANIFEST_FILE)) {
        Ok(src) => match parse_manifest(&src, MANIFEST_FILE) {
            Ok(m) => Some(m),
            Err(errs) => {
                diags.extend(errs);
                None
            }
        },
        Err(e) => {
            diags.push(
                Diagnostic::error("E0001", format!("cannot read {MANIFEST_FILE}: {e}"))
                    .in_file(MANIFEST_FILE),
            );
            None
        }
    };

    let mut walker = Walker {
        root,
        files: Vec::new(),
        diags: &mut diags,
    };
    walker.walk_root();
    let mut files = walker.files;
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));

    check_entry_point(root, &files, &mut diags);

    let logic = files.iter().filter(|f| f.kind == FileKind::Logic).count();
    if logic > 0 {
        diags.push(
            Diagnostic::warning(
                "W0205",
                format!("{logic} file(s) in src/{LOGIC_DIR}/ are not processed yet"),
            )
            .in_file(format!("{SRC_DIR}/{LOGIC_DIR}"))
            .with_help("state and events are planned for milestone M6"),
        );
    }

    let project = manifest.map(|manifest| Project {
        root: root.to_path_buf(),
        manifest,
        files,
    });
    (project, diags)
}

fn check_entry_point(root: &Path, files: &[ProjectFile], diags: &mut Vec<Diagnostic>) {
    if files.iter().any(|f| f.rel_path == ENTRY_FILE) {
        return;
    }
    let mut d = Diagnostic::error("E0201", format!("missing entry point `{ENTRY_FILE}`"));
    // Point out a near miss such as `src/App.era`.
    let near = files
        .iter()
        .find(|f| f.rel_path.eq_ignore_ascii_case(ENTRY_FILE));
    d = match near {
        Some(f) => d.with_help(format!(
            "found `{}`; rename it to `{ENTRY_FILE}`",
            f.rel_path
        )),
        None if !root.join(SRC_DIR).is_dir() => {
            d.with_help(format!("create `{SRC_DIR}/` with an `app.era` file"))
        }
        None => d.with_help("every ERA project starts from `src/app.era`"),
    };
    diags.push(d);
}

struct Walker<'a> {
    root: &'a Path,
    files: Vec<ProjectFile>,
    diags: &'a mut Vec<Diagnostic>,
}

/// Sorted, non-hidden directory entries. Symlinks are skipped.
fn entries(dir: &Path) -> Vec<(String, PathBuf, bool)> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<_> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            if name.starts_with('.') {
                return None;
            }
            let ft = e.file_type().ok()?;
            if ft.is_symlink() {
                return None;
            }
            Some((name, e.path(), ft.is_dir()))
        })
        .collect();
    out.sort();
    out
}

fn extension(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
}

impl Walker<'_> {
    fn rel(&self, path: &Path) -> String {
        let rel = path.strip_prefix(self.root).unwrap_or(path);
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    fn case_warning(&mut self, path: &Path, found: &str, expected: &str) {
        let d = Diagnostic::warning(
            "W0206",
            format!("folder `{found}` should be named `{expected}`"),
        )
        .in_file(self.rel(path))
        .with_help("ERA folder names are case-sensitive on Android build machines");
        self.diags.push(d);
    }

    fn walk_root(&mut self) {
        for (name, path, is_dir) in entries(self.root) {
            if !is_dir {
                continue;
            }
            for expected in [SRC_DIR, SKINS_DIR, ASSETS_DIR] {
                if name == expected {
                    match expected {
                        SRC_DIR => self.walk_src(&path, true),
                        SKINS_DIR => self.walk_skins(&path),
                        _ => self.walk_assets(&path),
                    }
                } else if name.eq_ignore_ascii_case(expected) {
                    self.case_warning(&path, &name, expected);
                }
            }
        }
    }

    fn walk_src(&mut self, dir: &Path, top: bool) {
        for (name, path, is_dir) in entries(dir) {
            if is_dir {
                if top {
                    if name == LOGIC_DIR {
                        self.walk_logic(&path);
                        continue;
                    }
                    if let Some(exp) = SRC_FOLDERS
                        .iter()
                        .find(|f| **f != name && f.eq_ignore_ascii_case(&name))
                    {
                        self.case_warning(&path, &name, exp);
                    }
                }
                self.walk_src(&path, false);
                continue;
            }
            match extension(&name).as_deref() {
                Some("era") => self.add_text_file(path, FileKind::Ui),
                Some("ers") => {
                    let d = Diagnostic::warning("W0202", "skin file outside `Skins/` is ignored")
                        .in_file(self.rel(&path))
                        .with_help("move `.ers` files into the `Skins/` folder");
                    self.diags.push(d);
                }
                _ => self.unexpected_file(&path, "`src/` holds `.era` files"),
            }
        }
    }

    fn walk_logic(&mut self, dir: &Path) {
        for (_, path, is_dir) in entries(dir) {
            if is_dir {
                self.walk_logic(&path);
            } else {
                let rel_path = self.rel(&path);
                self.files.push(ProjectFile {
                    rel_path,
                    abs_path: path,
                    kind: FileKind::Logic,
                });
            }
        }
    }

    fn walk_skins(&mut self, dir: &Path) {
        for (name, path, is_dir) in entries(dir) {
            if is_dir {
                self.walk_skins(&path);
                continue;
            }
            match extension(&name).as_deref() {
                Some("ers") => self.add_text_file(path, FileKind::Skin),
                Some("era") => {
                    let d = Diagnostic::warning("W0203", "UI file inside `Skins/` is ignored")
                        .in_file(self.rel(&path))
                        .with_help("move `.era` files into `src/`");
                    self.diags.push(d);
                }
                _ => self.unexpected_file(&path, "`Skins/` holds `.ers` files"),
            }
        }
    }

    fn walk_assets(&mut self, dir: &Path) {
        for (name, path, is_dir) in entries(dir) {
            if !is_dir {
                let d = Diagnostic::warning("W0302", "asset is not inside a category folder")
                    .in_file(self.rel(&path))
                    .with_help("move it into `Images/`, `Icons/`, `Fonts/` or `Files/`");
                self.diags.push(d);
                continue;
            }
            match assets::category_extensions(&name) {
                Some(exts) => self.walk_category(&path, &name, exts),
                None => {
                    if let Some((exp, _)) = assets::CATEGORIES
                        .iter()
                        .find(|(c, _)| c.eq_ignore_ascii_case(&name))
                    {
                        self.case_warning(&path, &name, exp);
                    } else {
                        let d =
                            Diagnostic::warning("W0303", format!("unknown asset folder `{name}`"))
                                .in_file(self.rel(&path))
                                .with_help(
                                    "asset folders are `Images/`, `Icons/`, `Fonts/` and `Files/`",
                                );
                        self.diags.push(d);
                    }
                }
            }
        }
    }

    fn walk_category(&mut self, dir: &Path, category: &str, exts: &[&str]) {
        for (name, path, is_dir) in entries(dir) {
            if is_dir {
                self.walk_category(&path, category, exts);
                continue;
            }
            let ext = extension(&name).unwrap_or_default();
            if !exts.is_empty() && !exts.contains(&ext.as_str()) {
                let d = Diagnostic::warning(
                    "W0301",
                    format!("`{name}` is not a supported {category} file"),
                )
                .in_file(self.rel(&path))
                .with_help(format!("{category}/ accepts: {}", exts.join(", ")));
                self.diags.push(d);
                continue;
            }
            let rel_path = self.rel(&path);
            self.files.push(ProjectFile {
                rel_path,
                abs_path: path,
                kind: FileKind::Asset {
                    category: category.to_string(),
                },
            });
        }
    }

    fn unexpected_file(&mut self, path: &Path, help: &str) {
        let d = Diagnostic::warning("W0204", "unexpected file is ignored")
            .in_file(self.rel(path))
            .with_help(help.to_string());
        self.diags.push(d);
    }

    /// Source files must be valid UTF-8.
    fn add_text_file(&mut self, path: PathBuf, kind: FileKind) {
        let rel_path = self.rel(&path);
        match fs::read(&path) {
            Ok(bytes) if std::str::from_utf8(&bytes).is_err() => {
                self.diags.push(
                    Diagnostic::error("E0207", "source file is not valid UTF-8")
                        .in_file(rel_path.clone())
                        .with_help("save the file with UTF-8 encoding"),
                );
            }
            Err(e) => {
                self.diags.push(
                    Diagnostic::error("E0208", format!("cannot read file: {e}"))
                        .in_file(rel_path.clone()),
                );
                return;
            }
            Ok(_) => {}
        }
        self.files.push(ProjectFile {
            rel_path,
            abs_path: path,
            kind,
        });
    }
}
