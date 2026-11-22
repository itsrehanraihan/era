use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn era_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_era"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to run era binary")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn version_prints_crate_version() {
    let tmp = tempfile::tempdir().unwrap();
    let out = era_in(tmp.path(), &["--version"]);
    assert!(out.status.success());
    assert_eq!(
        stdout(&out).trim(),
        format!("era {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn new_then_check_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let out = era_in(tmp.path(), &["new", "my_app"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("Created ERA project `my_app`"));

    let project = tmp.path().join("my_app");
    let out = era_in(&project, &["check"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with(
            "Checked `my_app` (dev.era.my_app): 1 UI file(s), 0 skin file(s), 0 asset(s), 0 warning(s)"
        ),
        "{}",
        stdout(&out)
    );

    // Also works from a subfolder and with an explicit path.
    assert!(era_in(&project.join("src"), &["check"]).status.success());
    assert!(era_in(tmp.path(), &["check", "my_app"]).status.success());
}

#[test]
fn new_rejects_invalid_name() {
    let tmp = tempfile::tempdir().unwrap();
    let out = era_in(tmp.path(), &["new", "My-App"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("error[E0110]"));
    assert!(stderr(&out).contains("try `era new my_app`"));
}

#[test]
fn check_outside_project_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let out = era_in(tmp.path(), &["check"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("error[E0001]: no `era.toml` found"));
}

#[test]
fn check_reports_manifest_error_with_line_and_column() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(era_in(tmp.path(), &["new", "demo"]).status.success());
    let manifest = tmp.path().join("demo/era.toml");
    let src = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, src.replace("dev.era.demo", "demo")).unwrap();

    let out = era_in(&tmp.path().join("demo"), &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("error[E0103]: invalid project id `demo`"),
        "{err}"
    );
    assert!(err.contains("--> era.toml:3:11"), "{err}");
    assert!(err.contains("id      = \"demo\""), "{err}");
    assert!(
        err.contains("check failed: 1 error(s), 0 warning(s)"),
        "{err}"
    );
}

#[test]
fn check_showcase_example() {
    let out = era_in(&repo_root(), &["check", "examples/showcase"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with(
            "Checked `showcase` (dev.era.showcase): 7 UI file(s), 3 skin file(s), 2 asset(s), 0 warning(s)"
        ),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out)
            .contains("Declarations: 1 app, 3 screens, 1 layout, 2 components, 1 skin, 1 store")
    );
}

fn new_project(tmp: &Path, name: &str) -> PathBuf {
    assert!(era_in(tmp, &["new", name]).status.success());
    tmp.join(name)
}

#[test]
fn new_project_declarations_are_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let out = era_in(&project, &["check"]);
    assert!(
        stdout(&out).contains("Declarations: 1 app, 1 screen, 0 layouts, 0 components, 0 skins"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_reports_syntax_error_in_era_file() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    std::fs::create_dir_all(project.join("src/Screens")).unwrap();
    std::fs::write(
        project.join("src/Screens/About.era"),
        "screen About {\n    column {\n        gap: 16qq\n        text \"About\"\n    }\n}\n",
    )
    .unwrap();

    let out = era_in(&project, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(err.contains("error[E1006]: unknown unit `qq`"), "{err}");
    assert!(err.contains("--> src/Screens/About.era:3:14"), "{err}");
    assert!(err.contains("3 |         gap: 16qq"), "{err}");
    assert!(err.contains("units are: px"), "{err}");
}

#[test]
fn check_reports_structure_errors_across_files() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    std::fs::write(
        project.join("src/More.era"),
        "screen Home { column {} }\nlayout Frame { column {} }\n",
    )
    .unwrap();
    std::fs::create_dir_all(project.join("Skins")).unwrap();
    std::fs::write(
        project.join("Skins/theme.ers"),
        "skin Studio { primary: #FF6B35 }\n",
    )
    .unwrap();

    let out = era_in(&project, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("error[E1201]: property `primary` is not allowed directly in `skin`"),
        "{err}"
    );
    assert!(
        err.contains("error[E1209]: layout `Frame` has no `slot`"),
        "{err}"
    );
    assert!(
        err.contains("error[E1215]: `Home` is already declared as a screen"),
        "{err}"
    );
    assert!(err.contains("first declared at src/app.era:5:8"), "{err}");
    assert!(
        err.contains("check failed: 3 error(s), 0 warning(s)"),
        "{err}"
    );
}

#[test]
fn new_project_uses_default_skin() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let out = era_in(&project, &["check"]);
    assert!(
        stdout(&out).contains("UI IR: 1 screen(s), 3 element(s), built-in default skin"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn check_reports_semantic_errors_with_suggestions() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let app = project.join("src/app.era");
    let src = std::fs::read_to_string(&app).unwrap();
    std::fs::write(
        &app,
        src.replace("gap: 8", "gapp: md")
            .replace("padding: 24", "padding: hueg"),
    )
    .unwrap();

    let out = era_in(&project, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("error[E2120]: `column` has no property `gapp`"),
        "{err}"
    );
    assert!(err.contains("= help: did you mean `gap`?"), "{err}");
    assert!(
        err.contains("error[E2132]: unknown spacing token `hueg`"),
        "{err}"
    );
    assert!(
        err.contains("the default skin defines: lg, md, sm, xl, xs"),
        "{err}"
    );
}

#[test]
fn check_ir_flag_prints_the_tree() {
    let out = era_in(&repo_root(), &["check", "examples/showcase", "--ir"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains("screen Home title=\"Home\" background=background(#FFFFFF)"),
        "{text}"
    );
    assert!(
        text.contains("icon \"Icons/star.svg\" {color=primary(#FF6B35) size=20}"),
        "{text}"
    );
    assert!(
        era_in(&repo_root(), &["check", "--ir", "examples/showcase"])
            .status
            .success()
    );
    assert_eq!(
        era_in(&repo_root(), &["check", "--bogus"]).status.code(),
        Some(2)
    );
}

#[test]
fn check_erenty_demo() {
    let out = era_in(&repo_root(), &["check", "examples/erenty"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains(
            "UI IR: 4 screen(s), 188 element(s), skin `Renty`, 25 asset reference(s), start screen `Splash`"
        ),
        "{}",
        stdout(&out)
    );
}

#[test]
fn unimplemented_command_fails_honestly() {
    let tmp = tempfile::tempdir().unwrap();
    let out = era_in(tmp.path(), &["build", "ios"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("not part of the ERA MVP"));
    let out = era_in(tmp.path(), &["build"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("usage: era build android [path]"));
}

fn era_env(dir: &Path, args: &[&str], set: &[(&str, &str)], remove: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_era"));
    cmd.args(args).current_dir(dir);
    for (k, v) in set {
        cmd.env(k, v);
    }
    for k in remove {
        cmd.env_remove(k);
    }
    cmd.output().expect("failed to run era binary")
}

#[test]
fn preview_rejects_bad_arguments() {
    let tmp = tempfile::tempdir().unwrap();
    for args in [&["preview", "--bogus"][..], &["preview", "a", "b"]] {
        let out = era_in(tmp.path(), args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(stderr(&out).contains("usage: era preview [path] [--png [dir]]"));
    }
}

#[test]
fn preview_refuses_project_with_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let app = project.join("src/app.era");
    let src = std::fs::read_to_string(&app).unwrap();
    std::fs::write(&app, src.replace("gap: 8", "gapp: 8")).unwrap();
    let out = era_in(&project, &["preview", "--png"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("error[E2120]"));
    assert!(!project.join("build/preview").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn preview_window_needs_a_display() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let out = era_env(&project, &["preview"], &[], &["DISPLAY", "WAYLAND_DISPLAY"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("run `era preview --png`"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn preview_writes_project_and_reports_missing_gradle() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let out = era_env(
        &project,
        &["preview", "--png", "shots"],
        &[("ERA_GRADLE", "/nonexistent/gradle")],
        &[],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("`/nonexistent/gradle` was not found"),
        "{}",
        stderr(&out)
    );

    let preview = project.join("build/preview");
    let build = std::fs::read_to_string(preview.join("build.gradle.kts")).unwrap();
    assert!(
        build.contains("mainClass = \"dev.era.demo.ui.PreviewMainKt\""),
        "{build}"
    );
    let kt = preview.join("src/main/kotlin/dev/era/demo/ui");
    for f in [
        "EraTheme.kt",
        "EraRuntime.kt",
        "EraScreens.kt",
        "EraAssets.kt",
        "PreviewMain.kt",
        "screens/HomeScreen.kt",
    ] {
        assert!(kt.join(f).is_file(), "{f}");
    }
    let props = std::fs::read_to_string(preview.join("era-preview.properties")).unwrap();
    assert!(props.contains("png=") && props.contains("shots"), "{props}");
    assert!(
        !std::fs::read_to_string(kt.join("EraAssets.kt"))
            .unwrap()
            .contains("{{PACKAGE}}")
    );
}

#[test]
fn preview_removes_stale_generated_screens() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let gradle = [("ERA_GRADLE", "/nonexistent/gradle")];
    std::fs::write(project.join("src/About.era"), "screen About { column {} }").unwrap();
    era_env(&project, &["preview", "--png"], &gradle, &[]);
    let screens = project.join("build/preview/src/main/kotlin/dev/era/demo/ui/screens");
    assert!(screens.join("AboutScreen.kt").is_file());
    std::fs::remove_file(project.join("src/About.era")).unwrap();
    era_env(&project, &["preview", "--png"], &gradle, &[]);
    assert!(!screens.join("AboutScreen.kt").exists());
    assert!(screens.join("HomeScreen.kt").is_file());
}

/// Real end-to-end render with Gradle. Needs Gradle 8.x and network access
/// (or a warm Gradle cache): `cargo test -p era_cli -- --ignored`.
#[test]
#[ignore]
fn preview_png_smoke_test() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "smoke");
    let out = era_in(&project, &["preview", "--png"]);
    assert!(out.status.success(), "{}\n{}", stdout(&out), stderr(&out));
    let png = std::fs::read(project.join("build/preview/png/Home.png")).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert_eq!((w, h), (804, 1748));
    // The default skin has dark mode, so a dark render is produced too.
    assert!(project.join("build/preview/png/Home-dark.png").is_file());
}

#[test]
fn bad_usage_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    for args in [&["frobnicate"][..], &["new"], &["check", "a", "b"]] {
        assert_eq!(era_in(tmp.path(), args).status.code(), Some(2), "{args:?}");
    }
}

/// A folder that looks like an Android SDK (enough for ERA's check).
fn fake_sdk() -> tempfile::TempDir {
    let sdk = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(sdk.path().join("platforms")).unwrap();
    sdk
}

#[test]
fn build_android_needs_an_sdk() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let out = era_env(
        &project,
        &["build", "android"],
        &[],
        &["ANDROID_HOME", "ANDROID_SDK_ROOT"],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("Android SDK not found"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn build_android_writes_the_project() {
    let sdk = fake_sdk();
    let sdk_path = sdk.path().to_str().unwrap();
    let out = era_env(
        &repo_root().join("examples/erenty"),
        &["build", "android"],
        &[
            ("ANDROID_HOME", sdk_path),
            ("ERA_GRADLE", "/nonexistent/gradle"),
        ],
        &["ANDROID_SDK_ROOT"],
    );
    // Gradle is missing on purpose: everything up to the Gradle run is checked.
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
    assert!(stderr(&out).contains("`/nonexistent/gradle` was not found"));

    let android = repo_root().join("examples/erenty/build/android");
    let build = std::fs::read_to_string(android.join("app/build.gradle.kts")).unwrap();
    for want in [
        "applicationId = \"dev.era.erenty\"",
        "minSdk = 24",
        "versionCode = 100",
        "versionName = \"0.1.0\"",
    ] {
        assert!(build.contains(want), "{want}");
    }
    let manifest =
        std::fs::read_to_string(android.join("app/src/main/AndroidManifest.xml")).unwrap();
    assert!(manifest.contains("android:name=\"dev.era.erenty.ui.MainActivity\""));
    let strings =
        std::fs::read_to_string(android.join("app/src/main/res/values/strings.xml")).unwrap();
    assert!(strings.contains("<string name=\"app_name\">E-Renty</string>"));
    let drawables = std::fs::read_dir(android.join("app/src/main/res/drawable"))
        .unwrap()
        .count();
    assert_eq!(drawables, 12);
    assert!(
        android
            .join("app/src/main/assets/Images/logo.png")
            .is_file()
    );
    assert!(
        android
            .join("app/src/main/assets/Fonts/Inter-Medium.ttf")
            .is_file()
    );
    let ids = std::fs::read_to_string(
        android.join("app/src/main/kotlin/dev/era/erenty/ui/EraIconIds.kt"),
    )
    .unwrap();
    assert!(
        ids.contains("\"Icons/home.svg\" to R.drawable.era_icon_icons_home,"),
        "{ids}"
    );
    let local = std::fs::read_to_string(android.join("local.properties")).unwrap();
    assert!(local.contains("sdk.dir="));
}

#[test]
fn build_android_reports_unsupported_icons() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    std::fs::create_dir_all(project.join("Assets/Icons")).unwrap();
    std::fs::write(
        project.join("Assets/Icons/spin.svg"),
        r#"<svg viewBox="0 0 24 24"><path d="M0 0h24v24H0z" transform="rotate(45)"/></svg>"#,
    )
    .unwrap();
    let app = project.join("src/app.era");
    let src = std::fs::read_to_string(&app).unwrap();
    std::fs::write(
        &app,
        src.replace(
            "text \"Hello from ERA\"",
            "text \"Hello from ERA\"\n        icon \"Icons/spin.svg\"",
        ),
    )
    .unwrap();
    let sdk = fake_sdk();
    let out = era_env(
        &project,
        &["build", "android"],
        &[("ANDROID_HOME", sdk.path().to_str().unwrap())],
        &[],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let err = stderr(&out);
    assert!(err.contains("error[E3001]: cannot convert icon `Icons/spin.svg` for Android: `transform` is not supported"), "{err}");
    assert!(err.contains("--> Assets/Icons/spin.svg"), "{err}");
}

/// Real APK build. Needs Gradle 8.x, ANDROID_HOME (platform 35, build-tools)
/// and network access (or a warm cache): `cargo test -p era_cli -- --ignored`.
#[test]
#[ignore]
fn build_android_apk_smoke_test() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "smoke");
    let out = era_in(&project, &["build", "android"]);
    assert!(out.status.success(), "{}\n{}", stdout(&out), stderr(&out));
    let apk = project.join("build/android/app/build/outputs/apk/debug/app-debug.apk");
    let bytes = std::fs::read(&apk).unwrap();
    assert_eq!(&bytes[..2], b"PK", "an APK is a zip file");
    assert!(stdout(&out).contains("adb install -r"));
}

/// Taps through the E-Renty demo in the real generated Compose code
/// (headless): Splash -> Home -> Menu -> Verify code -> Verify (back to Menu).
/// Needs Gradle: `cargo test -p era_cli -- --ignored`.
#[test]
#[ignore]
fn preview_tap_flow_smoke_test() {
    let demo = repo_root().join("examples/erenty");
    let out = era_in(&repo_root(), &["preview", "examples/erenty", "--png"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let preview = demo.join("build/preview");
    let props = preview.join("era-preview.properties");
    let original = std::fs::read_to_string(&props).unwrap();
    std::fs::write(
        &props,
        format!("{original}script=tap 201,437; tap 42,42; tap 200,200; tap 200,563\n"),
    )
    .unwrap();
    let gradle = std::env::var("ERA_GRADLE").unwrap_or_else(|_| "gradle".into());
    let run = Command::new(gradle)
        .args(["--console=plain", "-q", "--no-watch-fs", "run"])
        .current_dir(&preview)
        .output()
        .unwrap();
    std::fs::write(&props, original).unwrap();
    let text = stdout(&run);
    let screens: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("era-preview: step-"))
        .map(|l| l.split("screen=").nth(1).unwrap_or(""))
        .collect();
    assert_eq!(
        screens,
        ["Splash", "Home", "Menu", "VerifyCode", "Menu"],
        "{text}\n{}",
        stderr(&run)
    );
}

/// Builds the full E-Renty APK (stores, navigation, BackHandler).
/// Needs Gradle and ANDROID_HOME: `cargo test -p era_cli -- --ignored`.
#[test]
#[ignore]
fn build_erenty_apk_smoke_test() {
    let out = era_in(&repo_root(), &["build", "android", "examples/erenty"]);
    assert!(out.status.success(), "{}\n{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("app-debug.apk"));
}

// ----- M9: era export android ----------------------------------------------

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p))
        } else {
            out.push(p)
        }
    }
    out
}

#[test]
fn export_android_writes_a_standalone_project() {
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tmp.path().join("erenty-android");
    let out = era_in(
        &repo_root(),
        &[
            "export",
            "android",
            "examples/erenty",
            "--out",
            out_dir.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("Exported Android Studio project"));

    for f in [
        "settings.gradle.kts",
        "build.gradle.kts",
        "gradle.properties",
        "gradle/libs.versions.toml",
        "gradle/wrapper/gradle-wrapper.properties",
        "gradlew",
        "gradlew.bat",
        ".gitignore",
        "README.md",
        ".era-export",
        "app/build.gradle.kts",
        "app/src/main/AndroidManifest.xml",
        "app/src/main/kotlin/dev/era/erenty/ui/MainActivity.kt",
        "app/src/main/kotlin/dev/era/erenty/ui/screens/HomeScreen.kt",
        "app/src/main/assets/Images/bike.png",
        "app/src/main/res/drawable/era_icon_icons_home.xml",
    ] {
        assert!(out_dir.join(f).is_file(), "missing {f}");
    }
    let jar = std::fs::read(out_dir.join("gradle/wrapper/gradle-wrapper.jar")).unwrap();
    assert!(jar.len() > 40_000 && &jar[..2] == b"PK");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(out_dir.join("gradlew"))
            .unwrap()
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "gradlew must be executable");
    }
    let settings = std::fs::read_to_string(out_dir.join("settings.gradle.kts")).unwrap();
    assert!(
        settings.contains("rootProject.name = \"erenty\"")
            && settings.contains("include(\":app\")")
    );
    let wrapper =
        std::fs::read_to_string(out_dir.join("gradle/wrapper/gradle-wrapper.properties")).unwrap();
    assert!(wrapper.contains("gradle-8.14.3-bin.zip"));
    let readme = std::fs::read_to_string(out_dir.join("README.md")).unwrap();
    assert!(readme.contains("# E-Renty (Android)") && readme.contains("./gradlew assembleDebug"));

    // Exported code is the team's: no "do not edit", no internal IR ids.
    let home = std::fs::read_to_string(
        out_dir.join("app/src/main/kotlin/dev/era/erenty/ui/screens/HomeScreen.kt"),
    )
    .unwrap();
    assert!(
        home.starts_with(
            "// Generated by ERA (era export). This is ordinary Kotlin: edit it freely."
        ),
        "{home}"
    );
    assert!(!home.contains("@Home"), "no IR ids in exported code");
    assert!(
        home.contains("(from HeroCard)"),
        "component origins stay as plain comments"
    );
    for entry in walk(&out_dir) {
        if entry
            .extension()
            .is_some_and(|e| e == "kt" || e == "kts" || e == "xml" || e == "toml")
        {
            let t = std::fs::read_to_string(&entry).unwrap();
            assert!(!t.contains("Do not edit"), "{}", entry.display());
            assert!(
                !t.contains("{{"),
                "unfilled placeholder in {}",
                entry.display()
            );
        }
    }
}

#[test]
fn export_never_overwrites_by_accident() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    // A folder with someone else's files is refused, even with --force.
    let theirs = tmp.path().join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(theirs.join("notes.txt"), "keep me").unwrap();
    let out = era_in(
        &project,
        &[
            "export",
            "android",
            "--out",
            theirs.to_str().unwrap(),
            "--force",
        ],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("ERA only replaces folders it exported itself"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        std::fs::read_to_string(theirs.join("notes.txt")).unwrap(),
        "keep me"
    );

    // Default location; exporting again needs --force, which replaces it.
    assert!(era_in(&project, &["export", "android"]).status.success());
    let exported = project.join("export/android");
    std::fs::write(exported.join("stale.txt"), "old").unwrap();
    let again = era_in(&project, &["export", "android"]);
    assert_eq!(again.status.code(), Some(2));
    assert!(stderr(&again).contains("pass --force"));
    assert!(
        era_in(&project, &["export", "android", "--force"])
            .status
            .success()
    );
    assert!(!exported.join("stale.txt").exists());
    assert!(exported.join("app/build.gradle.kts").is_file());
}

#[test]
fn export_usage_and_ios_message() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(stderr(&era_in(tmp.path(), &["export", "ios"])).contains("SwiftUI"));
    assert_eq!(
        era_in(tmp.path(), &["export", "android", "--bogus"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(era_in(tmp.path(), &["export"]).status.code(), Some(2));
}

/// The exported project builds on its own with `./gradlew`, which downloads
/// the Gradle 8.14.3 distribution on first run. Needs ANDROID_HOME:
/// `cargo test -p era_cli -- --ignored`.
#[test]
#[ignore]
fn exported_project_builds_with_its_own_gradle_wrapper() {
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tmp.path().join("app-export");
    let out = era_in(
        &repo_root(),
        &[
            "export",
            "android",
            "examples/erenty",
            "--out",
            out_dir.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let build = Command::new(out_dir.join("gradlew"))
        .args(["--console=plain", "-q", "--no-watch-fs", "assembleDebug"])
        .current_dir(&out_dir)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}\n{}",
        stdout(&build),
        stderr(&build)
    );
    assert!(
        out_dir
            .join("app/build/outputs/apk/debug/app-debug.apk")
            .is_file()
    );
}

// ----- Release builds and the app icon (D-085, D-086) -----------------------

#[test]
fn app_icon_becomes_android_launcher_icons() {
    let tmp = tempfile::tempdir().unwrap();
    let out_dir = tmp.path().join("export");
    let out = era_in(
        &repo_root(),
        &[
            "export",
            "android",
            "examples/erenty",
            "--out",
            out_dir.to_str().unwrap(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let res = out_dir.join("app/src/main/res");
    for d in ["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"] {
        assert!(res.join(format!("mipmap-{d}/ic_launcher.png")).is_file());
        assert!(
            res.join(format!("mipmap-{d}/ic_launcher_foreground.png"))
                .is_file()
        );
    }
    let adaptive = std::fs::read_to_string(res.join("mipmap-anydpi-v26/ic_launcher.xml")).unwrap();
    assert!(adaptive.contains("@mipmap/ic_launcher_foreground"));
    let bg = std::fs::read_to_string(res.join("values/ic_launcher_background.xml")).unwrap();
    assert!(bg.contains("#FF00E6B5"), "{bg}");
    let manifest =
        std::fs::read_to_string(out_dir.join("app/src/main/AndroidManifest.xml")).unwrap();
    assert!(manifest.contains("android:icon=\"@mipmap/ic_launcher\""));
    // The icon is not copied into the app's runtime assets.
    assert!(
        !out_dir
            .join("app/src/main/assets/Images/app_icon.png")
            .exists()
    );
    // Release signing is configured but holds no secrets.
    let gradle = std::fs::read_to_string(out_dir.join("app/build.gradle.kts")).unwrap();
    assert!(gradle.contains("isMinifyEnabled = true") && gradle.contains("keystore.properties"));
    let ignore = std::fs::read_to_string(out_dir.join(".gitignore")).unwrap();
    assert!(ignore.contains("keystore.properties") && ignore.contains("*.jks"));
}

#[test]
fn app_icon_errors_are_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let app = project.join("src/app.era");
    let src = std::fs::read_to_string(&app).unwrap();
    std::fs::write(
        &app,
        src.replacen(
            "{",
            "{\n    icon: \"Images/missing.png\"\n    icon-background: 12",
            1,
        ),
    )
    .unwrap();
    let out = era_in(&project, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(err.contains("E2107"), "{err}");
    assert!(err.contains("missing.png"), "{err}");
}

#[test]
fn release_build_needs_a_signing_key() {
    let tmp = tempfile::tempdir().unwrap();
    let project = new_project(tmp.path(), "demo");
    let out = era_in(&project, &["build", "android", "--release"]);
    assert_eq!(out.status.code(), Some(2));
    let err = stderr(&out);
    assert!(
        err.contains("E3004") && err.contains("[android.signing]"),
        "{err}"
    );
    assert!(err.contains("keytool -genkeypair"), "{err}");
    assert_eq!(
        era_in(&project, &["build", "android", "--bogus"])
            .status
            .code(),
        Some(2)
    );
}

/// A signed, R8-optimised release APK and App Bundle. Needs ANDROID_HOME and
/// `keytool` (JDK): `cargo test -p era_cli -- --ignored`.
#[test]
#[ignore]
fn release_build_smoke_test() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("erenty");
    copy_dir(&repo_root().join("examples/erenty"), &project);
    let _ = std::fs::remove_dir_all(project.join("build"));
    std::fs::create_dir_all(project.join("keys")).unwrap();
    let keytool = Command::new("keytool")
        .args([
            "-genkeypair",
            "-keystore",
            "keys/upload.jks",
            "-alias",
            "upload",
            "-keyalg",
            "RSA",
            "-keysize",
            "2048",
            "-validity",
            "10000",
            "-storepass",
            "era-test-pass",
            "-dname",
            "CN=ERA Release Test",
        ])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(keytool.status.success(), "{}", stderr(&keytool));
    let toml = project.join("era.toml");
    let mut m = std::fs::read_to_string(&toml).unwrap();
    m.push_str("\n[android.signing]\nkeystore = \"keys/upload.jks\"\nalias = \"upload\"\n");
    std::fs::write(&toml, m).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_era"))
        .args(["build", "android", "--release"])
        .current_dir(&project)
        .env("ERA_KEYSTORE_PASSWORD", "era-test-pass")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}\n{}", stdout(&out), stderr(&out));
    let android = project.join("build/android/app/build/outputs");
    let apk = android.join("apk/release/app-release.apk");
    assert!(apk.is_file());
    assert!(android.join("bundle/release/app-release.aab").is_file());

    // Signed with the release key, not the debug key.
    let sdk = PathBuf::from(std::env::var("ANDROID_HOME").unwrap());
    let tools = std::fs::read_dir(sdk.join("build-tools"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .max()
        .unwrap();
    let certs = Command::new(tools.join("apksigner"))
        .args(["verify", "--print-certs"])
        .arg(&apk)
        .output()
        .unwrap();
    assert!(certs.status.success(), "{}", stderr(&certs));
    assert!(
        stdout(&certs).contains("CN=ERA Release Test"),
        "{}",
        stdout(&certs)
    );
    let badging = Command::new(tools.join("aapt2"))
        .args(["dump", "badging"])
        .arg(&apk)
        .output()
        .unwrap();
    let b = stdout(&badging);
    assert!(b.contains("application-icon-480:'res/"), "{b}");
    assert!(!b.contains("application-debuggable"), "{b}");
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let name = e.file_name();
        if name == "build" || name == "export" {
            continue;
        }
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(&name));
        } else {
            std::fs::copy(&p, to.join(&name)).unwrap();
        }
    }
}
