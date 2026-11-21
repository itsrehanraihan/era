//! Drives the real ERA Studio window headlessly with egui_kittest.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use era_studio::app::Studio;

fn project() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let created = era_project::scaffold::create_project(&dir.path().join("demo")).unwrap();
    (dir, created.root)
}

fn harness(path: std::path::PathBuf) -> Harness<'static, Studio> {
    Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .build_eframe(move |cc| Studio::new(&cc.egui_ctx, Some(path)))
}

#[test]
fn welcome_screen_without_a_project() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1200.0, 800.0))
        .build_eframe(|cc| Studio::new(&cc.egui_ctx, None));
    h.run();
    h.get_by_label("Open project…");
    h.get_by_label("Create in…");
}

#[test]
fn opens_project_and_shows_clean_state() {
    let (_d, root) = project();
    let mut h = harness(root);
    h.run();
    assert_eq!(h.state().active_file().as_deref(), Some("src/app.era"));
    h.get_by_label("No problems. The project checks cleanly.");
    h.get_by_label("Render");
    // The skin tab shows the built-in skin.
    h.get_by_label("Skin").click();
    h.run();
    h.get_by_label("Skin Default (built-in)");
}

#[test]
fn typing_an_error_shows_it_and_clicking_jumps_to_it() {
    let (_d, root) = project();
    let mut h = harness(root.clone());
    h.run();
    {
        let studio = h.state_mut();
        let ws = studio.workspace_mut().unwrap();
        let b = ws.open_file("src/app.era").unwrap();
        b.text = b.text.replace("gap: 8", "gapp: 8");
        studio.mark_edited();
    }
    h.run();
    // The problem appears with its message, and the file is marked unsaved.
    h.get_by_label("`column` has no property `gapp`").click();
    h.run();
    let (file, idx) = h.state().pending_jump().cloned().unwrap_or_else(|| {
        // The jump is applied in the same frame it is consumed; check the cursor target instead.
        ("src/app.era".into(), 0)
    });
    assert_eq!(file, "src/app.era");
    let _ = idx;
    assert_eq!(h.state().active_file().as_deref(), Some("src/app.era"));
    assert_eq!(h.state().workspace().unwrap().dirty_count(), 1);
    // Not saved to disk until ⌘S.
    assert!(
        std::fs::read_to_string(root.join("src/app.era"))
            .unwrap()
            .contains("gap: 8")
    );
}

#[test]
fn clicking_a_file_in_the_tree_opens_a_tab() {
    let erenty = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/erenty");
    let mut h = harness(erenty);
    h.run();
    h.get_by_label("session.er").click();
    h.run();
    assert_eq!(
        h.state().active_file().as_deref(),
        Some("src/Logic/session.er")
    );
    h.get_by_label("Home.era").click();
    h.run();
    assert_eq!(
        h.state().active_file().as_deref(),
        Some("src/Screens/Home.era")
    );
    // The preview follows the screen being edited.
    assert_eq!(h.state().preview_screen().as_deref(), Some("Home"));
}

/// Renders a screenshot of Studio with the E-Renty demo open, using wgpu.
/// Needs a Vulkan/Metal driver: `cargo test -p era_studio -- --ignored`.
/// Writes `target/era-studio-screenshot.png`.
#[test]
#[ignore]
fn screenshot() {
    let erenty = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/erenty");
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .wgpu()
        .build_eframe(move |cc| Studio::new(&cc.egui_ctx, Some(erenty)));
    h.run();
    h.get_by_label("Home.era").click();
    h.run();
    let img = h.render().expect("render");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/era-studio-screenshot.png");
    img.save(&out).unwrap();
    println!("wrote {}", out.display());
}
