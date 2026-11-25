//! The full check pipeline over a loaded project: parse every source file,
//! run structural checks, then (if those pass) build the UI IR.
//!
//! Shared by the `era` CLI and ERA Studio. Studio passes the text of unsaved
//! editor buffers through `read`, so problems update as you type.

use std::collections::BTreeSet;

use era_diagnostics::Diagnostic;
use era_project::{ENTRY_FILE, FileKind, Project, ProjectFile};
use era_syntax::{DeclKind, File, SourceKind, SourceUnit, check_structure, parse};

pub struct SyntaxReport {
    pub diagnostics: Vec<Diagnostic>,
    /// The UI IR, when syntax, structure and semantics are all error-free.
    pub app: Option<crate::ir::App>,
    /// Number of declarations per kind, plus distinct skin names.
    pub counts: DeclCounts,
}

#[derive(Default)]
pub struct DeclCounts {
    pub apps: usize,
    pub screens: usize,
    pub layouts: usize,
    pub components: usize,
    pub skins: usize,
    pub stores: usize,
}

impl DeclCounts {
    pub fn summary(&self) -> String {
        let plural = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
        [
            plural(self.apps, "app"),
            plural(self.screens, "screen"),
            plural(self.layouts, "layout"),
            plural(self.components, "component"),
            plural(self.skins, "skin"),
        ]
        .into_iter()
        .chain((self.stores > 0).then(|| plural(self.stores, "store")))
        .collect::<Vec<_>>()
        .join(", ")
    }
}

/// Check a project, reading sources from disk.
pub fn check_sources(project: &Project) -> SyntaxReport {
    check_sources_with(project, &|f| std::fs::read_to_string(&f.abs_path).ok())
}

/// Check a project; `read` supplies each source file's text (for example an
/// unsaved editor buffer) and returns `None` for unreadable files.
pub fn check_sources_with(
    project: &Project,
    read: &dyn Fn(&ProjectFile) -> Option<String>,
) -> SyntaxReport {
    let mut diagnostics = Vec::new();
    let mut parsed: Vec<(&str, String, SourceKind, File)> = Vec::new();

    for f in &project.files {
        let kind = match f.kind {
            FileKind::Ui => SourceKind::Ui,
            FileKind::Skin => SourceKind::Skin,
            FileKind::Logic => SourceKind::Logic,
            _ => continue,
        };
        // Unreadable or non-UTF-8 files were already reported (E0207/E0208).
        let Some(source) = read(f) else {
            continue;
        };
        let (file, diags) = parse(&source, &f.rel_path);
        diagnostics.extend(diags);
        parsed.push((f.rel_path.as_str(), source, kind, file));
    }

    // The entry file goes first, so "first declared at" points at app.era.
    parsed.sort_by_key(|(path, ..)| *path != ENTRY_FILE);
    let units: Vec<SourceUnit> = parsed
        .iter()
        .map(|(path, source, kind, file)| SourceUnit {
            path,
            source,
            kind: *kind,
            is_entry: *path == ENTRY_FILE,
            file,
        })
        .collect();
    diagnostics.extend(check_structure(&units, ENTRY_FILE));

    // Semantic analysis only runs on a structurally valid program, so
    // broken syntax does not produce follow-on noise.
    let mut app = None;
    if !diagnostics.iter().any(Diagnostic::is_error) {
        let files: Vec<crate::ParsedFile> = parsed
            .iter()
            .map(|(path, source, kind, file)| crate::ParsedFile {
                path,
                source,
                kind: *kind,
                file,
            })
            .collect();
        let (built, diags) = crate::build_app(project, &files);
        diagnostics.extend(diags);
        app = built;
    }

    let mut counts = DeclCounts::default();
    let mut skin_names = BTreeSet::new();
    for (_, _, _, file) in &parsed {
        for d in &file.decls {
            match d.kind {
                DeclKind::App => counts.apps += 1,
                DeclKind::Screen => counts.screens += 1,
                DeclKind::Layout => counts.layouts += 1,
                DeclKind::Component => counts.components += 1,
                DeclKind::Store => counts.stores += 1,
                DeclKind::Skin => {
                    skin_names.insert(d.name.name.clone());
                }
            }
        }
    }
    counts.skins = skin_names.len();

    SyntaxReport {
        diagnostics,
        app,
        counts,
    }
}

/// Count IR elements, for the summary line.
pub fn element_count(app: &crate::ir::App) -> usize {
    fn walk(e: &crate::ir::Element) -> usize {
        1 + e.children.iter().map(walk).sum::<usize>()
    }
    app.screens.iter().map(|s| walk(&s.root)).sum()
}
