//! `era preview`: generate Compose code plus the desktop host into
//! `<project>/build/preview/`, then run it with Gradle.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use era_ir::ir::App;

use crate::{EXIT_ERRORS, EXIT_USAGE};

const HOST: &[(&str, &str)] = &[
    (
        "settings.gradle.kts",
        include_str!("../../../platforms/desktop-preview/settings.gradle.kts"),
    ),
    (
        "build.gradle.kts",
        include_str!("../../../platforms/desktop-preview/build.gradle.kts"),
    ),
];
const HOST_KOTLIN: &[(&str, &str)] = &[
    (
        "EraAssets.kt",
        include_str!("../../../platforms/desktop-preview/EraAssets.kt"),
    ),
    (
        "PreviewMain.kt",
        include_str!("../../../platforms/desktop-preview/PreviewMain.kt"),
    ),
];

/// Write `contents` only when it differs, so Gradle's up-to-date checks work.
pub(crate) fn write_if_changed(path: &Path, contents: &str) -> std::io::Result<()> {
    if std::fs::read_to_string(path).ok().as_deref() == Some(contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)
}

/// Remove files under `dir` that are not in `keep` (stale generated code).
pub(crate) fn remove_stale(dir: &Path, keep: &BTreeSet<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            remove_stale(&p, keep);
        } else if !keep.contains(&p) {
            let _ = std::fs::remove_file(&p);
        }
    }
}

/// Generate the preview project. Returns its folder.
pub fn write_project(root: &Path, app: &App, png: Option<&Path>) -> std::io::Result<PathBuf> {
    let out = root.join("build/preview");
    let generated = era_codegen_compose::generate(app);
    let pkg = &generated.package;
    let pkg_dir = pkg.replace('.', "/");
    let src = out.join("src/main/kotlin");

    for (name, text) in HOST {
        write_if_changed(&out.join(name), &text.replace("{{PACKAGE}}", pkg))?;
    }
    let mut keep = BTreeSet::new();
    for (name, text) in HOST_KOTLIN {
        let p = src.join(&pkg_dir).join(name);
        write_if_changed(&p, &text.replace("{{PACKAGE}}", pkg))?;
        keep.insert(p);
    }
    for (rel, text) in &generated.files {
        let p = src.join(rel);
        write_if_changed(&p, text)?;
        keep.insert(p);
    }
    remove_stale(&src, &keep);

    // Java properties: escape backslashes (Windows paths) and colons.
    let prop = |p: &Path| {
        p.display()
            .to_string()
            .replace('\\', "\\\\")
            .replace(':', "\\:")
    };
    let props = format!(
        "# Written by `era preview`.\nassets={}\npng={}\n",
        prop(&root.join("Assets")),
        png.map(prop).unwrap_or_default()
    );
    write_if_changed(&out.join("era-preview.properties"), &props)?;
    Ok(out)
}

pub(crate) fn gradle_command() -> String {
    std::env::var("ERA_GRADLE").unwrap_or_else(|_| "gradle".into())
}

pub fn run(root: &Path, app: &App, png: Option<&Path>) -> ExitCode {
    if png.is_none()
        && cfg!(target_os = "linux")
        && std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
    {
        eprintln!("error: no display found for the preview window");
        eprintln!("   = help: run `era preview --png` to render every screen to PNG instead");
        return ExitCode::from(EXIT_USAGE);
    }

    let out = match write_project(root, app, png) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: cannot write the preview project: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    println!("Generated Compose code in {}", out.display());
    println!(
        "Building preview with Gradle (the first run downloads Kotlin and Compose and can take a few minutes)..."
    );

    let gradle = gradle_command();
    let status = Command::new(&gradle)
        .args(["--console=plain", "-q", "--no-watch-fs", "run"])
        .current_dir(&out)
        .status();
    match status {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("error: `{gradle}` was not found");
            eprintln!(
                "   = help: install Gradle 8.x (https://gradle.org/install/) or set ERA_GRADLE to its path"
            );
            ExitCode::from(EXIT_USAGE)
        }
        Err(e) => {
            eprintln!("error: cannot run `{gradle}`: {e}");
            ExitCode::from(EXIT_USAGE)
        }
        Ok(s) if !s.success() => {
            eprintln!(
                "error: the preview build failed (Gradle exit code {})",
                s.code().unwrap_or(-1)
            );
            eprintln!(
                "   = help: this is a bug in ERA's code generation if `era check` passes; please report it"
            );
            ExitCode::from(EXIT_ERRORS)
        }
        Ok(_) => {
            if let Some(dir) = png {
                println!("Preview images are in {}", dir.display());
            }
            ExitCode::SUCCESS
        }
    }
}
