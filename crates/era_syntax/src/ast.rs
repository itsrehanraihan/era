//! AST for ERA source. Every node keeps a byte [`Span`] into its file.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn to(self, other: Span) -> Span {
        Span::new(self.start, other.end)
    }

    pub fn range(self) -> Range<usize> {
        self.start..self.end
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    App,
    Skin,
    Screen,
    Layout,
    Component,
}

impl DeclKind {
    pub fn keyword(self) -> &'static str {
        match self {
            DeclKind::App => "app",
            DeclKind::Skin => "skin",
            DeclKind::Screen => "screen",
            DeclKind::Layout => "layout",
            DeclKind::Component => "component",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub kind: DeclKind,
    /// For `app`, the display-name string; otherwise the declared identifier.
    pub name: Ident,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Property(Property),
    Node(Node),
    Prop(PropDecl),
    Variant(VariantDecl),
    When(When),
    Slot(Span),
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Property(p) => p.span,
            Item::Node(n) => n.span,
            Item::Prop(p) => p.span,
            Item::Variant(v) => v.span,
            Item::When(w) => w.span,
            Item::Slot(s) => *s,
        }
    }

    /// Human-readable description used in diagnostics.
    pub fn describe(&self) -> String {
        match self {
            Item::Property(p) => format!("property `{}`", p.name.name),
            Item::Node(n) => format!("`{}`", n.name.name),
            Item::Prop(_) => "`prop`".into(),
            Item::Variant(_) => "`variant`".into(),
            Item::When(_) => "`when`".into(),
            Item::Slot(_) => "`slot`".into(),
        }
    }
}

/// `name: value`
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    pub name: Ident,
    pub value: Value,
    pub span: Span,
}

/// `name [argument] [{ ... }]`
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: Ident,
    pub arg: Option<Value>,
    pub block: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    Text,
    Number,
    Bool,
    Color,
    Image,
    Icon,
}

impl PropType {
    pub const ALL: &[(&str, PropType)] = &[
        ("text", PropType::Text),
        ("number", PropType::Number),
        ("bool", PropType::Bool),
        ("color", PropType::Color),
        ("image", PropType::Image),
        ("icon", PropType::Icon),
    ];

    pub fn from_name(name: &str) -> Option<PropType> {
        Self::ALL.iter().find(|(n, _)| *n == name).map(|(_, t)| *t)
    }
}

/// `prop name: type[?] [= default]`
#[derive(Debug, Clone, PartialEq)]
pub struct PropDecl {
    pub name: Ident,
    pub ty: PropType,
    pub ty_span: Span,
    pub optional: bool,
    pub default: Option<Value>,
    pub span: Span,
}

/// `variant name: a | b | c [= a]`
#[derive(Debug, Clone, PartialEq)]
pub struct VariantDecl {
    pub name: Ident,
    pub options: Vec<Ident>,
    pub default: Option<Ident>,
    pub span: Span,
}

/// `when variant is option { overrides }`
#[derive(Debug, Clone, PartialEq)]
pub struct When {
    pub variant: Ident,
    pub option: Ident,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub kind: ValueKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueKind {
    String(String),
    Number(f64),
    Color(Color),
    Ident(String),
    Bool(bool),
    None,
}

/// sRGB color with alpha. `#RGB` and `#RRGGBB` get alpha 255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Names of primitives whose string argument is an asset path relative to
/// `Assets/`. Used by [`asset_references`].
pub const ASSET_NODES: &[&str] = &["image", "icon"];

/// An asset path written in source, e.g. `image "Images/hero.png"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRef {
    pub node: String,
    pub path: String,
    pub span: Span,
}

/// Collect every literal asset path in a file, in source order. Paths passed
/// through props are resolved later, in the IR stage.
pub fn asset_references(file: &File) -> Vec<AssetRef> {
    fn walk(block: &Block, out: &mut Vec<AssetRef>) {
        for item in &block.items {
            match item {
                Item::Node(n) => {
                    if let (
                        true,
                        Some(Value {
                            kind: ValueKind::String(path),
                            span,
                        }),
                    ) = (ASSET_NODES.contains(&n.name.name.as_str()), &n.arg)
                    {
                        out.push(AssetRef {
                            node: n.name.name.clone(),
                            path: path.clone(),
                            span: *span,
                        });
                    }
                    if let Some(b) = &n.block {
                        walk(b, out);
                    }
                }
                Item::When(w) => walk(&w.block, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for d in &file.decls {
        walk(&d.body, &mut out);
    }
    out
}
