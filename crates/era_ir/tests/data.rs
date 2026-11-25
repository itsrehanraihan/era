//! Mock data lists and `for` loops (D-091).
mod common;
use common::*;
use era_ir::ir::*;

const PRODUCTS: &str = r#"[
  {"name": "Tea", "price": 3, "photo": "Images/a.png", "hot": true},
  {"name": "Juice", "price": 4.5, "photo": "Images/a.png"}
]"#;

fn menu(body: &str) -> Built {
    build(&[
        ("src/app.era", "app \"T\" { start: Home }"),
        ("Data/products.json", PRODUCTS),
        ("Assets/Images/a.png", "png"),
        (
            "src/Home.era",
            &format!("screen Home {{\n column {{\n{body}\n}}\n}}"),
        ),
    ])
}

#[test]
fn for_loops_repeat_elements_with_item_fields() {
    let b = menu(
        "for p in Products {\n row {\n image p.photo\n text \"{p.name}: {p.price}\"\n text \"Hot\" { visible: p.hot }\n }\n }",
    );
    let app = b.app();
    assert_eq!(app.data[0].name, "Products");
    assert_eq!(app.data[0].rows.len(), 2);
    let f = &app.screens[0].root.children[0];
    assert_eq!(f.kind, ElementKind::For);
    assert_eq!(
        f.content,
        Some(Content::Repeat {
            var: "p".into(),
            list: "Products".into()
        })
    );
    let row = &f.children[0];
    assert!(
        matches!(&row.children[0].content, Some(Content::ItemAsset { field, .. }) if field.field == "photo")
    );
    let Some(Content::Template(parts)) = &row.children[1].content else {
        panic!("{:?}", row.children[1].content)
    };
    assert!(matches!(&parts[0], TextPart::State(r) if r.store == "p" && r.field == "name"));
    assert!(
        matches!(&row.children[2].props["visible"], Value::State(r) if r.ty == StateType::Bool)
    );
    // Item images are checked and packaged like other images.
    assert!(app.assets.iter().any(|a| a.path == "Images/a.png"));
}

#[test]
fn for_loop_errors() {
    assert_eq!(
        menu("for p in Prodcts { text \"x\" }")
            .only("E2181")
            .help
            .as_deref(),
        Some("did you mean `Products`?")
    );
    assert_eq!(
        menu("for p in Products { text \"{p.nme}\" }")
            .only("E2182")
            .help
            .as_deref(),
        Some("did you mean `name`?")
    );
    assert_eq!(
        menu("for P in Products { text \"x\" }").only("E2181").code,
        "E2181"
    );
    assert_eq!(
        menu("for p in Products { gap: 4 }").only("E2181").code,
        "E2181"
    );
    assert_eq!(
        menu("for p in Products { image p.price }")
            .only("E2182")
            .code,
        "E2182"
    );
    // `p` only exists inside its loop.
    assert_eq!(
        menu("for p in Products { text \"a\" }\n text \"{p.name}\"")
            .codes()
            .len(),
        1
    );
}

#[test]
fn missing_item_images_are_reported() {
    let b = build(&[
        ("src/app.era", "app \"T\" { start: Home }"),
        ("Data/products.json", r#"[{"photo": "Images/missing.png"}]"#),
        (
            "src/Home.era",
            "screen Home { column { for p in Products { image p.photo } } }",
        ),
    ]);
    assert!(!b.codes().is_empty(), "missing image should be an error");
}

#[test]
fn data_items_can_be_passed_into_components() {
    let b = build(&[
        ("src/app.era", "app \"T\" { start: Home }"),
        ("Data/products.json", PRODUCTS),
        ("Assets/Images/a.png", "png"),
        (
            "src/Card.era",
            "component Card {\n prop name: text\n prop photo: image\n prop hot: bool = false\n row {\n image photo\n text name\n text \"Hot\" { visible: hot }\n }\n}",
        ),
        (
            "src/Home.era",
            "screen Home { column { for p in Products { Card { name: \"{p.name}\", photo: p.photo, hot: p.hot } } } }",
        ),
    ]);
    let row = &b.app().screens[0].root.children[0].children[0];
    assert!(
        matches!(&row.children[0].content, Some(Content::ItemAsset { field, .. }) if field.store == "p")
    );
    assert!(matches!(&row.children[2].props["visible"], Value::State(r) if r.field == "hot"));
    let bad = build(&[
        ("src/app.era", "app \"T\" { start: Home }"),
        ("Data/products.json", PRODUCTS),
        ("Assets/Images/a.png", "png"),
        (
            "src/Card.era",
            "component Card {\n prop on: bool = false\n row { text \"x\" { visible: on } }\n}",
        ),
        (
            "src/Home.era",
            "screen Home { column { for p in Products { Card { on: p.name } } } }",
        ),
    ]);
    assert_eq!(bad.only("E2133").code, "E2133");
}
