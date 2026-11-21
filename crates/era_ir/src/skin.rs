//! Typed design tokens ("Skins"): merging across files, validation, and the
//! built-in default skin used when `app` names none (DECISIONS D-020).

use std::collections::BTreeMap;

use era_diagnostics::{Diagnostic, line_col, suggest};
use era_syntax::{Color, Decl, Item, Node, Span, ValueKind};

pub const REQUIRED_COLORS: &[&str] = &[
    "background",
    "surface",
    "text",
    "muted",
    "border",
    "primary",
    "on_primary",
    "secondary",
    "success",
    "warning",
    "danger",
];
pub const REQUIRED_TEXT_STYLE: &str = "body";
pub const SYSTEM_FONT: &str = "system";
pub const DEFAULT_SKIN_NAME: &str = "Default";

const SECTIONS: &[&str] = &[
    "colors",
    "spacing",
    "radius",
    "border",
    "elevation",
    "typography",
];
const STYLE_KEYS: &[&str] = &["size", "weight", "line_height", "tracking"];
const WEIGHT_NAMES: &[(&str, u16)] = &[
    ("thin", 100),
    ("light", 300),
    ("regular", 400),
    ("medium", 500),
    ("semibold", 600),
    ("bold", 700),
    ("black", 900),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    pub name: String,
    /// True for the built-in default skin.
    pub builtin: bool,
    pub colors: BTreeMap<String, Color>,
    /// Dark-mode overrides; same keys as `colors` when present.
    pub dark_colors: Option<BTreeMap<String, Color>>,
    pub spacing: BTreeMap<String, f64>,
    pub radius: BTreeMap<String, f64>,
    pub border: BTreeMap<String, f64>,
    pub elevation: BTreeMap<String, f64>,
    pub typography: Typography,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    /// Font family: `system` or a family with files in `Assets/Fonts/`.
    pub font: String,
    /// Font files for the family, project-relative (`Assets/Fonts/...`).
    pub font_files: Vec<String>,
    pub styles: BTreeMap<String, TextStyle>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// sp
    pub size: f64,
    /// 100..=900
    pub weight: u16,
    /// sp
    pub line_height: Option<f64>,
    /// sp; letter spacing
    pub tracking: Option<f64>,
}

/// Which token table a property value is looked up in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    Color,
    Spacing,
    Radius,
    Border,
    Elevation,
    TextStyle,
}

impl TokenCategory {
    pub fn section(self) -> &'static str {
        match self {
            TokenCategory::Color => "colors",
            TokenCategory::Spacing => "spacing",
            TokenCategory::Radius => "radius",
            TokenCategory::Border => "border",
            TokenCategory::Elevation => "elevation",
            TokenCategory::TextStyle => "typography",
        }
    }
}

impl Skin {
    pub fn names(&self, cat: TokenCategory) -> Vec<&str> {
        fn keys(m: &BTreeMap<String, f64>) -> Vec<&str> {
            m.keys().map(String::as_str).collect()
        }
        match cat {
            TokenCategory::Color => self.colors.keys().map(String::as_str).collect(),
            TokenCategory::Spacing => keys(&self.spacing),
            TokenCategory::Radius => keys(&self.radius),
            TokenCategory::Border => keys(&self.border),
            TokenCategory::Elevation => keys(&self.elevation),
            TokenCategory::TextStyle => self.typography.styles.keys().map(String::as_str).collect(),
        }
    }

    /// Dimension token value in dp.
    pub fn dimension(&self, cat: TokenCategory, name: &str) -> Option<f64> {
        match cat {
            TokenCategory::Spacing => self.spacing.get(name),
            TokenCategory::Radius => self.radius.get(name),
            TokenCategory::Border => self.border.get(name),
            TokenCategory::Elevation => self.elevation.get(name),
            _ => None,
        }
        .copied()
    }
}

fn rgb(hex: u32) -> Color {
    Color {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
        a: 255,
    }
}

fn map<V: Clone>(entries: &[(&str, V)]) -> BTreeMap<String, V> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

/// Neutral built-in skin (D-020). Defines every required token so minimal
/// projects and the defaults in docs/UI_SPEC.md always resolve.
pub fn default_skin() -> Skin {
    let style = |size, weight, lh| TextStyle {
        size,
        weight,
        line_height: Some(lh),
        tracking: None,
    };
    Skin {
        name: DEFAULT_SKIN_NAME.into(),
        builtin: true,
        colors: map(&[
            ("background", rgb(0xFFFFFF)),
            ("surface", rgb(0xF5F5F5)),
            ("text", rgb(0x1A1A1A)),
            ("muted", rgb(0x6B6B6B)),
            ("border", rgb(0xE0E0E0)),
            ("primary", rgb(0x3D5AFE)),
            ("on_primary", rgb(0xFFFFFF)),
            ("secondary", rgb(0x37474F)),
            ("success", rgb(0x2E7D32)),
            ("warning", rgb(0xED6C02)),
            ("danger", rgb(0xD32F2F)),
        ]),
        dark_colors: Some(map(&[
            ("background", rgb(0x121212)),
            ("surface", rgb(0x1E1E1E)),
            ("text", rgb(0xF2F2F2)),
            ("muted", rgb(0xA0A0A0)),
            ("border", rgb(0x333333)),
            ("primary", rgb(0x8C9EFF)),
            ("on_primary", rgb(0x000000)),
            ("secondary", rgb(0xB0BEC5)),
            ("success", rgb(0x81C784)),
            ("warning", rgb(0xFFB74D)),
            ("danger", rgb(0xE57373)),
        ])),
        spacing: map(&[
            ("xs", 4.0),
            ("sm", 8.0),
            ("md", 16.0),
            ("lg", 24.0),
            ("xl", 32.0),
        ]),
        radius: map(&[("sm", 6.0), ("md", 12.0), ("lg", 20.0), ("pill", 999.0)]),
        border: map(&[("thin", 1.0), ("thick", 2.0)]),
        elevation: map(&[("none", 0.0), ("sm", 1.0), ("md", 4.0), ("lg", 12.0)]),
        typography: Typography {
            font: SYSTEM_FONT.into(),
            font_files: Vec::new(),
            styles: map(&[
                ("display", style(36.0, 700, 44.0)),
                ("heading", style(26.0, 700, 32.0)),
                ("title", style(20.0, 600, 26.0)),
                ("body", style(16.0, 400, 24.0)),
                ("caption", style(13.0, 500, 18.0)),
            ]),
        },
    }
}

/// One `skin Name { ... }` declaration and the file it came from.
pub struct SkinPart<'a> {
    pub path: &'a str,
    pub source: &'a str,
    pub decl: &'a Decl,
}

struct Loc<'a> {
    path: &'a str,
    source: &'a str,
    span: Span,
}

impl Loc<'_> {
    fn describe(&self) -> String {
        let (l, c) = line_col(self.source, self.span.start);
        format!("{}:{l}:{c}", self.path)
    }
}

struct Builder<'a> {
    diags: Vec<Diagnostic>,
    /// (category key, token) -> first definition, for duplicate detection.
    seen: BTreeMap<(String, String), Loc<'a>>,
    colors: BTreeMap<String, Color>,
    dark: Option<BTreeMap<String, Color>>,
    dark_loc: Option<Loc<'a>>,
    dark_keys: BTreeMap<String, Loc<'a>>,
    spacing: BTreeMap<String, f64>,
    radius: BTreeMap<String, f64>,
    border: BTreeMap<String, f64>,
    elevation: BTreeMap<String, f64>,
    font: Option<(String, Loc<'a>)>,
    styles: BTreeMap<String, TextStyle>,
    /// Tokens written with an invalid value (already reported), so they are
    /// not reported again as missing.
    invalid: std::collections::BTreeSet<String>,
}

/// Merge and validate every part of one named skin. `font_files` lists the
/// project's `Assets/Fonts/` files (project-relative paths).
pub fn build_skin(
    name: &str,
    parts: &[SkinPart],
    font_files: &[String],
) -> (Skin, Vec<Diagnostic>) {
    let mut b = Builder {
        diags: Vec::new(),
        seen: BTreeMap::new(),
        colors: BTreeMap::new(),
        dark: None,
        dark_loc: None,
        dark_keys: BTreeMap::new(),
        spacing: BTreeMap::new(),
        radius: BTreeMap::new(),
        border: BTreeMap::new(),
        elevation: BTreeMap::new(),
        font: None,
        styles: BTreeMap::new(),
        invalid: Default::default(),
    };
    for part in parts {
        for item in &part.decl.body.items {
            if let Item::Node(section) = item {
                b.section(part, section);
            }
        }
    }

    let first = &parts[0];
    let missing: Vec<&str> = REQUIRED_COLORS
        .iter()
        .copied()
        .filter(|k| !b.colors.contains_key(*k) && !b.invalid.contains(*k))
        .collect();
    if !missing.is_empty() {
        b.diags.push(
            Diagnostic::error(
                "E2008",
                format!(
                    "skin `{name}` is missing required colors: {}",
                    missing.join(", ")
                ),
            )
            .at(first.path, first.decl.name.span.range())
            .with_help("add them to its `colors { ... }` section"),
        );
    }
    if !b.styles.contains_key(REQUIRED_TEXT_STYLE) && !b.invalid.contains(REQUIRED_TEXT_STYLE) {
        b.diags.push(
            Diagnostic::error("E2008", format!("skin `{name}` has no `body` text style"))
                .at(first.path, first.decl.name.span.range())
                .with_help("add `typography { body { size: 16 } }`"),
        );
    }
    b.dark_parity();

    let (font, font_files) = b.font(font_files);
    let skin = Skin {
        name: name.to_string(),
        builtin: false,
        colors: b.colors,
        dark_colors: b.dark,
        spacing: b.spacing,
        radius: b.radius,
        border: b.border,
        elevation: b.elevation,
        typography: Typography {
            font,
            font_files,
            styles: b.styles,
        },
    };
    (skin, b.diags)
}

fn is_snake(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl<'a> Builder<'a> {
    fn err(
        &mut self,
        path: &str,
        code: &'static str,
        msg: String,
        span: Span,
        help: Option<String>,
    ) {
        let mut d = Diagnostic::error(code, msg).at(path, span.range());
        d.help = help;
        self.diags.push(d);
    }

    /// Record a token definition; false (and an error) if it is a duplicate.
    fn define(&mut self, part: &SkinPart<'a>, category: &str, name: &str, span: Span) -> bool {
        if !is_snake(name) {
            self.err(
                part.path,
                "E2006",
                format!("token name `{name}` must be snake_case"),
                span,
                None,
            );
        }
        let key = (category.to_string(), name.to_string());
        if let Some(first) = self.seen.get(&key) {
            let help = format!("first defined at {}", first.describe());
            self.err(
                part.path,
                "E2003",
                format!("`{name}` is already defined in `{category}`"),
                span,
                Some(help),
            );
            return false;
        }
        self.seen.insert(
            key,
            Loc {
                path: part.path,
                source: part.source,
                span,
            },
        );
        true
    }

    fn section(&mut self, part: &SkinPart<'a>, n: &Node) {
        let section = n.name.name.as_str();
        if !SECTIONS.contains(&section) {
            let help = match suggest(section, SECTIONS.iter().copied()) {
                Some(s) => format!("did you mean `{s}`?"),
                None => format!("skin sections are: {}", SECTIONS.join(", ")),
            };
            self.err(
                part.path,
                "E2001",
                format!("unknown skin section `{section}`"),
                n.name.span,
                Some(help),
            );
            return;
        }
        let dark = match &n.arg {
            None => false,
            Some(v) if section == "colors" && v.kind == ValueKind::Ident("dark".into()) => true,
            Some(v) => {
                let help = if section == "colors" {
                    "the only variant is `colors dark { ... }`"
                } else {
                    "only `colors` takes a mode, as in `colors dark`"
                };
                self.err(
                    part.path,
                    "E2002",
                    format!("unexpected value after `{section}`"),
                    v.span,
                    Some(help.into()),
                );
                return;
            }
        };
        let items = n
            .block
            .as_ref()
            .map(|b| b.items.as_slice())
            .unwrap_or_default();
        match section {
            "colors" => self.colors(part, n, items, dark),
            "typography" => self.typography(part, items),
            _ => self.numbers(part, section, items),
        }
    }

    fn colors(&mut self, part: &SkinPart<'a>, n: &Node, items: &[Item], dark: bool) {
        let category = if dark { "colors dark" } else { "colors" };
        if dark {
            if self.dark_loc.is_none() {
                self.dark_loc = Some(Loc {
                    path: part.path,
                    source: part.source,
                    span: n.name.span,
                });
            }
            self.dark.get_or_insert_with(BTreeMap::new);
        }
        for item in items {
            let Item::Property(p) = item else {
                self.err(
                    part.path,
                    "E2005",
                    format!("unexpected {} in `{category}`", item.describe()),
                    item.span(),
                    Some("write color tokens as `name: #RRGGBB`".into()),
                );
                continue;
            };
            let ValueKind::Color(c) = p.value.kind else {
                self.invalid.insert(p.name.name.clone());
                self.err(
                    part.path,
                    "E2004",
                    format!("color token `{}` needs a color like `#FF6B35`", p.name.name),
                    p.value.span,
                    None,
                );
                continue;
            };
            if !self.define(part, category, &p.name.name, p.name.span) {
                continue;
            }
            if dark {
                self.dark.as_mut().unwrap().insert(p.name.name.clone(), c);
                self.dark_keys.insert(
                    p.name.name.clone(),
                    Loc {
                        path: part.path,
                        source: part.source,
                        span: p.name.span,
                    },
                );
            } else {
                self.colors.insert(p.name.name.clone(), c);
            }
        }
    }

    fn numbers(&mut self, part: &SkinPart<'a>, section: &str, items: &[Item]) {
        for item in items {
            let Item::Property(p) = item else {
                self.err(
                    part.path,
                    "E2005",
                    format!("unexpected {} in `{section}`", item.describe()),
                    item.span(),
                    Some(format!("write {section} tokens as `name: 16`")),
                );
                continue;
            };
            let value = match p.value.kind {
                ValueKind::Number(v) if v >= 0.0 => v,
                _ => {
                    self.err(
                        part.path,
                        "E2004",
                        format!(
                            "{section} token `{}` needs a number of 0 or more",
                            p.name.name
                        ),
                        p.value.span,
                        None,
                    );
                    continue;
                }
            };
            if !self.define(part, section, &p.name.name, p.name.span) {
                continue;
            }
            let table = match section {
                "spacing" => &mut self.spacing,
                "radius" => &mut self.radius,
                "border" => &mut self.border,
                _ => &mut self.elevation,
            };
            table.insert(p.name.name.clone(), value);
        }
    }

    fn typography(&mut self, part: &SkinPart<'a>, items: &[Item]) {
        for item in items {
            match item {
                Item::Property(p) if p.name.name == "font" => {
                    let ValueKind::String(f) = &p.value.kind else {
                        self.err(part.path, "E2004", "`font` needs a family name in quotes, like `\"Inter\"`".into(),
                            p.value.span, None);
                        continue;
                    };
                    if let Some((_, first)) = &self.font {
                        let help = format!("first defined at {}", first.describe());
                        self.err(part.path, "E2003", "`font` is already defined".into(), p.name.span, Some(help));
                        continue;
                    }
                    self.font = Some((f.clone(), Loc { path: part.path, source: part.source, span: p.value.span }));
                }
                Item::Node(style) if style.arg.is_none() => self.text_style(part, style),
                _ => self.err(part.path, "E2005", format!("unexpected {} in `typography`", item.describe()), item.span(),
                    Some("`typography` holds `font: \"...\"` and text styles like `body { size: 16 }`".into())),
            }
        }
    }

    fn text_style(&mut self, part: &SkinPart<'a>, n: &Node) {
        let name = n.name.name.clone();
        let mut size = None;
        let mut weight = 400;
        let mut line_height = None;
        let mut tracking = None;
        for item in n
            .block
            .as_ref()
            .map(|b| b.items.as_slice())
            .unwrap_or_default()
        {
            let Item::Property(p) = item else {
                self.err(
                    part.path,
                    "E2005",
                    format!("unexpected {} in text style `{name}`", item.describe()),
                    item.span(),
                    None,
                );
                continue;
            };
            let key = p.name.name.as_str();
            let num = match p.value.kind {
                ValueKind::Number(v) => Some(v),
                _ => None,
            };
            match key {
                "size" | "line_height" => match num {
                    Some(v) if v > 0.0 => {
                        if key == "size" {
                            size = Some(v)
                        } else {
                            line_height = Some(v)
                        }
                    }
                    _ => self.err(
                        part.path,
                        "E2004",
                        format!("`{key}` needs a number greater than 0"),
                        p.value.span,
                        None,
                    ),
                },
                "tracking" => match num {
                    Some(v) => tracking = Some(v),
                    None => self.err(
                        part.path,
                        "E2004",
                        "`tracking` needs a number".into(),
                        p.value.span,
                        None,
                    ),
                },
                "weight" => {
                    let w = match &p.value.kind {
                        ValueKind::Ident(w) => {
                            WEIGHT_NAMES.iter().find(|(n, _)| n == w).map(|(_, v)| *v)
                        }
                        ValueKind::Number(v)
                            if v.fract() == 0.0
                                && (100.0..=900.0).contains(v)
                                && (*v as u16).is_multiple_of(100) =>
                        {
                            Some(*v as u16)
                        }
                        _ => None,
                    };
                    match w {
                        Some(w) => weight = w,
                        None => {
                            let names: Vec<_> = WEIGHT_NAMES.iter().map(|(n, _)| *n).collect();
                            self.err(
                                part.path,
                                "E2004",
                                "invalid font weight".into(),
                                p.value.span,
                                Some(format!(
                                    "use {} or 100-900 in steps of 100",
                                    names.join(", ")
                                )),
                            );
                        }
                    }
                }
                _ => {
                    let help = suggest(key, STYLE_KEYS.iter().copied())
                        .map(|s| format!("did you mean `{s}`?"))
                        .unwrap_or_else(|| {
                            format!("text style keys are: {}", STYLE_KEYS.join(", "))
                        });
                    self.err(
                        part.path,
                        "E2005",
                        format!("unknown text style key `{key}`"),
                        p.name.span,
                        Some(help),
                    );
                }
            }
        }
        let Some(size) = size else {
            self.invalid.insert(name.clone());
            self.err(
                part.path,
                "E2007",
                format!("text style `{name}` needs a `size`"),
                n.name.span,
                None,
            );
            return;
        };
        if self.define(part, "typography", &name, n.name.span) {
            self.styles.insert(
                name,
                TextStyle {
                    size,
                    weight,
                    line_height,
                    tracking,
                },
            );
        }
    }

    fn dark_parity(&mut self) {
        let Some(dark) = &self.dark else { return };
        let missing: Vec<String> = self
            .colors
            .keys()
            .filter(|k| !dark.contains_key(*k))
            .cloned()
            .collect();
        if !missing.is_empty() {
            let loc = self.dark_loc.as_ref().unwrap();
            let (path, span) = (loc.path.to_string(), loc.span);
            self.err(
                &path,
                "E2009",
                format!("`colors dark` is missing: {}", missing.join(", ")),
                span,
                Some("dark mode must define every color that light mode defines".into()),
            );
        }
        let extra: Vec<(String, String, Span)> = self
            .dark_keys
            .iter()
            .filter(|(k, _)| !self.colors.contains_key(*k))
            .map(|(k, l)| (k.clone(), l.path.to_string(), l.span))
            .collect();
        for (k, path, span) in extra {
            self.err(
                &path,
                "E2009",
                format!("dark color `{k}` has no light-mode value"),
                span,
                Some("add it to `colors { ... }` as well".into()),
            );
        }
    }

    fn font(&mut self, font_files: &[String]) -> (String, Vec<String>) {
        let Some((family, loc)) = self.font.take() else {
            return (SYSTEM_FONT.into(), Vec::new());
        };
        if family == SYSTEM_FONT {
            return (family, Vec::new());
        }
        let files: Vec<String> = font_files
            .iter()
            .filter(|f| {
                std::path::Path::new(f)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|stem| stem == family || stem.starts_with(&format!("{family}-")))
            })
            .cloned()
            .collect();
        if files.is_empty() {
            let (path, span) = (loc.path.to_string(), loc.span);
            self.err(
                &path,
                "E2010",
                format!("no font files found for `{family}`"),
                span,
                Some(format!(
                    "add `Assets/Fonts/{family}-Regular.ttf` (or `.otf`), or use `\"system\"`"
                )),
            );
        }
        (family, files)
    }
}

impl Skin {
    /// Fill required tokens that are missing (already reported as errors)
    /// from the default skin, so later analysis can continue safely.
    pub fn fill_required_from_default(&mut self) {
        let d = default_skin();
        for k in REQUIRED_COLORS {
            if !self.colors.contains_key(*k) {
                self.colors.insert(k.to_string(), d.colors[*k]);
            }
        }
        if !self.typography.styles.contains_key(REQUIRED_TEXT_STYLE) {
            self.typography.styles.insert(
                REQUIRED_TEXT_STYLE.into(),
                d.typography.styles[REQUIRED_TEXT_STYLE].clone(),
            );
        }
    }
}
