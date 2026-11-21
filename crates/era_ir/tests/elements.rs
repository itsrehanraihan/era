mod common;
use common::*;
use era_ir::ir::{Content, ElementKind, Size, Value};

#[test]
fn tokens_resolve_by_property_type_and_keep_names() {
    let b = screen(
        "column { gap: md, padding: 12, background: primary, radius: pill, border_color: #00000080 }",
    );
    let root = &b.app().screens[0].root;
    let dim = |k: &str| match &root.props[k] {
        Value::Dimension(d) => (d.token.clone(), d.dp),
        other => panic!("{other:?}"),
    };
    assert_eq!(dim("gap"), (Some("md".into()), 16.0));
    assert_eq!(dim("padding"), (None, 12.0));
    assert_eq!(dim("radius"), (Some("pill".into()), 999.0));
    let Value::Color(c) = &root.props["background"] else {
        panic!()
    };
    assert_eq!(c.token.as_deref(), Some("primary"));
    let Value::Color(c) = &root.props["border_color"] else {
        panic!()
    };
    assert_eq!((c.token.clone(), c.rgba.a), (None, 128));
}

#[test]
fn defaults_are_applied() {
    let b =
        screen("column { text \"a\"\n card { text \"b\" }\n button \"go\"\n divider\n input {} }");
    let root = &b.app().screens[0].root;
    let text = &root.children[0];
    assert_eq!(text.props["style"], Value::TextStyle("body".into()));
    let card = &root.children[1];
    let Value::Color(bg) = &card.props["background"] else {
        panic!()
    };
    assert_eq!(bg.token.as_deref(), Some("surface"));
    let button = &root.children[2];
    assert_eq!(button.content, Some(Content::Text("go".into())));
    assert!(button.props.contains_key("text_color"));
    assert_eq!(root.children[3].props["thickness"], Value::Number(1.0));
    assert!(root.children[4].props.contains_key("border_color"));
}

#[test]
fn screen_background_and_title() {
    let b = screen("title: \"Welcome\"\ncolumn {}");
    let s = &b.app().screens[0];
    assert_eq!(s.title.as_deref(), Some("Welcome"));
    assert_eq!(s.background.token.as_deref(), Some("background"));
}

#[test]
fn sizes_and_keywords() {
    let b = screen(
        "column { width: fill, height: 200, align: center, distribute: between\n text \"x\" { align: end, max_lines: 2 } }",
    );
    let root = &b.app().screens[0].root;
    assert_eq!(root.props["width"], Value::Size(Size::Fill));
    assert_eq!(root.props["height"], Value::Size(Size::Dp(200.0)));
    assert_eq!(root.props["distribute"], Value::Keyword("between"));
    assert_eq!(root.children[0].props["max_lines"], Value::Number(2.0));
}

#[test]
fn unknown_property_suggests() {
    let b = screen("column { gapp: md }");
    let d = b.only("E2120");
    assert_eq!(d.message, "`column` has no property `gapp`");
    assert_eq!(d.help.as_deref(), Some("did you mean `gap`?"));
}

#[test]
fn unknown_token_suggests_or_lists() {
    let b = screen("column { gap: mdd }");
    assert_eq!(b.only("E2132").help.as_deref(), Some("did you mean `md`?"));
    let b = screen("column { gap: huge }");
    let d = b.only("E2132");
    assert_eq!(d.message, "unknown spacing token `huge`");
    assert_eq!(
        d.help.as_deref(),
        Some("skin `Studio` defines: lg, md, sm, xs")
    );
}

#[test]
fn value_type_errors() {
    let b = screen(
        "column { gap: \"wide\", clip: yes, width: -4, align: middle\n text \"x\" { max_lines: 1.5 } }",
    );
    let msgs: Vec<_> = b
        .diags
        .iter()
        .map(|d| (d.code, d.message.as_str()))
        .collect();
    assert_eq!(
        msgs,
        [
            ("E2133", "`gap` needs a spacing token or a number"),
            ("E2133", "`clip` needs `true` or `false`"),
            ("E2133", "`width` needs `fill`, `fit` or a number"),
            ("E2133", "`align` cannot be `middle`"),
            ("E2133", "`max_lines` needs a whole number of at least 1"),
        ]
    );
}

#[test]
fn duplicate_property() {
    assert_eq!(
        screen("column { gap: md, gap: lg }").only("E2121").message,
        "`gap` is set more than once"
    );
}

#[test]
fn arguments_and_children_rules() {
    assert_eq!(
        screen("column { text }").only("E2116").message,
        "`text` needs a value, like `text \"Hello\"`"
    );
    assert_eq!(
        screen("column \"x\" {}").only("E2113").message,
        "`column` does not take a value"
    );
    assert_eq!(
        screen("column { text 42 }").only("E2115").message,
        "`text` needs text in quotes"
    );
    assert_eq!(
        screen("column { text heading }").only("E2114").message,
        "unknown prop `heading`"
    );
    assert_eq!(
        screen("text \"a\" { text \"b\" }").only("E2122").message,
        "`text` cannot contain elements"
    );
    assert_eq!(
        screen("scroll { text \"a\"\n text \"b\" }")
            .only("E2123")
            .message,
        "`scroll` must contain exactly one element"
    );
    assert_eq!(screen("scroll {}").only("E2123").code, "E2123");
}

#[test]
fn unknown_elements_suggest() {
    assert_eq!(
        screen("colum {}").only("E2112").help.as_deref(),
        Some("did you mean `column`?")
    );
    let b = with(
        "column { AppButon {} }",
        &[(
            "src/Components/AppButton.era",
            "component AppButton { button \"x\" }",
        )],
    );
    assert_eq!(
        b.only("E2112").help.as_deref(),
        Some("did you mean `AppButton`?")
    );
}

#[test]
fn layouts_only_at_screen_root_and_screens_are_not_elements() {
    let layout = ("src/Layout/Frame.era", "layout Frame { column { slot } }");
    let b = with("column { Frame {} }", &[layout]);
    assert_eq!(
        b.only("E2110").message,
        "layout `Frame` can only be the root of a screen"
    );
    let b = with("column { Home {} }", &[]);
    assert_eq!(
        b.only("E2111").message,
        "screen `Home` cannot be used as an element"
    );
}

#[test]
fn app_start_rules() {
    let b = build(&[
        ("src/app.era", "app \"T\" {}"),
        (
            "src/S.era",
            "screen A { column {} }\nscreen B { column {} }",
        ),
    ]);
    assert_eq!(b.only("E2105").help.as_deref(), Some("add `start: A`"));
    let b = build(&[(
        "src/app.era",
        "app \"T\" { start: Hmoe }\nscreen Home { column {} }",
    )]);
    assert_eq!(
        b.only("E2106").help.as_deref(),
        Some("did you mean `Home`?")
    );
    let b = build(&[("src/app.era", "app \"T\" {}")]);
    assert_eq!(b.only("E2104").message, "the project has no screens");
    let b = build(&[(
        "src/app.era",
        "app \"T\" { strat: Home }\nscreen Home { column {} }",
    )]);
    assert_eq!(
        b.only("E2102").help.as_deref(),
        Some("did you mean `start`?")
    );
    let b = build(&[(
        "src/app.era",
        "app \"T\" { start: \"Home\" }\nscreen Home { column {} }",
    )]);
    assert_eq!(b.only("E2103").code, "E2103");
    let b = build(&[("src/app.era", "app \"T\" {}\nscreen Home { column {} }")]);
    assert_eq!(b.app().start, "Home");
}

#[test]
fn ids_are_stable_paths() {
    let b = screen("column { row { text \"a\" }\n text \"b\" }");
    let root = &b.app().screens[0].root;
    assert_eq!(root.id, "Home");
    assert_eq!(root.children[0].children[0].id, "Home/0/0");
    assert_eq!(root.children[1].id, "Home/1");
    assert_eq!(root.children[1].kind, ElementKind::Text);
}
