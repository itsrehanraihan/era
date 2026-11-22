use std::path::Path;

use era_codegen_compose::{Output, generate};
use era_ir::ir::App;
use era_project::{FileKind, load_project};
use era_syntax::{SourceKind, parse};

const MANIFEST: &str = "[project]\nname = \"t\"\nid = \"dev.era.t\"\nversion = \"0.1.0\"\n\n[era]\nedition = \"2026\"\n";

fn build_app(root: &Path) -> App {
    let (project, d) = load_project(root);
    assert!(d.is_empty(), "{d:#?}");
    let project = project.unwrap();
    let mut sources = Vec::new();
    for f in &project.files {
        let kind = match f.kind {
            FileKind::Ui => SourceKind::Ui,
            FileKind::Skin => SourceKind::Skin,
            FileKind::Logic => SourceKind::Logic,
            _ => continue,
        };
        let text = std::fs::read_to_string(&f.abs_path).unwrap();
        let (file, d) = parse(&text, &f.rel_path);
        assert!(d.is_empty(), "{d:#?}");
        sources.push((f.rel_path.clone(), text, kind, file));
    }
    sources.sort_by_key(|(p, ..)| p != "src/app.era");
    let parsed: Vec<era_ir::ParsedFile> = sources
        .iter()
        .map(|(p, t, k, f)| era_ir::ParsedFile {
            path: p,
            source: t,
            kind: *k,
            file: f,
        })
        .collect();
    let (app, d) = era_ir::build_app(&project, &parsed);
    assert!(d.is_empty(), "{d:#?}");
    app.unwrap()
}

/// Generate code for one screen whose body is `body` (default skin).
fn screen_code(body: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("era.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.era"),
        format!("app \"T\" {{}}\nscreen Home {{\n{body}\n}}"),
    )
    .unwrap();
    let out = generate(&build_app(dir.path()));
    out.files
        .into_iter()
        .find(|(p, _)| p.ends_with("HomeScreen.kt"))
        .unwrap()
        .1
}

fn example(name: &str) -> Output {
    generate(&build_app(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(name),
    ))
}

#[test]
fn files_and_package() {
    let out = example("showcase");
    assert_eq!(out.package, "dev.era.showcase.ui");
    let paths: Vec<_> = out.files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        [
            "dev/era/showcase/ui/EraRuntime.kt",
            "dev/era/showcase/ui/EraScreens.kt",
            "dev/era/showcase/ui/EraStores.kt",
            "dev/era/showcase/ui/EraTheme.kt",
            "dev/era/showcase/ui/screens/HomeScreen.kt",
            "dev/era/showcase/ui/screens/LayoutScreen.kt",
        ]
    );
}

#[test]
fn screen_root_fills_and_uses_skin_background() {
    let code = screen_code("column { text \"Hi\" }");
    assert!(code.contains("fun HomeScreen() {"), "{code}");
    assert!(
        code.contains("BoxWithConstraints(Modifier.fillMaxSize().background(c.background)) {"),
        "{code}"
    );
    assert!(
        code.contains("Column(Modifier.fillMaxWidth().fillMaxHeight().safeDrawingPadding()) {"),
        "{code}"
    );
    assert!(
        code.contains(
            "BasicText(\"Hi\", modifier = Modifier, style = EraType.body.copy(color = c.text))"
        ),
        "{code}"
    );
}

#[test]
fn only_the_screen_root_gets_safe_area_padding() {
    let code = screen_code("column { row { text \"a\" } }");
    assert_eq!(code.matches("safeDrawingPadding()").count(), 1, "{code}");
}

#[test]
fn fill_follows_d041() {
    let code = screen_code(
        "column { row { text \"a\" { width: fill }\n spacer\n text \"b\" { height: fill } }\n text \"c\" { height: fill, width: fill } }",
    );
    // Main axis of a row -> weight; cross axis -> fillMax*.
    assert!(
        code.contains("BasicText(\"a\", modifier = Modifier.weight(1f)"),
        "{code}"
    );
    assert!(code.contains("Spacer(Modifier.weight(1f))"), "{code}");
    assert!(
        code.contains("BasicText(\"b\", modifier = Modifier.fillMaxHeight()"),
        "{code}"
    );
    // In a column: width is cross axis, height is main axis.
    assert!(
        code.contains("BasicText(\"c\", modifier = Modifier.fillMaxWidth().weight(1f)"),
        "{code}"
    );
}

#[test]
fn fill_is_ignored_along_a_scrolling_axis() {
    let code = screen_code("scroll { column { text \"a\" { height: fill, width: fill } } }");
    assert!(
        code.contains("verticalScroll(rememberScrollState())"),
        "{code}"
    );
    // No weight(1f) inside unbounded height (Compose would crash).
    assert!(
        code.contains("BasicText(\"a\", modifier = Modifier.fillMaxWidth()"),
        "{code}"
    );
    assert!(!code.contains("weight(1f)"), "{code}");
}

#[test]
fn tokens_become_theme_references() {
    let code = screen_code(
        "card { gap: md, padding_x: lg, padding_y: sm, radius: pill, background: primary\n text \"x\" { style: heading, color: #FF000080 } }",
    );
    assert!(
        code.contains("Column(Modifier.fillMaxWidth().fillMaxHeight().shadow(EraElevation.sm, RoundedCornerShape(EraRadius.pill), clip = false).background(c.primary, RoundedCornerShape(EraRadius.pill)).safeDrawingPadding().padding(horizontal = EraSpacing.lg, vertical = EraSpacing.sm), verticalArrangement = Arrangement.spacedBy(EraSpacing.md, Alignment.Top))"),
        "{code}"
    );
    assert!(
        code.contains("style = EraType.heading.copy(color = Color(0x80FF0000))"),
        "{code}"
    );
}

#[test]
fn padding_specific_sides_win() {
    let code = screen_code("column { padding: 4, padding_top: 10 }");
    assert!(
        code.contains(".padding(start = 4.dp, top = 10.dp, end = 4.dp, bottom = 4.dp)"),
        "{code}"
    );
}

#[test]
fn alignment_arrangement_and_stretch() {
    let code = screen_code(
        "column { align: stretch\n row { distribute: between, align: center\n text \"a\" }\n text \"b\" }",
    );
    assert!(code.contains("Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically)"), "{code}");
    assert!(
        code.contains("BasicText(\"b\", modifier = Modifier.fillMaxWidth()"),
        "{code}"
    );
}

#[test]
fn images_icons_and_text_escaping() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("era.toml"), MANIFEST).unwrap();
    for (p, c) in [
        ("Assets/Images/a.png", "x"),
        ("Assets/Icons/s.svg", "<svg/>"),
    ] {
        let path = dir.path().join(p);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, c).unwrap();
    }
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.era"),
        "app \"T\" {}\nscreen Home { column {\n image \"Images/a.png\" { fit: contain, radius: md, clip: true }\n icon \"Icons/s.svg\" { color: primary }\n text \"Price: $5 \\\"now\\\"\" } }",
    )
    .unwrap();
    let out = generate(&build_app(dir.path()));
    let code = &out
        .files
        .iter()
        .find(|(p, _)| p.ends_with("HomeScreen.kt"))
        .unwrap()
        .1;
    assert!(code.contains("Image(painter = eraImage(\"Images/a.png\"), contentDescription = null, modifier = Modifier.clip(RoundedCornerShape(EraRadius.md)), contentScale = ContentScale.Fit)"), "{code}");
    assert!(code.contains("Image(painter = eraIcon(\"Icons/s.svg\"), contentDescription = null, modifier = Modifier.size(24.dp), colorFilter = ColorFilter.tint(c.primary))"), "{code}");
    assert!(code.contains(r#"BasicText("Price: \$5 \"now\"""#), "{code}");
}

#[test]
fn theme_has_palettes_tokens_and_styles() {
    let out = example("showcase");
    let theme = &out
        .files
        .iter()
        .find(|(p, _)| p.ends_with("EraTheme.kt"))
        .unwrap()
        .1;
    assert!(theme.contains("    val onPrimary: Color,"), "{theme}");
    assert!(
        theme.contains("    primary = Color(0xFFFF6B35),"),
        "{theme}"
    );
    assert!(
        theme.contains("val EraDarkColors: EraColors = EraLightColors"),
        "{theme}"
    );
    assert!(
        theme.contains("const val EraHasDarkMode: Boolean = false"),
        "{theme}"
    );
    assert!(theme.contains("    val md = 16.dp"), "{theme}");
    assert!(theme.contains("val display = TextStyle(fontFamily = EraFont, fontSize = 40.sp, fontWeight = FontWeight(700), lineHeight = 46.sp, letterSpacing = (-0.5).sp)"), "{theme}");
    assert!(
        theme.contains("val EraFont: FontFamily = FontFamily.Default"),
        "{theme}"
    );
}

#[test]
fn custom_fonts_and_dark_mode() {
    let out = example("erenty");
    let theme = &out
        .files
        .iter()
        .find(|(p, _)| p.ends_with("EraTheme.kt"))
        .unwrap()
        .1;
    assert!(theme.contains("val EraFont: FontFamily by lazy { eraFontFamily(listOf(\"Fonts/Inter-Bold.ttf\", \"Fonts/Inter-Medium.ttf\", \"Fonts/Inter-Regular.ttf\", \"Fonts/Inter-SemiBold.ttf\")) }"), "{theme}");

    // The default skin has dark colors.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("era.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.era"),
        "app \"T\" {}\nscreen Home { column {} }",
    )
    .unwrap();
    let out = generate(&build_app(dir.path()));
    let theme = &out
        .files
        .iter()
        .find(|(p, _)| p.ends_with("EraTheme.kt"))
        .unwrap()
        .1;
    assert!(theme.contains("val EraDarkColors = EraColors("), "{theme}");
    assert!(
        theme.contains("const val EraHasDarkMode: Boolean = true"),
        "{theme}"
    );
}

#[test]
fn registry_lists_screens_in_order() {
    let out = example("erenty");
    let reg = &out
        .files
        .iter()
        .find(|(p, _)| p.ends_with("EraScreens.kt"))
        .unwrap()
        .1;
    assert!(
        reg.contains("const val EraStartScreen: String = \"Splash\""),
        "{reg}"
    );
    let list = &reg[reg.find("listOf(").unwrap()..];
    let order: Vec<_> = ["\"Home\"", "\"Menu\"", "\"Splash\"", "\"VerifyCode\""]
        .iter()
        .map(|n| list.find(n).unwrap())
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]));
}

fn snapshot(name: &str) {
    let out = example(name);
    let mut text = String::new();
    for (p, c) in &out.files {
        text.push_str(&format!("===== {p} =====\n{c}"));
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/snapshots/{name}.kt.txt"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let expected =
        std::fs::read_to_string(&path).expect("snapshot missing; run with UPDATE_SNAPSHOTS=1");
    assert_eq!(
        text, expected,
        "generated Kotlin changed; review and run with UPDATE_SNAPSHOTS=1"
    );
    // Deterministic: a second run is identical.
    assert_eq!(example(name), out);
}

#[test]
fn showcase_kotlin_snapshot() {
    snapshot("showcase");
}

#[test]
fn erenty_kotlin_snapshot() {
    snapshot("erenty");
}

// ----- M6: interactions ------------------------------------------------------

/// Generated files for an app with a store and two screens; Home's root is `body`.
fn interactive(body: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("era.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(dir.path().join("src/Logic")).unwrap();
    std::fs::write(
        dir.path().join("src/Logic/s.er"),
        "store Session {\n state code: text = \"$1\"\n state count: number = 2\n state on: bool\n action bump { add count 1\n toggle on\n set code = \"x\" }\n}",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("src/app.era"),
        format!("app \"T\" {{ start: Home }}\nscreen Home {{\n{body}\n}}\nscreen Other {{ column {{}} }}"),
    )
    .unwrap();
    generate(&build_app(dir.path()))
}

fn file<'a>(out: &'a Output, suffix: &str) -> &'a str {
    &out.files
        .iter()
        .find(|(p, _)| p.ends_with(suffix))
        .unwrap()
        .1
}

#[test]
fn stores_become_state_objects() {
    let out = interactive("column {}");
    let st = file(&out, "EraStores.kt");
    assert!(st.contains("object Session {"), "{st}");
    assert!(
        st.contains("    var code: String by mutableStateOf(\"\\$1\")"),
        "{st}"
    );
    assert!(
        st.contains("    var count: Double by mutableStateOf(2.0)"),
        "{st}"
    );
    assert!(
        st.contains("    var on: Boolean by mutableStateOf(false)"),
        "{st}"
    );
    assert!(
        st.contains(
            "    fun bump() {\n        count += 1.0\n        on = !on\n        code = \"x\"\n    }"
        ),
        "{st}"
    );
}

#[test]
fn on_press_becomes_clickable() {
    let out = interactive("column { on_press: Session.bump then go Other then back\n text \"a\" }");
    let code = file(&out, "HomeScreen.kt");
    assert!(
        code.contains("clickable { Session.bump(); EraNav.go(\"Other\"); EraNav.back() }"),
        "{code}"
    );
}

#[test]
fn visible_wraps_or_omits() {
    let out = interactive(
        "column { text \"shown\" { visible: Session.on }\n text \"gone\" { visible: false } }",
    );
    let code = file(&out, "HomeScreen.kt");
    assert!(code.contains("if (Session.on) {"), "{code}");
    assert!(code.contains("is hidden (visible: false)"), "{code}");
    assert!(!code.contains("\"gone\""), "{code}");
}

#[test]
fn templates_and_bind() {
    let out = interactive(
        "column { text \"Code {Session.code}: {Session.count} ($)\"\n input { bind: Session.code, placeholder: \"n={Session.count}\" } }",
    );
    let code = file(&out, "HomeScreen.kt");
    assert!(
        code.contains(r#"BasicText("Code ${Session.code}: ${eraText(Session.count)} (\$)""#),
        "{code}"
    );
    assert!(
        code.contains(r#"EraInput("n=${eraText(Session.count)}""#),
        "{code}"
    );
    assert!(
        code.contains("value = Session.code, onValueChange = { Session.code = it })"),
        "{code}"
    );
}

#[test]
fn runtime_has_navigation() {
    let out = interactive("column {}");
    let rt = file(&out, "EraRuntime.kt");
    assert!(rt.contains("object EraNav {"), "{rt}");
    assert!(rt.contains("fun EraApp() {"), "{rt}");
    // An app without stores gets no EraStores.kt.
    let plain = screen_code("column {}");
    assert!(!plain.contains("Session"));
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("era.toml"), MANIFEST).unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/app.era"),
        "app \"T\" {}\nscreen Home { column {} }",
    )
    .unwrap();
    let out = generate(&build_app(dir.path()));
    assert!(out.files.iter().all(|(p, _)| !p.ends_with("EraStores.kt")));
}

// ----- M8: CSS layout ---------------------------------------------------------

#[test]
fn css_sizes_become_compose_modifiers() {
    let code = screen_code(
        "column { row { text \"a\" { width: 25% }\n text \"b\" { width: 2fr }\n text \"c\" { width: 30vw; aspect-ratio: 2 } }\n text \"d\" { width: max-content; opacity: .5; z-index: 3 } }",
    );
    assert!(
        code.contains("BasicText(\"a\", modifier = Modifier.fillMaxWidth(0.25f)"),
        "{code}"
    );
    assert!(
        code.contains("BasicText(\"b\", modifier = Modifier.weight(2f)"),
        "{code}"
    );
    assert!(code.contains("width(eraVw(30f)).aspectRatio(2f)"), "{code}");
    assert!(
        code.contains("width(IntrinsicSize.Max).zIndex(3f).alpha(0.5f)"),
        "{code}"
    );
}

#[test]
fn margins_and_auto_centering() {
    let code = screen_code(
        "column { text \"a\" { margin: 8px 16px }\n column { width: 200px; margin: 0 auto } }",
    );
    assert!(
        code.contains(
            "BasicText(\"a\", modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)"
        ),
        "{code}"
    );
    assert!(
        code.contains(
            "Modifier.padding(vertical = 0.dp).align(Alignment.CenterHorizontally).width(200.dp)"
        ),
        "{code}"
    );
}

#[test]
fn absolute_fixed_and_relative_positioning() {
    let code = screen_code(
        "column { padding: 20px\n text \"x\" { position: relative; top: 4px }\n stack { position: absolute; top: -8px; right: -8px; width: 28px; height: 28px }\n stack { position: fixed; bottom: 24px; right: 20px } }",
    );
    // relative: offset only
    assert!(
        code.contains("BasicText(\"x\", modifier = Modifier.offset(x = 0.dp, y = 4.dp)"),
        "{code}"
    );
    // absolute: overlay against the padding box, not squeezed
    assert!(code.contains("Box(Modifier.matchParentSize())"), "{code}");
    assert!(code.contains("align(Alignment.TopEnd).wrapContentSize(Alignment.TopEnd, unbounded = true).offset(x = 8.dp, y = (-8).dp)"), "{code}");
    // fixed: pinned at screen level, bottom-end
    assert!(code.contains("align(Alignment.BottomEnd).wrapContentSize(Alignment.BottomEnd, unbounded = true).offset(x = (-20).dp, y = (-24).dp)"), "{code}");
}

#[test]
fn wrap_scroll_and_shadows_do_not_clip() {
    let code = screen_code(
        "column { row { flex-wrap: wrap; gap: 4px; text \"a\"; text \"b\" }\n column { overflow: auto; text \"c\" }\n card { text \"d\" } }",
    );
    assert!(code.contains("FlowRow(Modifier, horizontalArrangement = Arrangement.spacedBy(4.dp, Alignment.Start), verticalArrangement = Arrangement.spacedBy(4.dp))"), "{code}");
    assert!(
        code.contains("verticalScroll(rememberScrollState())"),
        "{code}"
    );
    assert!(code.contains("clip = false"), "{code}");
}
