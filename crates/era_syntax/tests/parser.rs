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
    assert_eq!(codes("screen A { column { gap: 16qq } }"), ["E1006"]);
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

// ----- M6: stores and action values ----------------------------------------

#[test]
fn parses_store_with_state_and_actions() {
    let f = ok(r#"
store Session {
    state code: text = ""
    state count: number
    state ok: bool = false

    action verify {
        set ok = true
        toggle ok
        add count -1
        set code = other
    }
}"#);
    let d = &f.decls[0];
    assert_eq!(d.kind, DeclKind::Store);
    let Item::State(code) = &d.body.items[0] else {
        panic!()
    };
    assert_eq!(
        (code.name.name.as_str(), code.ty.name.as_str()),
        ("code", "text")
    );
    let Item::State(count) = &d.body.items[1] else {
        panic!()
    };
    assert!(count.initial.is_none());
    let Item::Action(a) = &d.body.items[3] else {
        panic!()
    };
    assert_eq!(a.steps.len(), 4);
    assert!(matches!(&a.steps[0].kind, StepKind::Set { field, .. } if field.name == "ok"));
    assert!(matches!(&a.steps[1].kind, StepKind::Toggle { field } if field.name == "ok"));
    assert!(
        matches!(&a.steps[2].kind, StepKind::Add { amount, .. } if amount.kind == ValueKind::Number(-1.0))
    );
    assert!(
        matches!(&a.steps[3].kind, StepKind::Set { value, .. } if value.kind == ValueKind::Ident("other".into()))
    );
}

#[test]
fn parses_paths_go_back_and_then_chains() {
    let f = ok(
        "screen A { column {\n button \"x\" { on_press: Session.verify then go Home then back }\n text Session.code\n input { bind: Session.code }\n } }",
    );
    let col = root(&f);
    let items = &col.block.as_ref().unwrap().items;
    let Item::Node(button) = &items[0] else {
        panic!()
    };
    let Item::Property(p) = &button.block.as_ref().unwrap().items[0] else {
        panic!()
    };
    let ValueKind::Seq(parts) = &p.value.kind else {
        panic!("{:?}", p.value.kind)
    };
    assert_eq!(parts.len(), 3);
    assert!(matches!(&parts[0].kind, ValueKind::Path(v) if v.len() == 2 && v[1].name == "verify"));
    assert!(matches!(&parts[1].kind, ValueKind::Go(t) if t.name == "Home"));
    assert_eq!(parts[2].kind, ValueKind::Ident("back".into()));
    let Item::Node(text) = &items[1] else {
        panic!()
    };
    assert!(
        matches!(&text.arg.as_ref().unwrap().kind, ValueKind::Path(v) if v[0].name == "Session")
    );
}

#[test]
fn single_go_and_plain_idents_still_parse() {
    let f = ok("screen A { column { on_press: go Home, gap: md } }");
    let Item::Property(p) = &root(&f).block.as_ref().unwrap().items[0] else {
        panic!()
    };
    assert!(matches!(&p.value.kind, ValueKind::Go(t) if t.name == "Home"));
    let Item::Property(gap) = &root(&f).block.as_ref().unwrap().items[1] else {
        panic!()
    };
    assert_eq!(gap.value.kind, ValueKind::Ident("md".into()));
}

#[test]
fn store_syntax_errors() {
    assert_eq!(codes("store S { text \"x\" }"), ["E1109"]);
    assert_eq!(codes("store S { action a { jump x } }"), ["E1110"]);
    assert_eq!(codes("store S { state x text }"), ["E1101"]);
    assert_eq!(codes("store S { action a { set x 1 } }"), ["E1101"]);
    assert_eq!(
        codes("screen A { column { on_press: Session. } }"),
        ["E1101"]
    );
}

// ----- M8: CSS values ------------------------------------------------------

fn prop_value(src: &str) -> ValueKind {
    let f = ok(&format!("screen A {{ column {{ {src} }} }}"));
    let Item::Property(p) = &root(&f).block.as_ref().unwrap().items[0] else {
        panic!()
    };
    p.value.kind.clone()
}

#[test]
fn css_terms_calls_and_layers() {
    assert!(matches!(prop_value("padding: 8px 16px"), ValueKind::Terms(t) if t.len() == 2));
    let ValueKind::Terms(t) = prop_value("border: 1px solid #ccc") else {
        panic!()
    };
    assert_eq!(t[1].kind, ValueKind::Ident("solid".into()));
    let ValueKind::Call(name, args) = prop_value("background: rgba(0, 0, 0, .1)") else {
        panic!()
    };
    assert_eq!((name.as_str(), args.len()), ("rgba", 4));
    let ValueKind::Call(name, args) =
        prop_value("background: linear-gradient(135deg, #00E6B5, #00B894 80%)")
    else {
        panic!()
    };
    assert_eq!(name, "linear-gradient");
    assert!(matches!(&args[2].kind, ValueKind::Terms(t) if t.len() == 2));
    let ValueKind::Layers(l) = prop_value("box-shadow: 0 1px 2px red, 0 4px 8px rgba(0, 0, 0, .2)")
    else {
        panic!()
    };
    assert_eq!(l.len(), 2);
    let ValueKind::Terms(t) = prop_value("aspect-ratio: 16 / 9") else {
        panic!()
    };
    assert_eq!(t[1].kind, ValueKind::Slash);
    assert!(
        matches!(prop_value("grid-template-columns: repeat(3, 1fr)"), ValueKind::Call(n, _) if n == "repeat")
    );
}

#[test]
fn semicolons_and_old_commas_both_separate_items() {
    let f = ok(
        "screen A { column { gap: 8px; padding: 4px 8px; text \"a\", text \"b\", width: 50% } }",
    );
    assert_eq!(root(&f).block.as_ref().unwrap().items.len(), 5);
    // `!important` is ignored.
    let f = ok("screen A { column { gap: 8px !important } }");
    let Item::Property(p) = &root(&f).block.as_ref().unwrap().items[0] else {
        panic!()
    };
    assert_eq!(p.value.kind, ValueKind::Dimension(8.0, "px".into()));
}

#[test]
fn then_chains_still_work_after_terms() {
    let ValueKind::Seq(parts) = prop_value("on_press: Session.verify then go Home") else {
        panic!()
    };
    assert_eq!(parts.len(), 2);
}
