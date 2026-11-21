//! `era new`: create a minimal project.

use std::fs;
use std::path::{Path, PathBuf};

use era_diagnostics::Diagnostic;

use crate::names;
use crate::{ENTRY_FILE, MANIFEST_FILE};

pub struct Created {
    pub root: PathBuf,
    pub name: String,
    /// Project-relative paths of the files written, in order.
    pub files: Vec<&'static str>,
}

pub fn manifest_template(name: &str) -> String {
    format!(
        r#"[project]
name    = "{name}"
id      = "dev.era.{name}"
version = "0.1.0"

[era]
edition = "2026"
"#
    )
}

/// Minimal app in the approved M0 syntax. It does not use a skin, so it uses
/// plain numbers instead of tokens (see DECISIONS D-020).
pub fn app_template(display_name: &str) -> String {
    format!(
        r#"app "{display_name}" {{
    start: Home
}}

screen Home {{
    title: "Home"

    column {{
        padding: 24
        gap: 8
        text "Hello from ERA"
        text "Edit src/app.era to start designing."
    }}
}}
"#
    )
}

const GITIGNORE: &str = "/build\n";

/// Create a new project in `dir`. The directory's last component is the
/// project name. `dir` must not exist, or must be empty.
pub fn create_project(dir: &Path) -> Result<Created, Diagnostic> {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();

    if let Err(msg) = names::check_project_name(&name) {
        let mut d = Diagnostic::error("E0110", format!("invalid project name `{name}`: {msg}"));
        if let Some(s) = names::suggest_project_name(&name) {
            d = d.with_help(format!("try `era new {s}`"));
        }
        return Err(d);
    }

    if dir.exists() {
        let non_empty = fs::read_dir(dir)
            .map(|mut r| r.next().is_some())
            .unwrap_or(true);
        if !dir.is_dir() || non_empty {
            return Err(Diagnostic::error(
                "E0111",
                format!("`{}` already exists and is not empty", dir.display()),
            )
            .with_help("choose a new name, or remove the existing folder"));
        }
    }

    let io_err =
        |e: std::io::Error| Diagnostic::error("E0112", format!("cannot create project: {e}"));
    fs::create_dir_all(dir.join("src")).map_err(io_err)?;
    fs::write(dir.join(MANIFEST_FILE), manifest_template(&name)).map_err(io_err)?;
    fs::write(
        dir.join(ENTRY_FILE),
        app_template(&names::display_name(&name)),
    )
    .map_err(io_err)?;
    fs::write(dir.join(".gitignore"), GITIGNORE).map_err(io_err)?;

    Ok(Created {
        root: dir.to_path_buf(),
        name,
        files: vec![MANIFEST_FILE, ENTRY_FILE, ".gitignore"],
    })
}
