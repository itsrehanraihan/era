//! The canvas lays out the real examples.
use std::path::Path;

use era_canvas::{ApproxMeasure, layout_app};
use era_ir::ir::ElementKind;

fn app(name: &str) -> era_ir::ir::App {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    let (project, _) = era_project::load_project(&root);
    let report = era_ir::pipeline::check_sources(&project.expect("project"));
    report.app.expect("example has no errors")
}

#[test]
fn brew_lays_out_a_repeated_list() {
    let app = app("brew");
    let frames = layout_app(&app, 360.0, 780.0, &ApproxMeasure);
    let home = frames.iter().find(|f| f.screen == "Home").unwrap();
    // The screen root fills the screen.
    assert_eq!((home.items[0].rect.w, home.items[0].rect.h), (360.0, 780.0));
    // Three drink cards from Data/drinks.json, stacked top to bottom.
    let texts: Vec<&str> = home
        .items
        .iter()
        .filter_map(|i| i.text.as_ref().map(|t| t.text.as_str()))
        .collect();
    for name in ["Caramel latte", "Matcha tea", "Cold brew", "0 in your cart"] {
        assert!(texts.contains(&name), "{name} missing: {texts:?}");
    }
    let cards: Vec<_> = home
        .items
        .iter()
        .filter(|i| i.kind == ElementKind::Row && i.shadow.is_some())
        .collect();
    assert_eq!(cards.len(), 3);
    assert!(cards[0].rect.y < cards[1].rect.y && cards[1].rect.y < cards[2].rect.y);
    assert!(
        cards.iter().all(|c| (c.rect.w - 328.0).abs() < 0.5),
        "width: 100% minus padding"
    );
    assert!(cards[0].id.ends_with("#0") && cards[2].id.ends_with("#2"));
    // Every card points at the same source: the component use inside `for`.
    assert_eq!(cards[0].source, cards[1].source);
    // "Served hot" only for the hot drinks.
    assert_eq!(texts.iter().filter(|t| **t == "Served hot").count(), 2);
    // Hit testing finds the deepest element.
    let c = &cards[1];
    let hit = home.hit(c.rect.x + 2.0, c.rect.y + 2.0).unwrap();
    assert_eq!(hit.id, c.id);
}

#[test]
fn every_example_lays_out() {
    for name in ["showcase", "erenty", "brew"] {
        let app = app(name);
        for f in layout_app(&app, 360.0, 780.0, &ApproxMeasure) {
            assert!(!f.items.is_empty(), "{name}/{}", f.screen);
            for i in &f.items {
                assert!(
                    i.rect.w.is_finite() && i.rect.h.is_finite(),
                    "{name}/{}: {}",
                    f.screen,
                    i.id
                );
            }
        }
    }
}
