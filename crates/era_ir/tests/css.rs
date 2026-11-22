//! M8 phase 1: CSS names, units, shorthands and layout properties.
mod common;
use common::*;
use era_ir::ir::{Size, Value};

fn root_props(body: &str) -> std::collections::BTreeMap<&'static str, Value> {
    let b = screen(body);
    b.app().screens[0].root.props.clone()
}

#[test]
fn every_spelling_of_a_name_works() {
    for name in [
        "border-radius",
        "borderRadius",
        "border_radius",
        "borderradius",
        "radius",
    ] {
        let p = root_props(&format!("column {{ {name}: 12px }}"));
        assert!(p.contains_key("radius"), "{name}");
    }
    let p = root_props(
        "column { justify-content: space-between; align-items: center; background-color: primary }",
    );
    assert_eq!(p["distribute"], Value::Keyword("between"));
    assert_eq!(p["align"], Value::Keyword("center"));
    assert!(matches!(p["background"], Value::Color(_)));
}

#[test]
fn units_and_sizing_modes() {
    let p = root_props("column { width: 50%; height: 30vh; min-width: 2rem; max-width: 300pt }");
    assert_eq!(p["width"], Value::Size(Size::Percent(50.0)));
    assert_eq!(
        p["height"],
        Value::Size(Size::Viewport {
            vertical: true,
            percent: 30.0
        })
    );
    assert_eq!(p["min_width"], Value::Number(32.0));
    assert_eq!(p["max_width"], Value::Number(400.0));
    let b = screen(
        "row { text \"a\" { width: 2fr }\n text \"b\" { width: min-content }\n text \"c\" { width: auto }\n text \"d\" { width: 120px } }",
    );
    let kids = &b.app().screens[0].root.children;
    assert_eq!(kids[0].props["width"], Value::Size(Size::Fr(2.0)));
    assert_eq!(kids[1].props["width"], Value::Size(Size::MinContent));
    assert_eq!(kids[2].props["width"], Value::Size(Size::Fit));
    assert_eq!(kids[3].props["width"], Value::Size(Size::Dp(120.0)));
}

#[test]
fn padding_margin_inset_shorthands() {
    let p = root_props("column { padding: 4px 8px 12px; margin: 0 auto }");
    let dp = |k: &str| match &p[k] {
        Value::Dimension(d) => d.dp,
        other => panic!("{k}: {other:?}"),
    };
    assert_eq!(
        (
            dp("padding_top"),
            dp("padding_end"),
            dp("padding_bottom"),
            dp("padding_start")
        ),
        (4.0, 8.0, 12.0, 8.0)
    );
    assert_eq!(p["margin_start"], Value::Keyword("auto"));
    assert_eq!(dp("margin_top"), 0.0);
    let b = screen("stack { stack { position: absolute; inset: 4px 8px; width: 10px } }");
    let c = &b.app().screens[0].root.children[0].props;
    assert_eq!(c["top"], Value::Size(Size::Dp(4.0)));
    assert_eq!(c["right"], Value::Size(Size::Dp(8.0)));
    assert_eq!(c["position"], Value::Keyword("absolute"));
}

#[test]
fn border_shorthand() {
    let p = root_props("column { border: 2px solid primary }");
    assert!(matches!(&p["border_width"], Value::Dimension(d) if d.dp == 2.0));
    assert!(matches!(&p["border_color"], Value::Color(c) if c.token.as_deref() == Some("primary")));
    let p = root_props("column { border: none }");
    assert!(matches!(&p["border_width"], Value::Dimension(d) if d.dp == 0.0));
    assert_eq!(
        screen("column { border: 1px double red }")
            .only("E2135")
            .code,
        "E2135"
    );
}

// ----- M8 phase 3: borders, corners, outline ------------------------------

#[test]
fn border_styles_and_sides() {
    let p = root_props("column { border: 1px dashed primary }");
    assert_eq!(p["border_style"], Value::Keyword("dashed"));
    let p = root_props(
        "column { border-left: 4px solid primary; border-bottom: 1px dotted #ccc; border-top-width: 2px }",
    );
    assert!(matches!(&p["border_start_width"], Value::Dimension(d) if d.dp == 4.0));
    assert_eq!(p["border_bottom_style"], Value::Keyword("dotted"));
    assert!(matches!(&p["border_top_width"], Value::Dimension(d) if d.dp == 2.0));
    for name in [
        "border-right-color",
        "borderInlineEndColor",
        "border_right_color",
    ] {
        let p = root_props(&format!("column {{ {name}: primary }}"));
        assert!(p.contains_key("border_end_color"), "{name}");
    }
    // 1-4 values per side, CSS order (top, right, bottom, left).
    let p = root_props("column { border-width: 1px 2px; border-style: solid dashed none }");
    assert!(matches!(&p["border_end_width"], Value::Dimension(d) if d.dp == 2.0));
    assert!(matches!(&p["border_start_width"], Value::Dimension(d) if d.dp == 2.0));
    assert_eq!(p["border_bottom_style"], Value::Keyword("none"));
    // A later all-sides value replaces earlier per-side ones (cascade).
    let p = root_props("column { border-top: 3px solid #FF0000; border: 1px solid #0000FF }");
    assert!(!p.contains_key("border_top_width"));
}

#[test]
fn corner_radii() {
    let p = root_props("column { border-radius: 12px 0 }");
    assert!(matches!(&p["radius_top_start"], Value::Dimension(d) if d.dp == 12.0));
    assert!(matches!(&p["radius_top_end"], Value::Dimension(d) if d.dp == 0.0));
    assert!(matches!(&p["radius_bottom_end"], Value::Dimension(d) if d.dp == 12.0));
    let p = root_props("column { border-top-left-radius: 8px; borderBottomRightRadius: md }");
    assert!(p.contains_key("radius_top_start") && p.contains_key("radius_bottom_end"));
    assert_eq!(
        root_props("column { border-radius: 50% }")["radius"],
        Value::Size(Size::Percent(50.0))
    );
    assert_eq!(
        screen("column { border-radius: 10px / 20px }")
            .only("E2135")
            .code,
        "E2135"
    );
}

#[test]
fn outline() {
    let p = root_props("column { outline: 2px dashed primary; outline-offset: -2px }");
    assert!(matches!(&p["outline_width"], Value::Dimension(d) if d.dp == 2.0));
    assert_eq!(p["outline_style"], Value::Keyword("dashed"));
    assert_eq!(p["outline_offset"], Value::Size(Size::Dp(-2.0)));
}

#[test]
fn layout_and_effect_properties() {
    let p = root_props(
        "column { z-index: -1; opacity: 50%; aspect-ratio: 16 / 9; overflow: hidden; flex: 2; align-self: flex-end }",
    );
    assert_eq!(p["z_index"], Value::Number(-1.0));
    assert_eq!(p["opacity"], Value::Number(0.5));
    assert_eq!(p["aspect_ratio"], Value::Number(16.0 / 9.0));
    assert_eq!(p["overflow"], Value::Keyword("hidden"));
    assert_eq!(p["flex"], Value::Number(2.0));
    assert_eq!(p["align_self"], Value::Keyword("end"));
}

#[test]
fn display_none_and_flex_direction() {
    let b = screen(
        "column { text \"a\"\n text \"b\" { display: none }\n row { flex-direction: column-reverse; text \"1\"; text \"2\" } }",
    );
    let r = &b.app().screens[0].root;
    assert_eq!(r.children.len(), 2, "display: none removes the element");
    let flipped = &r.children[1];
    assert_eq!(flipped.kind, era_ir::ir::ElementKind::Column);
    assert_eq!(
        flipped.children[0].content,
        Some(era_ir::ir::Content::Text("2".into()))
    );
}

#[test]
fn wrong_units_and_values() {
    assert_eq!(
        screen("column { width: 45deg }").only("E2134").message,
        "`width` does not take `deg` values"
    );
    assert_eq!(screen("column { gap: 2fr }").only("E2134").code, "E2134");
    assert_eq!(screen("column { opacity: 2 }").only("E2133").code, "E2133");
    assert_eq!(
        screen("column { position: sticky }").only("E2133").code,
        "E2133"
    );
    assert_eq!(
        screen("column { positon: absolute }")
            .only("E2120")
            .help
            .as_deref(),
        Some("did you mean `position`?")
    );
}
