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
    /// `on_press`: `go Screen`, `back`, `Store.action`, chained with `then` (M6).
    Action,
    /// `visible`: `true`, `false`, bool state or a bool prop (M6).
    Condition,
    /// A length that may be negative or a percentage: `top: -8px`, `left: 10%` (M8).
    Offset,
    /// A spacing value or `auto` (`margin`, M8).
    Margin,
    /// `16 / 9` or `1.5` (`aspectRatio`, M8).
    Ratio,
    /// A number from 0 to 1 (`opacity`, M8).
    Unit,
    /// A CSS shorthand expanded into other properties (`border`, `inset`, M8).
    Shorthand,
    /// `bind`: text state an input reads and writes (M6).
    Binding,
    /// `box-shadow` layers (M8 P4b).
    Shadow,
    /// `filter` functions (M8 P4b).
    Filter,
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
            ValueType::Action => "an action: `go Screen`, `back` or `Store.action`".into(),
            ValueType::Condition => "`true`, `false` or bool state like `Session.verified`".into(),
            ValueType::Binding => "text state like `Session.code`".into(),
            ValueType::Offset => "a length like `8px`, `-4px` or `10%`".into(),
            ValueType::Margin => "a length like `8px`, a spacing token, or `auto`".into(),
            ValueType::Ratio => "a ratio like `16 / 9` or `1.5`".into(),
            ValueType::Unit => "a number from 0 to 1".into(),
            ValueType::Shorthand => "CSS shorthand values".into(),
            ValueType::Shadow => {
                "shadows like `0 4px 12px rgba(0, 0, 0, .15)`, several separated by commas, or `none`"
                    .into()
            }
            ValueType::Filter => "filter functions like `blur(4px) grayscale(1)`, or `none`".into(),
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
    ("on_press", ValueType::Action),
    ("visible", ValueType::Condition),
    // M8: CSS layout properties (docs/CSS_SPEC.md §4–§6).
    ("margin", ValueType::Margin),
    ("margin_x", ValueType::Margin),
    ("margin_y", ValueType::Margin),
    ("margin_top", ValueType::Margin),
    ("margin_bottom", ValueType::Margin),
    ("margin_start", ValueType::Margin),
    ("margin_end", ValueType::Margin),
    (
        "position",
        ValueType::Keyword(&["static", "relative", "absolute", "fixed"]),
    ),
    ("top", ValueType::Offset),
    ("right", ValueType::Offset),
    ("bottom", ValueType::Offset),
    ("left", ValueType::Offset),
    ("inset", ValueType::Shorthand),
    (
        "z_index",
        ValueType::Number {
            min: -1_000_000.0,
            integer: true,
        },
    ),
    ("opacity", ValueType::Unit),
    ("aspect_ratio", ValueType::Ratio),
    (
        "flex",
        ValueType::Number {
            min: 0.0,
            integer: false,
        },
    ),
    (
        "align_self",
        ValueType::Keyword(&["auto", "start", "center", "end", "stretch"]),
    ),
    (
        "display",
        ValueType::Keyword(&["flex", "block", "grid", "none"]),
    ),
    ("visibility", ValueType::Keyword(&["visible", "hidden"])),
    (
        "overflow",
        ValueType::Keyword(&["visible", "hidden", "clip", "scroll", "auto"]),
    ),
    ("border", ValueType::Shorthand),
    // M8 phase 3: per-side borders, per-corner radius, dashed/dotted, outline.
    ("border_style", BORDER_STYLE),
    ("border_top", ValueType::Shorthand),
    ("border_end", ValueType::Shorthand),
    ("border_bottom", ValueType::Shorthand),
    ("border_start", ValueType::Shorthand),
    ("border_top_width", BORDER_WIDTH),
    ("border_end_width", BORDER_WIDTH),
    ("border_bottom_width", BORDER_WIDTH),
    ("border_start_width", BORDER_WIDTH),
    ("border_top_color", ValueType::Color),
    ("border_end_color", ValueType::Color),
    ("border_bottom_color", ValueType::Color),
    ("border_start_color", ValueType::Color),
    ("border_top_style", BORDER_STYLE),
    ("border_end_style", BORDER_STYLE),
    ("border_bottom_style", BORDER_STYLE),
    ("border_start_style", BORDER_STYLE),
    ("radius_top_start", RADIUS),
    ("radius_top_end", RADIUS),
    ("radius_bottom_end", RADIUS),
    ("radius_bottom_start", RADIUS),
    ("outline", ValueType::Shorthand),
    ("outline_width", BORDER_WIDTH),
    ("outline_color", ValueType::Color),
    ("outline_style", BORDER_STYLE),
    ("outline_offset", ValueType::Offset),
    // M8 phase 4b: shadows and filters.
    ("box_shadow", ValueType::Shadow),
    ("filter", ValueType::Filter),
];

const BORDER_STYLE: ValueType = ValueType::Keyword(&["solid", "dashed", "dotted", "none"]);
const BORDER_WIDTH: ValueType = ValueType::Dimension(TokenCategory::Border);
const RADIUS: ValueType = ValueType::Dimension(TokenCategory::Radius);

/// Side and corner spellings mapped onto ERA's logical names before lookup:
/// `border-left-width` -> `border_start_width`,
/// `border-top-left-radius` -> `radius_top_start` (D-087).
const PREFIXES: &[(&str, &str)] = &[
    ("bordertopleftradius", "radiustopstart"),
    ("bordertoprightradius", "radiustopend"),
    ("borderbottomrightradius", "radiusbottomend"),
    ("borderbottomleftradius", "radiusbottomstart"),
    ("borderstartstartradius", "radiustopstart"),
    ("borderstartendradius", "radiustopend"),
    ("borderendendradius", "radiusbottomend"),
    ("borderendstartradius", "radiusbottomstart"),
    ("borderinlinestart", "borderstart"),
    ("borderinlineend", "borderend"),
    ("borderblockstart", "bordertop"),
    ("borderblockend", "borderbottom"),
    ("borderleft", "borderstart"),
    ("borderright", "borderend"),
];

/// Normalised spellings (lowercase, no `_`/`-`) that map to an ERA
/// property: CSS names and common synonyms. Every spelling of a property
/// works: `font-size`, `fontSize`, `font_size`, `fontsize` (D-072).
const ALIASES: &[(&str, &str)] = &[
    ("borderradius", "radius"),
    ("justifycontent", "distribute"),
    ("alignitems", "align"),
    ("textstyle", "style"),
    ("onclick", "on_press"),
    ("lineclamp", "max_lines"),
    ("webkitlineclamp", "max_lines"),
    ("objectfit", "fit"),
    ("backgroundcolor", "background"),
    ("paddingleft", "padding_start"),
    ("paddingright", "padding_end"),
    ("paddinginline", "padding_x"),
    ("paddingblock", "padding_y"),
    ("paddinginlinestart", "padding_start"),
    ("paddinginlineend", "padding_end"),
    ("paddingblockstart", "padding_top"),
    ("paddingblockend", "padding_bottom"),
    ("marginleft", "margin_start"),
    ("marginright", "margin_end"),
    ("margininline", "margin_x"),
    ("marginblock", "margin_y"),
    ("margininlinestart", "margin_start"),
    ("margininlineend", "margin_end"),
    ("marginblockstart", "margin_top"),
    ("marginblockend", "margin_bottom"),
    ("flexgrow", "flex"),
    ("rowgap", "gap"),
    ("columngap", "gap"),
    ("gridtemplatecolumns", "columns"),
    ("textalign", "align"),
];

/// Lowercase without `_` and `-`: `font-size`, `fontSize`, `font_size` and
/// `fontsize` all normalise to `fontsize`.
pub fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_' && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

const LINEAR: &[(&str, ValueType)] = &[
    (
        "flex_direction",
        ValueType::Keyword(&["row", "column", "row-reverse", "column-reverse"]),
    ),
    ("flex_wrap", ValueType::Keyword(&["nowrap", "wrap"])),
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
            ("bind", ValueType::Binding),
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

/// CSS typography on `text`, `button` and `input` (M8 phase 5, D-094).
pub const TYPOGRAPHY: &[(&str, ValueType)] = &[
    (
        "font_size",
        ValueType::Number {
            min: 1.0,
            integer: false,
        },
    ),
    (
        "font_weight",
        ValueType::Number {
            min: 1.0,
            integer: true,
        },
    ),
    (
        "font_style",
        ValueType::Keyword(&["normal", "italic", "oblique"]),
    ),
    (
        "font_family",
        ValueType::Keyword(&["serif", "sans-serif", "monospace", "cursive", "system-ui"]),
    ),
    (
        "line_height",
        ValueType::Number {
            min: 0.0,
            integer: false,
        },
    ),
    ("letter_spacing", ValueType::Offset),
    (
        "text_decoration",
        ValueType::Keyword(&["none", "underline", "line-through"]),
    ),
    (
        "text_transform",
        ValueType::Keyword(&["none", "uppercase", "lowercase", "capitalize"]),
    ),
    ("text_shadow", ValueType::Shadow),
    (
        "white_space",
        ValueType::Keyword(&["normal", "nowrap", "pre", "pre-wrap", "pre-line"]),
    ),
    ("text_overflow", ValueType::Keyword(&["clip", "ellipsis"])),
];

impl Primitive {
    fn typography(&self) -> &'static [(&'static str, ValueType)] {
        match self.kind {
            ElementKind::Text | ElementKind::Button | ElementKind::Input => TYPOGRAPHY,
            _ => &[],
        }
    }

    pub fn property(&self, name: &str) -> Option<ValueType> {
        let key = self.key(name)?;
        self.props
            .iter()
            .chain(self.typography())
            .chain(COMMON)
            .find(|(n, _)| *n == key)
            .map(|(_, t)| *t)
    }

    /// The canonical `&'static` key for any spelling of a property name
    /// (`border-radius`, `borderRadius`, `radius`, ...).
    pub fn key(&self, name: &str) -> Option<&'static str> {
        let n = normalize(name);
        let find = |target: &str| {
            self.props
                .iter()
                .chain(self.typography())
                .chain(COMMON)
                .find(|(k, _)| normalize(k) == target)
                .map(|(k, _)| *k)
        };
        if let Some(k) = find(&n) {
            return Some(k);
        }
        // `color` on a button or input means its text colour.
        if n == "color"
            && let Some(k) = find("textcolor")
        {
            return Some(k);
        }
        if let Some(k) = PREFIXES
            .iter()
            .find(|(from, _)| n.starts_with(from))
            .and_then(|(from, to)| find(&format!("{to}{}", &n[from.len()..])))
        {
            return Some(k);
        }
        ALIASES
            .iter()
            .find(|(a, _)| *a == n)
            .and_then(|(_, canon)| find(&normalize(canon)))
    }

    pub fn property_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.props
            .iter()
            .chain(self.typography())
            .chain(COMMON)
            .map(|(n, _)| *n)
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
