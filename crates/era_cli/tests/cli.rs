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
            "Checked `showcase` (dev.era.showcase): 5 UI file(s), 3 skin file(s), 2 asset(s), 0 warning(s)"
        ),
        "{}",
        stdout(&out)
    );
    assert!(stdout(&out).contains("Declarations: 1 app, 1 screen, 1 layout, 2 components, 1 skin"));
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
        "screen About {\n    column {\n        gap: 16px\n        text \"About\"\n    }\n}\n",
    )
    .unwrap();

    let out = era_in(&project, &["check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(
        err.contains("error[E1006]: unexpected `px` after number"),
        "{err}"
    );
    assert!(err.contains("--> src/Screens/About.era:3:14"), "{err}");
    assert!(err.contains("3 |         gap: 16px"), "{err}");
    assert!(err.contains("ERA numbers have no units"), "{err}");
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
            "UI IR: 4 screen(s), 187 element(s), skin `Renty`, 21 asset reference(s), start screen `Splash`"
        ),
        "{}",
        stdout(&out)
    );
}

#[test]
fn unimplemented_command_fails_honestly() {
    let tmp = tempfile::tempdir().unwrap();
    let out = era_in(tmp.path(), &["preview"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("not implemented yet"));
}

#[test]
fn bad_usage_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    for args in [&["frobnicate"][..], &["new"], &["check", "a", "b"]] {
        assert_eq!(era_in(tmp.path(), args).status.code(), Some(2), "{args:?}");
    }
}
