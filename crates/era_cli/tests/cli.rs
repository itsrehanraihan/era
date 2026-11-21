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
            "Checked `showcase` (dev.era.showcase): 5 UI file(s), 3 skin file(s), 0 asset(s), 0 warning(s)"
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
