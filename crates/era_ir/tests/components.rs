mod common;
use common::*;
use era_ir::ir::{Content, Value};

const BUTTON: &str = r#"component AppButton {
    prop label: text
    prop tint: color = primary
    variant tone: solid | outline = solid
    button label {
        background: tint
        when tone is outline { background: none, border_width: thin }
    }
}"#;

fn button(usage: &str) -> Built {
    with(
        &format!("column {{ {usage} }}"),
        &[("src/Components/AppButton.era", BUTTON)],
    )
}

#[test]
fn props_variants_and_defaults_expand() {
    let b = button(
        "AppButton { label: \"Go\" }\nAppButton { label: \"Alt\", tone: outline, tint: danger }",
    );
    let root = &b.app().screens[0].root;
    let solid = &root.children[0];
    assert_eq!(solid.content, Some(Content::Text("Go".into())));
    assert_eq!(solid.origin, ["AppButton"]);
    assert_eq!(solid.id, "Home/0:AppButton");
    let Value::Color(bg) = &solid.props["background"] else {
        panic!()
    };
    assert_eq!(bg.token.as_deref(), Some("primary"));
    assert!(!solid.props.contains_key("border_width"));

    let outline = &root.children[1];
    assert_eq!(outline.props["background"], Value::Transparent);
    assert!(outline.props.contains_key("border_width"));
}

#[test]
fn prop_values_pass_through_nested_components() {
    let swatch = (
        "src/Components/Swatch.era",
        "component Swatch { prop c: color\n prop name: text\n column { stack { background: c }\n text name } }",
    );
    let row = (
        "src/Components/Row.era",
        "component Palette { prop main: color\n row { Swatch { c: main, name: \"main\" } } }",
    );
    let b = with("Palette { main: success }", &[swatch, row]);
    let root = &b.app().screens[0].root;
    let stack = &root.children[0].children[0];
    let Value::Color(c) = &stack.props["background"] else {
        panic!()
    };
    assert_eq!(c.token.as_deref(), Some("success"));
    assert_eq!(stack.origin, ["Palette", "Swatch"]);
    assert_eq!(
        root.children[0].children[1].content,
        Some(Content::Text("main".into()))
    );
}

#[test]
fn missing_required_prop() {
    let d = button("AppButton {}").diags.clone();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].message, "`AppButton` is missing required prop `label`");
}

#[test]
fn unknown_prop_or_variant_suggests() {
    let b = button("AppButton { label: \"x\", lable: \"y\" }");
    assert_eq!(
        b.only("E2143").help.as_deref(),
        Some("did you mean `label`?")
    );
    let b = button("AppButton { label: \"x\", padding: md }");
    assert_eq!(
        b.only("E2143").help.as_deref(),
        Some("`AppButton` accepts: label, tint, tone")
    );
}

#[test]
fn bad_variant_option() {
    let b = button("AppButton { label: \"x\", tone: outlin }");
    let d = b.only("E2142");
    assert_eq!(
        d.message,
        "variant `tone` of `AppButton` has no option `outlin`"
    );
    assert_eq!(d.help.as_deref(), Some("did you mean `outline`?"));
}

#[test]
fn prop_value_types() {
    assert_eq!(
        button("AppButton { label: 3 }").only("E2133").message,
        "this text prop needs text in quotes"
    );
    assert_eq!(
        button("AppButton { label: \"x\", tint: nope }")
            .only("E2132")
            .message,
        "unknown colors token `nope`"
    );
}

#[test]
fn component_bodies_are_checked_even_when_unused() {
    let bad = (
        "src/Components/Bad.era",
        "component Bad { prop n: number\n column { gapp: md\n text n } }",
    );
    let b = with("column {}", &[bad]);
    let codes = b.codes();
    assert_eq!(codes, ["E2120", "E2130"], "{:#?}", b.diags);
    assert_eq!(
        b.diags[1].message,
        "prop `n` is number but text is needed here"
    );
}

#[test]
fn errors_in_components_are_reported_once() {
    let bad = (
        "src/Components/Bad.era",
        "component Bad { column { gapp: md } }",
    );
    let b = with("column { Bad {}\n Bad {}\n Bad {} }", &[bad]);
    assert_eq!(b.codes(), ["E2120"]);
}

#[test]
fn prop_cannot_shadow_a_token() {
    let c = (
        "src/Components/C.era",
        "component C { prop primary: color\n column { background: primary } }",
    );
    let b = with("C { primary: danger }", &[c]);
    assert_eq!(
        b.only("E2131").message,
        "prop `primary` has the same name as the colors token `primary`"
    );
}

#[test]
fn recursion_is_detected() {
    let a = ("src/Components/A.era", "component A { column { B {} } }");
    let bb = ("src/Components/B.era", "component B { column { A {} } }");
    let b = with("A {}", &[a, bb]);
    assert!(b.codes().iter().all(|c| *c == "E2140"), "{:#?}", b.diags);
    assert!(b.diags[0].message.ends_with("uses itself"));
}

#[test]
fn slots_are_filled_from_usage() {
    let frame = (
        "src/Layout/Frame.era",
        "layout Frame { prop heading: text\n column { text heading\n slot } }",
    );
    let card = ("src/Components/Box.era", "component Box { card { slot } }");
    let b = with(
        "Frame { heading: \"Hi\"\n Box { text \"inside\" }\n text \"after\" }",
        &[frame, card],
    );
    let root = &b.app().screens[0].root;
    assert_eq!(root.origin, ["Frame"]);
    assert_eq!(root.children.len(), 3);
    assert_eq!(root.children[0].content, Some(Content::Text("Hi".into())));
    let inner = &root.children[1].children[0];
    assert_eq!(inner.content, Some(Content::Text("inside".into())));
    assert!(
        inner.origin.is_empty(),
        "slot content belongs to the screen"
    );
    assert_eq!(inner.id, "Home/slot/0/slot/0");
}

#[test]
fn component_without_slot_rejects_children_and_values() {
    let b = button("AppButton { label: \"x\"\n text \"child\" }");
    assert_eq!(
        b.only("E2144").message,
        "`AppButton` has no `slot`, so it cannot contain elements"
    );
    let b = button("AppButton \"Go\" { label: \"x\" }");
    assert_eq!(
        b.only("E2141").message,
        "components take named props, not a value"
    );
}
