//! `era` command-line entry point.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use era_diagnostics::Diagnostic;
use era_project::{FileKind, find_root, load_project, scaffold};

const USAGE: &str = "\
era — design-first native app toolchain

USAGE:
    era <command> [args]

COMMANDS:
    new <name>       Create a project in a new folder <name>
    check [path]     Validate the project containing [path] (default: current folder)
    preview          Preview screens             (planned: M4)
    build android    Build an Android debug APK  (planned: M5)
    --version        Print version
    --help           Print this help";

/// Exit codes: 0 success, 1 the project has errors, 2 usage or environment error.
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
        ["check"] => cmd_check(None),
        ["check", path] => cmd_check(Some(Path::new(path))),
        [cmd @ ("preview" | "build"), ..] => {
            eprintln!("error: `era {cmd}` is not implemented yet (see MILESTONES.md)");
            ExitCode::from(EXIT_USAGE)
        }
        [cmd @ ("new" | "check"), ..] => {
            usage_error(&format!("too many arguments for `era {cmd}`"))
        }
        [other, ..] => usage_error(&format!("unknown command `{other}`")),
    }
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

fn cmd_check(path: Option<&Path>) -> ExitCode {
    let start: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("error: cannot read current folder: {e}");
                return ExitCode::from(EXIT_USAGE);
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
        return ExitCode::from(EXIT_USAGE);
    };

    let (project, diags) = load_project(&root);
    for d in &diags {
        eprint!("{}", render_with_source(d, &root));
        eprintln!();
    }
    let errors = diags.iter().filter(|d| d.is_error()).count();
    let warnings = diags.len() - errors;

    if errors > 0 {
        eprintln!("error: check failed: {errors} error(s), {warnings} warning(s)");
        return ExitCode::from(EXIT_ERRORS);
    }

    // No errors means the manifest loaded, so a project exists.
    let project = project.expect("project loads when there are no errors");
    println!(
        "Checked `{}` ({}): {} UI file(s), {} skin file(s), {} asset(s), {} warning(s)",
        project.manifest.name,
        project.manifest.id,
        project.count(|k| *k == FileKind::Ui),
        project.count(|k| *k == FileKind::Skin),
        project.count(|k| matches!(k, FileKind::Asset { .. })),
        warnings,
    );
    println!("note: only project structure is checked; syntax and design checks arrive in M2-M3");
    ExitCode::SUCCESS
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
