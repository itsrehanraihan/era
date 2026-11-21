//! Screen composables. Each IR element maps to Compose foundation code.
//!
//! Layout rules (docs/ANDROID_BACKEND.md):
//! - `fill` along a row/column main axis becomes `weight(1f)` (D-041); on
//!   the cross axis it becomes `fillMaxWidth/Height()`. Along an axis that is
//!   scrolling (unbounded) `fill` is ignored, because Compose cannot fill
//!   infinite space.
//! - Modifier order: size, shadow, clip, background, border, safe-area
//!   insets (screen root only), scroll, padding.

use std::fmt::Write as _;

use era_ir::ir::{App, Content, Element, ElementKind, Screen, Size, Value};
use era_ir::skin::TokenCategory;

use crate::GENERATED_HEADER;
use crate::kotlin::{color, dp, ident, num, string};

const IMPORTS: &str = "import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
";

pub fn registry(app: &App, package: &str) -> String {
    let mut o = format!("{GENERATED_HEADER}\npackage {package}\n\nimport {package}.screens.*\n\n");
    let _ = writeln!(o, "const val EraAppName: String = {}", string(&app.name));
    let _ = writeln!(
        o,
        "const val EraStartScreen: String = {}\n",
        string(&app.start)
    );
    o.push_str(
        "/** Every screen, in source order. */\nval EraScreens: List<EraScreen> = listOf(\n",
    );
    for s in &app.screens {
        let title = s
            .title
            .as_deref()
            .map(string)
            .unwrap_or_else(|| "null".into());
        let _ = writeln!(
            o,
            "    EraScreen({}, {title}) {{ {}Screen() }},",
            string(&s.name),
            s.name
        );
    }
    o.push_str(")\n");
    o
}

pub fn screen(s: &Screen, package: &str) -> String {
    let mut e = Emitter {
        out: String::new(),
        depth: 1,
    };
    let root_ctx = Ctx {
        parent: Parent::Box,
        v_unbounded: false,
        h_unbounded: false,
        stretch: false,
        root: true,
    };
    e.element(&s.root, root_ctx);
    let title = s
        .title
        .as_deref()
        .map(|t| format!(" ({t})"))
        .unwrap_or_default();
    format!(
        "{GENERATED_HEADER}
package {package}.screens

{IMPORTS}import {package}.*

/** Screen `{name}`{title}. */
@Composable
fun {name}Screen() {{
    val c = LocalEraColors.current
    Box(Modifier.fillMaxSize().background({bg})) {{
{body}    }}
}}
",
        name = s.name,
        bg = color_expr(&Value::Color(s.background.clone())).unwrap(),
        body = e.out,
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Parent {
    Column,
    Row,
    Box,
}

#[derive(Clone, Copy)]
struct Ctx {
    parent: Parent,
    /// The parent scrolls vertically/horizontally (infinite space on that axis).
    v_unbounded: bool,
    h_unbounded: bool,
    /// The parent's `align: stretch`: fill the cross axis by default.
    stretch: bool,
    /// The screen's root element: its background extends behind the system
    /// bars, and its content is padded clear of them (D-050).
    root: bool,
}

struct Emitter {
    out: String,
    depth: usize,
}

fn category(key: &str) -> TokenCategory {
    match key {
        "radius" => TokenCategory::Radius,
        "border_width" => TokenCategory::Border,
        "elevation" => TokenCategory::Elevation,
        _ => TokenCategory::Spacing,
    }
}

fn dim_expr(key: &str, v: &Value) -> Option<String> {
    match v {
        Value::Dimension(d) => Some(match &d.token {
            Some(t) => {
                let obj = match category(key) {
                    TokenCategory::Radius => "EraRadius",
                    TokenCategory::Border => "EraBorder",
                    TokenCategory::Elevation => "EraElevation",
                    _ => "EraSpacing",
                };
                format!("{obj}.{}", ident(t))
            }
            None => dp(d.dp),
        }),
        Value::Number(x) => Some(dp(*x)),
        Value::Size(Size::Dp(x)) => Some(dp(*x)),
        _ => None,
    }
}

fn dim_value(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Dimension(d)) => d.dp,
        Some(Value::Number(x)) => *x,
        _ => 0.0,
    }
}

/// `None` for missing or `none` (transparent) colors.
fn color_expr(v: &Value) -> Option<String> {
    match v {
        Value::Color(c) => Some(match &c.token {
            Some(t) => format!("c.{}", ident(t)),
            None => color(&c.rgba),
        }),
        _ => None,
    }
}

fn keyword(e: &Element, key: &str) -> Option<&'static str> {
    match e.props.get(key) {
        Some(Value::Keyword(k)) => Some(k),
        _ => None,
    }
}

impl Emitter {
    fn line(&mut self, s: &str) {
        let _ = writeln!(self.out, "{}{s}", "    ".repeat(self.depth + 1));
    }

    fn block(&mut self, head: &str, body: impl FnOnce(&mut Self)) {
        self.line(&format!("{head} {{"));
        self.depth += 1;
        body(self);
        self.depth -= 1;
        self.line("}");
    }

    fn shape(e: &Element) -> Option<String> {
        e.props
            .get("radius")
            .and_then(|v| dim_expr("radius", v))
            .map(|r| format!("RoundedCornerShape({r})"))
    }

    /// Size modifiers, honoring the parent's axis rules.
    fn size_mods(e: &Element, ctx: Ctx, m: &mut Vec<String>) {
        let width = e.props.get("width");
        let height = e.props.get("height");
        match width {
            Some(Value::Size(Size::Fill)) => match ctx.parent {
                Parent::Row if !ctx.h_unbounded => m.push("weight(1f)".into()),
                _ if ctx.h_unbounded => {}
                _ => m.push("fillMaxWidth()".into()),
            },
            Some(Value::Size(Size::Dp(x))) => m.push(format!("width({})", dp(*x))),
            Some(_) => {}
            None if ctx.stretch && ctx.parent == Parent::Column => m.push("fillMaxWidth()".into()),
            None => {}
        }
        match height {
            Some(Value::Size(Size::Fill)) => match ctx.parent {
                Parent::Column if !ctx.v_unbounded => m.push("weight(1f)".into()),
                _ if ctx.v_unbounded => {}
                _ => m.push("fillMaxHeight()".into()),
            },
            Some(Value::Size(Size::Dp(x))) => m.push(format!("height({})", dp(*x))),
            Some(_) => {}
            None if ctx.stretch && ctx.parent == Parent::Row => m.push("fillMaxHeight()".into()),
            None => {}
        }
        let get = |k: &str| e.props.get(k).and_then(|v| dim_expr(k, v));
        let (minw, maxw) = (get("min_width"), get("max_width"));
        if minw.is_some() || maxw.is_some() {
            let mut a = Vec::new();
            if let Some(v) = minw {
                a.push(format!("min = {v}"))
            }
            if let Some(v) = maxw {
                a.push(format!("max = {v}"))
            }
            m.push(format!("widthIn({})", a.join(", ")));
        }
        let (minh, maxh) = (get("min_height"), get("max_height"));
        if minh.is_some() || maxh.is_some() {
            let mut a = Vec::new();
            if let Some(v) = minh {
                a.push(format!("min = {v}"))
            }
            if let Some(v) = maxh {
                a.push(format!("max = {v}"))
            }
            m.push(format!("heightIn({})", a.join(", ")));
        }
    }

    /// The full modifier chain for an element.
    fn modifier(e: &Element, ctx: Ctx, extra_size: Option<String>, scroll: Option<&str>) -> String {
        let mut m = Vec::new();
        if let Some(s) = extra_size {
            m.push(s);
        }
        Self::size_mods(e, ctx, &mut m);
        let shape = Self::shape(e);
        let with_shape = |base: String| match &shape {
            Some(s) => format!("{base}, {s}"),
            None => base,
        };
        if dim_value(e.props.get("elevation")) > 0.0 {
            let el = dim_expr("elevation", &e.props["elevation"]).unwrap();
            m.push(format!("shadow({})", with_shape(el)));
        }
        if matches!(e.props.get("clip"), Some(Value::Bool(true))) {
            m.push(format!(
                "clip({})",
                shape
                    .clone()
                    .unwrap_or_else(|| "androidx.compose.ui.graphics.RectangleShape".into())
            ));
        }
        // `button`, `input`, `card` and containers draw their own background.
        if let Some(bg) = e.props.get("background").and_then(color_expr) {
            m.push(format!("background({})", with_shape(bg)));
        }
        if dim_value(e.props.get("border_width")) > 0.0 {
            let w = dim_expr("border_width", &e.props["border_width"]).unwrap();
            let col = e
                .props
                .get("border_color")
                .and_then(color_expr)
                .unwrap_or_else(|| "c.border".into());
            m.push(format!("border({w}, {})", with_shape(col)));
        }
        if ctx.root {
            m.push("safeDrawingPadding()".into());
        }
        if let Some(s) = scroll {
            m.push(s.to_string());
        }
        if let Some(p) = Self::padding(e) {
            m.push(p);
        }
        if m.is_empty() {
            "Modifier".into()
        } else {
            format!("Modifier.{}", m.join("."))
        }
    }

    /// `padding` < `padding_x`/`padding_y` < `padding_top`/... (most specific wins).
    fn padding(e: &Element) -> Option<String> {
        let get = |k: &str| e.props.get(k).and_then(|v| dim_expr(k, v));
        let all = get("padding");
        let x = get("padding_x").or_else(|| all.clone());
        let y = get("padding_y").or_else(|| all.clone());
        let start = get("padding_start").or_else(|| x.clone());
        let end = get("padding_end").or_else(|| x.clone());
        let top = get("padding_top").or_else(|| y.clone());
        let bottom = get("padding_bottom").or_else(|| y.clone());
        let sides = [&start, &top, &end, &bottom];
        if sides.iter().all(|s| s.is_none()) {
            return None;
        }
        if sides.iter().all(|s| *s == &start) {
            return Some(format!("padding({})", start.unwrap()));
        }
        if start == end && top == bottom {
            let mut a = Vec::new();
            if let Some(h) = &start {
                a.push(format!("horizontal = {h}"))
            }
            if let Some(v) = &top {
                a.push(format!("vertical = {v}"))
            }
            return Some(format!("padding({})", a.join(", ")));
        }
        let mut a = Vec::new();
        for (name, v) in [
            ("start", &start),
            ("top", &top),
            ("end", &end),
            ("bottom", &bottom),
        ] {
            if let Some(v) = v {
                a.push(format!("{name} = {v}"));
            }
        }
        Some(format!("padding({})", a.join(", ")))
    }

    fn arrangement(e: &Element, horizontal: bool) -> Option<String> {
        let gap = e
            .props
            .get("gap")
            .filter(|v| dim_value(Some(v)) > 0.0)
            .and_then(|v| dim_expr("gap", v));
        let (start, center, end) = if horizontal {
            (
                "Alignment.Start",
                "Alignment.CenterHorizontally",
                "Alignment.End",
            )
        } else {
            (
                "Alignment.Top",
                "Alignment.CenterVertically",
                "Alignment.Bottom",
            )
        };
        let dist = keyword(e, "distribute").unwrap_or("start");
        Some(match (dist, gap) {
            ("between", _) => "Arrangement.SpaceBetween".into(),
            ("around", _) => "Arrangement.SpaceAround".into(),
            ("evenly", _) => "Arrangement.SpaceEvenly".into(),
            ("start", None) => return None,
            ("start", Some(g)) => format!("Arrangement.spacedBy({g}, {start})"),
            ("center", Some(g)) => format!("Arrangement.spacedBy({g}, {center})"),
            ("end", Some(g)) => format!("Arrangement.spacedBy({g}, {end})"),
            ("center", None) => "Arrangement.Center".into(),
            (_, None) => format!("Arrangement.{}", if horizontal { "End" } else { "Bottom" }),
            (_, Some(g)) => format!("Arrangement.spacedBy({g})"),
        })
    }

    fn element(&mut self, e: &Element, ctx: Ctx) {
        let origin = if e.origin.is_empty() {
            String::new()
        } else {
            format!(" <{}>", e.origin.join(">"))
        };
        self.line(&format!("// {} @{}{origin}", e.kind.name(), e.id));
        match e.kind {
            ElementKind::Column | ElementKind::Card => self.linear(e, ctx, false),
            ElementKind::Row => self.linear(e, ctx, true),
            ElementKind::Stack => {
                let align = match keyword(e, "align").unwrap_or("top_start") {
                    "top" => "TopCenter",
                    "top_end" => "TopEnd",
                    "start" => "CenterStart",
                    "center" => "Center",
                    "end" => "CenterEnd",
                    "bottom_start" => "BottomStart",
                    "bottom" => "BottomCenter",
                    "bottom_end" => "BottomEnd",
                    _ => "TopStart",
                };
                let m = Self::modifier(e, ctx, None, None);
                let child = Ctx {
                    parent: Parent::Box,
                    stretch: false,
                    root: false,
                    ..ctx
                };
                self.block(
                    &format!("Box({m}, contentAlignment = Alignment.{align})"),
                    |s| {
                        for c in &e.children {
                            s.element(c, child);
                        }
                    },
                );
            }
            ElementKind::Scroll => {
                let vertical = keyword(e, "direction") != Some("horizontal");
                let scroll = if vertical {
                    "verticalScroll(rememberScrollState())"
                } else {
                    "horizontalScroll(rememberScrollState())"
                };
                let m = Self::modifier(e, ctx, None, Some(scroll));
                let child = Ctx {
                    parent: Parent::Box,
                    v_unbounded: vertical,
                    h_unbounded: !vertical,
                    stretch: false,
                    root: false,
                };
                self.block(&format!("Box({m})"), |s| {
                    for c in &e.children {
                        s.element(c, child);
                    }
                });
            }
            ElementKind::Grid => self.grid(e, ctx),
            ElementKind::Text => {
                let Some(Content::Text(t)) = &e.content else {
                    return;
                };
                let m = Self::modifier(e, ctx, None, None);
                let mut args = vec![
                    string(t),
                    format!("modifier = {m}"),
                    format!("style = {}", Self::text_style(e, "color")),
                ];
                if let Some(Value::Number(n)) = e.props.get("max_lines") {
                    args.push(format!("maxLines = {}", num(*n)));
                    args.push("overflow = TextOverflow.Ellipsis".into());
                }
                self.line(&format!("BasicText({})", args.join(", ")));
            }
            ElementKind::Button => {
                let label = match &e.content {
                    Some(Content::Text(t)) => t.clone(),
                    _ => String::new(),
                };
                let m = Self::modifier(e, ctx, None, None);
                let style = Self::text_style(e, "text_color");
                self.block(
                    &format!("Box({m}, contentAlignment = Alignment.Center)"),
                    |s| {
                        s.line(&format!("BasicText({}, style = {style})", string(&label)));
                    },
                );
            }
            ElementKind::Input => {
                let m = Self::modifier(e, ctx, None, None);
                let style = Self::text_style(e, "text_color");
                let placeholder = match e.props.get("placeholder") {
                    Some(Value::Text(t)) => t.clone(),
                    _ => String::new(),
                };
                let field = format!("EraInput({}, {style}, c.muted, {m})", string(&placeholder));
                match e.props.get("label") {
                    Some(Value::Text(label)) => self.block(
                        "Column(verticalArrangement = Arrangement.spacedBy(4.dp))",
                        |s| {
                            s.line(&format!(
                                "BasicText({}, style = EraType.body.copy(color = c.muted))",
                                string(label)
                            ));
                            s.line(&field);
                        },
                    ),
                    _ => self.line(&field),
                }
            }
            ElementKind::Image | ElementKind::Icon => {
                let Some(Content::Asset(a)) = &e.content else {
                    return;
                };
                let icon = e.kind == ElementKind::Icon;
                let has_size = e.props.contains_key("width") || e.props.contains_key("height");
                let extra = if icon && !has_size {
                    e.props
                        .get("size")
                        .and_then(|v| dim_expr("size", v))
                        .map(|s| format!("size({s})"))
                } else {
                    None
                };
                let m = Self::modifier(e, ctx, extra, None);
                let mut args = vec![
                    format!(
                        "painter = {}({})",
                        if icon { "eraIcon" } else { "eraImage" },
                        string(&a.path)
                    ),
                    "contentDescription = null".into(),
                    format!("modifier = {m}"),
                ];
                if icon {
                    if let Some(col) = e.props.get("color").and_then(color_expr) {
                        args.push(format!("colorFilter = ColorFilter.tint({col})"));
                    }
                } else {
                    let scale = match keyword(e, "fit") {
                        Some("contain") => "Fit",
                        Some("fill") => "FillBounds",
                        _ => "Crop",
                    };
                    args.push(format!("contentScale = ContentScale.{scale}"));
                }
                self.line(&format!("Image({})", args.join(", ")));
            }
            ElementKind::Divider => {
                let col = e
                    .props
                    .get("color")
                    .and_then(color_expr)
                    .unwrap_or_else(|| "c.border".into());
                let t = e
                    .props
                    .get("thickness")
                    .and_then(|v| dim_expr("thickness", v))
                    .unwrap_or_else(|| "1.dp".into());
                let base = Self::modifier(e, ctx, None, None);
                self.line(&format!(
                    "Box({base}.fillMaxWidth().height({t}).background({col}))"
                ));
            }
            ElementKind::Spacer => {
                let m = match e.props.get("size").and_then(|v| dim_expr("size", v)) {
                    Some(s) => format!("Modifier.size({s})"),
                    None => match ctx.parent {
                        Parent::Row if !ctx.h_unbounded => "Modifier.weight(1f)".into(),
                        Parent::Column if !ctx.v_unbounded => "Modifier.weight(1f)".into(),
                        _ => "Modifier".into(),
                    },
                };
                self.line(&format!("Spacer({m})"));
            }
        }
    }

    fn text_style(e: &Element, color_key: &str) -> String {
        let base = match e.props.get("style") {
            Some(Value::TextStyle(s)) => format!("EraType.{}", ident(s)),
            _ => "EraType.body".into(),
        };
        let mut copy = Vec::new();
        if let Some(col) = e.props.get(color_key).and_then(color_expr) {
            copy.push(format!("color = {col}"));
        }
        if let Some(a) = keyword(e, "align").filter(|_| e.kind == ElementKind::Text) {
            let a = match a {
                "center" => "Center",
                "end" => "End",
                _ => "Start",
            };
            copy.push(format!("textAlign = TextAlign.{a}"));
        }
        if copy.is_empty() {
            base
        } else {
            format!("{base}.copy({})", copy.join(", "))
        }
    }

    fn linear(&mut self, e: &Element, ctx: Ctx, row: bool) {
        let m = Self::modifier(e, ctx, None, None);
        let align_kw = keyword(e, "align");
        let mut args = vec![m];
        if let Some(a) = Self::arrangement(e, row) {
            args.push(format!(
                "{} = {a}",
                if row {
                    "horizontalArrangement"
                } else {
                    "verticalArrangement"
                }
            ));
        }
        if let Some(a) = align_kw.filter(|a| *a != "start" && *a != "stretch") {
            let a = match (row, a) {
                (false, "center") => "CenterHorizontally",
                (false, _) => "End",
                (true, "center") => "CenterVertically",
                (true, _) => "Bottom",
            };
            args.push(format!(
                "{} = Alignment.{a}",
                if row {
                    "verticalAlignment"
                } else {
                    "horizontalAlignment"
                }
            ));
        }
        let child = Ctx {
            parent: if row { Parent::Row } else { Parent::Column },
            stretch: align_kw == Some("stretch"),
            root: false,
            ..ctx
        };
        let head = format!(
            "{}({})",
            if row { "Row" } else { "Column" },
            args.join(", ")
        );
        self.block(&head, |s| {
            for c in &e.children {
                s.element(c, child);
            }
        });
    }

    fn grid(&mut self, e: &Element, ctx: Ctx) {
        let cols = match e.props.get("columns") {
            Some(Value::Number(n)) => (*n as usize).max(1),
            _ => 2,
        };
        let gap = e
            .props
            .get("gap")
            .and_then(|v| dim_expr("gap", v))
            .unwrap_or_else(|| "0.dp".into());
        let m = Self::modifier(e, ctx, None, None);
        let cell = Ctx {
            parent: Parent::Box,
            stretch: false,
            root: false,
            ..ctx
        };
        self.block(&format!("Column({m}, verticalArrangement = Arrangement.spacedBy({gap}))"), |s| {
            for chunk in e.children.chunks(cols) {
                s.block(&format!("Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy({gap}))"), |s| {
                    for c in chunk {
                        s.block("Box(Modifier.weight(1f))", |s| s.element(c, cell));
                    }
                    for _ in chunk.len()..cols {
                        s.line("Spacer(Modifier.weight(1f))");
                    }
                });
            }
        });
    }
}
