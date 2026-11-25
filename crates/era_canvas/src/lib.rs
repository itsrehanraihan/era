//! Layout for the Era Builder canvas (D-095): turns a screen of the UI IR
//! into positioned, painted boxes, fast enough to redraw on every edit.
//!
//! Layout uses Taffy (CSS flexbox and grid). It follows the same rules as
//! the Compose backend (D-041 fill, D-043 screen root, CSS sizing), but it
//! is an editor preview: the Compose render ("Render real") stays the
//! reference for exact output.

use std::collections::BTreeMap;

use era_ir::ir::{
    App, Content, Element, ElementKind, GradientKind, Literal, Screen, Size as IrSize, SourceRef,
    StateType, TextPart, Value,
};
use taffy::prelude::*;

/// RGBA, 0-255.
pub type Rgba = [u8; 4];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Fill {
    None,
    Solid(Rgba),
    /// CSS angle in degrees (0 = up), stops as (position 0-1, colour).
    Linear(f32, Vec<(f32, Rgba)>),
    Radial(Vec<(f32, Rgba)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Rgba,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextPaint {
    pub text: String,
    pub size: f32,
    pub weight: u16,
    pub italic: bool,
    pub color: Rgba,
    /// 0 start, 1 center, 2 end
    pub align: u8,
    pub line_height: Option<f32>,
}

/// One drawn element.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// IR id; repeated items get `#n`.
    pub id: String,
    pub kind: ElementKind,
    pub source: Option<SourceRef>,
    /// Inside a component expansion: not directly editable here.
    pub from_component: bool,
    pub rect: Rect,
    pub depth: usize,
    /// Can hold children (drop target).
    pub container: bool,
    pub fill: Fill,
    /// top-start, top-end, bottom-end, bottom-start
    pub radius: [f32; 4],
    pub border: Option<(f32, Rgba)>,
    pub shadow: Option<Shadow>,
    pub opacity: f32,
    pub text: Option<TextPaint>,
    /// Image path relative to `Assets/`.
    pub image: Option<String>,
    /// Icon path and tint.
    pub icon: Option<(String, Rgba)>,
    pub clip: bool,
}

/// A laid-out screen, items in paint order (parents before children).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub screen: String,
    pub width: f32,
    pub height: f32,
    pub background: Rgba,
    pub items: Vec<Item>,
}

impl Frame {
    /// The deepest item under a point.
    pub fn hit(&self, x: f32, y: f32) -> Option<&Item> {
        self.items
            .iter()
            .filter(|i| i.rect.contains(x, y))
            .max_by_key(|i| i.depth)
    }

    /// The deepest container under a point (a drop target).
    pub fn container_at(&self, x: f32, y: f32) -> Option<&Item> {
        self.items
            .iter()
            .filter(|i| i.container && !i.from_component && i.rect.contains(x, y))
            .max_by_key(|i| i.depth)
    }
}

/// Text measuring, provided by the host (Era Builder uses egui's fonts).
pub trait Measure {
    /// Size of `text` at `size`/`weight`, wrapped to `max_width` if given.
    fn measure(&self, text: &str, size: f32, weight: u16, max_width: Option<f32>) -> (f32, f32);
}

/// A rough measure for tests and headless use.
pub struct ApproxMeasure;

impl Measure for ApproxMeasure {
    fn measure(&self, text: &str, size: f32, _weight: u16, max_width: Option<f32>) -> (f32, f32) {
        let w = text.chars().count() as f32 * size * 0.55;
        let line = size * 1.3;
        match max_width {
            Some(m) if m > 0.0 && w > m => (m, (w / m).ceil() * line),
            _ => (w, line),
        }
    }
}

fn rgba(v: Option<&Value>) -> Option<Rgba> {
    match v {
        Some(Value::Color(c)) => Some([c.rgba.r, c.rgba.g, c.rgba.b, c.rgba.a]),
        _ => None,
    }
}

fn num(v: Option<&Value>) -> Option<f32> {
    match v {
        Some(Value::Dimension(d)) => Some(d.dp as f32),
        Some(Value::Number(x)) => Some(*x as f32),
        Some(Value::Size(IrSize::Dp(x))) => Some(*x as f32),
        _ => None,
    }
}

fn keyword<'a>(e: &'a Element, k: &str) -> Option<&'a str> {
    match e.props.get(k) {
        Some(Value::Keyword(w)) => Some(w),
        _ => None,
    }
}

fn literal_text(l: &Literal) -> String {
    match l {
        Literal::Text(t) => t.clone(),
        Literal::Bool(b) => b.to_string(),
        Literal::Number(x) if x.fract() == 0.0 && x.abs() < 1e15 => format!("{}", *x as i64),
        Literal::Number(x) => x.to_string(),
    }
}

/// Values for state and loop items while drawing.
struct Env<'a> {
    app: &'a App,
    /// Loop item name -> (field -> value)
    items: Vec<(String, BTreeMap<String, Literal>)>,
}

impl Env<'_> {
    fn lookup(&self, store: &str, field: &str) -> Option<Literal> {
        if let Some((_, row)) = self.items.iter().rev().find(|(v, _)| v == store) {
            return row.get(field).cloned();
        }
        let st = self.app.stores.iter().find(|s| s.name == store)?;
        st.fields
            .iter()
            .find(|f| f.name == field)
            .map(|f| f.initial.clone())
    }

    fn text(&self, parts: &[TextPart]) -> String {
        parts
            .iter()
            .map(|p| match p {
                TextPart::Lit(s) => s.clone(),
                TextPart::State(r) => self
                    .lookup(&r.store, &r.field)
                    .map(|l| literal_text(&l))
                    .unwrap_or_default(),
            })
            .collect()
    }

    fn visible(&self, e: &Element) -> bool {
        if keyword(e, "display") == Some("none") {
            return false;
        }
        match e.props.get("visible") {
            Some(Value::Bool(b)) => *b,
            Some(Value::State(r)) => {
                matches!(self.lookup(&r.store, &r.field), Some(Literal::Bool(true)))
                    || (r.ty != StateType::Bool)
            }
            _ => true,
        }
    }
}

/// What the measure function needs to size a leaf.
#[derive(Clone)]
enum Leaf {
    Text {
        text: String,
        size: f32,
        weight: u16,
        pad: (f32, f32),
    },
    Fixed(f32, f32),
}

struct Builder<'a> {
    env: Env<'a>,
    tree: TaffyTree<Leaf>,
    /// Parallel to taffy nodes: the paint data, filled after layout.
    nodes: Vec<(NodeId, Item)>,
    screen_w: f32,
    screen_h: f32,
}

fn dimension(v: Option<&Value>, sw: f32, sh: f32) -> Option<Dimension> {
    match v? {
        Value::Size(IrSize::Dp(x)) => Some(length(*x as f32)),
        Value::Size(IrSize::Percent(p)) => Some(percent(*p as f32 / 100.0)),
        Value::Size(IrSize::Viewport {
            vertical,
            percent: p,
        }) => Some(length(if *vertical { sh } else { sw } * *p as f32 / 100.0)),
        Value::Number(x) => Some(length(*x as f32)),
        Value::Dimension(d) => Some(length(d.dp as f32)),
        _ => None,
    }
}

fn sides(e: &Element, prefix: &str) -> [f32; 4] {
    let all = num(e.props.get(prefix));
    let x = num(e.props.get(format!("{prefix}_x").as_str()));
    let y = num(e.props.get(format!("{prefix}_y").as_str()));
    let side = |s: &str, axis: Option<f32>| {
        num(e.props.get(format!("{prefix}_{s}").as_str()))
            .or(axis)
            .or(all)
            .unwrap_or(0.0)
    };
    [
        side("top", y),
        side("end", x),
        side("bottom", y),
        side("start", x),
    ]
}

impl<'a> Builder<'a> {
    fn text_paint(&self, e: &Element, color_key: &str) -> TextPaint {
        let skin = &self.env.app.skin;
        let base = match e.props.get("style") {
            Some(Value::TextStyle(s)) => skin.typography.styles.get(s),
            _ => skin.typography.styles.get("body"),
        };
        let size = num(e.props.get("font_size"))
            .or(base.map(|b| b.size as f32))
            .unwrap_or(16.0);
        let weight = num(e.props.get("font_weight"))
            .map(|w| w as u16)
            .or(base.map(|b| b.weight))
            .unwrap_or(400);
        let text = match &e.content {
            Some(Content::Text(t)) => t.clone(),
            Some(Content::Template(p)) => self.env.text(p),
            _ => String::new(),
        };
        let text = match keyword(e, "text_transform") {
            Some("uppercase") => text.to_uppercase(),
            Some("lowercase") => text.to_lowercase(),
            _ => text,
        };
        TextPaint {
            text,
            size,
            weight,
            italic: matches!(keyword(e, "font_style"), Some("italic" | "oblique")),
            color: rgba(e.props.get(color_key)).unwrap_or([23, 23, 23, 255]),
            align: match keyword(e, "align") {
                Some("center") => 1,
                Some("end") => 2,
                _ => 0,
            },
            line_height: match e.props.get("line_height") {
                Some(Value::Number(m)) => Some(*m as f32 * size),
                Some(Value::Size(IrSize::Dp(x))) => Some(*x as f32),
                _ => base.and_then(|b| b.line_height.map(|l| l as f32)),
            },
        }
    }

    fn paint(&self, e: &Element, depth: usize) -> Item {
        let fill = match e.props.get("background") {
            Some(Value::Layers(l)) => fill_of(l.last()),
            v => fill_of(v),
        };
        let r = num(e.props.get("radius"));
        let pct = matches!(e.props.get("radius"), Some(Value::Size(IrSize::Percent(_))));
        let corner = |k: &str| num(e.props.get(k)).or(r).unwrap_or(0.0);
        let mut radius = [
            corner("radius_top_start"),
            corner("radius_top_end"),
            corner("radius_bottom_end"),
            corner("radius_bottom_start"),
        ];
        if pct {
            radius = [f32::INFINITY; 4];
        }
        let border = num(e.props.get("border_width"))
            .filter(|w| *w > 0.0 && keyword(e, "border_style") != Some("none"))
            .map(|w| {
                (
                    w,
                    rgba(e.props.get("border_color")).unwrap_or([220, 220, 220, 255]),
                )
            });
        let shadow = match e.props.get("box_shadow") {
            Some(Value::Shadows(s)) => s.iter().find(|s| !s.inset).map(|s| Shadow {
                x: s.x as f32,
                y: s.y as f32,
                blur: s.blur as f32,
                spread: s.spread as f32,
                color: [
                    s.color.rgba.r,
                    s.color.rgba.g,
                    s.color.rgba.b,
                    s.color.rgba.a,
                ],
            }),
            _ => num(e.props.get("elevation"))
                .filter(|e| *e > 0.0)
                .map(|el| Shadow {
                    x: 0.0,
                    y: el,
                    blur: el * 2.0,
                    spread: 0.0,
                    color: [0, 0, 0, 40],
                }),
        };
        let text = match e.kind {
            ElementKind::Text => Some(self.text_paint(e, "color")),
            ElementKind::Button => Some(self.text_paint(e, "text_color")),
            ElementKind::Input => {
                let mut t = self.text_paint(e, "text_color");
                if let Some(Value::Text(p)) = e.props.get("placeholder") {
                    t.text = p.clone();
                    t.color[3] = 110;
                }
                Some(t)
            }
            _ => None,
        };
        let asset = |e: &Element| match &e.content {
            Some(Content::Asset(a)) => Some(a.path.clone()),
            Some(Content::ItemAsset { field, .. }) => self
                .env
                .lookup(&field.store, &field.field)
                .map(|l| literal_text(&l)),
            _ => None,
        };
        Item {
            id: e.id.clone(),
            kind: e.kind,
            source: e.source.clone(),
            from_component: !e.origin.is_empty(),
            rect: Rect::default(),
            depth,
            container: matches!(
                e.kind,
                ElementKind::Column
                    | ElementKind::Row
                    | ElementKind::Stack
                    | ElementKind::Grid
                    | ElementKind::Card
                    | ElementKind::Scroll
            ),
            fill,
            radius,
            border,
            shadow,
            opacity: match keyword(e, "visibility") {
                Some("hidden") => 0.0,
                _ => num(e.props.get("opacity")).unwrap_or(1.0),
            },
            text,
            image: (e.kind == ElementKind::Image).then(|| asset(e)).flatten(),
            icon: (e.kind == ElementKind::Icon)
                .then(|| {
                    asset(e).map(|p| (p, rgba(e.props.get("color")).unwrap_or([23, 23, 23, 255])))
                })
                .flatten(),
            clip: matches!(
                keyword(e, "overflow"),
                Some("hidden" | "clip" | "scroll" | "auto")
            ) || matches!(e.props.get("clip"), Some(Value::Bool(true)))
                || e.kind == ElementKind::Image,
        }
    }

    /// Build the taffy node for `e` (and its children). `main_row` is the
    /// parent's main axis, for `fill` (D-041).
    fn node(
        &mut self,
        e: &Element,
        depth: usize,
        parent_row: Option<bool>,
        root: bool,
    ) -> Vec<NodeId> {
        if !self.env.visible(e) {
            return vec![];
        }
        if e.kind == ElementKind::For {
            return self.repeat(e, depth, parent_row);
        }
        let (sw, sh) = (self.screen_w, self.screen_h);
        let mut style = Style {
            display: Display::Flex,
            ..Default::default()
        };
        // Size: `fill` grows along the parent's main axis and stretches across it.
        let along = |row_parent: Option<bool>, horizontal: bool| row_parent == Some(horizontal);
        for (horizontal, key) in [(true, "width"), (false, "height")] {
            let v = e.props.get(key);
            let dim = match v {
                Some(Value::Size(IrSize::Fill)) => {
                    if along(parent_row, horizontal) {
                        style.flex_grow = 1.0;
                        style.flex_basis = length(0.0_f32);
                        None
                    } else if parent_row.is_some() {
                        style.align_self = Some(AlignSelf::Stretch);
                        None
                    } else {
                        Some(percent(1.0_f32))
                    }
                }
                Some(Value::Size(IrSize::Fr(f))) if along(parent_row, horizontal) => {
                    style.flex_grow = *f as f32;
                    style.flex_basis = length(0.0_f32);
                    None
                }
                _ => dimension(v, sw, sh),
            };
            if let Some(d) = dim {
                if horizontal {
                    style.size.width = d;
                } else {
                    style.size.height = d;
                }
            }
        }
        if root {
            if style.size.width == Dimension::auto() {
                style.size.width = length(sw);
            }
            if style.size.height == Dimension::auto() {
                style.size.height = length(sh);
            }
        }
        if let Some(f) = num(e.props.get("flex")).filter(|f| *f > 0.0) {
            style.flex_grow = f;
            style.flex_basis = length(0.0_f32);
        }
        for (k, slot) in [
            ("min_width", 0),
            ("max_width", 1),
            ("min_height", 2),
            ("max_height", 3),
        ] {
            if let Some(d) = dimension(e.props.get(k), sw, sh) {
                match slot {
                    0 => style.min_size.width = d,
                    1 => style.max_size.width = d,
                    2 => style.min_size.height = d,
                    _ => style.max_size.height = d,
                }
            }
        }
        if let Some(Value::Number(r)) = e.props.get("aspect_ratio") {
            style.aspect_ratio = Some(*r as f32);
        }
        let [pt, pe, pb, ps] = sides(e, "padding");
        style.padding = taffy::Rect {
            left: length(ps),
            right: length(pe),
            top: length(pt),
            bottom: length(pb),
        };
        let margin = |s: &str, axis: &str| match e
            .props
            .get(format!("margin_{s}").as_str())
            .or(e.props.get(format!("margin_{axis}").as_str()))
            .or(e.props.get("margin"))
        {
            Some(Value::Keyword("auto")) => LengthPercentageAuto::auto(),
            v => length(num(v).unwrap_or(0.0)),
        };
        style.margin = taffy::Rect {
            left: margin("start", "x"),
            right: margin("end", "x"),
            top: margin("top", "y"),
            bottom: margin("bottom", "y"),
        };
        match keyword(e, "position") {
            Some("absolute" | "fixed") => {
                style.position = Position::Absolute;
                let inset = |k: &str| match e.props.get(k) {
                    Some(Value::Size(IrSize::Dp(x))) => length(*x as f32),
                    Some(Value::Size(IrSize::Percent(p))) => percent(*p as f32 / 100.0),
                    _ => LengthPercentageAuto::auto(),
                };
                style.inset = taffy::Rect {
                    left: inset("left"),
                    right: inset("right"),
                    top: inset("top"),
                    bottom: inset("bottom"),
                };
            }
            Some("relative") => {
                style.position = Position::Relative;
                let off = |k: &str| match e.props.get(k) {
                    Some(Value::Size(IrSize::Dp(x))) => length(*x as f32),
                    _ => LengthPercentageAuto::auto(),
                };
                style.inset = taffy::Rect {
                    left: off("left"),
                    right: off("right"),
                    top: off("top"),
                    bottom: off("bottom"),
                };
            }
            _ => {}
        }
        if let Some(a) = keyword(e, "align_self") {
            style.align_self = Some(match a {
                "start" => AlignSelf::FlexStart,
                "end" => AlignSelf::FlexEnd,
                "center" => AlignSelf::Center,
                _ => AlignSelf::Stretch,
            });
        }

        // Children and container behaviour.
        let gap = num(e.props.get("gap")).unwrap_or(0.0);
        let row = match e.kind {
            ElementKind::Row => true,
            ElementKind::Column | ElementKind::Card | ElementKind::Scroll => {
                matches!(keyword(e, "flex_direction"), Some("row" | "row-reverse"))
                    || (e.kind == ElementKind::Scroll
                        && keyword(e, "direction") == Some("horizontal"))
            }
            _ => false,
        };
        let mut children = Vec::new();
        let item = self.paint(e, depth);
        match e.kind {
            ElementKind::Column | ElementKind::Row | ElementKind::Card | ElementKind::Scroll => {
                style.flex_direction = match (row, keyword(e, "flex_direction")) {
                    (_, Some("row-reverse")) => FlexDirection::RowReverse,
                    (_, Some("column-reverse")) => FlexDirection::ColumnReverse,
                    (true, _) => FlexDirection::Row,
                    (false, _) => FlexDirection::Column,
                };
                if keyword(e, "flex_wrap") == Some("wrap") {
                    style.flex_wrap = FlexWrap::Wrap;
                }
                style.gap = taffy::Size {
                    width: length(gap),
                    height: length(gap),
                };
                style.align_items = Some(match keyword(e, "align") {
                    Some("center") => AlignItems::Center,
                    Some("end") => AlignItems::FlexEnd,
                    Some("stretch") => AlignItems::Stretch,
                    _ => AlignItems::FlexStart,
                });
                style.justify_content = Some(match keyword(e, "distribute") {
                    Some("center") => JustifyContent::Center,
                    Some("end") => JustifyContent::FlexEnd,
                    Some("between") => JustifyContent::SpaceBetween,
                    Some("around") => JustifyContent::SpaceAround,
                    Some("evenly") => JustifyContent::SpaceEvenly,
                    _ => JustifyContent::FlexStart,
                });
                if e.kind == ElementKind::Scroll {
                    style.overflow = taffy::Point {
                        x: taffy::Overflow::Clip,
                        y: taffy::Overflow::Clip,
                    };
                }
                for c in &e.children {
                    children.extend(self.node(c, depth + 1, Some(row), false));
                }
            }
            ElementKind::Stack => {
                style.display = Display::Grid;
                for c in &e.children {
                    let ids = self.node(c, depth + 1, None, false);
                    for id in &ids {
                        let mut s = self.tree.style(*id).unwrap().clone();
                        if s.position != Position::Absolute {
                            s.grid_row = taffy::Line {
                                start: line(1),
                                end: line(2),
                            };
                            s.grid_column = taffy::Line {
                                start: line(1),
                                end: line(2),
                            };
                            s.align_self = s.align_self.or(Some(match keyword(e, "align") {
                                Some(a) if a.starts_with("bottom") => AlignSelf::End,
                                Some("center" | "start" | "end") => AlignSelf::Center,
                                _ => AlignSelf::Start,
                            }));
                            s.justify_self = Some(match keyword(e, "align") {
                                Some(a) if a.ends_with("end") => AlignSelf::End,
                                Some("center" | "top" | "bottom") => AlignSelf::Center,
                                _ => AlignSelf::Start,
                            });
                        }
                        self.tree.set_style(*id, s).unwrap();
                    }
                    children.extend(ids);
                }
            }
            ElementKind::Grid => {
                style.display = Display::Grid;
                let cols = num(e.props.get("columns")).unwrap_or(2.0).max(1.0) as usize;
                style.grid_template_columns = (0..cols).map(|_| fr(1.0_f32)).collect();
                style.gap = taffy::Size {
                    width: length(gap),
                    height: length(gap),
                };
                for c in &e.children {
                    children.extend(self.node(c, depth + 1, None, false));
                }
            }
            _ => {}
        }

        let leaf = match e.kind {
            ElementKind::Text | ElementKind::Button | ElementKind::Input => {
                let t = item.text.clone().unwrap();
                Some(Leaf::Text {
                    text: if t.text.is_empty() {
                        " ".into()
                    } else {
                        t.text
                    },
                    size: t.size,
                    weight: t.weight,
                    pad: (0.0, 0.0),
                })
            }
            ElementKind::Image => Some(Leaf::Fixed(120.0, 120.0)),
            ElementKind::Icon => {
                let s = num(e.props.get("size")).unwrap_or(24.0);
                Some(Leaf::Fixed(s, s))
            }
            ElementKind::Divider => {
                let t = num(e.props.get("thickness")).unwrap_or(1.0);
                if parent_row == Some(true) {
                    style.size.width = length(t);
                    style.align_self = Some(AlignSelf::Stretch);
                } else {
                    style.size.height = length(t);
                    if style.size.width == Dimension::auto() {
                        style.align_self = Some(AlignSelf::Stretch);
                    }
                }
                None
            }
            ElementKind::Spacer => {
                match num(e.props.get("size")) {
                    Some(s) => {
                        style.size = taffy::Size {
                            width: length(s),
                            height: length(s),
                        }
                    }
                    None => style.flex_grow = 1.0,
                }
                None
            }
            _ => None,
        };
        let id = match leaf {
            Some(l) => self.tree.new_leaf_with_context(style, l).unwrap(),
            None => self.tree.new_with_children(style, &children).unwrap(),
        };
        self.nodes.push((id, item));
        vec![id]
    }

    /// `for p in List`: the children once per data row, in place.
    fn repeat(&mut self, e: &Element, depth: usize, parent_row: Option<bool>) -> Vec<NodeId> {
        let Some(Content::Repeat { var, list }) = &e.content else {
            return vec![];
        };
        let Some(data) = self.env.app.data.iter().find(|d| &d.name == list) else {
            return vec![];
        };
        let rows: Vec<BTreeMap<String, Literal>> = data
            .rows
            .iter()
            .map(|r| {
                data.fields
                    .iter()
                    .map(|(n, _)| n.clone())
                    .zip(r.iter().cloned())
                    .collect()
            })
            .collect();
        let mut out = Vec::new();
        for (i, row) in rows.into_iter().enumerate() {
            self.env.items.push((var.clone(), row));
            for c in &e.children {
                let start = self.nodes.len();
                out.extend(self.node(c, depth, parent_row, false));
                for n in &mut self.nodes[start..] {
                    n.1.id = format!("{}#{i}", n.1.id);
                }
            }
            self.env.items.pop();
        }
        out
    }
}

fn fill_of(v: Option<&Value>) -> Fill {
    match v {
        Some(Value::Color(c)) => Fill::Solid([c.rgba.r, c.rgba.g, c.rgba.b, c.rgba.a]),
        Some(Value::Gradient(g)) => {
            let stops = g
                .stops
                .iter()
                .map(|s| {
                    let c = s.color.rgba;
                    (s.position as f32, [c.r, c.g, c.b, c.a])
                })
                .collect();
            match g.kind {
                GradientKind::Linear(a) => Fill::Linear(a as f32, stops),
                GradientKind::LinearToCorner { right, bottom } => Fill::Linear(
                    match (right, bottom) {
                        (true, false) => 45.0,
                        (true, true) => 135.0,
                        (false, true) => 225.0,
                        (false, false) => 315.0,
                    },
                    stops,
                ),
                GradientKind::Radial => Fill::Radial(stops),
            }
        }
        _ => Fill::None,
    }
}

/// Lay out one screen at `width` x `height` dp.
pub fn layout_screen(
    app: &App,
    screen: &Screen,
    width: f32,
    height: f32,
    m: &dyn Measure,
) -> Frame {
    let mut b = Builder {
        env: Env {
            app,
            items: Vec::new(),
        },
        tree: TaffyTree::new(),
        nodes: Vec::new(),
        screen_w: width,
        screen_h: height,
    };
    let roots = b.node(&screen.root, 0, None, true);
    let mut items = Vec::new();
    if let Some(root) = roots.first() {
        let _ = b.tree.compute_layout_with_measure(
            *root,
            taffy::Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            },
            |known, available, _id, ctx, _style| match ctx {
                Some(Leaf::Fixed(w, h)) => taffy::Size {
                    width: known.width.unwrap_or(*w),
                    height: known.height.unwrap_or(*h),
                },
                Some(Leaf::Text {
                    text,
                    size,
                    weight,
                    pad,
                }) => {
                    let max = known.width.or(match available.width {
                        AvailableSpace::Definite(w) => Some(w),
                        _ => None,
                    });
                    let (w, h) = m.measure(text, *size, *weight, max.map(|x| (x - pad.0).max(1.0)));
                    taffy::Size {
                        width: known.width.unwrap_or(w + pad.0),
                        height: known.height.unwrap_or(h + pad.1),
                    }
                }
                None => taffy::Size::ZERO,
            },
        );
        // Absolute positions: walk down from the root.
        let index: BTreeMap<u64, usize> = b
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (u64::from(n.0), i))
            .collect();
        fn place(
            b: &Builder,
            index: &BTreeMap<u64, usize>,
            id: NodeId,
            ox: f32,
            oy: f32,
            out: &mut Vec<Item>,
        ) {
            let l = b.tree.layout(id).unwrap();
            let (x, y) = (ox + l.location.x, oy + l.location.y);
            if let Some(i) = index.get(&u64::from(id)) {
                let mut item = b.nodes[*i].1.clone();
                item.rect = Rect {
                    x,
                    y,
                    w: l.size.width,
                    h: l.size.height,
                };
                out.push(item);
            }
            for c in b.tree.children(id).unwrap() {
                place(b, index, c, x, y, out);
            }
        }
        place(&b, &index, *root, 0.0, 0.0, &mut items);
    }
    let bg = screen.background.rgba;
    Frame {
        screen: screen.name.clone(),
        width,
        height,
        background: [bg.r, bg.g, bg.b, bg.a],
        items,
    }
}

/// Lay out every screen of the app.
pub fn layout_app(app: &App, width: f32, height: f32, m: &dyn Measure) -> Vec<Frame> {
    app.screens
        .iter()
        .map(|s| layout_screen(app, s, width, height, m))
        .collect()
}
