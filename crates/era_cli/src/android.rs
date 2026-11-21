//! `era build android`: generate an Android project into
//! `<project>/build/android/` and build a debug APK with Gradle.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use era_codegen_compose::android as android_gen;
use era_diagnostics::Diagnostic;
use era_ir::ir::App;
use era_project::Manifest;

use crate::preview::{gradle_command, remove_stale, write_if_changed};
use crate::{EXIT_ERRORS, EXIT_USAGE};

const ROOT_FILES: &[(&str, &str)] = &[
    (
        "settings.gradle.kts",
        include_str!("../../../platforms/android/settings.gradle.kts"),
    ),
    (
        "build.gradle.kts",
        include_str!("../../../platforms/android/build.gradle.kts"),
    ),
    (
        "gradle.properties",
        include_str!("../../../platforms/android/gradle.properties"),
    ),
    (
        "src/main/AndroidManifest.xml",
        include_str!("../../../platforms/android/AndroidManifest.xml"),
    ),
];
const HOST_KOTLIN: &[(&str, &str)] = &[
    (
        "MainActivity.kt",
        include_str!("../../../platforms/android/MainActivity.kt"),
    ),
    (
        "EraAssets.kt",
        include_str!("../../../platforms/android/EraAssets.kt"),
    ),
];

pub const APK_NAME: &str = "era-android-debug.apk";

fn fill(template: &str, vars: &[(&str, String)]) -> String {
    let mut s = template.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{{{k}}}}}"), v);
    }
    s
}

/// Copy a file only when its bytes differ.
fn copy_if_changed(from: &Path, to: &Path) -> std::io::Result<()> {
    let new = std::fs::read(from)?;
    if std::fs::read(to).ok().as_deref() == Some(new.as_slice()) {
        return Ok(());
    }
    if let Some(p) = to.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(to, new)
}

/// The Android SDK from `ANDROID_HOME` or `ANDROID_SDK_ROOT`.
pub fn find_sdk() -> Option<PathBuf> {
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .find(|p| p.join("platforms").is_dir())
}

/// Write the Android project. Icon conversion problems are diagnostics.
pub fn write_project(
    root: &Path,
    manifest: &Manifest,
    app: &App,
    sdk: &Path,
) -> Result<PathBuf, Vec<Diagnostic>> {
    let io = |e: std::io::Error| {
        vec![Diagnostic::error(
            "E3002",
            format!("cannot write the Android project: {e}"),
        )]
    };
    let out = root.join("build/android");
    let generated = era_codegen_compose::generate(app);
    let pkg = generated.package.clone();
    let assets = root.join("Assets");
    let android = android_gen::generate(app, &pkg, |p| {
        std::fs::read_to_string(assets.join(p)).map_err(|e| format!("cannot read file: {e}"))
    })
    .map_err(|problems| {
        problems
            .into_iter()
            .map(|p| {
                Diagnostic::error("E3001", format!("cannot convert icon `{}` for Android: {}", p.path, p.message))
                    .in_file(format!("Assets/{}", p.path))
                    .with_help("simplify the SVG (outline strokes, flatten transforms) or export it again from your design tool")
            })
            .collect::<Vec<_>>()
    })?;

    let vars = [
        ("APP_ID", app.id.clone()),
        ("PACKAGE", pkg.clone()),
        ("MIN_SDK", manifest.android.min_sdk.to_string()),
        (
            "VERSION_CODE",
            android_gen::version_code(&app.version).to_string(),
        ),
        ("VERSION_NAME", app.version.clone()),
    ];
    let main = out.join("src/main");
    let mut keep = BTreeSet::new();
    for (name, text) in ROOT_FILES {
        let p = out.join(name);
        write_if_changed(&p, &fill(text, &vars)).map_err(io)?;
        keep.insert(p);
    }
    let kotlin = main.join("kotlin");
    for (name, text) in HOST_KOTLIN {
        let p = kotlin.join(pkg.replace('.', "/")).join(name);
        write_if_changed(&p, &fill(text, &vars)).map_err(io)?;
        keep.insert(p);
    }
    for (rel, text) in &generated.files {
        let p = kotlin.join(rel);
        write_if_changed(&p, text).map_err(io)?;
        keep.insert(p);
    }
    for (rel, text) in &android.files {
        let p = out.join(rel);
        write_if_changed(&p, text).map_err(io)?;
        keep.insert(p);
    }
    for (from, to) in &android.copies {
        let p = out.join(to);
        copy_if_changed(&assets.join(from), &p).map_err(io)?;
        keep.insert(p);
    }
    remove_stale(&main, &keep);

    let sdk_dir = sdk
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace(':', "\\:");
    write_if_changed(
        &out.join("local.properties"),
        &format!("# Written by `era build android`.\nsdk.dir={sdk_dir}\n"),
    )
    .map_err(io)?;
    Ok(out)
}

pub fn run(root: &Path, manifest: &Manifest, app: &App) -> ExitCode {
    let Some(sdk) = find_sdk() else {
        eprintln!("error: Android SDK not found");
        eprintln!(
            "   = help: install the Android SDK (platform 35, build-tools) and set ANDROID_HOME to its folder"
        );
        return ExitCode::from(EXIT_USAGE);
    };
    let out = match write_project(root, manifest, app, &sdk) {
        Ok(o) => o,
        Err(diags) => {
            for d in &diags {
                eprint!("{}", d.render(None));
                eprintln!();
            }
            eprintln!("error: build failed: {} error(s)", diags.len());
            return ExitCode::from(EXIT_ERRORS);
        }
    };
    println!("Generated Android project in {}", out.display());
    println!(
        "Building debug APK with Gradle (the first run downloads the Android build tools and can take several minutes)..."
    );

    let gradle = gradle_command();
    let status = Command::new(&gradle)
        .args(["--console=plain", "-q", "--no-watch-fs", "assembleDebug"])
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
                "error: the Android build failed (Gradle exit code {})",
                s.code().unwrap_or(-1)
            );
            eprintln!(
                "   = help: this is a bug in ERA's code generation if `era check` passes; please report it"
            );
            ExitCode::from(EXIT_ERRORS)
        }
        Ok(_) => {
            let apk = out.join("build/outputs/apk/debug").join(APK_NAME);
            let size = std::fs::metadata(&apk).map(|m| m.len()).unwrap_or(0);
            println!(
                "Built {} ({:.1} MB)",
                apk.display(),
                size as f64 / 1_048_576.0
            );
            println!("\nInstall and launch on a connected device or emulator:");
            println!("  adb install -r \"{}\"", apk.display());
            println!(
                "  adb shell am start -n {}/{}.MainActivity",
                app.id,
                era_codegen_compose::package_for(app)
            );
            ExitCode::SUCCESS
        }
    }
}
