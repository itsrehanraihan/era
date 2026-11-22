//! Semantic analysis: parsed files -> typed UI IR.
//!
//! Components and layouts are expanded inline (DECISIONS D-029); each
//! expanded element records its `origin`. Every component and layout is also
//! checked once on its own ("validation mode", with placeholder prop values
//! and every `when` block checked), so mistakes in unused or rarely-used
//! components are still reported. Duplicate diagnostics are removed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use era_diagnostics::{Diagnostic, suggest};
use era_project::assets::{AssetError, resolve_asset};
use era_project::{FileKind, Project};
use era_syntax::{
    Decl, DeclKind, File, Item, Node, PropType, SourceKind, Span, Value as AstValue, ValueKind,
};

use crate::ir::*;
use crate::logic::{self, Stores};
use crate::schema::{self, ArgType, Children, Primitive, ValueType};
use crate::skin::{self, Skin, SkinPart, TokenCategory};

/// A parsed source file, as produced by `era_syntax::parse`.
pub struct ParsedFile<'a> {
    pub path: &'a str,
    pub source: &'a str,
    pub kind: SourceKind,
    pub file: &'a File,
}

#[derive(Clone, Copy)]
struct DeclRef<'a> {
    path: &'a str,
    decl: &'a Decl,
}

/// A value bound to a component/layout prop.
#[derive(Debug, Clone)]
enum Bound {
    Text(String),
    Number(f64),
    Bool(bool),
    /// `Value::Color` or `Value::Transparent`.
    Color(Value),
    Asset(AssetRef),
    /// Text with `{Store.field}` interpolation (M6).
    Template(Vec<TextPart>),
    /// What an `action` prop does (M6).
    Effects(Vec<Effect>),
    /// Optional prop with no value and no default.
    Absent,
    /// Validation mode: a value of the declared type, not known yet.
    Placeholder,
}

struct Binding {
    ty: PropType,
    value: Bound,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Screen,
    Layout,
    Component,
}

struct Ctx<'c> {
    path: &'c str,
    scope: Scope,
    props: BTreeMap<String, Binding>,
    /// Selected option per variant; `None` in validation mode.
    variants: BTreeMap<String, Option<String>>,
    slot: Vec<Element>,
    origin: Vec<String>,
    validate_only: bool,
}

struct Resolver<'a> {
    root: &'a Path,
    skin: Skin,
    decls: BTreeMap<&'a str, DeclRef<'a>>,
    diags: Vec<Diagnostic>,
    seen: BTreeSet<(String, usize, usize, &'static str, String)>,
    assets: BTreeSet<AssetRef>,
    /// Component expansion stack, for cycle detection.
    stack: Vec<String>,
    /// App state and actions (M6).
    stores: Stores,
    /// Screen names, for `go Screen`.
    screens: Vec<String>,
    /// `icon` and `icon-background` from `app`, resolved once the skin is known.
    app_icon: (Option<AstValue>, Option<AstValue>),
}

/// Build the UI IR. Returns `None` for the app if any error was reported.
pub fn build_app(project: &Project, files: &[ParsedFile]) -> (Option<App>, Vec<Diagnostic>) {
    let mut app_decl: Option<DeclRef> = None;
    let mut skins: BTreeMap<&str, Vec<SkinPart>> = BTreeMap::new();
    let mut decls: BTreeMap<&str, DeclRef> = BTreeMap::new();
    let mut screen_order: Vec<&str> = Vec::new();
    let mut store_decls: Vec<(&str, &era_syntax::Decl)> = Vec::new();

    for f in files {
        for decl in &f.file.decls {
            let r = DeclRef { path: f.path, decl };
            match decl.kind {
                DeclKind::App => {
                    app_decl.get_or_insert(r);
                }
                DeclKind::Skin => skins.entry(&decl.name.name).or_default().push(SkinPart {
                    path: f.path,
                    source: f.source,
                    decl,
                }),
                DeclKind::Store => store_decls.push((f.path, decl)),
                _ => {
                    if decls.insert(&decl.name.name, r).is_none() && decl.kind == DeclKind::Screen {
                        screen_order.push(&decl.name.name);
                    }
                }
            }
        }
    }

    let font_files: Vec<String> = project
        .files
        .iter()
        .filter(|f| {
            f.kind
                == (FileKind::Asset {
                    category: "Fonts".into(),
                })
        })
        .map(|f| f.rel_path.clone())
        .collect();

    let mut diags = Vec::new();
    let mut built: BTreeMap<&str, Skin> = BTreeMap::new();
    for (name, parts) in &skins {
        let (s, d) = skin::build_skin(name, parts, &font_files);
        diags.extend(d);
        built.insert(name, s);
    }

    let mut r = Resolver {
        root: &project.root,
        skin: skin::default_skin(),
        decls,
        diags: Vec::new(),
        seen: BTreeSet::new(),
        assets: BTreeSet::new(),
        stack: Vec::new(),
        stores: Stores::default(),
        screens: screen_order.iter().map(|s| s.to_string()).collect(),
        app_icon: (None, None),
    };
    let (stores, store_table, store_diags) = logic::build_stores(&store_decls);
    diags.extend(store_diags);
    r.stores = store_table;
    for d in diags {
        r.push(d);
    }

    let Some(app_ref) = app_decl else {
        // E1202 is reported by the structure check.
        return (None, r.diags);
    };
    let (skin_name, start) = r.app_settings(app_ref, &built, &screen_order);
    if let Some(name) = skin_name
        && let Some(s) = built.get(name.as_str())
    {
        r.skin = s.clone();
        r.skin.fill_required_from_default();
    }
    let icon = r.app_icon(app_ref.path);

    // Check every component and layout once, on its own.
    let reusable: Vec<DeclRef> = r
        .decls
        .values()
        .filter(|d| matches!(d.decl.kind, DeclKind::Component | DeclKind::Layout))
        .copied()
        .collect();
    for d in reusable {
        r.validate_reusable(d);
    }

    let screens: Vec<Screen> = screen_order
        .iter()
        .filter_map(|name| {
            let d = r.decls[name];
            r.screen(d)
        })
        .collect();

    let mut assets: BTreeSet<AssetRef> = std::mem::take(&mut r.assets);
    for f in &r.skin.typography.font_files {
        assets.insert(AssetRef {
            path: f.trim_start_matches("Assets/").to_string(),
            kind: AssetKind::Font,
        });
    }

    let has_errors = r.diags.iter().any(Diagnostic::is_error);
    let app = match (has_errors, start) {
        (false, Some(start)) => Some(App {
            name: app_ref.decl.name.name.clone(),
            id: project.manifest.id.clone(),
            version: project.manifest.version.clone(),
            skin: r.skin.clone(),
            start,
            screens,
            assets: assets.into_iter().collect(),
            stores,
            icon,
        }),
        _ => None,
    };
    (app, r.diags)
}

fn hint(name: &str, candidates: &[&str], fallback: &str) -> String {
    match suggest(name, candidates.iter().copied()) {
        Some(s) => format!("did you mean `{s}`?"),
        None if candidates.is_empty() => fallback.to_string(),
        None => format!("{fallback}: {}", candidates.join(", ")),
    }
}

fn is_pascal(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}

fn prop_type_name(t: PropType) -> &'static str {
    PropType::ALL.iter().find(|(_, p)| *p == t).unwrap().0
}

impl<'a> Resolver<'a> {
    fn push(&mut self, d: Diagnostic) {
        let (path, span) = match &d.location {
            Some(l) => (l.path.clone(), l.span.clone().unwrap_or(0..0)),
            None => (String::new(), 0..0),
        };
        if self
            .seen
            .insert((path, span.start, span.end, d.code, d.message.clone()))
        {
            self.diags.push(d);
        }
    }

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
        self.push(d);
    }

    // ----- app ---------------------------------------------------------

    fn app_settings(
        &mut self,
        app: DeclRef,
        skins: &BTreeMap<&str, Skin>,
        screens: &[&str],
    ) -> (Option<String>, Option<String>) {
        let mut skin_name = None;
        let mut start = None;
        for item in &app.decl.body.items {
            let Item::Property(p) = item else { continue };
            match schema::normalize(&p.name.name).as_str() {
                "icon" => {
                    self.app_icon.0 = Some(p.value.clone());
                    continue;
                }
                "iconbackground" | "iconbackgroundcolor" => {
                    self.app_icon.1 = Some(p.value.clone());
                    continue;
                }
                _ => {}
            }
            match (p.name.name.as_str(), &p.value.kind) {
                ("skin", ValueKind::Ident(n)) => {
                    if skins.contains_key(n.as_str()) {
                        skin_name = Some(n.clone());
                    } else {
                        let names: Vec<&str> = skins.keys().copied().collect();
                        self.err(
                            app.path,
                            "E2101",
                            format!("unknown skin `{n}`"),
                            p.value.span,
                            Some(hint(n, &names, "skins defined in `Skins/`")),
                        );
                    }
                }
                ("start", ValueKind::Ident(n)) => {
                    if screens.contains(&n.as_str()) {
                        start = Some(n.clone());
                    } else {
                        self.err(
                            app.path,
                            "E2106",
                            format!("unknown screen `{n}`"),
                            p.value.span,
                            Some(hint(n, screens, "screens")),
                        );
                    }
                }
                ("skin" | "start", _) => self.err(
                    app.path,
                    "E2103",
                    format!(
                        "`{}` needs a name, like `{}: Home`",
                        p.name.name, p.name.name
                    ),
                    p.value.span,
                    None,
                ),
                (other, _) => self.err(
                    app.path,
                    "E2102",
                    format!("unknown app property `{other}`"),
                    p.name.span,
                    Some(hint(
                        other,
                        &["skin", "start", "icon", "icon-background"],
                        "app properties",
                    )),
                ),
            }
        }
        let has_start = app
            .decl
            .body
            .items
            .iter()
            .any(|i| matches!(i, Item::Property(p) if p.name.name == "start"));
        if !has_start {
            match screens {
                [] => self.err(
                    app.path,
                    "E2104",
                    "the project has no screens".into(),
                    app.decl.name.span,
                    Some("add `screen Home { column { ... } }`".into()),
                ),
                [only] => start = Some(only.to_string()),
                _ => self.err(
                    app.path,
                    "E2105",
                    "`app` needs `start` when there are several screens".into(),
                    app.decl.name.span,
                    Some(format!("add `start: {}`", screens[0])),
                ),
            }
        }
        (skin_name, start)
    }

    // ----- declarations ------------------------------------------------

    fn screen(&mut self, d: DeclRef<'a>) -> Option<Screen> {
        let mut title = None;
        let mut root = None;
        for item in &d.decl.body.items {
            match item {
                Item::Property(p) if p.name.name == "title" => match &p.value.kind {
                    ValueKind::String(s) => title = Some(s.clone()),
                    _ => self.err(
                        d.path,
                        "E2133",
                        "`title` needs text in quotes".into(),
                        p.value.span,
                        None,
                    ),
                },
                Item::Property(p) => self.err(
                    d.path,
                    "E2120",
                    format!("unknown screen property `{}`", p.name.name),
                    p.name.span,
                    Some(hint(&p.name.name, &["title"], "screen properties")),
                ),
                Item::Node(n) => root = Some(n),
                _ => {}
            }
        }
        let mut ctx = Ctx {
            path: d.path,
            scope: Scope::Screen,
            props: BTreeMap::new(),
            variants: BTreeMap::new(),
            slot: Vec::new(),
            origin: Vec::new(),
            validate_only: false,
        };
        let mut root = self.element(root?, &mut ctx, d.decl.name.name.clone(), true)?;
        // A screen's root element fills the screen unless sized (D-043).
        root.props.entry("width").or_insert(Value::Size(Size::Fill));
        root.props
            .entry("height")
            .or_insert(Value::Size(Size::Fill));
        let background = self.color_token("background");
        Some(Screen {
            name: d.decl.name.name.clone(),
            title,
            background,
            root,
        })
    }

    fn validate_reusable(&mut self, d: DeclRef<'a>) {
        let Some(root) = d.decl.body.items.iter().find_map(|i| match i {
            Item::Node(n) => Some(n),
            _ => None,
        }) else {
            return;
        };
        let (props, variants) = self.declared_members(d, true);
        let mut ctx = Ctx {
            path: d.path,
            scope: if d.decl.kind == DeclKind::Layout {
                Scope::Layout
            } else {
                Scope::Component
            },
            props,
            variants: variants.into_keys().map(|k| (k, None)).collect(),
            slot: Vec::new(),
            origin: vec![d.decl.name.name.clone()],
            validate_only: true,
        };
        self.stack.push(d.decl.name.name.clone());
        self.element(root, &mut ctx, d.decl.name.name.clone(), false);
        self.stack.pop();
    }

    /// Props with their defaults (or placeholders), and variants with
    /// (options, default option).
    #[allow(clippy::type_complexity)]
    fn declared_members(
        &mut self,
        d: DeclRef<'a>,
        placeholders: bool,
    ) -> (
        BTreeMap<String, Binding>,
        BTreeMap<String, (Vec<String>, String)>,
    ) {
        let mut props = BTreeMap::new();
        let mut variants = BTreeMap::new();
        for item in &d.decl.body.items {
            match item {
                Item::Prop(p) => {
                    let value = if placeholders {
                        Bound::Placeholder
                    } else if let Some(default) = &p.default {
                        let mut own = self.empty_ctx(d.path, true);
                        self.prop_value(p.ty, default, &mut own)
                            .unwrap_or(Bound::Placeholder)
                    } else {
                        Bound::Absent
                    };
                    if placeholders && let Some(default) = &p.default {
                        let mut own = self.empty_ctx(d.path, true);
                        self.prop_value(p.ty, default, &mut own);
                    }
                    props.insert(p.name.name.clone(), Binding { ty: p.ty, value });
                }
                Item::Variant(v) => {
                    let options: Vec<String> = v.options.iter().map(|o| o.name.clone()).collect();
                    let default = v
                        .default
                        .as_ref()
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| options[0].clone());
                    variants.insert(v.name.name.clone(), (options, default));
                }
                _ => {}
            }
        }
        (props, variants)
    }

    fn empty_ctx(&self, path: &'a str, validate_only: bool) -> Ctx<'a> {
        Ctx {
            path,
            scope: Scope::Component,
            props: BTreeMap::new(),
            variants: BTreeMap::new(),
            slot: Vec::new(),
            origin: Vec::new(),
            validate_only,
        }
    }

    // ----- elements ----------------------------------------------------

    fn element(
        &mut self,
        node: &Node,
        ctx: &mut Ctx,
        id: String,
        screen_root: bool,
    ) -> Option<Element> {
        let name = node.name.name.as_str();
        if let Some(prim) = schema::primitive(name) {
            return self.primitive(prim, node, ctx, id);
        }
        if let Some(d) = self.decls.get(name).copied() {
            return match d.decl.kind {
                DeclKind::Component => self.expand(d, node, ctx, id),
                DeclKind::Layout if screen_root => self.expand(d, node, ctx, id),
                DeclKind::Layout => {
                    self.err(
                        ctx.path,
                        "E2110",
                        format!("layout `{name}` can only be the root of a screen"),
                        node.name.span,
                        Some("use a component for reusable pieces inside a screen".into()),
                    );
                    None
                }
                _ => {
                    self.err(
                        ctx.path,
                        "E2111",
                        format!("screen `{name}` cannot be used as an element"),
                        node.name.span,
                        Some("navigation between screens arrives in M6".into()),
                    );
                    None
                }
            };
        }
        let mut candidates: Vec<&str> = schema::PRIMITIVES.iter().map(|p| p.name).collect();
        if is_pascal(name) {
            candidates = self
                .decls
                .values()
                .filter(|d| d.decl.kind != DeclKind::Screen)
                .map(|d| d.decl.name.name.as_str())
                .collect();
        }
        let fallback = if is_pascal(name) {
            "components and layouts"
        } else {
            "built-in elements"
        };
        self.err(
            ctx.path,
            "E2112",
            format!("unknown element `{name}`"),
            node.name.span,
            Some(hint(name, &candidates, fallback)),
        );
        None
    }

    fn primitive(
        &mut self,
        prim: &'static Primitive,
        node: &Node,
        ctx: &mut Ctx,
        id: String,
    ) -> Option<Element> {
        let content = match self.content(prim, node, ctx) {
            Ok(c) => c,
            Err(()) => return None,
        };

        let mut props: BTreeMap<&'static str, Value> = BTreeMap::new();
        let mut set_here: BTreeMap<&'static str, Span> = BTreeMap::new();
        let mut children = Vec::new();
        let mut child_items = 0usize;
        let items = node
            .block
            .as_ref()
            .map(|b| b.items.as_slice())
            .unwrap_or_default();

        for item in items {
            match item {
                Item::Property(p) => {
                    let Some(key) = self.property_key(prim, &p.name.name, p.name.span, ctx) else {
                        continue;
                    };
                    if set_here.insert(key, p.name.span).is_some() {
                        self.err(
                            ctx.path,
                            "E2121",
                            format!("`{key}` is set more than once"),
                            p.name.span,
                            None,
                        );
                        continue;
                    }
                    self.assign(prim, key, &p.value, ctx, &mut props, true);
                }
                Item::Node(child) => {
                    child_items += 1;
                    if prim.children == Children::None {
                        self.err(
                            ctx.path,
                            "E2122",
                            format!("`{}` cannot contain elements", prim.name),
                            child.name.span,
                            Some("put both elements inside a `column`, `row` or `stack`".into()),
                        );
                        continue;
                    }
                    let child_id = format!("{id}/{}", children.len());
                    if let Some(e) = self.element(child, ctx, child_id, false) {
                        children.push(e);
                    }
                }
                Item::Slot(span) => {
                    child_items += 1;
                    if prim.children == Children::None {
                        self.err(
                            ctx.path,
                            "E2122",
                            format!("`{}` cannot contain a `slot`", prim.name),
                            *span,
                            None,
                        );
                        continue;
                    }
                    children.extend(ctx.slot.iter().cloned());
                }
                Item::When(w) => {
                    let selected = ctx.variants.get(&w.variant.name).cloned().flatten();
                    let apply = selected.as_deref() == Some(w.option.name.as_str());
                    if !ctx.validate_only && !apply {
                        continue;
                    }
                    for o in &w.block.items {
                        let Item::Property(p) = o else { continue };
                        let Some(key) = self.property_key(prim, &p.name.name, p.name.span, ctx)
                        else {
                            continue;
                        };
                        self.assign(prim, key, &p.value, ctx, &mut props, apply);
                    }
                }
                Item::Prop(_) | Item::Variant(_) | Item::State(_) | Item::Action(_) => {} // reported by the structure check
            }
        }

        // `display: none` removes the element, as in CSS.
        if props.get("display") == Some(&Value::Keyword("none")) {
            return None;
        }
        // `flex-direction` turns a column into a row (and back), reversed if asked.
        let mut kind = prim.kind;
        if let Some(Value::Keyword(dir)) = props.get("flex_direction").cloned() {
            if matches!(kind, ElementKind::Column | ElementKind::Row) {
                kind = if dir.starts_with("row") {
                    ElementKind::Row
                } else {
                    ElementKind::Column
                };
            }
            if dir.ends_with("reverse") {
                children.reverse();
            }
        }

        if prim.children == Children::One && child_items != 1 {
            self.err(
                ctx.path,
                "E2123",
                format!("`{}` must contain exactly one element", prim.name),
                node.name.span,
                Some("wrap its content in a `column`".into()),
            );
        }
        if prim.children == Children::One && children.len() > 1 {
            // A slot filled with several elements: wrap them in a column.
            let wrapped = Element {
                id: format!("{id}/0"),
                kind: ElementKind::Column,
                content: None,
                props: BTreeMap::new(),
                children: std::mem::take(&mut children),
                origin: ctx.origin.clone(),
            };
            children.push(wrapped);
        }

        self.apply_defaults(prim.kind, &mut props);
        Some(Element {
            id,
            kind,
            content,
            props,
            children,
            origin: ctx.origin.clone(),
        })
    }

    fn property_key(
        &mut self,
        prim: &Primitive,
        name: &str,
        span: Span,
        ctx: &Ctx,
    ) -> Option<&'static str> {
        let key = prim.key(name);
        if key.is_none() {
            let names: Vec<&str> = prim.property_names().collect();
            let help = match suggest(name, names.iter().copied()) {
                Some(s) => format!("did you mean `{s}`?"),
                None => format!("see docs/UI_SPEC.md for `{}` properties", prim.name),
            };
            self.err(
                ctx.path,
                "E2120",
                format!("`{}` has no property `{name}`", prim.name),
                span,
                Some(help),
            );
        }
        key
    }

    /// The element's argument: text, an asset, or nothing.
    /// `Err` means the element should be dropped (already reported, or an
    /// absent optional asset).
    fn content(
        &mut self,
        prim: &Primitive,
        node: &Node,
        ctx: &mut Ctx,
    ) -> Result<Option<Content>, ()> {
        let arg = node.arg.as_ref();
        match (prim.arg, arg) {
            (ArgType::None, None) => Ok(None),
            (ArgType::None, Some(a)) => {
                self.err(
                    ctx.path,
                    "E2113",
                    format!("`{}` does not take a value", prim.name),
                    a.span,
                    Some("set properties inside `{ }` instead".into()),
                );
                Ok(None)
            }
            (ArgType::Text | ArgType::Asset(_), None) => {
                let example = match prim.arg {
                    ArgType::Text => format!("{} \"Hello\"", prim.name),
                    ArgType::Asset(AssetKind::Icon) => "icon \"Icons/star.svg\"".into(),
                    _ => "image \"Images/hero.png\"".into(),
                };
                self.err(
                    ctx.path,
                    "E2116",
                    format!("`{}` needs a value, like `{example}`", prim.name),
                    node.name.span,
                    None,
                );
                Err(())
            }
            (ArgType::Text, Some(a)) => match &a.kind {
                ValueKind::String(s) => match self.template_parts(s, a.span, ctx) {
                    Some(parts) => Ok(Some(text_content(parts))),
                    None => Err(()),
                },
                ValueKind::Path(parts) => match self.state_path(parts, ctx) {
                    Some(r) => Ok(Some(Content::Template(vec![TextPart::State(r)]))),
                    None => Err(()),
                },
                ValueKind::Ident(n) => {
                    match self.prop_ref(n, &[PropType::Text], a.span, "text", ctx) {
                        Some(Bound::Text(s)) => Ok(Some(Content::Text(s))),
                        Some(Bound::Template(p)) => Ok(Some(Content::Template(p))),
                        Some(_) => Ok(Some(Content::Text(String::new()))),
                        None => Err(()),
                    }
                }
                _ => {
                    self.err(
                        ctx.path,
                        "E2115",
                        format!("`{}` needs text in quotes", prim.name),
                        a.span,
                        None,
                    );
                    Err(())
                }
            },
            (ArgType::Asset(kind), Some(a)) => match &a.kind {
                ValueKind::String(s) => match self.asset(s, kind, a.span, ctx.path) {
                    Some(r) => Ok(Some(Content::Asset(r))),
                    None => Err(()),
                },
                ValueKind::Ident(n) => {
                    let want = if kind == AssetKind::Icon {
                        PropType::Icon
                    } else {
                        PropType::Image
                    };
                    match self.prop_ref(n, &[want], a.span, prop_type_name(want), ctx) {
                        Some(Bound::Asset(r)) => Ok(Some(Content::Asset(r))),
                        Some(Bound::Placeholder) => Ok(None),
                        _ => Err(()),
                    }
                }
                _ => {
                    self.err(
                        ctx.path,
                        "E2115",
                        format!("`{}` needs an asset path in quotes", prim.name),
                        a.span,
                        None,
                    );
                    Err(())
                }
            },
        }
    }

    /// Look up a prop used by name; checks it exists and has an allowed type.
    fn prop_ref(
        &mut self,
        name: &str,
        allowed: &[PropType],
        span: Span,
        want: &str,
        ctx: &Ctx,
    ) -> Option<Bound> {
        match ctx.props.get(name) {
            Some(b) if allowed.contains(&b.ty) => Some(b.value.clone()),
            Some(b) => {
                self.err(
                    ctx.path,
                    "E2130",
                    format!(
                        "prop `{name}` is {} but {want} is needed here",
                        prop_type_name(b.ty)
                    ),
                    span,
                    None,
                );
                None
            }
            None => {
                let help = if ctx.scope == Scope::Screen {
                    "put text in quotes, like `\"Hello\"`".to_string()
                } else {
                    let names: Vec<&str> = ctx.props.keys().map(String::as_str).collect();
                    hint(name, &names, "props declared here")
                };
                self.err(
                    ctx.path,
                    "E2114",
                    format!("unknown prop `{name}`"),
                    span,
                    Some(help),
                );
                None
            }
        }
    }

    // ----- values ------------------------------------------------------

    fn value(&mut self, vt: ValueType, v: &AstValue, prop: &str, ctx: &Ctx) -> Option<Value> {
        // A bare identifier naming a prop of the enclosing component/layout.
        if let ValueKind::Ident(n) = &v.kind
            && let Some(b) = ctx.props.get(n)
        {
            return self.prop_as_value(n, b.ty, &b.value, vt, v.span, prop, ctx);
        }
        match (vt, &v.kind) {
            (ValueType::Action, _) => return self.effects(v, ctx).map(Value::Effects),
            (ValueType::Condition, _) => return self.condition(v, prop, ctx),
            (ValueType::Binding, _) => return self.binding(v, prop, ctx),
            (ValueType::Text, ValueKind::String(s)) => {
                return self.template_parts(s, v.span, ctx).map(text_value);
            }
            (ValueType::Text, ValueKind::Path(parts)) => {
                return self
                    .state_path(parts, ctx)
                    .map(|r| Value::Template(vec![TextPart::State(r)]));
            }
            _ => {}
        }
        if let Some(result) = self.css_value(vt, v, prop, ctx) {
            return result;
        }
        let type_err = |r: &mut Self| {
            r.err(
                ctx.path,
                "E2133",
                format!("`{prop}` needs {}", vt.describe()),
                v.span,
                None,
            );
            None
        };
        match (vt, &v.kind) {
            (ValueType::Color, ValueKind::Color(c)) => Some(Value::Color(ColorValue {
                token: None,
                rgba: *c,
            })),
            (ValueType::Color, ValueKind::None) => Some(Value::Transparent),
            (ValueType::Color, ValueKind::Ident(n)) => match self.skin.colors.get(n) {
                Some(c) => Some(Value::Color(ColorValue {
                    token: Some(n.clone()),
                    rgba: *c,
                })),
                None => self.unknown_token(TokenCategory::Color, n, v.span, ctx),
            },
            // `none` on a spacing/radius/border/elevation property means 0 (D-039).
            (ValueType::Dimension(_), ValueKind::None) => Some(Value::Dimension(Dimension {
                token: None,
                dp: 0.0,
            })),
            (ValueType::Dimension(_), ValueKind::Number(x)) if *x >= 0.0 => {
                Some(Value::Dimension(Dimension {
                    token: None,
                    dp: *x,
                }))
            }
            (ValueType::Dimension(cat), ValueKind::Ident(n)) => match self.skin.dimension(cat, n) {
                Some(dp) => Some(Value::Dimension(Dimension {
                    token: Some(n.clone()),
                    dp,
                })),
                None => self.unknown_token(cat, n, v.span, ctx),
            },
            (ValueType::TextStyle, ValueKind::Ident(n)) => {
                if self.skin.typography.styles.contains_key(n) {
                    Some(Value::TextStyle(n.clone()))
                } else {
                    self.unknown_token(TokenCategory::TextStyle, n, v.span, ctx)
                }
            }
            (ValueType::Size, ValueKind::Ident(n)) if n == "fill" => Some(Value::Size(Size::Fill)),
            (ValueType::Size, ValueKind::Ident(n)) if n == "fit" => Some(Value::Size(Size::Fit)),
            (ValueType::Size, ValueKind::Number(x)) if *x >= 0.0 => Some(Value::Size(Size::Dp(*x))),
            (ValueType::Number { min, integer }, ValueKind::Number(x))
                if *x >= min && (!integer || x.fract() == 0.0) =>
            {
                Some(Value::Number(*x))
            }
            (ValueType::Bool, ValueKind::Bool(b)) => Some(Value::Bool(*b)),
            (ValueType::Keyword(opts), ValueKind::Ident(n)) => match opts.iter().find(|o| **o == n)
            {
                Some(o) => Some(Value::Keyword(o)),
                None => {
                    self.err(
                        ctx.path,
                        "E2133",
                        format!("`{prop}` cannot be `{n}`"),
                        v.span,
                        Some(hint(n, opts, "options")),
                    );
                    None
                }
            },
            (ValueType::Text, ValueKind::String(s)) => Some(Value::Text(s.clone())),
            (ValueType::Text, ValueKind::Ident(n))
                if ctx.scope == Scope::Screen || !ctx.props.contains_key(n) =>
            {
                self.err(
                    ctx.path,
                    "E2114",
                    format!("unknown prop `{n}`"),
                    v.span,
                    Some("put text in quotes, like `\"Hello\"`".into()),
                );
                None
            }
            _ => type_err(self),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prop_as_value(
        &mut self,
        name: &str,
        ty: PropType,
        bound: &Bound,
        vt: ValueType,
        span: Span,
        prop: &str,
        ctx: &Ctx,
    ) -> Option<Value> {
        // A prop must not hide a skin token used by this property (GRAMMAR §5).
        let category = match vt {
            ValueType::Color => Some(TokenCategory::Color),
            ValueType::Dimension(c) => Some(c),
            ValueType::TextStyle => Some(TokenCategory::TextStyle),
            _ => None,
        };
        if let Some(cat) = category
            && self.skin.names(cat).contains(&name)
        {
            self.err(
                ctx.path,
                "E2131",
                format!(
                    "prop `{name}` has the same name as the {} token `{name}`",
                    cat.section()
                ),
                span,
                Some("rename the prop so it is clear which one is meant".into()),
            );
            return None;
        }
        let compatible = matches!(
            (ty, vt),
            (PropType::Text, ValueType::Text)
                | (
                    PropType::Number,
                    ValueType::Number { .. } | ValueType::Size | ValueType::Dimension(_)
                )
                | (PropType::Bool, ValueType::Bool | ValueType::Condition)
                | (PropType::Color, ValueType::Color)
                | (PropType::Action, ValueType::Action)
        );
        if !compatible {
            self.err(
                ctx.path,
                "E2130",
                format!(
                    "prop `{name}` is {} but `{prop}` needs {}",
                    prop_type_name(ty),
                    vt.describe()
                ),
                span,
                None,
            );
            return None;
        }
        match bound {
            Bound::Absent => None,
            Bound::Placeholder => Some(match vt {
                ValueType::Color => Value::Transparent,
                ValueType::Text => Value::Text(String::new()),
                ValueType::Bool => Value::Bool(false),
                ValueType::Condition => Value::Bool(true),
                ValueType::Action => Value::Effects(Vec::new()),
                ValueType::Size => Value::Size(Size::Fit),
                ValueType::Dimension(_) => Value::Dimension(Dimension {
                    token: None,
                    dp: 0.0,
                }),
                _ => Value::Number(0.0),
            }),
            Bound::Text(s) => Some(Value::Text(s.clone())),
            Bound::Template(p) => Some(Value::Template(p.clone())),
            Bound::Effects(e) => Some(Value::Effects(e.clone())),
            Bound::Bool(b) => Some(Value::Bool(*b)),
            Bound::Color(v) => Some(v.clone()),
            Bound::Number(x) => match vt {
                ValueType::Size => Some(Value::Size(Size::Dp(*x))),
                ValueType::Dimension(_) => Some(Value::Dimension(Dimension {
                    token: None,
                    dp: *x,
                })),
                _ => Some(Value::Number(*x)),
            },
            Bound::Asset(_) => None,
        }
    }

    // ----- M8: CSS values -----------------------------------------------------

    /// CSS forms of values (units, keywords, ratios). `None` means "not a CSS
    /// form, use the ERA rules"; `Some(result)` is final.
    fn css_value(
        &mut self,
        vt: ValueType,
        v: &AstValue,
        prop: &str,
        ctx: &Ctx,
    ) -> Option<Option<Value>> {
        let unit_err = |r: &mut Self, unit: &str| {
            r.err(
                ctx.path,
                "E2134",
                format!("`{prop}` does not take `{unit}` values"),
                v.span,
                Some(format!("`{prop}` needs {}", vt.describe())),
            );
            Some(None)
        };
        match (vt, &v.kind) {
            (ValueType::Size, ValueKind::Dimension(n, u)) => Some(match u.as_str() {
                "%" => Some(Value::Size(Size::Percent(*n))),
                "fr" => Some(Value::Size(Size::Fr(*n))),
                "vw" | "vh" => Some(Value::Size(Size::Viewport {
                    vertical: u == "vh",
                    percent: *n,
                })),
                _ => match css_length(*n, u) {
                    Some(dp) if dp >= 0.0 => Some(Value::Size(Size::Dp(dp))),
                    Some(_) => return unit_err(self, "negative"),
                    None => return unit_err(self, u),
                },
            }),
            (ValueType::Size, ValueKind::Ident(w)) => match w.as_str() {
                "auto" | "fit-content" => Some(Some(Value::Size(Size::Fit))),
                "stretch" => Some(Some(Value::Size(Size::Fill))),
                "min-content" => Some(Some(Value::Size(Size::MinContent))),
                "max-content" => Some(Some(Value::Size(Size::MaxContent))),
                _ => None,
            },
            (ValueType::Dimension(_) | ValueType::Number { .. }, ValueKind::Dimension(n, u)) => {
                match css_length(*n, u) {
                    Some(dp) => {
                        let plain = AstValue {
                            kind: ValueKind::Number(dp),
                            span: v.span,
                        };
                        Some(self.value(vt, &plain, prop, ctx))
                    }
                    None => unit_err(self, u),
                }
            }
            (ValueType::Offset, ValueKind::Number(n)) => Some(Some(Value::Size(Size::Dp(*n)))),
            (ValueType::Offset, ValueKind::Dimension(n, u)) => {
                match (u.as_str(), css_length(*n, u)) {
                    ("%", _) => Some(Some(Value::Size(Size::Percent(*n)))),
                    (_, Some(dp)) => Some(Some(Value::Size(Size::Dp(dp)))),
                    _ => unit_err(self, u),
                }
            }
            (ValueType::Offset, _) => {
                self.err(
                    ctx.path,
                    "E2133",
                    format!("`{prop}` needs {}", vt.describe()),
                    v.span,
                    None,
                );
                Some(None)
            }
            (ValueType::Margin, ValueKind::Ident(w)) if w == "auto" => {
                Some(Some(Value::Keyword("auto")))
            }
            (ValueType::Margin, _) => {
                Some(self.value(ValueType::Dimension(TokenCategory::Spacing), v, prop, ctx))
            }
            (ValueType::Ratio, ValueKind::Number(n)) if *n > 0.0 => Some(Some(Value::Number(*n))),
            (ValueType::Ratio, ValueKind::Terms(t)) => match t.as_slice() {
                [
                    AstValue {
                        kind: ValueKind::Number(a),
                        ..
                    },
                    AstValue {
                        kind: ValueKind::Slash,
                        ..
                    },
                    AstValue {
                        kind: ValueKind::Number(b),
                        ..
                    },
                ] if *a > 0.0 && *b > 0.0 => Some(Some(Value::Number(a / b))),
                _ => None,
            },
            (ValueType::Unit, ValueKind::Number(n)) if (0.0..=1.0).contains(n) => {
                Some(Some(Value::Number(*n)))
            }
            (ValueType::Unit, ValueKind::Dimension(n, u))
                if u == "%" && (0.0..=100.0).contains(n) =>
            {
                Some(Some(Value::Number(n / 100.0)))
            }
            // `display: none`: `none` is an ERA keyword, but a valid CSS word here.
            (ValueType::Keyword(opts), ValueKind::None) if opts.contains(&"none") => {
                Some(Some(Value::Keyword("none")))
            }
            (ValueType::Keyword(opts), ValueKind::Ident(w)) => {
                // CSS spellings of ERA keywords: `flex-start`, `space-between`, `left`.
                let mapped = match w.as_str() {
                    "flex-start" | "self-start" | "left" | "top" | "normal" | "baseline" => "start",
                    "flex-end" | "self-end" | "right" | "bottom" => "end",
                    "space-between" => "between",
                    "space-around" => "around",
                    "space-evenly" => "evenly",
                    "fill" if opts.contains(&"stretch") => "stretch",
                    _ => return None,
                };
                opts.iter()
                    .find(|o| **o == mapped)
                    .map(|o| Some(Value::Keyword(o)))
            }
            _ => None,
        }
    }

    /// Resolve one property into `props`, expanding CSS shorthands:
    /// `padding`/`margin`/`inset` with 1–4 values, and `border: 1px solid #ccc`.
    fn assign(
        &mut self,
        prim: &Primitive,
        key: &'static str,
        v: &AstValue,
        ctx: &Ctx,
        props: &mut BTreeMap<&'static str, Value>,
        apply: bool,
    ) {
        let sides = |names: [&'static str; 4], t: &[AstValue]| -> Vec<(&'static str, AstValue)> {
            let (top, right, bottom, left) = match t.len() {
                1 => (0, 0, 0, 0),
                2 => (0, 1, 0, 1),
                3 => (0, 1, 2, 1),
                _ => (0, 1, 2, 3),
            };
            vec![
                (names[0], t[top].clone()),
                (names[1], t[right].clone()),
                (names[2], t[bottom].clone()),
                (names[3], t[left].clone()),
            ]
        };
        let side_names = match key {
            "padding" => Some([
                "padding_top",
                "padding_end",
                "padding_bottom",
                "padding_start",
            ]),
            "margin" => Some(["margin_top", "margin_end", "margin_bottom", "margin_start"]),
            "inset" => Some(["top", "right", "bottom", "left"]),
            _ => None,
        };
        let parts: Vec<(&'static str, AstValue)> = match (side_names, &v.kind) {
            (Some(names), ValueKind::Terms(t)) if t.len() <= 4 => sides(names, t),
            (Some(_), ValueKind::Terms(_)) => {
                self.err(
                    ctx.path,
                    "E2133",
                    format!("`{key}` takes 1 to 4 values"),
                    v.span,
                    None,
                );
                return;
            }
            (Some(names), _) if key == "inset" => sides(names, std::slice::from_ref(v)),
            _ if key == "border" => match self.border_parts(v, ctx) {
                Some(p) => p,
                None => return,
            },
            _ => vec![(key, v.clone())],
        };
        for (k, val) in parts {
            let vt = prim.property(k).unwrap();
            if let Some(r) = self.value(vt, &val, k, ctx)
                && apply
            {
                props.insert(k, r);
            }
        }
    }

    /// `border: 1px solid #ccc` -> width and colour; `border: none` -> no border.
    fn border_parts(&mut self, v: &AstValue, ctx: &Ctx) -> Option<Vec<(&'static str, AstValue)>> {
        let terms: Vec<AstValue> = match &v.kind {
            ValueKind::Terms(t) => t.clone(),
            _ => vec![v.clone()],
        };
        let mut out = Vec::new();
        for t in terms {
            match &t.kind {
                ValueKind::None => out.push((
                    "border_width",
                    AstValue {
                        kind: ValueKind::Number(0.0),
                        span: t.span,
                    },
                )),
                ValueKind::Ident(w) if w == "solid" => {}
                ValueKind::Ident(w) if w == "dashed" || w == "dotted" || w == "double" => {
                    self.err(
                        ctx.path,
                        "E2135",
                        format!("`{w}` borders are planned (CSS phase P3)"),
                        t.span,
                        Some("use `solid` for now".into()),
                    );
                    return None;
                }
                ValueKind::Number(_) | ValueKind::Dimension(..) => {
                    out.push(("border_width", t.clone()))
                }
                ValueKind::Ident(w) if self.skin.dimension(TokenCategory::Border, w).is_some() => {
                    out.push(("border_width", t.clone()))
                }
                _ => out.push(("border_color", t.clone())),
            }
        }
        Some(out)
    }

    // ----- M6: state, actions and templates -------------------------------

    fn template_parts(&mut self, text: &str, span: Span, ctx: &Ctx) -> Option<Vec<TextPart>> {
        match self.stores.template(text, span, ctx.path) {
            Ok(p) => Some(p),
            Err(d) => {
                self.push(d);
                None
            }
        }
    }

    fn state_path(&mut self, parts: &[era_syntax::Ident], ctx: &Ctx) -> Option<StateRef> {
        match self.stores.state_ref(parts, ctx.path) {
            Ok(r) => Some(r),
            Err(d) => {
                self.push(d);
                None
            }
        }
    }

    /// `on_press` values: `go Screen`, `back`, `Store.action`, an `action`
    /// prop, or several joined with `then`. `none` does nothing.
    fn effects(&mut self, v: &AstValue, ctx: &Ctx) -> Option<Vec<Effect>> {
        let items: Vec<&AstValue> = match &v.kind {
            ValueKind::Seq(vs) => vs.iter().collect(),
            _ => vec![v],
        };
        let mut out = Vec::new();
        let mut ok = true;
        for it in items {
            match &it.kind {
                ValueKind::Go(target) => {
                    if self.screens.contains(&target.name) {
                        out.push(Effect::Go(target.name.clone()));
                    } else {
                        let names: Vec<&str> = self.screens.iter().map(String::as_str).collect();
                        self.err(
                            ctx.path,
                            "E2165",
                            format!("unknown screen `{}`", target.name),
                            target.span,
                            Some(hint(&target.name, &names, "screens")),
                        );
                        ok = false;
                    }
                }
                ValueKind::Ident(n) if n == "back" => out.push(Effect::Back),
                ValueKind::Ident(n) if ctx.props.contains_key(n) => {
                    let b = &ctx.props[n];
                    match (&b.ty, &b.value) {
                        (PropType::Action, Bound::Effects(e)) => out.extend(e.iter().cloned()),
                        (PropType::Action, _) => {}
                        (ty, _) => {
                            self.err(
                                ctx.path,
                                "E2130",
                                format!(
                                    "prop `{n}` is {} but an action is needed here",
                                    prop_type_name(*ty)
                                ),
                                it.span,
                                Some("declare it as `prop name: action`".into()),
                            );
                            ok = false;
                        }
                    }
                }
                ValueKind::Path(parts) => match self.stores.action(parts, ctx.path) {
                    Ok(e) => out.push(e),
                    Err(d) => {
                        self.push(d);
                        ok = false;
                    }
                },
                ValueKind::None => {}
                ValueKind::Seq(_) => unreachable!("sequences are flat"),
                _ => {
                    self.err(
                        ctx.path,
                        "E2164",
                        "this is not an action".into(),
                        it.span,
                        Some(
                            "use `go Screen`, `back` or `Store.action`, joined with `then`".into(),
                        ),
                    );
                    ok = false;
                }
            }
        }
        ok.then_some(out)
    }

    /// `visible`: a bool literal or bool state.
    fn condition(&mut self, v: &AstValue, prop: &str, ctx: &Ctx) -> Option<Value> {
        match &v.kind {
            ValueKind::Bool(b) => Some(Value::Bool(*b)),
            ValueKind::Path(parts) => {
                let r = self.state_path(parts, ctx)?;
                if r.ty != StateType::Bool {
                    self.err(
                        ctx.path,
                        "E2166",
                        format!(
                            "`{prop}` needs bool state, but `{}.{}` is {}",
                            r.store,
                            r.field,
                            r.ty.name()
                        ),
                        v.span,
                        None,
                    );
                    return None;
                }
                Some(Value::State(r))
            }
            _ => {
                self.err(
                    ctx.path,
                    "E2133",
                    format!("`{prop}` needs {}", ValueType::Condition.describe()),
                    v.span,
                    None,
                );
                None
            }
        }
    }

    /// `bind`: text state.
    fn binding(&mut self, v: &AstValue, prop: &str, ctx: &Ctx) -> Option<Value> {
        let ValueKind::Path(parts) = &v.kind else {
            self.err(
                ctx.path,
                "E2133",
                format!("`{prop}` needs {}", ValueType::Binding.describe()),
                v.span,
                None,
            );
            return None;
        };
        let r = self.state_path(parts, ctx)?;
        if r.ty != StateType::Text {
            self.err(
                ctx.path,
                "E2166",
                format!(
                    "`{prop}` needs text state, but `{}.{}` is {}",
                    r.store,
                    r.field,
                    r.ty.name()
                ),
                v.span,
                None,
            );
            return None;
        }
        Some(Value::State(r))
    }

    fn unknown_token(
        &mut self,
        cat: TokenCategory,
        name: &str,
        span: Span,
        ctx: &Ctx,
    ) -> Option<Value> {
        let names = self.skin.names(cat);
        let skin = if self.skin.builtin {
            "the default skin".to_string()
        } else {
            format!("skin `{}`", self.skin.name)
        };
        let help = match suggest(name, names.iter().copied()) {
            Some(s) => format!("did you mean `{s}`?"),
            None if names.is_empty() => format!(
                "{skin} has no `{}` tokens; use a number or add a section",
                cat.section()
            ),
            None => format!("{skin} defines: {}", names.join(", ")),
        };
        self.err(
            ctx.path,
            "E2132",
            format!("unknown {} token `{name}`", cat.section()),
            span,
            Some(help),
        );
        None
    }

    /// The launcher icon (`icon: "Images/app_icon.png"`, D-085). It is not
    /// added to the app's assets unless a screen also shows it.
    fn app_icon(&mut self, file: &str) -> Option<AppIcon> {
        let (icon, background) = std::mem::take(&mut self.app_icon);
        let background = background.and_then(|v| match &v.kind {
            ValueKind::Color(c) => Some(ColorValue {
                token: None,
                rgba: *c,
            }),
            ValueKind::Ident(n) if self.skin.colors.contains_key(n) => Some(ColorValue {
                token: Some(n.clone()),
                rgba: self.skin.colors[n],
            }),
            _ => {
                self.err(
                    file,
                    "E2107",
                    "`icon-background` needs a colour, like `#FFFFFF` or a skin colour".into(),
                    v.span,
                    None,
                );
                None
            }
        });
        let v = icon?;
        let ValueKind::String(path) = &v.kind else {
            self.err(
                file,
                "E2107",
                "`icon` needs an image path, like `icon: \"Images/app_icon.png\"`".into(),
                v.span,
                None,
            );
            return None;
        };
        let already = self.assets.iter().any(|a| &a.path == path);
        let asset = self.asset(path, AssetKind::Image, v.span, file)?;
        if !already {
            self.assets.remove(&asset);
        }
        Some(AppIcon {
            path: asset.path,
            background,
        })
    }

    fn asset(&mut self, path: &str, kind: AssetKind, span: Span, file: &str) -> Option<AssetRef> {
        let exts = schema::asset_extensions(kind);
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let what = if kind == AssetKind::Icon {
            "icon"
        } else {
            "image"
        };
        match resolve_asset(self.root, path) {
            Ok(_) if !exts.contains(&ext.as_str()) => {
                self.err(
                    file,
                    "E2152",
                    format!("`{path}` is not a supported {what} file"),
                    span,
                    Some(format!("{what} files can be: {}", exts.join(", "))),
                );
                None
            }
            Ok(_) => {
                let r = AssetRef {
                    path: path.to_string(),
                    kind,
                };
                self.assets.insert(r.clone());
                Some(r)
            }
            Err(AssetError::NotRelative | AssetError::Empty) => {
                self.err(
                    file,
                    "E2151",
                    format!("invalid asset path `{path}`"),
                    span,
                    Some("asset paths are relative to `Assets/`, like `Images/hero.png`".into()),
                );
                None
            }
            Err(e) => {
                self.err(
                    file,
                    "E2150",
                    e.to_string(),
                    span,
                    Some(format!("place the file at `Assets/{path}`")),
                );
                None
            }
        }
    }

    // ----- component/layout expansion -----------------------------------

    fn expand(
        &mut self,
        d: DeclRef<'a>,
        usage: &Node,
        caller: &mut Ctx,
        id: String,
    ) -> Option<Element> {
        let name = d.decl.name.name.clone();
        let kind = d.decl.kind.keyword();
        if self.stack.contains(&name) {
            self.err(
                caller.path,
                "E2140",
                format!("{kind} `{name}` uses itself"),
                usage.name.span,
                Some(format!(
                    "expansion chain: {} -> {name}",
                    self.stack.join(" -> ")
                )),
            );
            return None;
        }
        if let Some(a) = &usage.arg {
            self.err(
                caller.path,
                "E2141",
                format!("{kind}s take named props, not a value"),
                a.span,
                Some(format!("write it like `{name} {{ label: \"...\" }}`")),
            );
        }

        let (mut props, variant_decls) = self.declared_members(d, false);
        let mut variants: BTreeMap<String, Option<String>> = variant_decls
            .iter()
            .map(|(k, (_, def))| (k.clone(), Some(def.clone())))
            .collect();
        let has_slot = has_slot(d.decl);
        let mut slot = Vec::new();
        let mut bound_here: BTreeSet<String> = BTreeSet::new();
        let items = usage
            .block
            .as_ref()
            .map(|b| b.items.as_slice())
            .unwrap_or_default();

        for item in items {
            match item {
                Item::Property(p) => {
                    let pname = p.name.name.as_str();
                    if !bound_here.insert(pname.to_string()) {
                        self.err(
                            caller.path,
                            "E2121",
                            format!("`{pname}` is set more than once"),
                            p.name.span,
                            None,
                        );
                        continue;
                    }
                    if let Some(ty) = props.get(pname).map(|b| b.ty) {
                        if let Some(v) = self.prop_value(ty, &p.value, caller) {
                            props.insert(pname.to_string(), Binding { ty, value: v });
                        }
                    } else if let Some((options, _)) = variant_decls.get(pname) {
                        match &p.value.kind {
                            ValueKind::Ident(o) if options.contains(o) => {
                                variants.insert(pname.to_string(), Some(o.clone()));
                            }
                            _ => {
                                let opts: Vec<&str> = options.iter().map(String::as_str).collect();
                                let shown = match &p.value.kind {
                                    ValueKind::Ident(o) => o.clone(),
                                    _ => "this value".into(),
                                };
                                self.err(
                                    caller.path,
                                    "E2142",
                                    format!(
                                        "variant `{pname}` of `{name}` has no option `{shown}`"
                                    ),
                                    p.value.span,
                                    Some(hint(&shown, &opts, "options")),
                                );
                            }
                        }
                    } else {
                        let mut members: Vec<&str> = props.keys().map(String::as_str).collect();
                        members.extend(variant_decls.keys().map(String::as_str));
                        let help = match suggest(pname, members.iter().copied()) {
                            Some(s) => format!("did you mean `{s}`?"),
                            None if members.is_empty() => {
                                format!("`{name}` declares no props or variants")
                            }
                            None => format!("`{name}` accepts: {}", members.join(", ")),
                        };
                        self.err(
                            caller.path,
                            "E2143",
                            format!("`{name}` has no prop or variant `{pname}`"),
                            p.name.span,
                            Some(help),
                        );
                    }
                }
                Item::Node(child) => {
                    if !has_slot {
                        self.err(
                            caller.path,
                            "E2144",
                            format!("`{name}` has no `slot`, so it cannot contain elements"),
                            child.name.span,
                            None,
                        );
                        continue;
                    }
                    let child_id = format!("{id}/slot/{}", slot.len());
                    if let Some(e) = self.element(child, caller, child_id, false) {
                        slot.push(e);
                    }
                }
                Item::Slot(span) => {
                    if !has_slot {
                        self.err(
                            caller.path,
                            "E2144",
                            format!("`{name}` has no `slot`, so it cannot contain elements"),
                            *span,
                            None,
                        );
                        continue;
                    }
                    slot.extend(caller.slot.iter().cloned());
                }
                _ => {}
            }
        }

        // Required props: no default and not optional.
        for item in &d.decl.body.items {
            if let Item::Prop(p) = item
                && p.default.is_none()
                && !p.optional
                && !bound_here.contains(&p.name.name)
            {
                self.err(
                    caller.path,
                    "E2145",
                    format!("`{name}` is missing required prop `{}`", p.name.name),
                    usage.name.span,
                    Some(format!("add `{}: ...` inside `{name} {{ }}`", p.name.name)),
                );
                props.get_mut(&p.name.name).unwrap().value = Bound::Placeholder;
            }
        }

        let root = d.decl.body.items.iter().find_map(|i| match i {
            Item::Node(n) => Some(n),
            _ => None,
        })?;
        let mut origin = caller.origin.clone();
        origin.push(name.clone());
        let mut ctx = Ctx {
            path: d.path,
            scope: if d.decl.kind == DeclKind::Layout {
                Scope::Layout
            } else {
                Scope::Component
            },
            props,
            variants,
            slot,
            origin,
            validate_only: caller.validate_only,
        };
        self.stack.push(name.clone());
        let el = self.element(root, &mut ctx, format!("{id}:{name}"), false);
        self.stack.pop();
        el
    }

    /// Resolve a value passed to a prop, in the caller's context.
    fn prop_value(&mut self, ty: PropType, v: &AstValue, caller: &mut Ctx) -> Option<Bound> {
        if let ValueKind::Ident(n) = &v.kind
            && let Some(b) = caller.props.get(n)
        {
            let ok = b.ty == ty;
            if !ok {
                self.err(
                    caller.path,
                    "E2130",
                    format!(
                        "prop `{n}` is {} but {} is needed here",
                        prop_type_name(b.ty),
                        prop_type_name(ty)
                    ),
                    v.span,
                    None,
                );
                return None;
            }
            return Some(b.value.clone());
        }
        let want = |r: &mut Self, what: &str| {
            r.err(
                caller.path,
                "E2133",
                format!("this {} prop needs {what}", prop_type_name(ty)),
                v.span,
                None,
            );
            None
        };
        match (ty, &v.kind) {
            (PropType::Text, ValueKind::String(s)) => {
                let parts = self.template_parts(s, v.span, caller)?;
                Some(match logic::plain(&parts) {
                    Some(t) => Bound::Text(t),
                    None => Bound::Template(parts),
                })
            }
            (PropType::Text, ValueKind::Path(parts)) => {
                let r = self.state_path(parts, caller)?;
                Some(Bound::Template(vec![TextPart::State(r)]))
            }
            (PropType::Action, ValueKind::None) => Some(Bound::Absent),
            (PropType::Action, _) => self.effects(v, caller).map(Bound::Effects),
            (PropType::Text, _) => want(self, "text in quotes"),
            (PropType::Number, ValueKind::Number(x)) => Some(Bound::Number(*x)),
            (PropType::Number, _) => want(self, "a number"),
            (PropType::Bool, ValueKind::Bool(b)) => Some(Bound::Bool(*b)),
            (PropType::Bool, _) => want(self, "`true` or `false`"),
            (PropType::Color, _) => self
                .value(ValueType::Color, v, "color", caller)
                .map(Bound::Color),
            (PropType::Image | PropType::Icon, ValueKind::String(s)) => {
                let kind = if ty == PropType::Icon {
                    AssetKind::Icon
                } else {
                    AssetKind::Image
                };
                self.asset(s, kind, v.span, caller.path).map(Bound::Asset)
            }
            (PropType::Image | PropType::Icon, ValueKind::None) => Some(Bound::Absent),
            (PropType::Image | PropType::Icon, _) => want(self, "an asset path in quotes"),
        }
    }

    // ----- defaults ----------------------------------------------------

    fn color_token(&self, name: &str) -> ColorValue {
        ColorValue {
            token: Some(name.into()),
            rgba: self.skin.colors[name],
        }
    }

    fn dim(&self, cat: TokenCategory, name: &str, fallback: f64) -> Value {
        Value::Dimension(match self.skin.dimension(cat, name) {
            Some(dp) => Dimension {
                token: Some(name.into()),
                dp,
            },
            None => Dimension {
                token: None,
                dp: fallback,
            },
        })
    }

    /// Defaults from docs/UI_SPEC.md, applied only where not set.
    fn apply_defaults(&self, kind: ElementKind, props: &mut BTreeMap<&'static str, Value>) {
        use TokenCategory::*;
        // Required colors and `body` always exist: validated, and filled
        // from the default skin when missing (see `fill_required_from_default`).
        let color = |n: &str| Value::Color(self.color_token(n));
        let body = || Value::TextStyle("body".into());
        let defaults: Vec<(&'static str, Value)> = match kind {
            ElementKind::Text => vec![("style", body()), ("color", color("text"))],
            ElementKind::Button => vec![
                ("style", body()),
                ("background", color("primary")),
                ("text_color", color("on_primary")),
                ("radius", self.dim(Radius, "md", 12.0)),
                ("padding_x", self.dim(Spacing, "md", 16.0)),
                ("padding_y", self.dim(Spacing, "sm", 8.0)),
            ],
            ElementKind::Card => vec![
                ("background", color("surface")),
                ("radius", self.dim(Radius, "md", 12.0)),
                ("padding", self.dim(Spacing, "md", 16.0)),
                ("elevation", self.dim(Elevation, "sm", 1.0)),
            ],
            ElementKind::Input => vec![
                ("style", body()),
                ("text_color", color("text")),
                ("background", color("surface")),
                ("border_width", self.dim(Border, "thin", 1.0)),
                ("border_color", color("border")),
                ("radius", self.dim(Radius, "sm", 6.0)),
                ("padding", self.dim(Spacing, "sm", 8.0)),
            ],
            ElementKind::Divider => vec![
                ("color", color("border")),
                ("thickness", Value::Number(1.0)),
            ],
            ElementKind::Icon => vec![("size", Value::Number(24.0)), ("color", color("text"))],
            ElementKind::Image => vec![("fit", Value::Keyword("cover"))],
            ElementKind::Scroll => vec![("direction", Value::Keyword("vertical"))],
            ElementKind::Grid => vec![("columns", Value::Number(2.0))],
            _ => vec![],
        };
        for (k, v) in defaults {
            props.entry(k).or_insert(v);
        }
    }
}

fn has_slot(decl: &Decl) -> bool {
    fn walk(items: &[Item]) -> bool {
        items.iter().any(|i| match i {
            Item::Slot(_) => true,
            Item::Node(n) => n.block.as_ref().is_some_and(|b| walk(&b.items)),
            _ => false,
        })
    }
    walk(&decl.body.items)
}

fn text_content(parts: Vec<TextPart>) -> Content {
    match logic::plain(&parts) {
        Some(t) => Content::Text(t),
        None => Content::Template(parts),
    }
}

fn text_value(parts: Vec<TextPart>) -> Value {
    match logic::plain(&parts) {
        Some(t) => Value::Text(t),
        None => Value::Template(parts),
    }
}

/// Absolute CSS lengths in dp: `px` and `dp` are 1:1 (a CSS px is
/// density-independent), `pt` is 4/3, `rem`/`em` are 16. `None` for other units.
fn css_length(n: f64, unit: &str) -> Option<f64> {
    match unit {
        "px" | "dp" => Some(n),
        "pt" => Some(n * 4.0 / 3.0),
        "rem" | "em" => Some(n * 16.0),
        _ => None,
    }
}
