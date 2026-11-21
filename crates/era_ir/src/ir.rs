//! The UI intermediate representation.
//!
//! Every value is resolved and typed. Token references keep their token name
//! next to the resolved value, so backends can emit theme references
//! (`EraTheme.colors.primary`) instead of raw numbers. Collections are ordered
//! (`Vec` in source order, `BTreeMap` by key), so the IR is deterministic.

use std::collections::BTreeMap;

pub use era_syntax::Color;

use crate::skin::Skin;

#[derive(Debug, Clone, PartialEq)]
pub struct App {
    /// Display name from `app "..."`.
    pub name: String,
    /// Reverse-DNS id from `era.toml`.
    pub id: String,
    pub version: String,
    pub skin: Skin,
    /// Name of the first screen shown.
    pub start: String,
    /// In source order (entry file first, then by path).
    pub screens: Vec<Screen>,
    /// Every asset the UI references, by project-relative path (`Assets/...`).
    pub assets: Vec<AssetRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Screen {
    pub name: String,
    pub title: Option<String>,
    pub background: ColorValue,
    pub root: Element,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementKind {
    Column,
    Row,
    Stack,
    Grid,
    Scroll,
    Card,
    Text,
    Image,
    Icon,
    Button,
    Input,
    Divider,
    Spacer,
}

impl ElementKind {
    pub fn name(self) -> &'static str {
        match self {
            ElementKind::Column => "column",
            ElementKind::Row => "row",
            ElementKind::Stack => "stack",
            ElementKind::Grid => "grid",
            ElementKind::Scroll => "scroll",
            ElementKind::Card => "card",
            ElementKind::Text => "text",
            ElementKind::Image => "image",
            ElementKind::Icon => "icon",
            ElementKind::Button => "button",
            ElementKind::Input => "input",
            ElementKind::Divider => "divider",
            ElementKind::Spacer => "spacer",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Stable id: screen name plus child-index path, with component
    /// expansions marked, e.g. `Home/0/3:AppButton/0`.
    pub id: String,
    pub kind: ElementKind,
    /// Text for `text`/`button`, asset for `image`/`icon`.
    pub content: Option<Content>,
    /// Resolved properties, including defaults, keyed by property name.
    pub props: BTreeMap<&'static str, Value>,
    pub children: Vec<Element>,
    /// Components/layouts this element was expanded from, outermost first.
    pub origin: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    Text(String),
    Asset(AssetRef),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssetRef {
    /// Path as written in source, relative to `Assets/` (`Images/hero.png`).
    pub path: String,
    pub kind: AssetKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssetKind {
    Image,
    Icon,
    Font,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Color(ColorValue),
    /// `none` for a color property: draw nothing.
    Transparent,
    /// Spacing, radius, border or elevation, in dp.
    Dimension(Dimension),
    Size(Size),
    Number(f64),
    Bool(bool),
    /// One of a fixed set of keywords, e.g. `center`, `cover`.
    Keyword(&'static str),
    Text(String),
    /// A typography style from the skin, by name.
    TextStyle(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorValue {
    /// Skin token name, if the color came from a token.
    pub token: Option<String>,
    /// Resolved light-mode color.
    pub rgba: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dimension {
    pub token: Option<String>,
    pub dp: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Fill,
    Fit,
    Dp(f64),
}
