mod common;
use common::*;
use era_ir::ir::*;

const STORE: &str = r#"store Session {
    state code: text = "1234"
    state count: number
    state verified: bool = false

    action verify {
        set verified = true
        add count 1
    }
    action copy { set code = code }
    action flip { toggle verified }
}"#;

/// A two-screen app with the Session store; `body` is the Home screen's root.
fn app(body: &str) -> Built {
    build(&[
        ("src/app.era", "app \"T\" { start: Home }"),
        ("src/Logic/session.er", STORE),
        ("src/Home.era", &format!("screen Home {{\n{body}\n}}")),
        ("src/Other.era", "screen Other { column {} }"),
    ])
}

fn root(b: &Built) -> &Element {
    &b.app().screens[0].root
}

#[test]
fn stores_are_typed_with_defaults() {
    let b = app("column {}");
    let s = &b.app().stores[0];
    assert_eq!(s.name, "Session");
    let fields: Vec<_> = s
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.ty, f.initial.clone()))
        .collect();
    assert_eq!(
        fields,
        [
            ("code", StateType::Text, Literal::Text("1234".into())),
            ("count", StateType::Number, Literal::Number(0.0)),
            ("verified", StateType::Bool, Literal::Bool(false)),
        ]
    );
    assert_eq!(
        s.actions[0].steps,
        [
            ActionStep::Set {
                field: "verified".into(),
                value: StepValue::Literal(Literal::Bool(true))
            },
            ActionStep::Add {
                field: "count".into(),
                amount: 1.0
            },
        ]
    );
    assert_eq!(
        s.actions[1].steps,
        [ActionStep::Set {
            field: "code".into(),
            value: StepValue::Field("code".into())
        }]
    );
}

#[test]
fn store_errors() {
    let bad = |text: &str| {
        build(&[
            ("src/app.era", "app \"T\" {}\nscreen Home { column {} }"),
            ("src/Logic/s.er", text),
        ])
    };
    assert_eq!(
        bad("store S { state a: txt }")
            .only("E2170")
            .help
            .as_deref(),
        Some("did you mean `text`?")
    );
    assert_eq!(
        bad("store S { state a: bool = 1 }").only("E2171").code,
        "E2171"
    );
    assert_eq!(
        bad("store S { state a: bool\n action x { toggle b } }")
            .only("E2172")
            .code,
        "E2172"
    );
    assert_eq!(
        bad("store S { state a: text\n action x { toggle a } }")
            .only("E2173")
            .message,
        "`toggle` needs bool state, but `a` is text"
    );
    assert_eq!(
        bad("store S { state a: bool\n action x { add a 1 } }")
            .only("E2173")
            .code,
        "E2173"
    );
    assert_eq!(
        bad("store S { state a: number\n action x { set a = \"no\" } }")
            .only("E2173")
            .code,
        "E2173"
    );
}

#[test]
fn on_press_effects_in_order() {
    let b = app("column { on_press: Session.verify then go Other then back }");
    assert_eq!(
        root(&b).props["on_press"],
        Value::Effects(vec![
            Effect::Call {
                store: "Session".into(),
                action: "verify".into()
            },
            Effect::Go("Other".into()),
            Effect::Back,
        ])
    );
}

#[test]
fn action_errors_suggest() {
    assert_eq!(
        app("column { on_press: Sesion.verify }")
            .only("E2161")
            .help
            .as_deref(),
        Some("did you mean `Session`?")
    );
    assert_eq!(
        app("column { on_press: Session.verfy }")
            .only("E2163")
            .help
            .as_deref(),
        Some("did you mean `verify`?")
    );
    assert_eq!(
        app("column { on_press: Session.code }")
            .only("E2163")
            .help
            .as_deref(),
        Some("`code` is state; add an action that changes it")
    );
    assert_eq!(
        app("column { on_press: go Othr }")
            .only("E2165")
            .help
            .as_deref(),
        Some("did you mean `Other`?")
    );
    assert_eq!(
        app("column { on_press: \"go\" }").only("E2164").code,
        "E2164"
    );
    assert_eq!(
        app("column { on_press: forward }").only("E2164").code,
        "E2164"
    );
}

#[test]
fn visible_and_bind() {
    let b = app(
        "column { visible: Session.verified\n input { bind: Session.code }\n text \"x\" { visible: false } }",
    );
    let r = root(&b);
    assert_eq!(
        r.props["visible"],
        Value::State(StateRef {
            store: "Session".into(),
            field: "verified".into(),
            ty: StateType::Bool
        })
    );
    assert!(matches!(&r.children[0].props["bind"], Value::State(s) if s.field == "code"));
    assert_eq!(r.children[1].props["visible"], Value::Bool(false));
}

#[test]
fn visible_and_bind_type_errors() {
    assert_eq!(
        app("column { visible: Session.code }")
            .only("E2166")
            .message,
        "`visible` needs bool state, but `Session.code` is text"
    );
    assert_eq!(
        app("column { input { bind: Session.count } }")
            .only("E2166")
            .code,
        "E2166"
    );
    assert_eq!(
        app("column { input { bind: \"x\" } }").only("E2133").code,
        "E2133"
    );
    assert_eq!(
        app("column { visible: Session.verify }")
            .only("E2162")
            .message,
        "`Session.verify` is an action, not state"
    );
    assert_eq!(
        app("column { visible: Session.verifed }")
            .only("E2162")
            .help
            .as_deref(),
        Some("did you mean `verified`?")
    );
}

#[test]
fn text_interpolation() {
    let b = app(
        "column { text \"Code {Session.code}, {{literal}} x{Session.count}\"\n text Session.verified\n text \"plain\" }",
    );
    let r = root(&b);
    let st = |f: &str, ty| {
        TextPart::State(StateRef {
            store: "Session".into(),
            field: f.into(),
            ty,
        })
    };
    assert_eq!(
        r.children[0].content,
        Some(Content::Template(vec![
            TextPart::Lit("Code ".into()),
            st("code", StateType::Text),
            TextPart::Lit(", {literal} x".into()),
            st("count", StateType::Number),
        ]))
    );
    assert_eq!(
        r.children[1].content,
        Some(Content::Template(vec![st("verified", StateType::Bool)]))
    );
    assert_eq!(r.children[2].content, Some(Content::Text("plain".into())));
}

#[test]
fn interpolation_errors() {
    assert_eq!(
        app("column { text \"open {Session.code\" }")
            .only("E2160")
            .code,
        "E2160"
    );
    assert_eq!(app("column { text \"{}\" }").only("E2160").code, "E2160");
    assert_eq!(
        app("column { text \"{Session.cod}\" }")
            .only("E2162")
            .help
            .as_deref(),
        Some("did you mean `code`?")
    );
}

#[test]
fn action_props_pass_through_components() {
    let btn = (
        "src/Components/Btn.era",
        "component Btn { prop label: text\n prop tap: action?\n button label { on_press: tap } }",
    );
    let b = build(&[
        ("src/app.era", "app \"T\" { start: Home }"),
        ("src/Logic/session.er", STORE),
        (
            "src/Home.era",
            "screen Home { column { Btn { label: \"Go {Session.count}\", tap: Session.verify then go Other }\n Btn { label: \"Plain\" } } }",
        ),
        ("src/Other.era", "screen Other { column {} }"),
        btn,
    ]);
    let r = root(&b);
    assert_eq!(
        r.children[0].props["on_press"],
        Value::Effects(vec![
            Effect::Call {
                store: "Session".into(),
                action: "verify".into()
            },
            Effect::Go("Other".into())
        ])
    );
    assert!(matches!(&r.children[0].content, Some(Content::Template(p)) if p.len() == 2));
    // Optional action with no value: no on_press at all.
    assert!(!r.children[1].props.contains_key("on_press"));
}

#[test]
fn action_prop_type_mismatch() {
    let c = (
        "src/C.era",
        "component C { prop label: text\n button label { on_press: label } }",
    );
    let b = build(&[
        (
            "src/app.era",
            "app \"T\" {}\nscreen Home { C { label: \"x\" } }",
        ),
        c,
    ]);
    assert_eq!(
        b.only("E2130").message,
        "prop `label` is text but `on_press` needs an action: `go Screen`, `back` or `Store.action`"
    );
}

#[test]
fn persist_state() {
    let b = build(&[
        ("src/app.era", "app \"T\" { start: Home }"),
        (
            "src/Logic/cart.er",
            "store Cart {\n    persist state count: number = 2\n    state open: bool\n}",
        ),
        ("src/Home.era", "screen Home { column {} }"),
    ]);
    let f = &b.app().stores[0].fields;
    assert!(f[0].persist && !f[1].persist);
}
