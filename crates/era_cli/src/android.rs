//! Android output. One project plan in the standard Android Studio layout
//! (`app/` module, version catalog), used by:
//! - `era build android`: writes `build/android/` and runs Gradle;
//! - `era export android`: writes a standalone project the team owns,
//!   with the Gradle wrapper, `.gitignore` and a README (D-084).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use era_codegen_compose::android as android_gen;
use era_codegen_compose::{Options, generate_with};
use era_diagnostics::Diagnostic;
use era_ir::ir::App;
use era_project::Manifest;

use crate::preview::{gradle_command, remove_stale, write_if_changed};
use crate::{EXIT_ERRORS, EXIT_USAGE};

macro_rules! template {
    ($path:literal) => {
        include_str!(concat!("../../../platforms/android/", $path))
    };
}

/// `(project path, template)`; placeholders are `{{NAME}}`.
const TEMPLATES: &[(&str, &str)] = &[
    ("settings.gradle.kts", template!("settings.gradle.kts")),
    ("build.gradle.kts", template!("build.gradle.kts")),
    ("gradle.properties", template!("gradle.properties")),
    (
        "gradle/libs.versions.toml",
        template!("gradle/libs.versions.toml"),
    ),
    ("app/build.gradle.kts", template!("app/build.gradle.kts")),
    (
        "app/proguard-rules.pro",
        template!("app/proguard-rules.pro"),
    ),
    (
        "app/src/main/AndroidManifest.xml",
        template!("app/AndroidManifest.xml"),
    ),
];
const HOST_KOTLIN: &[(&str, &str)] = &[
    ("MainActivity.kt", template!("app/MainActivity.kt")),
    ("EraAssets.kt", template!("app/EraAssets.kt")),
];
/// Export only: the Gradle wrapper (Gradle 8.14.3), so `./gradlew` works anywhere.
const WRAPPER_TEXT: &[(&str, &str)] = &[
    ("gradlew", template!("gradlew")),
    ("gradlew.bat", template!("gradlew.bat")),
    (
        "gradle/wrapper/gradle-wrapper.properties",
        template!("gradle/wrapper/gradle-wrapper.properties"),
    ),
    (".gitignore", template!("gitignore.txt")),
];
const WRAPPER_JAR: &[u8] =
    include_bytes!("../../../platforms/android/gradle/wrapper/gradle-wrapper.jar");
/// Marks a folder written by `era export`, so `--force` never deletes anything else.
pub const EXPORT_MARKER: &str = ".era-export";

pub const APK_PATH: &str = "app/build/outputs/apk/debug/app-debug.apk";
pub const RELEASE_APK_PATH: &str = "app/build/outputs/apk/release/app-release.apk";
pub const RELEASE_AAB_PATH: &str = "app/build/outputs/bundle/release/app-release.aab";

/// Release signing for `era build android --release` (D-086): the keystore
/// from `[android.signing]` and its password from `ERA_KEYSTORE_PASSWORD`.
fn release_signing(root: &Path, manifest: &Manifest) -> Result<(PathBuf, String), String> {
    let keytool = "keytool -genkeypair -v -keystore keys/upload.jks -alias upload -keyalg RSA -keysize 2048 -validity 10000";
    let Some(s) = &manifest.android.signing else {
        return Err(format!(
            "release builds need a signing key, and era.toml has no [android.signing]
   = help: create a key once (keep it and its password safe; you need both for every update):
       {keytool}
     then add to era.toml:
       [android.signing]
       keystore = \"keys/upload.jks\"
       alias = \"upload\""
        ));
    };
    let keystore = root.join(&s.keystore);
    if !keystore.is_file() {
        return Err(format!(
            "the keystore `{}` from [android.signing] does not exist
   = help: create it with: {keytool}",
            s.keystore
        ));
    }
    if std::env::var_os("ERA_KEYSTORE_PASSWORD").is_none() {
        return Err("set ERA_KEYSTORE_PASSWORD to the keystore password (and ERA_KEY_PASSWORD if the key has its own)
   = help: passwords are never stored in era.toml".into());
    }
    Ok((keystore, s.alias.clone()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Build,
    Export,
}

/// Everything to write for one Android project.
pub struct Plan {
    pub text: Vec<(String, String)>,
    pub bytes: Vec<(String, Vec<u8>)>,
    /// `(source file, project path)` copies of images and fonts.
    pub copies: Vec<(PathBuf, String)>,
}

fn fill(template: &str, vars: &[(&str, String)]) -> String {
    let mut s = template.to_string();
    for (k, v) in vars {
        s = s.replace(&format!("{{{{{k}}}}}"), v);
    }
    s
}

pub fn plan(
    root: &Path,
    manifest: &Manifest,
    app: &App,
    mode: Mode,
) -> Result<Plan, Vec<Diagnostic>> {
    let generated = generate_with(
        app,
        Options {
            export: mode == Mode::Export,
        },
    );
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
        ("PROJECT_NAME", manifest.name.clone()),
        ("MIN_SDK", manifest.android.min_sdk.to_string()),
        (
            "VERSION_CODE",
            android_gen::version_code(&app.version).to_string(),
        ),
        ("VERSION_NAME", app.version.clone()),
        (
            "ICON_ATTRS",
            if app.icon.is_some() {
                "\n        android:icon=\"@mipmap/ic_launcher\"\n        android:roundIcon=\"@mipmap/ic_launcher\"".into()
            } else {
                String::new()
            },
        ),
    ];
    let mut text = Vec::new();
    for (path, t) in TEMPLATES {
        text.push((path.to_string(), fill(t, &vars)));
    }
    let kotlin = format!("app/src/main/kotlin/{}", pkg.replace('.', "/"));
    for (name, t) in HOST_KOTLIN {
        text.push((format!("{kotlin}/{name}"), fill(t, &vars)));
    }
    for (rel, t) in &generated.files {
        text.push((format!("app/src/main/kotlin/{rel}"), t.clone()));
    }
    for (rel, t) in &android.files {
        let t = match mode {
            Mode::Export => era_codegen_compose::export_text(t),
            Mode::Build => t.clone(),
        };
        text.push((format!("app/{rel}"), t));
    }
    let copies = android
        .copies
        .iter()
        .map(|(from, to)| (assets.join(from), format!("app/{to}")))
        .collect();

    let mut bytes = Vec::new();
    if let Some(icon) = &app.icon {
        let icons = crate::icon::launcher_icons(
            &assets.join(&icon.path),
            icon.background.as_ref().map(|c| c.rgba),
        )
        .map_err(|e| {
            vec![
                Diagnostic::error(
                    "E3005",
                    format!("cannot make the app icon from `{}`: {e}", icon.path),
                )
                .in_file(format!("Assets/{}", icon.path))
                .with_help(
                    "use a square PNG, JPEG or WebP image, ideally 512x512 pixels or larger",
                ),
            ]
        })?;
        for (rel, b) in icons.png {
            bytes.push((format!("app/src/main/{rel}"), b));
        }
        for (rel, t) in icons.text {
            let t = match mode {
                Mode::Export => era_codegen_compose::export_text(&t),
                Mode::Build => t,
            };
            text.push((format!("app/src/main/{rel}"), t));
        }
    }
    if mode == Mode::Export {
        for (path, t) in WRAPPER_TEXT {
            text.push((path.to_string(), t.to_string()));
        }
        bytes.push((
            "gradle/wrapper/gradle-wrapper.jar".to_string(),
            WRAPPER_JAR.to_vec(),
        ));
        text.push(("README.md".into(), readme(app, manifest, &pkg)));
        text.push((
            EXPORT_MARKER.into(),
            format!(
                "Exported by era {} from `{}`.\n",
                env!("CARGO_PKG_VERSION"),
                manifest.name
            ),
        ));
    }
    text.sort();
    Ok(Plan {
        text,
        bytes,
        copies,
    })
}

fn readme(app: &App, manifest: &Manifest, pkg: &str) -> String {
    format!(
        "# {name} (Android)

Exported from the ERA project `{project}` with `era export android`.
This is a standard Android Studio project (Kotlin and Jetpack Compose).
Your team owns this code: build it, edit it and ship it like any Android app.

## Build

```bash
./gradlew assembleDebug
```

The APK is written to `{apk}`.

You can also open this folder in Android Studio (**File → Open**).

## Layout

| Path | Contents |
|---|---|
| `app/src/main/kotlin/{pkg_dir}/screens/` | One composable per screen |
| `app/src/main/kotlin/{pkg_dir}/EraTheme.kt` | Colours, spacing, radii and text styles (your skin) |
| `app/src/main/kotlin/{pkg_dir}/EraStores.kt` | App state and actions (if the app has stores) |
| `app/src/main/kotlin/{pkg_dir}/EraRuntime.kt` | Navigation and small helpers |
| `app/src/main/kotlin/{pkg_dir}/MainActivity.kt` | Entry point |
| `app/src/main/res/`, `app/src/main/assets/` | Icons, images and fonts |
| `gradle/libs.versions.toml` | Library and plugin versions |

## Release build (Google Play)

Create `keystore.properties` next to this README (it is git-ignored):

```properties
storeFile=keys/upload.jks
storePassword=...
keyAlias=upload
keyPassword=...
```

Or set `ERA_KEYSTORE`, `ERA_KEYSTORE_PASSWORD`, `ERA_KEY_ALIAS` and
`ERA_KEY_PASSWORD`. Then:

```bash
./gradlew bundleRelease
```

The App Bundle for Google Play is written to
`app/build/outputs/bundle/release/app-release.aab`. Release builds are
optimised and shrunk with R8.

## Re-exporting

`era export android --force` replaces this folder with a fresh export,
which overwrites any edits made here. Commit your changes first, or keep
editing the ERA sources and re-export.

Application id: `{id}`, version {version}.
",
        name = app.name,
        project = manifest.name,
        apk = APK_PATH,
        pkg_dir = pkg.replace('.', "/"),
        id = app.id,
        version = app.version,
    )
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

/// Write a plan into `out`. Returns every path written.
pub fn write_plan(out: &Path, plan: &Plan) -> std::io::Result<BTreeSet<PathBuf>> {
    let mut written = BTreeSet::new();
    for (rel, t) in &plan.text {
        let p = out.join(rel);
        write_if_changed(&p, t)?;
        written.insert(p);
    }
    for (rel, b) in &plan.bytes {
        let p = out.join(rel);
        if std::fs::read(&p).ok().as_deref() != Some(b.as_slice()) {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&p, b)?;
        }
        written.insert(p);
    }
    for (from, rel) in &plan.copies {
        let p = out.join(rel);
        copy_if_changed(from, &p)?;
        written.insert(p);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let gradlew = out.join("gradlew");
        if gradlew.is_file() {
            std::fs::set_permissions(&gradlew, std::fs::Permissions::from_mode(0o755))?;
        }
    }
    Ok(written)
}

/// The Android SDK from `ANDROID_HOME` or `ANDROID_SDK_ROOT`.
pub fn find_sdk() -> Option<PathBuf> {
    ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .find(|p| p.join("platforms").is_dir())
}

fn local_properties(out: &Path, sdk: &Path) -> std::io::Result<()> {
    let sdk_dir = sdk
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace(':', "\\:");
    write_if_changed(
        &out.join("local.properties"),
        &format!("sdk.dir={sdk_dir}\n"),
    )
}

fn print_diags(diags: &[Diagnostic], what: &str) -> ExitCode {
    for d in diags {
        eprint!("{}", d.render(None));
        eprintln!();
    }
    eprintln!("error: {what} failed: {} error(s)", diags.len());
    ExitCode::from(EXIT_ERRORS)
}

/// `era build android`.
pub fn run(root: &Path, manifest: &Manifest, app: &App, release: bool) -> ExitCode {
    let signing = if release {
        match release_signing(root, manifest) {
            Ok(s) => Some(s),
            Err(msg) => {
                eprintln!("error[E3004]: {msg}");
                return ExitCode::from(EXIT_USAGE);
            }
        }
    } else {
        None
    };
    let Some(sdk) = find_sdk() else {
        eprintln!("error: Android SDK not found");
        eprintln!(
            "   = help: install the Android SDK (platform 35, build-tools) and set ANDROID_HOME to its folder"
        );
        return ExitCode::from(EXIT_USAGE);
    };
    let plan = match plan(root, manifest, app, Mode::Build) {
        Ok(p) => p,
        Err(d) => return print_diags(&d, "build"),
    };
    let out = root.join("build/android");
    let written =
        match write_plan(&out, &plan).and_then(|w| local_properties(&out, &sdk).map(|_| w)) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("error: cannot write the Android project: {e}");
                return ExitCode::from(EXIT_USAGE);
            }
        };
    remove_stale(&out.join("app/src/main"), &written);
    println!("Generated Android project in {}", out.display());
    println!(
        "Building {} with Gradle (the first run downloads the Android build tools and can take several minutes)...",
        if release {
            "the signed release APK and App Bundle"
        } else {
            "debug APK"
        }
    );

    let gradle = gradle_command();
    let mut cmd = Command::new(&gradle);
    cmd.args(["--console=plain", "-q", "--no-watch-fs"])
        .current_dir(&out);
    match &signing {
        Some((keystore, alias)) => {
            cmd.args([":app:assembleRelease", ":app:bundleRelease"])
                .env("ERA_KEYSTORE", keystore)
                .env("ERA_KEY_ALIAS", alias);
        }
        None => {
            cmd.arg(":app:assembleDebug");
        }
    }
    let status = cmd.status();
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
        Ok(_) if release => {
            for (what, path) in [("APK", RELEASE_APK_PATH), ("App Bundle", RELEASE_AAB_PATH)] {
                let f = out.join(path);
                let size = std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
                println!(
                    "Built release {what} {} ({:.1} MB)",
                    f.display(),
                    size as f64 / 1_048_576.0
                );
            }
            println!("\nUpload the .aab to Google Play, or install the APK directly:");
            println!(
                "  adb install -r \"{}\"",
                out.join(RELEASE_APK_PATH).display()
            );
            ExitCode::SUCCESS
        }
        Ok(_) => {
            let apk = out.join(APK_PATH);
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

/// `era export android`.
pub fn export(
    root: &Path,
    manifest: &Manifest,
    app: &App,
    out: Option<&Path>,
    force: bool,
) -> ExitCode {
    let out = out
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("export/android"));
    let non_empty = std::fs::read_dir(&out)
        .map(|mut d| d.next().is_some())
        .unwrap_or(false);
    if non_empty {
        let ours = out.join(EXPORT_MARKER).is_file();
        if !force || !ours {
            eprintln!(
                "error[E3003]: `{}` already exists and is not empty",
                out.display()
            );
            if ours {
                eprintln!(
                    "   = help: pass --force to replace it with a fresh export (edits made there will be lost)"
                );
            } else {
                eprintln!(
                    "   = help: choose an empty folder with --out; ERA only replaces folders it exported itself"
                );
            }
            return ExitCode::from(EXIT_USAGE);
        }
        if let Err(e) = std::fs::remove_dir_all(&out) {
            eprintln!("error: cannot clear `{}`: {e}", out.display());
            return ExitCode::from(EXIT_USAGE);
        }
    }
    let plan = match plan(root, manifest, app, Mode::Export) {
        Ok(p) => p,
        Err(d) => return print_diags(&d, "export"),
    };
    if let Err(e) = write_plan(&out, &plan) {
        eprintln!("error: cannot write the exported project: {e}");
        return ExitCode::from(EXIT_USAGE);
    }
    if let Some(sdk) = find_sdk() {
        // Not committed (.gitignore); Android Studio would create it anyway.
        let _ = local_properties(&out, &sdk);
    }
    let files = plan.text.len() + plan.bytes.len() + plan.copies.len();
    println!(
        "Exported Android Studio project to {} ({files} files)",
        out.display()
    );
    println!("\nNext:");
    println!("  cd \"{}\"", out.display());
    println!("  ./gradlew assembleDebug");
    println!("or open the folder in Android Studio.");
    ExitCode::SUCCESS
}
