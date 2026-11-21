use era_syntax::*;

fn ok(src: &str) -> File {
    let (file, diags) = parse(src, "t.era");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:#?}");
    file
}

fn errors(src: &str) -> Vec<era_diagnostics::Diagnostic> {
    let (_, diags) = parse(src, "t.era");
    assert!(!diags.is_empty(), "expected diagnostics");
    diags
}

fn codes(src: &str) -> Vec<&'static str> {
    errors(src).iter().map(|d| d.code).collect()
}

fn root(file: &File) -> &Node {
    match &file.decls[0].body.items.last().unwrap() {
        Item::Node(n) => n,
        other => panic!("expected node, got {other:?}"),
    }
}

#[test]
fn parses_app_with_keyword_property() {
    let f = ok("app \"Studio Preview\" {\n    skin: Studio\n    start: Home\n}\n");
    let d = &f.decls[0];
    assert_eq!(d.kind, DeclKind::App);
    assert_eq!(d.name.name, "Studio Preview");
    let Item::Property(p) = &d.body.items[0] else {
        panic!()
    };
    assert_eq!(p.name.name, "skin");
    assert_eq!(p.value.kind, ValueKind::Ident("Studio".into()));
}

#[test]
fn parses_nested_nodes_args_and_values() {
    let f = ok(r#"
screen Home {
    title: "Home"
    column {
        gap: md, padding: 16
        text "Hello" { style: heading, color: #FF6B35 }
        image "Images/hero.png"
        divider
        input { enabled: false, background: none }
        text label
    }
}"#);
    let col = root(&f);
    assert_eq!(col.name.name, "column");
    let items = &col.block.as_ref().unwrap().items;
    assert_eq!(items.len(), 7);
    let Item::Node(text) = &items[2] else {
        panic!()
    };
    assert_eq!(
        text.arg.as_ref().unwrap().kind,
        ValueKind::String("Hello".into())
    );
    assert_eq!(text.block.as_ref().unwrap().items.len(), 2);
    let Item::Node(divider) = &items[4] else {
        panic!()
    };
    assert!(divider.arg.is_none() && divider.block.is_none());
    let Item::Node(input) = &items[5] else {
        panic!()
    };
    let Item::Property(enabled) = &input.block.as_ref().unwrap().items[0] else {
        panic!()
    };
    assert_eq!(enabled.value.kind, ValueKind::Bool(false));
    let Item::Node(t) = &items[6] else { panic!() };
    assert_eq!(
        t.arg.as_ref().unwrap().kind,
        ValueKind::Ident("label".into())
    );
}

#[test]
fn parses_component_props_variants_when_and_slot() {
    let f = ok(r#"
component AppButton {
    prop label: text
    prop icon: icon? = none
    prop count: number = 3
    variant size: small | medium | large = medium
    button label {
        when size is small { padding_x: md }
        slot
    }
}"#);
    let items = &f.decls[0].body.items;
    let Item::Prop(label) = &items[0] else {
        panic!()
    };
    assert_eq!(
        (label.name.name.as_str(), label.ty, label.optional),
        ("label", PropType::Text, false)
    );
    let Item::Prop(icon) = &items[1] else {
        panic!()
    };
    assert!(icon.optional);
    assert_eq!(icon.default.as_ref().unwrap().kind, ValueKind::None);
    let Item::Variant(v) = &items[3] else {
        panic!()
    };
    let opts: Vec<_> = v.options.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(opts, ["small", "medium", "large"]);
    assert_eq!(v.default.as_ref().unwrap().name, "medium");
    let Item::Node(button) = &items[4] else {
        panic!()
    };
    let body = &button.block.as_ref().unwrap().items;
    let Item::When(w) = &body[0] else { panic!() };
    assert_eq!(
        (w.variant.name.as_str(), w.option.name.as_str()),
        ("size", "small")
    );
    assert!(matches!(body[1], Item::Slot(_)));
}

#[test]
fn parses_skin_sections() {
    let f = ok(
        "skin Studio {\n colors { primary: #FF6B35 }\n colors dark { primary: #000 }\n elevation { none: 0 }\n typography { body { size: 16, tracking: -0.5 } }\n}",
    );
    let items = &f.decls[0].body.items;
    assert_eq!(items.len(), 4);
    let Item::Node(dark) = &items[1] else {
        panic!()
    };
    assert_eq!(
        dark.arg.as_ref().unwrap().kind,
        ValueKind::Ident("dark".into())
    );
    let Item::Node(elev) = &items[2] else {
        panic!()
    };
    let Item::Property(none) = &elev.block.as_ref().unwrap().items[0] else {
        panic!()
    };
    assert_eq!(none.name.name, "none");
}

#[test]
fn one_line_blocks_and_trailing_commas() {
    let f = ok("screen A { row { gap: 4, text \"a\", text \"b\", } }");
    assert_eq!(root(&f).block.as_ref().unwrap().items.len(), 3);
}

#[test]
fn spans_cover_source() {
    let src = "screen Home {\n    text \"Hi\" { style: body }\n}";
    let f = ok(src);
    let d = &f.decls[0];
    assert_eq!(&src[d.span.range()], src);
    assert_eq!(&src[d.name.span.range()], "Home");
    let n = root(&f);
    assert_eq!(&src[n.span.range()], "text \"Hi\" { style: body }");
}

#[test]
fn asset_references_are_collected() {
    let f = ok(
        "component C {\n variant v: a | b\n column {\n image \"Images/a.png\"\n icon \"Icons/s.svg\" { size: 24 }\n image photo\n when v is a { }\n }\n}",
    );
    let refs = asset_references(&f);
    let paths: Vec<_> = refs
        .iter()
        .map(|r| (r.node.as_str(), r.path.as_str()))
        .collect();
    assert_eq!(paths, [("image", "Images/a.png"), ("icon", "Icons/s.svg")]);
}

#[test]
fn empty_file_is_fine() {
    assert!(ok("// nothing yet\n").decls.is_empty());
}

// ----- failure cases -------------------------------------------------------

#[test]
fn missing_closing_brace_points_at_opening_brace() {
    let src = "screen Home {\n  column {\n    text \"x\"\n";
    let e = errors(src);
    assert_eq!(
        e.iter().map(|d| d.code).collect::<Vec<_>>(),
        ["E1102", "E1102"]
    );
    let span = e[1].location.as_ref().unwrap().span.clone().unwrap();
    assert_eq!(&src[span], "{");
}

#[test]
fn top_level_element_and_use_get_help() {
    let e = errors("column { }");
    assert_eq!(e[0].code, "E1101");
    assert!(e[0].help.as_ref().unwrap().contains("inside a `screen`"));
    let e = errors("use skin \"x.ers\"\napp \"A\" {}");
    assert!(e[0].help.as_ref().unwrap().contains("skin: Studio"));
}

#[test]
fn nested_screen_inside_app_is_rejected() {
    // The brief's original syntax nested screens in `app`.
    let e = errors("app \"A\" {\n  screen Home {\n    column {}\n  }\n}\nscreen B { column {} }");
    assert_eq!(e.iter().map(|d| d.code).collect::<Vec<_>>(), ["E1104"]);
}

#[test]
fn bad_declaration_headers() {
    assert_eq!(codes("app Studio {}"), ["E1101"]);
    assert_eq!(codes("screen {}"), ["E1101"]);
    assert_eq!(codes("screen slot {}"), ["E1107"]);
    assert_eq!(codes("screen Home column {}"), ["E1101"]);
}

#[test]
fn missing_value_and_block_as_value() {
    assert_eq!(codes("screen A { column { gap: } }"), ["E1101"]);
    assert_eq!(codes("screen A { column: { } }"), ["E1105"]);
}

#[test]
fn items_need_separators() {
    let e = errors("screen A { column { gap: md padding: lg } }");
    assert_eq!(e[0].code, "E1103");
}

#[test]
fn bad_prop_and_variant_syntax() {
    assert_eq!(
        codes("component C { prop label: string\n column {} }"),
        ["E1106"]
    );
    assert_eq!(
        codes("component C { prop label text\n column {} }"),
        ["E1101"]
    );
    assert_eq!(
        codes("component C { variant v: a | \n column {} }"),
        ["E1101"]
    );
    assert_eq!(
        codes("component C { column { when v small {} } }"),
        ["E1101"]
    );
}

#[test]
fn recovery_reports_independent_errors() {
    let src =
        "screen A {\n column {\n gap: \n text \"ok\"\n padding: {\n }\n }\n}\nscreen B { 42 }";
    let e = errors(src);
    assert_eq!(
        e.iter().map(|d| d.code).collect::<Vec<_>>(),
        ["E1101", "E1105", "E1101"]
    );
}

#[test]
fn lexer_errors_do_not_cascade() {
    // `16px` is reported once by the lexer, not again by the parser.
    assert_eq!(codes("screen A { column { gap: 16px } }"), ["E1006"]);
    assert_eq!(codes("screen A { column { background: #12 } }"), ["E1003"]);
}

#[test]
fn diagnostics_are_sorted_by_position() {
    let (_, diags) = parse("screen A { column { gap: } }\nscreen B { x: @ }", "t.era");
    let starts: Vec<_> = diags
        .iter()
        .map(|d| d.location.as_ref().unwrap().span.clone().unwrap().start)
        .collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted);
}

#[test]
fn use_skin_inside_app_gets_dedicated_error() {
    let e = errors("app \"A\" {\n    use skin \"../Skins/theme.ers\"\n}");
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].code, "E1108");
    assert!(e[0].help.as_ref().unwrap().contains("skin: Studio"));
}
