use std::fs;
use std::path::Path;

use era_project::scaffold::create_project;
use era_project::{FileKind, find_root, load_project};

const MANIFEST: &str = "[project]\nname = \"demo\"\nid = \"dev.era.demo\"\nversion = \"0.1.0\"\n\n[era]\nedition = \"2026\"\n";

fn write(root: &Path, rel: &str, contents: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, contents).unwrap();
}

fn minimal() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", MANIFEST.as_bytes());
    write(dir.path(), "src/app.era", b"app \"Demo\" {}\n");
    dir
}

fn codes(diags: &[era_diagnostics::Diagnostic]) -> Vec<&'static str> {
    diags.iter().map(|d| d.code).collect()
}

#[test]
fn minimal_project_is_clean() {
    let dir = minimal();
    let (project, diags) = load_project(dir.path());
    assert!(diags.is_empty(), "{diags:?}");
    let project = project.unwrap();
    assert_eq!(project.manifest.name, "demo");
    assert_eq!(project.files.len(), 1);
    assert_eq!(project.files[0].rel_path, "src/app.era");
    assert_eq!(project.files[0].kind, FileKind::Ui);
}

#[test]
fn find_root_walks_up() {
    let dir = minimal();
    fs::create_dir_all(dir.path().join("src/Screens/deep")).unwrap();
    let found = find_root(&dir.path().join("src/Screens/deep")).unwrap();
    assert_eq!(found, dir.path());
}

#[test]
fn find_root_fails_outside_a_project() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(find_root(dir.path()), None);
}

#[test]
fn full_structure_is_classified_and_sorted() {
    let dir = minimal();
    let r = dir.path();
    write(r, "src/Screens/Home.era", b"");
    write(r, "src/Components/AppButton.era", b"");
    write(r, "src/Layout/MainLayout.era", b"");
    write(r, "Skins/theme.ers", b"");
    write(r, "Assets/Images/hero.PNG", b"x");
    write(r, "Assets/Fonts/Inter.ttf", b"x");
    write(r, "Assets/Files/data.json", b"x");
    write(r, ".hidden/ignored.era", b"");
    write(r, "README.md", b"");

    let (project, diags) = load_project(r);
    assert!(diags.is_empty(), "{diags:?}");
    let paths: Vec<_> = project
        .unwrap()
        .files
        .iter()
        .map(|f| f.rel_path.clone())
        .collect();
    assert_eq!(
        paths,
        [
            "Assets/Files/data.json",
            "Assets/Fonts/Inter.ttf",
            "Assets/Images/hero.PNG",
            "Skins/theme.ers",
            "src/Components/AppButton.era",
            "src/Layout/MainLayout.era",
            "src/Screens/Home.era",
            "src/app.era",
        ]
    );
}

#[test]
fn missing_entry_point_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", MANIFEST.as_bytes());
    let (project, diags) = load_project(dir.path());
    assert!(project.is_some());
    assert_eq!(codes(&diags), ["E0201"]);
    assert!(diags[0].help.as_ref().unwrap().contains("create `src/`"));
}

#[test]
fn entry_point_with_wrong_case_gets_a_hint() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", MANIFEST.as_bytes());
    write(dir.path(), "src/App.era", b"");
    let (_, diags) = load_project(dir.path());
    assert_eq!(codes(&diags), ["E0201"]);
    assert!(diags[0].help.as_ref().unwrap().contains("src/App.era"));
}

#[test]
fn invalid_manifest_still_reports_file_problems() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", b"[project]\n");
    let (project, diags) = load_project(dir.path());
    assert!(project.is_none());
    assert_eq!(codes(&diags), ["E0101", "E0201"]);
}

#[test]
fn misplaced_and_unexpected_files_warn() {
    let dir = minimal();
    let r = dir.path();
    write(r, "src/colors.ers", b"");
    write(r, "Skins/Home.era", b"");
    write(r, "src/notes.txt", b"");
    write(r, "Assets/logo.png", b"x");
    write(r, "Assets/Icons/star.png", b"x");
    write(r, "Assets/Videos/a.mp4", b"x");
    write(r, "src/Logic/app_state.er", b"");
    write(r, "src/Logic/notes.md", b""); // not a logic file

    let (project, diags) = load_project(r);
    let mut got = codes(&diags);
    got.sort();
    assert_eq!(
        got,
        [
            "W0202", "W0203", "W0204", "W0204", "W0301", "W0302", "W0303"
        ]
    );
    assert!(diags.iter().all(|d| !d.is_error()));
    // Ignored files are not part of the project; Logic files are tracked.
    let project = project.unwrap();
    assert_eq!(project.count(|k| *k == FileKind::Ui), 1);
    assert_eq!(project.count(|k| *k == FileKind::Logic), 1);
    assert_eq!(project.count(|k| matches!(k, FileKind::Asset { .. })), 0);
}

#[test]
fn wrong_folder_case_warns() {
    let dir = minimal();
    let r = dir.path();
    write(r, "skins/theme.ers", b"");
    write(r, "src/screens/Home.era", b"");
    write(r, "Assets/images/a.png", b"x");
    let (_, diags) = load_project(r);
    assert_eq!(codes(&diags), ["W0206", "W0206", "W0206"], "{diags:?}");
    let msgs: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(msgs.contains(&"folder `skins` should be named `Skins`"));
    assert!(msgs.contains(&"folder `screens` should be named `Screens`"));
    assert!(msgs.contains(&"folder `images` should be named `Images`"));
}

#[test]
fn non_utf8_source_is_an_error() {
    let dir = minimal();
    write(dir.path(), "src/Screens/Bad.era", &[0xff, 0xfe, 0x00]);
    let (_, diags) = load_project(dir.path());
    assert_eq!(codes(&diags), ["E0207"]);
    assert_eq!(
        diags[0].location.as_ref().unwrap().path,
        "src/Screens/Bad.era"
    );
}

#[test]
fn scaffolded_project_loads_cleanly() {
    let parent = tempfile::tempdir().unwrap();
    let created = create_project(&parent.path().join("my_app")).unwrap();
    assert_eq!(created.name, "my_app");
    let (project, diags) = load_project(&created.root);
    assert!(diags.is_empty(), "{diags:?}");
    let project = project.unwrap();
    assert_eq!(project.manifest.id, "dev.era.my_app");
    let app = fs::read_to_string(created.root.join("src/app.era")).unwrap();
    assert!(app.starts_with("app \"My App\" {"));
}

#[test]
fn scaffold_rejects_bad_names_with_suggestion() {
    let parent = tempfile::tempdir().unwrap();
    let err = create_project(&parent.path().join("My-App")).err().unwrap();
    assert_eq!(err.code, "E0110");
    assert_eq!(err.help.as_deref(), Some("try `era new my_app`"));
    assert!(!parent.path().join("My-App").exists());
}

#[test]
fn scaffold_refuses_non_empty_folder() {
    let parent = tempfile::tempdir().unwrap();
    write(parent.path(), "taken/keep.txt", b"user work");
    let err = create_project(&parent.path().join("taken")).err().unwrap();
    assert_eq!(err.code, "E0111");
    assert_eq!(
        fs::read_to_string(parent.path().join("taken/keep.txt")).unwrap(),
        "user work"
    );
}

#[test]
fn scaffold_accepts_existing_empty_folder() {
    let parent = tempfile::tempdir().unwrap();
    fs::create_dir(parent.path().join("empty")).unwrap();
    assert!(create_project(&parent.path().join("empty")).is_ok());
}
