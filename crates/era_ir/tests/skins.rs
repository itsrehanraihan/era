mod common;
use common::*;

fn skin_project(skin: &str) -> Built {
    build(&[
        (
            "src/app.era",
            "app \"T\" { skin: Studio }\nscreen Home { column {} }",
        ),
        ("Skins/theme.ers", skin),
    ])
}

#[test]
fn full_skin_is_typed() {
    let b = screen("column {}");
    let s = &b.app().skin;
    assert_eq!(s.name, "Studio");
    assert!(!s.builtin);
    assert_eq!(
        s.colors["primary"],
        era_ir::ir::Color {
            r: 255,
            g: 107,
            b: 53,
            a: 255
        }
    );
    assert_eq!(s.spacing["md"], 16.0);
    assert_eq!(s.typography.styles["heading"].weight, 700);
    assert_eq!(s.typography.styles["body"].weight, 400);
    assert_eq!(s.typography.font, "system");
}

#[test]
fn default_skin_when_app_names_none() {
    let b = build(&[(
        "src/app.era",
        "app \"T\" {}\nscreen Home { column { gap: md\n text \"x\" } }",
    )]);
    let app = b.app();
    assert!(app.skin.builtin);
    assert_eq!(app.skin.name, "Default");
    assert!(app.skin.dark_colors.is_some());
}

#[test]
fn unknown_skin_is_reported() {
    let b = build(&[
        (
            "src/app.era",
            "app \"T\" { skin: Studoi }\nscreen Home { column {} }",
        ),
        ("Skins/theme.ers", SKIN),
    ]);
    assert_eq!(
        b.only("E2101").help.as_deref(),
        Some("did you mean `Studio`?")
    );
}

#[test]
fn skin_split_across_files_merges() {
    let colors = SKIN.split("spacing").next().unwrap().to_string() + "}";
    let b = build(&[
        (
            "src/app.era",
            "app \"T\" { skin: Studio }\nscreen Home { column { gap: md } }",
        ),
        ("Skins/colors.ers", &colors),
        (
            "Skins/rest.ers",
            "skin Studio { spacing { md: 16 }\n typography { body { size: 16 } } }",
        ),
    ]);
    assert_eq!(b.app().skin.spacing["md"], 16.0);
}

#[test]
fn duplicate_token_across_files_cites_first() {
    let b = build(&[
        (
            "src/app.era",
            "app \"T\" { skin: Studio }\nscreen Home { column {} }",
        ),
        ("Skins/a.ers", SKIN),
        ("Skins/b.ers", "skin Studio { spacing { md: 20 } }"),
    ]);
    let d = b.only("E2003");
    assert_eq!(d.location.as_ref().unwrap().path, "Skins/b.ers");
    assert!(
        d.help
            .as_ref()
            .unwrap()
            .starts_with("first defined at Skins/a.ers:")
    );
}

#[test]
fn missing_required_colors_and_body() {
    let b = skin_project(
        "skin Studio { colors { primary: #FFF }\n typography { title { size: 20 } } }",
    );
    assert_eq!(b.codes(), ["E2008", "E2008"]);
    assert!(
        b.diags[0]
            .message
            .contains("background, surface, text, muted, border, on_primary")
    );
    assert!(b.diags[1].message.contains("`body`"));
}

#[test]
fn dark_mode_parity() {
    let dark = SKIN.replacen(
        "spacing",
        "colors dark { background: #000, extra: #111 }\n    spacing",
        1,
    );
    let b = skin_project(&dark);
    let mut codes = b.codes();
    codes.sort();
    assert_eq!(codes, ["E2009", "E2009"]);
    assert!(b.diags.iter().any(|d| {
        d.message
            .starts_with("`colors dark` is missing: border, danger")
    }));
    assert!(
        b.diags
            .iter()
            .any(|d| d.message == "dark color `extra` has no light-mode value")
    );
}

#[test]
fn section_and_value_errors() {
    let bad = SKIN
        .replace("spacing {", "spaceing {")
        .replace("radius { sm: 8", "radius { sm: -8")
        .replace("border { thin: 1 }", "border dark { thin: 1 }")
        .replace(
            "heading { size: 28, weight: bold }",
            "heading { weight: heavy, sise: 3 }",
        );
    let b = skin_project(&bad);
    let mut codes = b.codes();
    codes.sort();
    assert_eq!(
        codes,
        ["E2001", "E2002", "E2004", "E2004", "E2005", "E2007"],
        "{:#?}",
        b.diags
    );
    let unknown = b.diags.iter().find(|d| d.code == "E2001").unwrap();
    assert_eq!(unknown.help.as_deref(), Some("did you mean `spacing`?"));
}

#[test]
fn token_values_must_match_their_section() {
    let bad = SKIN
        .replace("primary: #FF6B35", "primary: \"orange\"")
        .replace("xs: 4", "xs: big");
    let b = skin_project(&bad);
    assert_eq!(b.codes(), ["E2004", "E2004"]);
}

#[test]
fn weights_accept_names_and_hundreds() {
    let s = SKIN.replace(
        "heading { size: 28, weight: bold }",
        "heading { size: 28, weight: 600 }, cap { size: 12, weight: 650 }",
    );
    let b = skin_project(&s);
    assert_eq!(b.only("E2004").message, "invalid font weight");
}

#[test]
fn font_must_exist_unless_system() {
    let with_font = SKIN.replace("typography {", "typography { font: \"Inter\"\n");
    let b = skin_project(&with_font);
    assert_eq!(b.only("E2010").message, "no font files found for `Inter`");

    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "era.toml", MANIFEST.as_bytes());
    write(
        dir.path(),
        "src/app.era",
        b"app \"T\" { skin: Studio }\nscreen Home { column {} }",
    );
    write(dir.path(), "Skins/theme.ers", with_font.as_bytes());
    write(dir.path(), "Assets/Fonts/Inter-Regular.ttf", b"x");
    write(dir.path(), "Assets/Fonts/Inter-Bold.ttf", b"x");
    write(dir.path(), "Assets/Fonts/Interstate.ttf", b"x");
    let b = build_dir(dir);
    let app = b.app();
    assert_eq!(
        app.skin.typography.font_files,
        [
            "Assets/Fonts/Inter-Bold.ttf",
            "Assets/Fonts/Inter-Regular.ttf"
        ]
    );
    let fonts: Vec<_> = app.assets.iter().map(|a| a.path.as_str()).collect();
    assert_eq!(fonts, ["Fonts/Inter-Bold.ttf", "Fonts/Inter-Regular.ttf"]);
}

#[test]
fn token_names_must_be_snake_case() {
    let b = skin_project(&SKIN.replace("xs: 4", "XS: 4"));
    assert_eq!(
        b.only("E2006").message,
        "token name `XS` must be snake_case"
    );
}

#[test]
fn unused_skins_are_still_validated() {
    let b = build(&[
        (
            "src/app.era",
            "app \"T\" { skin: Studio }\nscreen Home { column {} }",
        ),
        ("Skins/theme.ers", SKIN),
        ("Skins/other.ers", "skin Other { colors { primary: #FFF } }"),
    ]);
    assert_eq!(b.codes(), ["E2008", "E2008"]);
}
