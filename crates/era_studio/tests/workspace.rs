use std::path::Path;

use era_studio::workspace::Workspace;

fn new_project() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let created = era_project::scaffold::create_project(&dir.path().join("demo")).unwrap();
    (dir, created.root)
}

#[test]
fn opens_from_a_subfolder_and_checks_cleanly() {
    let (_d, root) = new_project();
    let ws = Workspace::open(&root.join("src")).unwrap();
    assert_eq!(ws.root, std::fs::canonicalize(&root).unwrap());
    assert_eq!(ws.name(), "demo");
    assert!(ws.problems().is_empty(), "{:?}", ws.problems());
    assert_eq!(ws.app().unwrap().screens[0].name, "Home");
}

#[test]
fn not_a_project() {
    let dir = tempfile::tempdir().unwrap();
    let err = Workspace::open(dir.path()).err().unwrap();
    assert!(err.contains("no `era.toml`"), "{err}");
}

#[test]
fn unsaved_edits_are_checked_live() {
    let (_d, root) = new_project();
    let mut ws = Workspace::open(&root).unwrap();
    let b = ws.open_file("src/app.era").unwrap();
    b.text = b.text.replace("gap: 8", "gapp: 8");
    assert!(b.is_dirty());
    ws.analyze();
    let p = &ws.problems()[0];
    assert_eq!(p.code, "E2120");
    assert_eq!(p.path.as_deref(), Some("src/app.era"));
    assert_eq!(p.help.as_deref(), Some("did you mean `gap`?"));
    let (line, col) = p.line.unwrap();
    let text = &ws.buffers["src/app.era"].text;
    let line_text = text.lines().nth(line - 1).unwrap();
    assert_eq!(&line_text[col - 1..col - 1 + 4], "gapp");
    // The char index points at the same place.
    let idx = p.char_index.unwrap();
    assert_eq!(text.chars().skip(idx).take(4).collect::<String>(), "gapp");
    // Nothing was written to disk.
    assert!(
        std::fs::read_to_string(root.join("src/app.era"))
            .unwrap()
            .contains("gap: 8")
    );
    assert!(ws.app().is_none());
}

#[test]
fn char_index_counts_characters_not_bytes() {
    let (_d, root) = new_project();
    let mut ws = Workspace::open(&root).unwrap();
    let b = ws.open_file("src/app.era").unwrap();
    b.text = b.text.replace(
        "text \"Hello from ERA\"",
        "text \"Héllo ✓\"\n        colum {}",
    );
    ws.analyze();
    let p = ws.problems().iter().find(|p| p.code == "E2112").unwrap();
    let text = &ws.buffers["src/app.era"].text;
    assert_eq!(
        text.chars()
            .skip(p.char_index.unwrap())
            .take(5)
            .collect::<String>(),
        "colum"
    );
}

#[test]
fn save_writes_and_clears_dirty() {
    let (_d, root) = new_project();
    let mut ws = Workspace::open(&root).unwrap();
    ws.open_file("src/app.era")
        .unwrap()
        .text
        .push_str("\n// edited\n");
    assert_eq!(ws.dirty_count(), 1);
    assert_eq!(ws.save_all().unwrap(), 1);
    assert_eq!(ws.dirty_count(), 0);
    assert!(
        std::fs::read_to_string(root.join("src/app.era"))
            .unwrap()
            .ends_with("// edited\n")
    );
}

#[test]
fn tree_groups_example_project() {
    let erenty = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/erenty");
    let ws = Workspace::open(&erenty).unwrap();
    let names: Vec<&str> = ws.tree().iter().map(|g| g.name).collect();
    assert_eq!(
        names,
        [
            "App",
            "Screens",
            "Components",
            "Layout",
            "Logic",
            "Skins",
            "Assets",
            "Project"
        ]
    );
    let screens = &ws.tree()[1].files;
    assert!(screens.contains(&"src/Screens/Home.era".to_string()));
    assert!(Workspace::is_text("src/Logic/session.er"));
    assert!(!Workspace::is_text("Assets/Images/bike.png"));
    assert!(ws.problems().is_empty());
}

#[test]
fn era_binary_prefers_env() {
    // SAFETY: tests in this file do not read ERA_BIN concurrently.
    unsafe { std::env::set_var("ERA_BIN", "/custom/era") };
    assert_eq!(
        era_studio::jobs::era_binary(),
        std::path::PathBuf::from("/custom/era")
    );
    unsafe { std::env::remove_var("ERA_BIN") };
    let args = era_studio::jobs::args(era_studio::jobs::JobKind::Preview, Path::new("/p"));
    assert_eq!(args, ["preview", "/p", "--png"]);
}
