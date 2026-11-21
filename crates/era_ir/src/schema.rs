//! Property tables for the built-in primitives (docs/UI_SPEC.md §2).
//!
//! Each property has exactly one [`ValueType`]; the type decides which skin
//! token table a bare identifier is looked up in (docs/GRAMMAR.md §5).

use crate::ir::{AssetKind, ElementKind};
use crate::skin::TokenCategory;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    /// Color token, `#RRGGBB` literal, or `none`.
    Color,
    /// Spacing/radius/border/elevation token, or a number (dp).
    Dimension(TokenCategory),
    /// Typography style token.
    TextStyle,
    /// `fill`, `fit`, or a number (dp).
    Size,
    Number {
        min: f64,
        integer: bool,
    },
    Bool,
    Keyword(&'static [&'static str]),
    Text,
}

impl ValueType {
    pub fn describe(self) -> String {
        match self {
            ValueType::Color => "a color (token, #RRGGBB or `none`)".into(),
            ValueType::Dimension(c) => {
                let s = c.section();
                let article = if s.starts_with(['a', 'e', 'i', 'o', 'u']) {
                    "an"
                } else {
                    "a"
                };
                format!("{article} {s} token, a number or `none`")
            }
            ValueType::TextStyle => "a text style token".into(),
            ValueType::Size => "`fill`, `fit` or a number".into(),
            ValueType::Number { integer: true, min } => format!("a whole number of at least {min}"),
            ValueType::Number { min, .. } => format!("a number of at least {min}"),
            ValueType::Bool => "`true` or `false`".into(),
            ValueType::Keyword(opts) => format!("one of: {}", opts.join(", ")),
            ValueType::Text => "text in quotes".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    None,
    /// Required string (or text prop).
    Text,
    /// Required asset path (or image/icon prop).
    Asset(AssetKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Children {
    None,
    One,
    Many,
}

pub struct Primitive {
    pub name: &'static str,
    pub kind: ElementKind,
    pub arg: ArgType,
    pub children: Children,
    /// Element-specific properties; [`COMMON`] applies to all.
    pub props: &'static [(&'static str, ValueType)],
}

const SPACING: ValueType = ValueType::Dimension(TokenCategory::Spacing);
const ALIGN: ValueType = ValueType::Keyword(&["start", "center", "end", "stretch"]);
const DISTRIBUTE: ValueType =
    ValueType::Keyword(&["start", "center", "end", "between", "around", "evenly"]);
const STACK_ALIGN: ValueType = ValueType::Keyword(&[
    "top_start",
    "top",
    "top_end",
    "start",
    "center",
    "end",
    "bottom_start",
    "bottom",
    "bottom_end",
]);
const PX: ValueType = ValueType::Number {
    min: 0.0,
    integer: false,
};
const COUNT: ValueType = ValueType::Number {
    min: 1.0,
    integer: true,
};

/// Properties every primitive accepts.
pub const COMMON: &[(&str, ValueType)] = &[
    ("width", ValueType::Size),
    ("height", ValueType::Size),
    ("min_width", PX),
    ("max_width", PX),
    ("min_height", PX),
    ("max_height", PX),
    ("padding", SPACING),
    ("padding_x", SPACING),
    ("padding_y", SPACING),
    ("padding_top", SPACING),
    ("padding_bottom", SPACING),
    ("padding_start", SPACING),
    ("padding_end", SPACING),
    ("background", ValueType::Color),
    ("border_width", ValueType::Dimension(TokenCategory::Border)),
    ("border_color", ValueType::Color),
    ("radius", ValueType::Dimension(TokenCategory::Radius)),
    ("elevation", ValueType::Dimension(TokenCategory::Elevation)),
    ("clip", ValueType::Bool),
];

const LINEAR: &[(&str, ValueType)] = &[
    ("gap", SPACING),
    ("align", ALIGN),
    ("distribute", DISTRIBUTE),
];

pub const PRIMITIVES: &[Primitive] = &[
    Primitive {
        name: "column",
        kind: ElementKind::Column,
        arg: ArgType::None,
        children: Children::Many,
        props: LINEAR,
    },
    Primitive {
        name: "row",
        kind: ElementKind::Row,
        arg: ArgType::None,
        children: Children::Many,
        props: LINEAR,
    },
    Primitive {
        name: "card",
        kind: ElementKind::Card,
        arg: ArgType::None,
        children: Children::Many,
        props: LINEAR,
    },
    Primitive {
        name: "stack",
        kind: ElementKind::Stack,
        arg: ArgType::None,
        children: Children::Many,
        props: &[("align", STACK_ALIGN)],
    },
    Primitive {
        name: "grid",
        kind: ElementKind::Grid,
        arg: ArgType::None,
        children: Children::Many,
        props: &[("columns", COUNT), ("gap", SPACING)],
    },
    Primitive {
        name: "scroll",
        kind: ElementKind::Scroll,
        arg: ArgType::None,
        children: Children::One,
        props: &[("direction", ValueType::Keyword(&["vertical", "horizontal"]))],
    },
    Primitive {
        name: "text",
        kind: ElementKind::Text,
        arg: ArgType::Text,
        children: Children::None,
        props: &[
            ("style", ValueType::TextStyle),
            ("color", ValueType::Color),
            ("align", ValueType::Keyword(&["start", "center", "end"])),
            ("max_lines", COUNT),
        ],
    },
    Primitive {
        name: "image",
        kind: ElementKind::Image,
        arg: ArgType::Asset(AssetKind::Image),
        children: Children::None,
        props: &[("fit", ValueType::Keyword(&["cover", "contain", "fill"]))],
    },
    Primitive {
        name: "icon",
        kind: ElementKind::Icon,
        arg: ArgType::Asset(AssetKind::Icon),
        children: Children::None,
        props: &[("size", PX), ("color", ValueType::Color)],
    },
    Primitive {
        name: "button",
        kind: ElementKind::Button,
        arg: ArgType::Text,
        children: Children::None,
        props: &[
            ("style", ValueType::TextStyle),
            ("text_color", ValueType::Color),
        ],
    },
    Primitive {
        name: "input",
        kind: ElementKind::Input,
        arg: ArgType::None,
        children: Children::None,
        props: &[
            ("placeholder", ValueType::Text),
            ("label", ValueType::Text),
            ("style", ValueType::TextStyle),
            ("text_color", ValueType::Color),
        ],
    },
    Primitive {
        name: "divider",
        kind: ElementKind::Divider,
        arg: ArgType::None,
        children: Children::None,
        props: &[("color", ValueType::Color), ("thickness", PX)],
    },
    Primitive {
        name: "spacer",
        kind: ElementKind::Spacer,
        arg: ArgType::None,
        children: Children::None,
        props: &[("size", SPACING)],
    },
];

pub fn primitive(name: &str) -> Option<&'static Primitive> {
    PRIMITIVES.iter().find(|p| p.name == name)
}

impl Primitive {
    pub fn property(&self, name: &str) -> Option<ValueType> {
        self.props
            .iter()
            .chain(COMMON)
            .find(|(n, _)| *n == name)
            .map(|(_, t)| *t)
    }

    /// The `&'static` key for a property name (for IR maps).
    pub fn key(&self, name: &str) -> Option<&'static str> {
        self.props
            .iter()
            .chain(COMMON)
            .find(|(n, _)| *n == name)
            .map(|(n, _)| *n)
    }

    pub fn property_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.props.iter().chain(COMMON).map(|(n, _)| *n)
    }
}

/// Allowed file extensions for asset arguments.
pub fn asset_extensions(kind: AssetKind) -> &'static [&'static str] {
    match kind {
        AssetKind::Image => &["png", "jpg", "jpeg", "webp"],
        AssetKind::Icon => &["svg"],
        AssetKind::Font => &["ttf", "otf"],
    }
}
