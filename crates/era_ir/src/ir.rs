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
    /// App state and actions from `.er` logic files (M6), sorted by name.
    pub stores: Vec<Store>,
    /// Launcher icon from `app { icon: ... }` (D-085).
    pub icon: Option<AppIcon>,
}

/// `icon: "Images/app_icon.png"` and `icon-background: #FFFFFF` on `app`.
#[derive(Debug, Clone, PartialEq)]
pub struct AppIcon {
    /// Path relative to `Assets/` (PNG, JPEG or WebP).
    pub path: String,
    /// Background of the adaptive icon. `None` means white.
    pub background: Option<ColorValue>,
}

/// `store Name { state ... action ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct Store {
    pub name: String,
    pub fields: Vec<StateField>,
    pub actions: Vec<StoreAction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateField {
    pub name: String,
    pub ty: StateType,
    pub initial: Literal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateType {
    Text,
    Number,
    Bool,
}

impl StateType {
    pub fn name(self) -> &'static str {
        match self {
            StateType::Text => "text",
            StateType::Number => "number",
            StateType::Bool => "bool",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Text(String),
    Number(f64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreAction {
    pub name: String,
    pub steps: Vec<ActionStep>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionStep {
    Set { field: String, value: StepValue },
    Toggle { field: String },
    Add { field: String, amount: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepValue {
    Literal(Literal),
    /// Another field of the same store.
    Field(String),
}

/// A reference to one piece of state, `Store.field`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateRef {
    pub store: String,
    pub field: String,
    pub ty: StateType,
}

impl PartialOrd for StateType {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StateType {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

/// What a tap does, in order (`Session.verify then go Home`).
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Run a store action.
    Call { store: String, action: String },
    /// Show a screen (pushed on the navigation stack).
    Go(String),
    /// Return to the previous screen.
    Back,
}

/// Part of a text with `{Store.field}` interpolation.
#[derive(Debug, Clone, PartialEq)]
pub enum TextPart {
    Lit(String),
    State(StateRef),
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
    /// Text that shows state, e.g. `"Code: {Session.code}"` (M6).
    Template(Vec<TextPart>),
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
    /// Text with state interpolation (M6).
    Template(Vec<TextPart>),
    /// `on_press` effects (M6).
    Effects(Vec<Effect>),
    /// A bool condition (`visible`) or text binding (`bind`) read from state (M6).
    State(StateRef),
    /// `linear-gradient(...)` / `radial-gradient(...)` on `background` (M8 P4).
    Gradient(Gradient),
    /// Several `background` layers, top first, like CSS (M8 P4).
    Layers(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub kind: GradientKind,
    /// At least two stops, positions from 0 to 1, in order.
    pub stops: Vec<GradientStop>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GradientKind {
    /// CSS angle in degrees: 0 points up, 90 points right.
    Linear(f64),
    /// `to top right` and friends: the angle depends on the element's size.
    LinearToCorner { right: bool, bottom: bool },
    /// A circle from the centre to the farthest corner.
    Radial,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub color: ColorValue,
    pub position: f64,
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
    /// `50%` of the parent's content box (M8).
    Percent(f64),
    /// `2fr`: shares of the free space along a row/column (M8).
    Fr(f64),
    /// `50vw` / `30vh`: a share of the screen (M8).
    Viewport {
        vertical: bool,
        percent: f64,
    },
    /// `min-content` / `max-content` (M8).
    MinContent,
    MaxContent,
}
