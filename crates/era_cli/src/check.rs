//! `era check` pipeline: project structure (M1), then syntax and structural
//! checks for every `.era` and `.ers` file (M2).

use std::collections::BTreeSet;

use era_diagnostics::Diagnostic;
use era_project::{ENTRY_FILE, FileKind, Project};
use era_syntax::{DeclKind, File, SourceKind, SourceUnit, check_structure, parse};

pub struct SyntaxReport {
    pub diagnostics: Vec<Diagnostic>,
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
        .join(", ")
    }
}

/// Parse every UI and skin file and run structural checks.
pub fn check_sources(project: &Project) -> SyntaxReport {
    let mut diagnostics = Vec::new();
    let mut parsed: Vec<(&str, String, SourceKind, File)> = Vec::new();

    for f in &project.files {
        let kind = match f.kind {
            FileKind::Ui => SourceKind::Ui,
            FileKind::Skin => SourceKind::Skin,
            _ => continue,
        };
        // Unreadable or non-UTF-8 files were already reported (E0207/E0208).
        let Ok(source) = std::fs::read_to_string(&f.abs_path) else {
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

    let mut counts = DeclCounts::default();
    let mut skin_names = BTreeSet::new();
    for (_, _, _, file) in &parsed {
        for d in &file.decls {
            match d.kind {
                DeclKind::App => counts.apps += 1,
                DeclKind::Screen => counts.screens += 1,
                DeclKind::Layout => counts.layouts += 1,
                DeclKind::Component => counts.components += 1,
                DeclKind::Skin => {
                    skin_names.insert(d.name.name.clone());
                }
            }
        }
    }
    counts.skins = skin_names.len();

    SyntaxReport {
        diagnostics,
        counts,
    }
}
