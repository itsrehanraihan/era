#![allow(dead_code)]

use std::path::Path;

use era_diagnostics::Diagnostic;
use era_ir::ir::App;
use era_project::{FileKind, Project, load_project};
use era_syntax::{SourceKind, SourceUnit, check_structure, parse};

pub const MANIFEST: &str = "[project]\nname = \"t\"\nid = \"dev.era.t\"\nversion = \"0.1.0\"\n\n[era]\nedition = \"2026\"\n";

/// A complete skin that satisfies every requirement.
pub const SKIN: &str = r#"skin Studio {
    colors {
        background: #FFFFFF, surface: #F7F7F7, text: #171717, muted: #737373, border: #E8E8E8
        primary: #FF6B35, on_primary: #FFFFFF, secondary: #2B2D42, success: #2E9E5B
        warning: #F2A93B, danger: #E5484D
    }
    spacing { xs: 4, sm: 8, md: 16, lg: 24 }
    radius { sm: 8, md: 14, pill: 999 }
    border { thin: 1 }
    elevation { sm: 1 }
    typography { body { size: 16 }, heading { size: 28, weight: bold } }
}
"#;

pub fn write(root: &Path, rel: &str, contents: &[u8]) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, contents).unwrap();
}

pub struct Built {
    pub app: Option<App>,
    pub diags: Vec<Diagnostic>,
    _dir: tempfile::TempDir,
}

impl Built {
    pub fn codes(&self) -> Vec<&'static str> {
        self.diags.iter().map(|d| d.code).collect()
    }

    pub fn app(&self) -> &App {
        self.app
            .as_ref()
            .unwrap_or_else(|| panic!("no app; diagnostics: {:#?}", self.diags))
    }

    pub fn only(&self, code: &str) -> &Diagnostic {
        assert_eq!(self.codes(), [code], "{:#?}", self.diags);
        &self.diags[0]
    }
}

/// Build a project from `(path, contents)` pairs. Parse and structure errors
/// fail the test: these tests target the semantic stage only.
pub fn build(files: &[(&str, &str)]) -> Built {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", MANIFEST.as_bytes());
    for (p, c) in files {
        write(dir.path(), p, c.as_bytes());
    }
    build_dir(dir)
}

pub fn build_dir(dir: tempfile::TempDir) -> Built {
    let (project, _) = load_project(dir.path());
    let project = project.expect("manifest is valid");
    let (app, diags) = build_project(&project);
    Built {
        app,
        diags,
        _dir: dir,
    }
}

pub fn build_project(project: &Project) -> (Option<App>, Vec<Diagnostic>) {
    let mut sources = Vec::new();
    for f in &project.files {
        let kind = match f.kind {
            FileKind::Ui => SourceKind::Ui,
            FileKind::Skin => SourceKind::Skin,
            FileKind::Logic => SourceKind::Logic,
            _ => continue,
        };
        let text = std::fs::read_to_string(&f.abs_path).unwrap();
        let (file, d) = parse(&text, &f.rel_path);
        assert!(d.is_empty(), "{}: {d:#?}", f.rel_path);
        sources.push((f.rel_path.clone(), text, kind, file));
    }
    sources.sort_by_key(|(p, ..)| p != "src/app.era");
    let units: Vec<SourceUnit> = sources
        .iter()
        .map(|(p, t, k, f)| SourceUnit {
            path: p,
            source: t,
            kind: *k,
            is_entry: p == "src/app.era",
            file: f,
        })
        .collect();
    let d = check_structure(&units, "src/app.era");
    assert!(d.is_empty(), "structure: {d:#?}");
    let parsed: Vec<era_ir::ParsedFile> = sources
        .iter()
        .map(|(p, t, k, f)| era_ir::ParsedFile {
            path: p,
            source: t,
            kind: *k,
            file: f,
        })
        .collect();
    era_ir::build_app(project, &parsed)
}

/// One screen whose root is `body`, with the full skin.
pub fn screen(body: &str) -> Built {
    build(&[
        ("src/app.era", "app \"T\" { skin: Studio }"),
        ("Skins/theme.ers", SKIN),
        (
            "src/Screens/Home.era",
            &format!("screen Home {{\n{body}\n}}"),
        ),
    ])
}

/// Like [`screen`] plus extra `src/` files.
pub fn with(body: &str, extra: &[(&str, &str)]) -> Built {
    let screen_src = format!("screen Home {{\n{body}\n}}");
    let mut files = vec![
        ("src/app.era", "app \"T\" { skin: Studio, start: Home }"),
        ("Skins/theme.ers", SKIN),
        ("src/Screens/Home.era", screen_src.as_str()),
    ];
    files.extend_from_slice(extra);
    build(&files)
}
