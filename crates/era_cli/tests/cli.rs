use std::process::Command;

fn era(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_era"))
        .args(args)
        .output()
        .expect("failed to run era binary")
}

#[test]
fn version_prints_crate_version() {
    let out = era(&["--version"]);
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.trim(), format!("era {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn unimplemented_command_fails_honestly() {
    let out = era(&["check"]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("not implemented yet"));
}

#[test]
fn unknown_command_is_rejected() {
    let out = era(&["frobnicate"]);
    assert_eq!(out.status.code(), Some(2));
}
