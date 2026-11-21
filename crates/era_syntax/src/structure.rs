//! Structural checks over parsed files (docs/GRAMMAR.md §4 and §5):
//! which forms may appear where, one root element, slots, variants, naming
//! conventions, the single `app`, and duplicate declarations.
//!
//! Token, property and asset validation is *not* done here (IR stage, M3).

use std::collections::BTreeMap;

use era_diagnostics::{Diagnostic, line_col};

use crate::ast::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// `.era` file under `src/`.
    Ui,
    /// `.ers` file under `Skins/`.
    Skin,
}

pub struct SourceUnit<'a> {
    pub path: &'a str,
    pub source: &'a str,
    pub kind: SourceKind,
    /// True for `src/app.era`.
    pub is_entry: bool,
    pub file: &'a File,
}

/// Run every structural check. `entry_path` is used for the "no app" error.
pub fn check_structure(units: &[SourceUnit], entry_path: &str) -> Vec<Diagnostic> {
    let mut c = Checker { diags: Vec::new() };
    for unit in units {
        for decl in &unit.file.decls {
            c.decl(unit, decl);
        }
    }
    c.project(units, entry_path);
    c.diags
}

struct Checker {
    diags: Vec<Diagnostic>,
}

fn is_pascal(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_snake(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn to_pascal(name: &str) -> String {
    name.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            let first = cs.next().unwrap().to_ascii_uppercase();
            std::iter::once(first).chain(cs).collect::<String>()
        })
        .collect()
}

fn to_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else if c == '-' {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    out
}

fn location(unit: &SourceUnit, span: Span) -> String {
    let (line, col) = line_col(unit.source, span.start);
    format!("{}:{line}:{col}", unit.path)
}

/// Context for items nested inside a declaration's root element.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Nested {
    Screen,
    Layout,
    Component,
    SkinSection,
}

impl Checker {
    fn err(
        &mut self,
        unit: &SourceUnit,
        code: &'static str,
        msg: String,
        span: Span,
        help: Option<String>,
    ) {
        let mut d = Diagnostic::error(code, msg).at(unit.path, span.range());
        d.help = help;
        self.diags.push(d);
    }

    fn not_allowed(&mut self, unit: &SourceUnit, item: &Item, place: &str, help: Option<String>) {
        self.err(
            unit,
            "E1201",
            format!("{} is not allowed {place}", item.describe()),
            item.span(),
            help,
        );
    }

    fn decl(&mut self, unit: &SourceUnit, decl: &Decl) {
        let kw = decl.kind.keyword();
        match (unit.kind, decl.kind) {
            (SourceKind::Ui, DeclKind::Skin) => {
                self.err(
                    unit,
                    "E1216",
                    "`skin` declarations belong in `Skins/*.ers` files".into(),
                    decl.name.span,
                    Some("move this skin into a `.ers` file in `Skins/`".into()),
                );
                return;
            }
            (SourceKind::Skin, k) if k != DeclKind::Skin => {
                self.err(
                    unit,
                    "E1216",
                    format!("`{kw}` declarations belong in `src/*.era` files"),
                    decl.name.span,
                    Some("`.ers` files may only contain `skin` declarations".into()),
                );
                return;
            }
            _ => {}
        }

        if decl.kind != DeclKind::App && !is_pascal(&decl.name.name) {
            let suggestion = to_pascal(&decl.name.name);
            self.err(
                unit,
                "E1217",
                format!("{kw} name `{}` must be PascalCase", decl.name.name),
                decl.name.span,
                (!suggestion.is_empty() && is_pascal(&suggestion))
                    .then(|| format!("rename it to `{suggestion}`")),
            );
        }

        match decl.kind {
            DeclKind::App => self.app(unit, decl),
            DeclKind::Skin => self.skin(unit, decl),
            DeclKind::Screen => self.screen(unit, decl),
            DeclKind::Layout | DeclKind::Component => self.reusable(unit, decl),
        }
    }

    fn app(&mut self, unit: &SourceUnit, decl: &Decl) {
        for item in &decl.body.items {
            if !matches!(item, Item::Property(_)) {
                self.not_allowed(unit, item, "in `app`", Some(
                    "`app` only sets properties such as `skin` and `start`; UI goes in a `screen`".into(),
                ));
            }
        }
    }

    fn skin(&mut self, unit: &SourceUnit, decl: &Decl) {
        for item in &decl.body.items {
            match item {
                Item::Node(n) => {
                    self.nested_block(unit, n.block.as_ref(), Nested::SkinSection, false)
                }
                Item::Property(_) => self.not_allowed(
                    unit,
                    item,
                    "directly in `skin`",
                    Some("put tokens inside a section such as `colors { ... }`".into()),
                ),
                _ => self.not_allowed(unit, item, "in `skin`", None),
            }
        }
    }

    fn screen(&mut self, unit: &SourceUnit, decl: &Decl) {
        let mut roots = Vec::new();
        for item in &decl.body.items {
            match item {
                Item::Property(_) => {}
                Item::Node(n) => roots.push(n),
                _ => self.not_allowed(unit, item, "in `screen`", None),
            }
        }
        self.single_root(unit, decl, &roots);
        for n in roots {
            self.nested_block(unit, n.block.as_ref(), Nested::Screen, false);
        }
    }

    fn single_root(&mut self, unit: &SourceUnit, decl: &Decl, roots: &[&Node]) {
        let kw = decl.kind.keyword();
        match roots {
            [] => self.err(
                unit,
                "E1206",
                format!("{kw} `{}` has no content", decl.name.name),
                decl.name.span,
                Some("add one root element, such as `column { ... }`".into()),
            ),
            [_] => {}
            [_, second, ..] => self.err(
                unit,
                "E1207",
                format!(
                    "{kw} `{}` must have exactly one root element",
                    decl.name.name
                ),
                second.name.span,
                Some("wrap the elements in a `column`, `row` or `stack`".into()),
            ),
        }
    }

    /// Layouts and components share most rules.
    fn reusable(&mut self, unit: &SourceUnit, decl: &Decl) {
        let is_layout = decl.kind == DeclKind::Layout;
        let place = if is_layout {
            "in `layout`"
        } else {
            "in `component`"
        };
        let mut roots = Vec::new();
        let mut names: BTreeMap<&str, Span> = BTreeMap::new();
        let mut variants: BTreeMap<&str, &VariantDecl> = BTreeMap::new();

        for item in &decl.body.items {
            match item {
                Item::Node(n) => roots.push(n),
                Item::Prop(p) => {
                    self.member_name(unit, &p.name, &mut names);
                }
                Item::Variant(v) if !is_layout => {
                    self.member_name(unit, &v.name, &mut names);
                    self.variant(unit, v);
                    variants.insert(v.name.name.as_str(), v);
                }
                Item::Slot(_) => self.not_allowed(
                    unit,
                    item,
                    &format!("at the top of a {}", decl.kind.keyword()),
                    Some("place `slot` inside the root element".into()),
                ),
                Item::Property(_) => self.not_allowed(
                    unit,
                    item,
                    place,
                    Some("style properties go on the root element".into()),
                ),
                Item::When(_) => self.not_allowed(
                    unit,
                    item,
                    &format!("at the top of a {}", decl.kind.keyword()),
                    Some("place `when` inside the element it changes".into()),
                ),
                Item::Variant(_) => self.not_allowed(
                    unit,
                    item,
                    place,
                    Some("variants are only available in components".into()),
                ),
            }
        }
        self.single_root(unit, decl, &roots);

        let ctx = if is_layout {
            Nested::Layout
        } else {
            Nested::Component
        };
        let mut slots = Vec::new();
        for n in &roots {
            self.collect_slots(n.block.as_ref(), &mut slots);
            self.whens(unit, n.block.as_ref(), &variants);
            self.nested_block(unit, n.block.as_ref(), ctx, false);
        }
        if is_layout && slots.is_empty() && !roots.is_empty() {
            self.err(
                unit,
                "E1209",
                format!("layout `{}` has no `slot`", decl.name.name),
                decl.name.span,
                Some("add `slot` where the screen's content should appear".into()),
            );
        }
        if let Some(extra) = slots.get(1) {
            self.err(
                unit,
                "E1210",
                format!(
                    "{} `{}` has more than one `slot`",
                    decl.kind.keyword(),
                    decl.name.name
                ),
                *extra,
                Some("a layout or component can have only one `slot`".into()),
            );
        }
    }

    fn member_name<'a>(
        &mut self,
        unit: &SourceUnit,
        name: &'a Ident,
        seen: &mut BTreeMap<&'a str, Span>,
    ) {
        if !is_snake(&name.name) {
            self.err(
                unit,
                "E1218",
                format!("name `{}` must be snake_case", name.name),
                name.span,
                Some(format!("rename it to `{}`", to_snake(&name.name))),
            );
        }
        if let Some(first) = seen.insert(name.name.as_str(), name.span) {
            self.err(
                unit,
                "E1213",
                format!("`{}` is declared more than once", name.name),
                name.span,
                Some(format!("first declared at {}", location(unit, first))),
            );
        }
    }

    fn variant(&mut self, unit: &SourceUnit, v: &VariantDecl) {
        let mut seen = BTreeMap::new();
        for opt in &v.options {
            self.member_name(unit, opt, &mut seen);
        }
        if let Some(d) = &v.default
            && !v.options.iter().any(|o| o.name == d.name)
        {
            self.err(
                unit,
                "E1214",
                format!(
                    "default `{}` is not an option of variant `{}`",
                    d.name, v.name.name
                ),
                d.span,
                Some(format!("options are: {}", option_list(v))),
            );
        }
    }

    fn collect_slots(&self, block: Option<&Block>, out: &mut Vec<Span>) {
        for item in block.map(|b| b.items.as_slice()).unwrap_or_default() {
            match item {
                Item::Slot(s) => out.push(*s),
                Item::Node(n) => self.collect_slots(n.block.as_ref(), out),
                _ => {}
            }
        }
    }

    fn whens(
        &mut self,
        unit: &SourceUnit,
        block: Option<&Block>,
        variants: &BTreeMap<&str, &VariantDecl>,
    ) {
        for item in block.map(|b| b.items.as_slice()).unwrap_or_default() {
            match item {
                Item::When(w) => match variants.get(w.variant.name.as_str()) {
                    None => {
                        let help = if variants.is_empty() {
                            "declare it first, e.g. `variant size: small | large`".to_string()
                        } else {
                            format!(
                                "declared variants: {}",
                                variants.keys().copied().collect::<Vec<_>>().join(", ")
                            )
                        };
                        self.err(
                            unit,
                            "E1211",
                            format!("unknown variant `{}`", w.variant.name),
                            w.variant.span,
                            Some(help),
                        );
                    }
                    Some(v) if !v.options.iter().any(|o| o.name == w.option.name) => {
                        self.err(
                            unit,
                            "E1212",
                            format!(
                                "`{}` is not an option of variant `{}`",
                                w.option.name, v.name.name
                            ),
                            w.option.span,
                            Some(format!("options are: {}", option_list(v))),
                        );
                    }
                    Some(_) => {}
                },
                Item::Node(n) => self.whens(unit, n.block.as_ref(), variants),
                _ => {}
            }
        }
    }

    /// Items inside an element: properties and child elements everywhere;
    /// `slot` in layouts/components; `when` in components (not nested).
    fn nested_block(
        &mut self,
        unit: &SourceUnit,
        block: Option<&Block>,
        ctx: Nested,
        in_when: bool,
    ) {
        for item in block.map(|b| b.items.as_slice()).unwrap_or_default() {
            match item {
                Item::Property(_) => {}
                Item::Node(n) if in_when => self.not_allowed(
                    unit,
                    item,
                    "in a `when` block",
                    Some(format!(
                        "`when` can only change properties, not add elements like `{}`",
                        n.name.name
                    )),
                ),
                Item::Node(n) => self.nested_block(unit, n.block.as_ref(), ctx, false),
                Item::Slot(_) if in_when => self.not_allowed(unit, item, "in a `when` block", None),
                Item::Slot(_) if matches!(ctx, Nested::Layout | Nested::Component) => {}
                Item::Slot(_) => self.not_allowed(
                    unit,
                    item,
                    "here",
                    Some("`slot` is only used inside a layout or component".into()),
                ),
                Item::When(w) if ctx == Nested::Component && !in_when => {
                    self.nested_block(unit, Some(&w.block), ctx, true)
                }
                Item::When(_) => self.not_allowed(
                    unit,
                    item,
                    "here",
                    Some(
                        "`when` is only used inside a component's elements, to style variants"
                            .into(),
                    ),
                ),
                Item::Prop(_) | Item::Variant(_) => self.not_allowed(
                    unit,
                    item,
                    "inside an element",
                    Some("declare props and variants at the top of the component".into()),
                ),
            }
        }
    }

    fn project(&mut self, units: &[SourceUnit], entry_path: &str) {
        let mut app: Option<(&SourceUnit, &Decl)> = None;
        let mut names: BTreeMap<&str, (&SourceUnit, &Decl)> = BTreeMap::new();

        for unit in units {
            for decl in &unit.file.decls {
                match decl.kind {
                    DeclKind::App => {
                        if !unit.is_entry {
                            self.err(
                                unit,
                                "E1204",
                                "`app` must be declared in `src/app.era`".into(),
                                decl.name.span,
                                None,
                            );
                        }
                        match app {
                            None => app = Some((unit, decl)),
                            Some((first_unit, first)) => self.err(
                                unit,
                                "E1203",
                                "a project has exactly one `app`".into(),
                                decl.name.span,
                                Some(format!(
                                    "first declared at {}",
                                    location(first_unit, first.name.span)
                                )),
                            ),
                        }
                    }
                    DeclKind::Skin => {} // skins with the same name merge (checked in M3)
                    DeclKind::Screen | DeclKind::Layout | DeclKind::Component => {
                        if let Some((first_unit, first)) = names.get(decl.name.name.as_str()) {
                            self.err(
                                unit,
                                "E1215",
                                format!(
                                    "`{}` is already declared as a {}",
                                    decl.name.name,
                                    first.kind.keyword()
                                ),
                                decl.name.span,
                                Some(format!(
                                    "first declared at {}",
                                    location(first_unit, first.name.span)
                                )),
                            );
                        } else {
                            names.insert(decl.name.name.as_str(), (unit, decl));
                        }
                    }
                }
            }
        }

        if app.is_none() && units.iter().any(|u| u.is_entry) {
            self.diags.push(
                Diagnostic::error("E1202", "no `app` declaration found")
                    .in_file(entry_path)
                    .with_help("add `app \"My App\" { start: Home }` to src/app.era"),
            );
        }
    }
}

fn option_list(v: &VariantDecl) -> String {
    v.options
        .iter()
        .map(|o| o.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
