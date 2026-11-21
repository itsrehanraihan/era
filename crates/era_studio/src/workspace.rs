//! The open project: files, editor buffers, and live analysis that uses
//! unsaved buffer text, so problems update while you type.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use era_diagnostics::{Diagnostic, Severity, line_col};
use era_ir::pipeline::{SyntaxReport, check_sources_with};
use era_project::{FileKind, Project, find_root, load_project};

/// An open file. `saved` is the text on disk when last loaded or saved.
#[derive(Debug, Clone)]
pub struct Buffer {
    pub text: String,
    pub saved: String,
}

impl Buffer {
    pub fn is_dirty(&self) -> bool {
        self.text != self.saved
    }
}

/// One problem, located for display and navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub help: Option<String>,
    /// Project-relative path, if the problem points at a file.
    pub path: Option<String>,
    /// 1-based line and column, when the problem has a span.
    pub line: Option<(usize, usize)>,
    /// Character index (not byte) of the span start, for moving the cursor.
    pub char_index: Option<usize>,
}

pub struct Workspace {
    pub root: PathBuf,
    pub project: Option<Project>,
    pub buffers: BTreeMap<String, Buffer>,
    load_diags: Vec<Diagnostic>,
    report: Option<SyntaxReport>,
    problems: Vec<Problem>,
}

/// A group of files in the project tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeGroup {
    pub name: &'static str,
    pub files: Vec<String>,
}

const GROUPS: &[(&str, &str)] = &[
    ("App", "src/app.era"),
    ("Screens", "src/Screens/"),
    ("Components", "src/Components/"),
    ("Layout", "src/Layout/"),
    ("Logic", "src/Logic/"),
    ("Skins", "Skins/"),
    ("Assets", "Assets/"),
];

impl Workspace {
    /// Open the project containing `path`.
    pub fn open(path: &Path) -> Result<Self, String> {
        let path = std::fs::canonicalize(path)
            .map_err(|e| format!("cannot open `{}`: {e}", path.display()))?;
        let root = find_root(&path).ok_or_else(|| {
            format!(
                "no `era.toml` found in `{}` or any parent folder",
                path.display()
            )
        })?;
        let mut ws = Workspace {
            root,
            project: None,
            buffers: BTreeMap::new(),
            load_diags: Vec::new(),
            report: None,
            problems: Vec::new(),
        };
        ws.reload();
        Ok(ws)
    }

    pub fn name(&self) -> String {
        match &self.project {
            Some(p) => p.manifest.name.clone(),
            None => self
                .root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }

    /// Re-scan the project folder and re-check everything.
    pub fn reload(&mut self) {
        let (project, diags) = load_project(&self.root);
        self.project = project;
        self.load_diags = diags;
        self.analyze();
    }

    /// Open (or return the already open) buffer for a project file.
    pub fn open_file(&mut self, rel: &str) -> std::io::Result<&mut Buffer> {
        if !self.buffers.contains_key(rel) {
            let text = std::fs::read_to_string(self.root.join(rel))?;
            self.buffers.insert(
                rel.to_string(),
                Buffer {
                    saved: text.clone(),
                    text,
                },
            );
        }
        Ok(self.buffers.get_mut(rel).unwrap())
    }

    pub fn close_file(&mut self, rel: &str) {
        self.buffers.remove(rel);
        self.analyze();
    }

    pub fn save(&mut self, rel: &str) -> std::io::Result<()> {
        if let Some(b) = self.buffers.get_mut(rel) {
            std::fs::write(self.root.join(rel), &b.text)?;
            b.saved = b.text.clone();
        }
        Ok(())
    }

    pub fn save_all(&mut self) -> std::io::Result<usize> {
        let dirty: Vec<String> = self
            .buffers
            .iter()
            .filter(|(_, b)| b.is_dirty())
            .map(|(k, _)| k.clone())
            .collect();
        for rel in &dirty {
            self.save(rel)?;
        }
        Ok(dirty.len())
    }

    pub fn dirty_count(&self) -> usize {
        self.buffers.values().filter(|b| b.is_dirty()).count()
    }

    /// Re-run the full ERA check, using unsaved buffer text where open.
    pub fn analyze(&mut self) {
        let buffers = &self.buffers;
        self.report = self.project.as_ref().map(|p| {
            check_sources_with(p, &|f| match buffers.get(&f.rel_path) {
                Some(b) => Some(b.text.clone()),
                None => std::fs::read_to_string(&f.abs_path).ok(),
            })
        });
        let mut diags = self.load_diags.clone();
        if let Some(r) = &self.report {
            diags.extend(r.diagnostics.iter().cloned());
        }
        self.problems = diags.iter().map(|d| self.locate(d)).collect();
        // Errors first, then by file and position.
        self.problems
            .sort_by(|a, b| (a.severity, &a.path, a.line).cmp(&(b.severity, &b.path, b.line)));
    }

    fn locate(&self, d: &Diagnostic) -> Problem {
        let path = d.location.as_ref().map(|l| l.path.clone());
        let mut line = None;
        let mut char_index = None;
        if let Some(loc) = &d.location
            && let Some(span) = &loc.span
        {
            let text = match self.buffers.get(&loc.path) {
                Some(b) => Some(b.text.clone()),
                None => std::fs::read_to_string(self.root.join(&loc.path)).ok(),
            };
            if let Some(text) = text {
                let start = span.start.min(text.len());
                line = Some(line_col(&text, start));
                let mut boundary = start;
                while !text.is_char_boundary(boundary) {
                    boundary -= 1;
                }
                char_index = Some(text[..boundary].chars().count());
            }
        }
        Problem {
            severity: d.severity,
            code: d.code,
            message: d.message.clone(),
            help: d.help.clone(),
            path,
            line,
            char_index,
        }
    }

    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    pub fn error_count(&self) -> usize {
        self.problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .count()
    }

    /// The resolved app, when the project has no errors.
    pub fn app(&self) -> Option<&era_ir::ir::App> {
        self.report.as_ref().and_then(|r| r.app.as_ref())
    }

    /// Project files grouped for the tree (sources and assets), sorted.
    pub fn tree(&self) -> Vec<TreeGroup> {
        let Some(project) = &self.project else {
            return Vec::new();
        };
        let mut groups: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        let mut other = Vec::new();
        for f in &project.files {
            let idx = GROUPS.iter().position(|(_, prefix)| {
                if prefix.ends_with('/') {
                    f.rel_path.starts_with(prefix)
                } else {
                    f.rel_path == *prefix
                }
            });
            match idx {
                Some(i) => groups.entry(i).or_default().push(f.rel_path.clone()),
                None if f.kind
                    != (FileKind::Asset {
                        category: String::new(),
                    }) =>
                {
                    other.push(f.rel_path.clone())
                }
                None => {}
            }
        }
        let mut out: Vec<TreeGroup> = groups
            .into_iter()
            .map(|(i, files)| TreeGroup {
                name: GROUPS[i].0,
                files,
            })
            .collect();
        if !other.is_empty() {
            out.push(TreeGroup {
                name: "Other",
                files: other,
            });
        }
        out.push(TreeGroup {
            name: "Project",
            files: vec![era_project::MANIFEST_FILE.to_string()],
        });
        out
    }

    /// Whether a project file is editable text (sources and `era.toml`).
    pub fn is_text(rel: &str) -> bool {
        [".era", ".ers", ".er", ".toml"]
            .iter()
            .any(|e| rel.ends_with(e))
    }

    /// Where `era preview --png` writes its images.
    pub fn preview_dir(&self) -> PathBuf {
        self.root.join("build/preview/png")
    }
}
