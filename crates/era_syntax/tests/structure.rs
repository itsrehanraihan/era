use era_syntax::*;

struct Src {
    path: &'static str,
    text: String,
    kind: SourceKind,
}

fn ui(path: &'static str, text: &str) -> Src {
    Src {
        path,
        text: text.into(),
        kind: SourceKind::Ui,
    }
}

fn skin(path: &'static str, text: &str) -> Src {
    Src {
        path,
        text: text.into(),
        kind: SourceKind::Skin,
    }
}

const APP: &str = "app \"A\" { start: Home }";

fn check(srcs: &[Src]) -> Vec<era_diagnostics::Diagnostic> {
    let files: Vec<File> = srcs
        .iter()
        .map(|s| {
            let (f, d) = parse(&s.text, s.path);
            assert!(d.is_empty(), "{}: {d:#?}", s.path);
            f
        })
        .collect();
    let units: Vec<SourceUnit> = srcs
        .iter()
        .zip(&files)
        .map(|(s, f)| SourceUnit {
            path: s.path,
            source: &s.text,
            kind: s.kind,
            is_entry: s.path == "src/app.era",
            file: f,
        })
        .collect();
    check_structure(&units, "src/app.era")
}

fn codes(srcs: &[Src]) -> Vec<&'static str> {
    check(srcs).iter().map(|d| d.code).collect()
}

/// Check one UI file alongside a valid app.
fn one(text: &str) -> Vec<&'static str> {
    codes(&[ui("src/app.era", APP), ui("src/X.era", text)])
}

#[test]
fn valid_project_has_no_errors() {
    let srcs = [
        ui("src/app.era", "app \"A\" { skin: Studio, start: Home }"),
        ui(
            "src/Screens/Home.era",
            "screen Home { title: \"Home\"\n Main { heading: \"Hi\"\n text \"x\" } }",
        ),
        ui(
            "src/Layout/Main.era",
            "layout Main { prop heading: text\n column { text heading\n slot } }",
        ),
        ui(
            "src/Components/Btn.era",
            "component Btn { prop label: text\n variant tone: solid | ghost = solid\n button label { when tone is ghost { background: none } } }",
        ),
        skin("Skins/a.ers", "skin Studio { colors { primary: #FFF } }"),
        skin("Skins/b.ers", "skin Studio { spacing { md: 16 } }"),
    ];
    assert_eq!(codes(&srcs), Vec::<&str>::new());
}

#[test]
fn app_rules() {
    assert_eq!(codes(&[ui("src/app.era", "")]), ["E1202"]);
    assert_eq!(
        codes(&[ui("src/app.era", "app \"A\" { column {} }")]),
        ["E1201"]
    );
    assert_eq!(
        codes(&[ui("src/app.era", APP), ui("src/Other.era", APP)]),
        ["E1204", "E1203"]
    );
    // No entry file in the list: E0201 is reported elsewhere, not E1202.
    assert_eq!(
        codes(&[ui("src/X.era", "screen Home { column {} }")]),
        Vec::<&str>::new()
    );
}

#[test]
fn duplicate_app_points_at_first() {
    let d = check(&[ui("src/app.era", &format!("{APP}\n{APP}"))]);
    assert_eq!(d[0].code, "E1203");
    assert_eq!(
        d[0].help.as_deref(),
        Some("first declared at src/app.era:1:5")
    );
}

#[test]
fn file_kind_rules() {
    assert_eq!(one("skin S { colors { a: #FFF } }"), ["E1216"]);
    assert_eq!(
        codes(&[
            ui("src/app.era", APP),
            skin("Skins/x.ers", "screen Home { column {} }")
        ]),
        ["E1216"]
    );
}

#[test]
fn naming_conventions() {
    let d = check(&[
        ui("src/app.era", APP),
        ui("src/X.era", "screen home_page { column {} }"),
    ]);
    assert_eq!(d[0].code, "E1217");
    assert_eq!(d[0].help.as_deref(), Some("rename it to `HomePage`"));
    assert_eq!(
        one("component C { prop Label: text\n column {} }"),
        ["E1218"]
    );
    assert_eq!(
        one("component C { variant size: Small | large\n column {} }"),
        ["E1218"]
    );
}

#[test]
fn screen_root_rules() {
    assert_eq!(one("screen Home { title: \"x\" }"), ["E1206"]);
    assert_eq!(one("screen Home { column {}\n row {} }"), ["E1207"]);
    assert_eq!(one("screen Home { prop a: text\n column {} }"), ["E1201"]);
    assert_eq!(one("screen Home { column { slot } }"), ["E1201"]);
    assert_eq!(one("screen Home { column { when a is b {} } }"), ["E1201"]);
}

#[test]
fn layout_rules() {
    assert_eq!(one("layout L { column {} }"), ["E1209"]);
    assert_eq!(
        one("layout L { column { slot\n row { slot } } }"),
        ["E1210"]
    );
    assert_eq!(
        one("layout L { variant v: a | b\n column { slot } }"),
        ["E1201"]
    );
    assert_eq!(one("layout L { padding: 4\n column { slot } }"), ["E1201"]);
    assert_eq!(one("layout L { slot\n column { slot } }"), ["E1201"]);
}

#[test]
fn component_rules() {
    assert_eq!(one("component C { prop a: text }"), ["E1206"]);
    assert_eq!(one("component C { column {}\n row {} }"), ["E1207"]);
    assert_eq!(one("component C { column { slot\n slot } }"), ["E1210"]);
    assert_eq!(one("component C { column { prop a: text } }"), ["E1201"]);
    assert_eq!(one("component C { when a is b {}\n column {} }"), ["E1201"]);
}

#[test]
fn variant_and_when_rules() {
    assert_eq!(
        one("component C { variant v: a | b = c\n column {} }"),
        ["E1214"]
    );
    assert_eq!(
        one("component C { variant v: a | a\n column {} }"),
        ["E1213"]
    );
    assert_eq!(
        one("component C { prop v: text\n variant v: a | b\n column {} }"),
        ["E1213"]
    );
    assert_eq!(
        one("component C { column { when size is small {} } }"),
        ["E1211"]
    );
    assert_eq!(
        one("component C { variant v: a | b\n column { when v is c {} } }"),
        ["E1212"]
    );
    assert_eq!(
        one("component C { variant v: a | b\n column { when v is a { text \"no\" } } }"),
        ["E1201"]
    );
    assert_eq!(
        one("component C { variant v: a | b\n column { when v is a { when v is b {} } } }"),
        ["E1201"]
    );
}

#[test]
fn duplicate_declarations_across_files() {
    let d = check(&[
        ui("src/app.era", APP),
        ui("src/Screens/Home.era", "screen Home { column {} }"),
        ui("src/Components/Home.era", "component Home { column {} }"),
    ]);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "E1215");
    assert_eq!(d[0].message, "`Home` is already declared as a screen");
    assert_eq!(
        d[0].help.as_deref(),
        Some("first declared at src/Screens/Home.era:1:8")
    );
}

#[test]
fn skin_rules() {
    assert_eq!(
        codes(&[
            ui("src/app.era", APP),
            skin("Skins/a.ers", "skin S { primary: #FFF }")
        ]),
        ["E1201"]
    );
    assert_eq!(
        codes(&[
            ui("src/app.era", APP),
            skin("Skins/a.ers", "skin S { colors { slot } }")
        ]),
        ["E1201"]
    );
}
