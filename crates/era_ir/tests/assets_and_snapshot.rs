mod common;
use common::*;
use era_ir::ir::{AssetKind, Content};

fn media(body: &str) -> Built {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", MANIFEST.as_bytes());
    write(dir.path(), "src/app.era", b"app \"T\" {}");
    write(
        dir.path(),
        "src/Home.era",
        format!("screen Home {{ column {{\n{body}\n}} }}").as_bytes(),
    );
    write(dir.path(), "Assets/Images/hero.png", b"png");
    write(dir.path(), "Assets/Icons/star.svg", b"<svg/>");
    build_dir(dir)
}

#[test]
fn assets_resolve_and_are_listed() {
    let b = media("image \"Images/hero.png\"\nicon \"Icons/star.svg\"\nimage \"Images/hero.png\"");
    let app = b.app();
    let assets: Vec<_> = app
        .assets
        .iter()
        .map(|a| (a.kind, a.path.as_str()))
        .collect();
    // Sorted by path, de-duplicated.
    assert_eq!(
        assets,
        [
            (AssetKind::Icon, "Icons/star.svg"),
            (AssetKind::Image, "Images/hero.png")
        ]
    );
    let Some(Content::Asset(a)) = &app.screens[0].root.children[0].content else {
        panic!()
    };
    assert_eq!(a.path, "Images/hero.png");
}

#[test]
fn asset_errors() {
    let b = media("image \"Images/nope.png\"");
    let d = b.only("E2150");
    assert_eq!(d.message, "asset `Images/nope.png` does not exist");
    assert_eq!(
        d.help.as_deref(),
        Some("place the file at `Assets/Images/nope.png`")
    );
    assert_eq!(media("image \"../secret.png\"").only("E2151").code, "E2151");
    assert_eq!(
        media("image \"Icons/star.svg\"").only("E2152").message,
        "`Icons/star.svg` is not a supported image file"
    );
    assert_eq!(
        media("icon \"Images/hero.png\"").only("E2152").message,
        "`Images/hero.png` is not a supported icon file"
    );
}

#[test]
fn image_props_take_asset_paths() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", MANIFEST.as_bytes());
    write(
        dir.path(),
        "src/app.era",
        b"app \"T\" {}\nscreen Home { Avatar { photo: \"Images/hero.png\" } }",
    );
    write(dir.path(), "src/Avatar.era", b"component Avatar { prop photo: image\n prop badge: icon? \n column { image photo\n icon badge } }");
    write(dir.path(), "Assets/Images/hero.png", b"png");
    let b = build_dir(dir);
    let root = &b.app().screens[0].root;
    // The optional icon has no value, so the `icon` element is left out.
    assert_eq!(root.children.len(), 1);
    assert!(
        matches!(&root.children[0].content, Some(Content::Asset(a)) if a.path == "Images/hero.png")
    );
}

fn showcase_dump() -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/showcase");
    let (project, d) = era_project::load_project(&root);
    assert!(d.is_empty(), "{d:#?}");
    let (app, d) = build_project(&project.unwrap());
    assert!(d.is_empty(), "{d:#?}");
    era_ir::dump::dump(&app.unwrap())
}

/// Snapshot of the showcase IR. Regenerate with `UPDATE_SNAPSHOTS=1 cargo test`.
#[test]
fn showcase_ir_snapshot() {
    let dump = showcase_dump();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots/showcase.ir");
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &dump).unwrap();
    }
    let expected =
        std::fs::read_to_string(&path).expect("snapshot missing; run with UPDATE_SNAPSHOTS=1");
    assert_eq!(
        dump, expected,
        "IR changed; review and run with UPDATE_SNAPSHOTS=1"
    );
}

#[test]
fn ir_is_deterministic() {
    assert_eq!(showcase_dump(), showcase_dump());
}
