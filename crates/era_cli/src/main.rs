//! `era` command-line entry point.

mod android;
mod check;
mod icon;
mod preview;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use era_diagnostics::Diagnostic;
use era_project::{FileKind, Project, find_root, load_project, scaffold};

const USAGE: &str = "\
era — design-first native app toolchain

USAGE:
    era <command> [args]

COMMANDS:
    new <name>              Create a project in a new folder <name>
    check [path] [--ir]     Validate the project containing [path] (default: current folder)
                            --ir also prints the resolved UI IR
    preview [path]          Open a desktop preview window of every screen
    preview [path] --png [dir]
                            Render every screen to PNG (default dir: build/preview/png)
    build android [path] [--release]
                            Build an Android debug APK, or with --release a signed,
                            R8-optimised APK and App Bundle (needs JDK 17+,
                            Gradle 8.x, ANDROID_HOME)
    export android [path] [--out dir] [--force]
                            Write a standalone Android Studio project you own
                            (default dir: export/android)
    --version               Print version
    --help                  Print this help";

/// Exit codes: 0 success, 1 the project has errors (or a build failed),
/// 2 usage or environment error.
const EXIT_ERRORS: u8 = 1;
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["--help" | "-h" | "help"] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["--version" | "-V"] => {
            println!("era {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["new", name] => cmd_new(Path::new(name)),
        ["new"] => usage_error("`era new` needs a project name, e.g. `era new my_app`"),
        ["new", ..] => usage_error("too many arguments for `era new`"),
        ["check", rest @ ..] => {
            let ir = rest.contains(&"--ir");
            let paths: Vec<&str> = rest.iter().copied().filter(|a| *a != "--ir").collect();
            match paths.as_slice() {
                [] => cmd_check(None, ir),
                [p] if !p.starts_with('-') => cmd_check(Some(Path::new(p)), ir),
                _ => usage_error("usage: era check [path] [--ir]"),
            }
        }
        ["preview", rest @ ..] => match parse_preview_args(rest) {
            Ok((path, png)) => cmd_preview(path.as_deref(), png),
            Err(msg) => usage_error(&msg),
        },
        ["build", "android", rest @ ..] => match parse_build_args(rest) {
            Ok((path, release)) => cmd_build_android(path.as_deref(), release),
            Err(msg) => usage_error(&msg),
        },
        ["export", "android", rest @ ..] => match parse_export_args(rest) {
            Ok((path, out, force)) => cmd_export_android(path.as_deref(), out.as_deref(), force),
            Err(msg) => usage_error(&msg),
        },
        ["export", "ios", ..] => {
            eprintln!(
                "error: iOS export (SwiftUI) is planned after production Android (see DECISIONS D-082)"
            );
            ExitCode::from(EXIT_USAGE)
        }
        ["export", ..] => usage_error("usage: era export android [path] [--out dir] [--force]"),
        ["build", "ios", ..] => {
            eprintln!("error: iOS output is not part of the ERA MVP (see DECISIONS D-002)");
            ExitCode::from(EXIT_USAGE)
        }
        ["build", ..] => usage_error("usage: era build android [path] [--release]"),
        [other, ..] => usage_error(&format!("unknown command `{other}`")),
    }
}

/// `[path] [--png [dir]]`. A value after `--png` is the output folder.
#[allow(clippy::type_complexity)]
fn parse_preview_args(args: &[&str]) -> Result<(Option<PathBuf>, Option<Option<PathBuf>>), String> {
    let usage = "usage: era preview [path] [--png [dir]]".to_string();
    let mut path = None;
    let mut png = None;
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--png" => {
                let dir = args
                    .get(i + 1)
                    .filter(|a| !a.starts_with('-'))
                    .map(PathBuf::from);
                if dir.is_some() {
                    i += 1;
                }
                png = Some(dir);
            }
            a if a.starts_with('-') => return Err(usage),
            a if path.is_none() => path = Some(PathBuf::from(a)),
            _ => return Err(usage),
        }
        i += 1;
    }
    Ok((path, png))
}

#[allow(clippy::type_complexity)]
fn parse_build_args(args: &[&str]) -> Result<(Option<PathBuf>, bool), String> {
    let usage = "usage: era build android [path] [--release]".to_string();
    let (mut path, mut release) = (None, false);
    for a in args {
        match *a {
            "--release" => release = true,
            a if a.starts_with('-') => return Err(usage),
            a if path.is_none() => path = Some(PathBuf::from(a)),
            _ => return Err(usage),
        }
    }
    Ok((path, release))
}

fn parse_export_args(args: &[&str]) -> Result<(Option<PathBuf>, Option<PathBuf>, bool), String> {
    let usage = "usage: era export android [path] [--out dir] [--force]".to_string();
    let (mut path, mut out, mut force) = (None, None, false);
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--force" => force = true,
            "--out" => {
                out = Some(PathBuf::from(args.get(i + 1).ok_or(usage.clone())?));
                i += 1;
            }
            a if a.starts_with('-') => return Err(usage),
            a if path.is_none() => path = Some(PathBuf::from(a)),
            _ => return Err(usage),
        }
        i += 1;
    }
    Ok((path, out, force))
}

fn cmd_export_android(path: Option<&Path>, out: Option<&Path>, force: bool) -> ExitCode {
    let a = match analyze(path) {
        Ok(a) => a,
        Err(code) => return code,
    };
    let Some(app) = &a.report.app else {
        eprintln!("error: no UI to export");
        return ExitCode::from(EXIT_ERRORS);
    };
    let out = out.map(|o| {
        if o.is_absolute() {
            o.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|c| c.join(o))
                .unwrap_or_else(|_| o.to_path_buf())
        }
    });
    android::export(&a.root, &a.project.manifest, app, out.as_deref(), force)
}

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("error: {msg}\n\n{USAGE}");
    ExitCode::from(EXIT_USAGE)
}

fn cmd_new(dir: &Path) -> ExitCode {
    match scaffold::create_project(dir) {
        Ok(created) => {
            println!(
                "Created ERA project `{}` in {}/",
                created.name,
                dir.display()
            );
            for f in &created.files {
                println!("  {f}");
            }
            println!("\nNext:\n  cd {}\n  era check", dir.display());
            ExitCode::SUCCESS
        }
        Err(d) => {
            eprint!("{}", d.render(None));
            ExitCode::from(EXIT_USAGE)
        }
    }
}

/// A project that passed every check.
struct Analyzed {
    root: PathBuf,
    project: Project,
    report: check::SyntaxReport,
    warnings: usize,
}

/// Find, load and fully check a project, printing diagnostics.
fn analyze(path: Option<&Path>) -> Result<Analyzed, ExitCode> {
    let start: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("error: cannot read current folder: {e}");
                return Err(ExitCode::from(EXIT_USAGE));
            }
        },
    };
    let start = std::fs::canonicalize(&start).unwrap_or(start);

    let Some(root) = find_root(&start) else {
        let d = Diagnostic::error(
            "E0001",
            format!(
                "no `era.toml` found in `{}` or any parent folder",
                start.display()
            ),
        )
        .with_help("run `era new <name>` to create a project");
        eprint!("{}", d.render(None));
        return Err(ExitCode::from(EXIT_USAGE));
    };

    let (project, mut diags) = load_project(&root);
    let report = project.as_ref().map(check::check_sources);
    if let Some(r) = &report {
        diags.extend(r.diagnostics.iter().cloned());
    }
    for d in &diags {
        eprint!("{}", render_with_source(d, &root));
        eprintln!();
    }
    let errors = diags.iter().filter(|d| d.is_error()).count();
    let warnings = diags.len() - errors;
    if errors > 0 {
        eprintln!("error: check failed: {errors} error(s), {warnings} warning(s)");
        return Err(ExitCode::from(EXIT_ERRORS));
    }
    Ok(Analyzed {
        root,
        project: project.expect("project loads when there are no errors"),
        report: report.expect("report exists when the project loads"),
        warnings,
    })
}

fn cmd_check(path: Option<&Path>, print_ir: bool) -> ExitCode {
    let a = match analyze(path) {
        Ok(a) => a,
        Err(code) => return code,
    };
    let project = &a.project;
    println!(
        "Checked `{}` ({}): {} UI file(s), {} skin file(s), {} asset(s), {} warning(s)",
        project.manifest.name,
        project.manifest.id,
        project.count(|k| *k == FileKind::Ui),
        project.count(|k| *k == FileKind::Skin),
        project.count(|k| matches!(k, FileKind::Asset { .. })),
        a.warnings,
    );
    println!("Declarations: {}", a.report.counts.summary());
    if let Some(app) = &a.report.app {
        println!("UI IR: {}", ir_summary(app));
        if print_ir {
            print!("\n{}", era_ir::dump::dump(app));
        }
    }
    ExitCode::SUCCESS
}

fn ir_summary(app: &era_ir::ir::App) -> String {
    let skin = if app.skin.builtin {
        "built-in default skin".to_string()
    } else {
        format!("skin `{}`", app.skin.name)
    };
    format!(
        "{} screen(s), {} element(s), {skin}, {} asset reference(s), start screen `{}`",
        app.screens.len(),
        check::element_count(app),
        app.assets.len(),
        app.start
    )
}

fn cmd_preview(path: Option<&Path>, png: Option<Option<PathBuf>>) -> ExitCode {
    let a = match analyze(path) {
        Ok(a) => a,
        Err(code) => return code,
    };
    let Some(app) = &a.report.app else {
        eprintln!("error: no UI to preview");
        return ExitCode::from(EXIT_ERRORS);
    };
    println!("Checked `{}`: {}", a.project.manifest.name, ir_summary(app));
    let png = png.map(|dir| {
        let dir = dir.unwrap_or_else(|| a.root.join("build/preview/png"));
        if dir.is_absolute() {
            dir
        } else {
            std::env::current_dir().map(|c| c.join(&dir)).unwrap_or(dir)
        }
    });
    preview::run(&a.root, app, png.as_deref())
}

fn cmd_build_android(path: Option<&Path>, release: bool) -> ExitCode {
    let a = match analyze(path) {
        Ok(a) => a,
        Err(code) => return code,
    };
    let Some(app) = &a.report.app else {
        eprintln!("error: no UI to build");
        return ExitCode::from(EXIT_ERRORS);
    };
    println!("Checked `{}`: {}", a.project.manifest.name, ir_summary(app));
    android::run(&a.root, &a.project.manifest, app, release)
}

/// Render a diagnostic, loading its file's text when it has a span.
fn render_with_source(d: &Diagnostic, root: &Path) -> String {
    let source = d
        .location
        .as_ref()
        .filter(|loc| loc.span.is_some())
        .and_then(|loc| std::fs::read_to_string(root.join(&loc.path)).ok());
    d.render(source.as_deref())
}
