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

use era_ir::ir::{
    App, Content, Effect, Element, ElementKind, GradientKind, Screen, Size, StateRef, StateType,
    TextPart, Value,
};
use era_ir::skin::TokenCategory;

use crate::GENERATED_HEADER;
use crate::kotlin::{color, dp, ident, num, string};

const IMPORTS: &str = "import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CornerSize
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
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.BlurredEdgeTreatment
import androidx.compose.ui.zIndex
import androidx.compose.runtime.CompositionLocalProvider
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

pub fn screen(s: &Screen, package: &str, export: bool) -> String {
    let mut e = Emitter {
        out: String::new(),
        depth: 2,
        fixed: Vec::new(),
        ids: !export,
    };
    let root_ctx = Ctx {
        parent: Parent::Box,
        v_unbounded: false,
        h_unbounded: false,
        stretch: false,
        root: true,
        overlay: false,
    };
    e.element(&s.root, root_ctx);
    // `position: fixed` elements are pinned to the screen, above everything.
    let fixed = std::mem::take(&mut e.fixed);
    if !fixed.is_empty() {
        let overlay = Ctx {
            root: false,
            overlay: true,
            ..root_ctx
        };
        e.block("Box(Modifier.matchParentSize())", |s| {
            for f in &fixed {
                s.element(f, overlay);
            }
        });
    }
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
@OptIn(ExperimentalLayoutApi::class)
fun {name}Screen() {{
    val c = LocalEraColors.current
    BoxWithConstraints(Modifier.fillMaxSize().background({bg})) {{
        CompositionLocalProvider(LocalEraScreenSize provides DpSize(maxWidth, maxHeight)) {{
{body}        }}
    }}
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
    /// A child of a positioning overlay (`position: absolute/fixed`).
    overlay: bool,
}

struct Emitter {
    out: String,
    depth: usize,
    /// `position: fixed` elements, emitted at the end of the screen.
    fixed: Vec<Element>,
    /// Include IR ids in element comments (not in exported code).
    ids: bool,
}

fn category(key: &str) -> TokenCategory {
    match key {
        _ if key.starts_with("radius") => TokenCategory::Radius,
        _ if key.ends_with("_width")
            && (key.starts_with("border") || key.starts_with("outline")) =>
        {
            TokenCategory::Border
        }
        "elevation" => TokenCategory::Elevation,
        _ => TokenCategory::Spacing,
    }
}

const SIDES: [&str; 4] = ["top", "end", "bottom", "start"];
const CORNERS: [&str; 4] = [
    "radius_top_start",
    "radius_top_end",
    "radius_bottom_end",
    "radius_bottom_start",
];

/// A Compose `CornerSize` for one corner radius (`12.dp` or `50%`).
fn corner_size(key: &str, v: &Value) -> Option<String> {
    match v {
        Value::Size(Size::Percent(p)) => Some(format!("CornerSize({})", p.round() as i64)),
        _ => dim_expr(key, v).map(|d| format!("CornerSize({d})")),
    }
}

/// A Kotlin `Float` literal, rounded to 6 decimals so generated code is
/// short and stable: `-0.574f`, `255f`.
fn float_lit(v: f64) -> String {
    let r = (v * 1e6).round() / 1e6;
    if r == 0.0 {
        "0f".into()
    } else if r.fract() == 0.0 {
        format!("{}f", r as i64)
    } else {
        format!("{r}f")
    }
}

/// A colour or a gradient brush for `background`.
fn paint_expr(v: &Value) -> Option<String> {
    let Value::Gradient(g) = v else {
        return color_expr(v);
    };
    let colors: Vec<String> = g
        .stops
        .iter()
        .map(|s| color_expr(&Value::Color(s.color.clone())).unwrap())
        .collect();
    let stops: Vec<String> = g
        .stops
        .iter()
        .map(|s| format!("{}f", num(s.position)))
        .collect();
    let lists = format!(
        "listOf({}), listOf({})",
        colors.join(", "),
        stops.join(", ")
    );
    Some(match &g.kind {
        GradientKind::Linear(a) => format!("eraLinearGradient({}f, {lists})", num(*a)),
        GradientKind::LinearToCorner { right, bottom } => {
            format!("eraCornerGradient({right}, {bottom}, {lists})")
        }
        GradientKind::Radial => format!("eraRadialGradient({lists})"),
    })
}

/// `EraLine.*` for a `border-style`, or `None` for `none`.
fn line_kind(v: Option<&Value>) -> Option<&'static str> {
    match v {
        Some(Value::Keyword("none")) => None,
        Some(Value::Keyword("dashed")) => Some("EraLine.Dashed"),
        Some(Value::Keyword("dotted")) => Some("EraLine.Dotted"),
        _ => Some("EraLine.Solid"),
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
        let uniform = e.props.get("radius");
        if CORNERS.iter().all(|c| !e.props.contains_key(c)) {
            return match uniform? {
                Value::Size(Size::Percent(p)) => {
                    Some(format!("RoundedCornerShape({})", p.round() as i64))
                }
                v => dim_expr("radius", v).map(|r| format!("RoundedCornerShape({r})")),
            };
        }
        let parts: Vec<String> = CORNERS
            .iter()
            .zip(["topStart", "topEnd", "bottomEnd", "bottomStart"])
            .map(|(c, name)| {
                let size = e
                    .props
                    .get(c)
                    .or(uniform)
                    .and_then(|v| corner_size(c, v))
                    .unwrap_or_else(|| "CornerSize(0.dp)".into());
                format!("{name} = {size}")
            })
            .collect();
        Some(format!("RoundedCornerShape({})", parts.join(", ")))
    }

    /// CSS borders: Compose's `border` for solid all-round borders,
    /// `eraBorder` for dashed/dotted, `eraSideBorders` for per-side ones.
    fn borders(e: &Element, shape: &Option<String>, m: &mut Vec<String>) {
        let rect = || "androidx.compose.ui.graphics.RectangleShape".to_string();
        let side_key = |side: &str, part: &str| format!("border_{side}_{part}");
        let per_side = SIDES.iter().any(|s| {
            ["width", "color", "style"]
                .iter()
                .any(|p| e.props.contains_key(side_key(s, p).as_str()))
        });
        let color_of =
            |v: Option<&Value>| v.and_then(color_expr).unwrap_or_else(|| "c.border".into());
        if !per_side {
            let Some(line) = line_kind(e.props.get("border_style")) else {
                return;
            };
            if dim_value(e.props.get("border_width")) <= 0.0 {
                return;
            }
            let w = dim_expr("border_width", &e.props["border_width"]).unwrap();
            let col = color_of(e.props.get("border_color"));
            if line == "EraLine.Solid" {
                m.push(match shape {
                    Some(sh) => format!("border({w}, {col}, {sh})"),
                    None => format!("border({w}, {col})"),
                });
            } else {
                let sh = shape.clone().unwrap_or_else(rect);
                m.push(format!("eraBorder({w}, {col}, {sh}, {line})"));
            }
            return;
        }
        let args: Vec<String> = SIDES
            .iter()
            .map(|side| {
                let wk = side_key(side, "width");
                let (wkey, wv) = match e.props.get(wk.as_str()) {
                    Some(v) => (wk.as_str(), Some(v)),
                    None => ("border_width", e.props.get("border_width")),
                };
                let style = e
                    .props
                    .get(side_key(side, "style").as_str())
                    .or(e.props.get("border_style"));
                let value = match (wv, line_kind(style)) {
                    (Some(v), Some(line)) if dim_value(Some(v)) > 0.0 => {
                        let w = dim_expr(wkey, v).unwrap();
                        let col = color_of(
                            e.props
                                .get(side_key(side, "color").as_str())
                                .or(e.props.get("border_color")),
                        );
                        format!("EraSide({w}, {col}, {line})")
                    }
                    _ => "null".into(),
                };
                format!("{side} = {value}")
            })
            .collect();
        m.push(format!("eraSideBorders({})", args.join(", ")));
    }

    /// CSS `box-shadow`: outer shadows behind the element, inset shadows
    /// over its background.
    fn box_shadows(e: &Element, shape: &Option<String>, inset: bool, m: &mut Vec<String>) {
        let Some(Value::Shadows(list)) = e.props.get("box_shadow") else {
            return;
        };
        let items: Vec<String> = list
            .iter()
            .filter(|s| s.inset == inset)
            .map(|s| {
                format!(
                    "EraShadow({}, {}, {}, {}, {})",
                    dp(s.x),
                    dp(s.y),
                    dp(s.blur),
                    dp(s.spread),
                    color_expr(&Value::Color(s.color.clone())).unwrap()
                )
            })
            .collect();
        if items.is_empty() {
            return;
        }
        let sh = shape
            .clone()
            .unwrap_or_else(|| "androidx.compose.ui.graphics.RectangleShape".into());
        let f = if inset {
            "eraInsetShadow"
        } else {
            "eraBoxShadow"
        };
        m.push(format!("{f}(listOf({}), {sh})", items.join(", ")));
    }

    /// CSS `filter`: `blur()` uses Compose `blur`; colour filters become one
    /// colour matrix.
    fn filters(e: &Element, m: &mut Vec<String>) {
        let Some(Value::Filters(list)) = e.props.get("filter") else {
            return;
        };
        let blur: f64 = list
            .iter()
            .filter(|f| f.name == "blur")
            .map(|f| f.amount)
            .sum();
        if blur > 0.0 {
            m.push(format!(
                "blur({}, BlurredEdgeTreatment.Unbounded)",
                dp(blur)
            ));
        }
        let colour: Vec<(&str, f64)> = list.iter().map(|f| (f.name, f.amount)).collect();
        if let Some(matrix) = crate::draw::color_matrix(&colour) {
            let values: Vec<String> = matrix.iter().map(|v| float_lit(*v)).collect();
            m.push(format!(
                "eraColorFilter(floatArrayOf({}))",
                values.join(", ")
            ));
        }
    }

    /// CSS `outline` (+ `outline-offset`): drawn outside, takes no space.
    fn outline(e: &Element, shape: &Option<String>, m: &mut Vec<String>) {
        let Some(line) = line_kind(e.props.get("outline_style")) else {
            return;
        };
        if dim_value(e.props.get("outline_width")) <= 0.0 {
            return;
        }
        let w = dim_expr("outline_width", &e.props["outline_width"]).unwrap();
        let col = e
            .props
            .get("outline_color")
            .and_then(color_expr)
            .unwrap_or_else(|| "c.text".into());
        let offset = match e.props.get("outline_offset") {
            Some(Value::Size(Size::Dp(x))) => dp(*x),
            _ => "0.dp".into(),
        };
        let sh = shape
            .clone()
            .unwrap_or_else(|| "androidx.compose.ui.graphics.RectangleShape".into());
        m.push(format!("eraOutline({w}, {col}, {sh}, {line}, {offset})"));
    }

    /// Size modifiers, honoring the parent's axis rules.
    /// One axis of `width`/`height` (D-041, M8 sizing modes).
    fn axis(size: Option<&Value>, horizontal: bool, ctx: Ctx, m: &mut Vec<String>) {
        let (fill, fixed) = if horizontal {
            ("fillMaxWidth", "width")
        } else {
            ("fillMaxHeight", "height")
        };
        let intrinsic = fixed;
        let main = if horizontal {
            ctx.parent == Parent::Row
        } else {
            ctx.parent == Parent::Column
        };
        let unbounded = if horizontal {
            ctx.h_unbounded
        } else {
            ctx.v_unbounded
        };
        let stretch_here = ctx.stretch
            && ctx.parent
                == if horizontal {
                    Parent::Column
                } else {
                    Parent::Row
                };
        match size {
            Some(Value::Size(Size::Fill)) if unbounded => {}
            Some(Value::Size(Size::Fill)) if main => m.push("weight(1f)".into()),
            Some(Value::Size(Size::Fill)) => m.push(format!("{fill}()")),
            Some(Value::Size(Size::Fr(_))) if unbounded => {}
            Some(Value::Size(Size::Fr(n))) if main => m.push(format!("weight({}f)", num(*n))),
            Some(Value::Size(Size::Fr(_))) => m.push(format!("{fill}()")),
            Some(Value::Size(Size::Percent(_))) if unbounded => {}
            Some(Value::Size(Size::Percent(p))) => m.push(format!("{fill}({}f)", num(p / 100.0))),
            Some(Value::Size(Size::Dp(x))) => m.push(format!("{fixed}({})", dp(*x))),
            Some(Value::Size(Size::Viewport { vertical, percent })) => {
                let f = if *vertical { "eraVh" } else { "eraVw" };
                m.push(format!("{fixed}({f}({}f))", num(*percent)));
            }
            Some(Value::Size(Size::MinContent)) => {
                m.push(format!("{intrinsic}(IntrinsicSize.Min)"))
            }
            Some(Value::Size(Size::MaxContent)) => {
                m.push(format!("{intrinsic}(IntrinsicSize.Max)"))
            }
            Some(_) => {}
            None if stretch_here => m.push(format!("{fill}()")),
            None => {}
        }
    }

    /// Size modifiers, honoring the parent's axis rules.
    fn size_mods(e: &Element, ctx: Ctx, m: &mut Vec<String>) {
        Self::axis(e.props.get("width"), true, ctx, m);
        Self::axis(e.props.get("height"), false, ctx, m);
        // `flex: 2` along the parent's main axis.
        if let Some(Value::Number(n)) = e.props.get("flex") {
            let (main_set, unbounded) = match ctx.parent {
                Parent::Row => (e.props.contains_key("width"), ctx.h_unbounded),
                Parent::Column => (e.props.contains_key("height"), ctx.v_unbounded),
                Parent::Box => (true, true),
            };
            if !main_set && !unbounded && *n > 0.0 {
                m.push(format!("weight({}f)", num(*n)));
            }
        }
        // `align-self`.
        if let Some(Value::Keyword(a)) = e.props.get("align_self") {
            match (ctx.parent, *a) {
                (Parent::Column, "stretch") => m.push("fillMaxWidth()".into()),
                (Parent::Row, "stretch") => m.push("fillMaxHeight()".into()),
                (Parent::Column, "start") => m.push("align(Alignment.Start)".into()),
                (Parent::Column, "center") => m.push("align(Alignment.CenterHorizontally)".into()),
                (Parent::Column, "end") => m.push("align(Alignment.End)".into()),
                (Parent::Row, "start") => m.push("align(Alignment.Top)".into()),
                (Parent::Row, "center") => m.push("align(Alignment.CenterVertically)".into()),
                (Parent::Row, "end") => m.push("align(Alignment.Bottom)".into()),
                _ => {}
            }
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
        if ctx.overlay {
            Self::placement(e, &mut m);
        }
        // `margin` is space outside the element: padding before everything else.
        if let Some(p) = Self::sides(e, "margin") {
            m.push(p);
        }
        if !ctx.overlay {
            Self::auto_margins(e, ctx, &mut m);
        }
        if keyword(e, "position") == Some("relative") {
            let off = |k: &str| match e.props.get(k) {
                Some(Value::Size(Size::Dp(x))) => Some(*x),
                _ => None,
            };
            let x = off("left").or(off("right").map(|r| -r)).unwrap_or(0.0);
            let y = off("top").or(off("bottom").map(|b| -b)).unwrap_or(0.0);
            if x != 0.0 || y != 0.0 {
                m.push(format!("offset(x = {}, y = {})", dp(x), dp(y)));
            }
        }
        if let Some(s) = extra_size {
            m.push(s);
        }
        Self::size_mods(e, ctx, &mut m);
        if let Some(Value::Number(r)) = e.props.get("aspect_ratio") {
            m.push(format!("aspectRatio({}f)", num(*r)));
        }
        if let Some(Value::Number(z)) = e.props.get("z_index") {
            m.push(format!("zIndex({}f)", num(*z)));
        }
        if keyword(e, "visibility") == Some("hidden") {
            m.push("alpha(0f)".into());
        } else if let Some(Value::Number(o)) = e.props.get("opacity") {
            m.push(format!("alpha({}f)", num(*o)));
        }
        let shape = Self::shape(e);
        let with_shape = |base: String| match &shape {
            Some(s) => format!("{base}, {s}"),
            None => base,
        };
        // CSS `filter` applies to the whole element, shadows included.
        Self::filters(e, &mut m);
        // Before the clip, so `overflow: hidden` never clips the outline.
        Self::outline(e, &shape, &mut m);
        Self::box_shadows(e, &shape, false, &mut m);
        if dim_value(e.props.get("elevation")) > 0.0 {
            let el = dim_expr("elevation", &e.props["elevation"]).unwrap();
            // CSS box-shadow never clips content; only `overflow: hidden` does.
            m.push(format!("shadow({}, clip = false)", with_shape(el)));
        }
        let overflow_clip = matches!(keyword(e, "overflow"), Some("hidden" | "clip"));
        if overflow_clip || matches!(e.props.get("clip"), Some(Value::Bool(true))) {
            m.push(format!(
                "clip({})",
                shape
                    .clone()
                    .unwrap_or_else(|| "androidx.compose.ui.graphics.RectangleShape".into())
            ));
        }
        // `button`, `input`, `card` and containers draw their own background.
        // CSS layers are listed top first; Compose draws earlier modifiers first.
        let layers: Vec<&Value> = match e.props.get("background") {
            Some(Value::Layers(l)) => l.iter().rev().collect(),
            Some(v) => vec![v],
            None => vec![],
        };
        for layer in layers {
            if let Some(bg) = paint_expr(layer) {
                m.push(format!("background({})", with_shape(bg)));
            }
        }
        Self::box_shadows(e, &shape, true, &mut m);
        Self::borders(e, &shape, &mut m);
        if let Some(Value::Effects(effects)) = e.props.get("on_press")
            && !effects.is_empty()
        {
            m.push(format!("clickable {{ {} }}", effects_code(effects)));
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

    /// `position: absolute/fixed` inside a positioning overlay: align to the
    /// nearest edges, then offset; both left and right (or top and bottom)
    /// stretch between them.
    fn placement(e: &Element, m: &mut Vec<String>) {
        let off = |k: &str| match e.props.get(k) {
            Some(Value::Size(Size::Dp(x))) => Some(*x),
            _ => None,
        };
        let (t, r, b, l) = (off("top"), off("right"), off("bottom"), off("left"));
        let (h, dx, fill_w) = match (l, r) {
            (Some(l), Some(r)) => ("Start", 0.0, Some((l, r))),
            (None, Some(r)) => ("End", -r, None),
            (l, None) => ("Start", l.unwrap_or(0.0), None),
        };
        let (v, dy, fill_h) = match (t, b) {
            (Some(t), Some(b)) => ("Top", 0.0, Some((t, b))),
            (None, Some(b)) => ("Bottom", -b, None),
            (t, None) => ("Top", t.unwrap_or(0.0), None),
        };
        m.push(format!("align(Alignment.{v}{h})"));
        // Out of flow: not squeezed by the parent (CSS absolute positioning).
        m.push(format!(
            "wrapContentSize(Alignment.{v}{h}, unbounded = true)"
        ));
        if dx != 0.0 || dy != 0.0 {
            m.push(format!("offset(x = {}, y = {})", dp(dx), dp(dy)));
        }
        if let Some((l, r)) = fill_w {
            m.push(format!(
                "fillMaxWidth().padding(start = {}, end = {})",
                dp(l),
                dp(r)
            ));
        }
        if let Some((t, b)) = fill_h {
            m.push(format!(
                "fillMaxHeight().padding(top = {}, bottom = {})",
                dp(t),
                dp(b)
            ));
        }
    }

    /// `margin: 0 auto` centres in a column; `margin-left: auto` pushes to the end.
    fn auto_margins(e: &Element, ctx: Ctx, m: &mut Vec<String>) {
        let auto = |k: &str| e.props.get(k) == Some(&Value::Keyword("auto"));
        let all = auto("margin");
        let x = auto("margin_x") || all;
        let y = auto("margin_y") || all;
        let (start, end) = (auto("margin_start") || x, auto("margin_end") || x);
        let (top, bottom) = (auto("margin_top") || y, auto("margin_bottom") || y);
        let a = match ctx.parent {
            Parent::Column => match (start, end) {
                (true, true) => "CenterHorizontally",
                (true, false) => "End",
                (false, true) => "Start",
                _ => return,
            },
            Parent::Row => match (top, bottom) {
                (true, true) => "CenterVertically",
                (true, false) => "Bottom",
                (false, true) => "Top",
                _ => return,
            },
            Parent::Box => {
                let v = match (top, bottom) {
                    (true, true) => "Center",
                    (true, false) => "Bottom",
                    _ => "Top",
                };
                let h = match (start, end) {
                    (true, true) => "Center",
                    (true, false) => "End",
                    _ => "Start",
                };
                if !(start || end || top || bottom) {
                    return;
                }
                let name = match (v, h) {
                    ("Center", "Center") => "Center".to_string(),
                    ("Center", h) => format!("Center{h}"),
                    (v, "Center") => format!("{v}Center"),
                    (v, h) => format!("{v}{h}"),
                };
                m.push(format!("align(Alignment.{name})"));
                return;
            }
        };
        m.push(format!("align(Alignment.{a})"));
    }

    /// `padding` < `padding_x`/`padding_y` < `padding_top`/... (most specific wins).
    fn padding(e: &Element) -> Option<String> {
        Self::sides(e, "padding")
    }

    /// The same rule for `padding` and `margin` (`auto` margins are skipped).
    fn sides(e: &Element, prefix: &str) -> Option<String> {
        let get = |k: &str| {
            let key = if k.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}_{k}")
            };
            e.props
                .get(key.as_str())
                .and_then(|v| dim_expr("padding", v))
        };
        let all = get("");
        let x = get("x").or_else(|| all.clone());
        let y = get("y").or_else(|| all.clone());
        let start = get("start").or_else(|| x.clone());
        let end = get("end").or_else(|| x.clone());
        let top = get("top").or_else(|| y.clone());
        let bottom = get("bottom").or_else(|| y.clone());
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
        match e.props.get("visible") {
            Some(Value::Bool(false)) => {
                let id = if self.ids {
                    format!(" @{}", e.id)
                } else {
                    String::new()
                };
                self.line(&format!(
                    "// {}{id} is hidden (visible: false)",
                    e.kind.name()
                ));
            }
            Some(Value::State(r)) => {
                let cond = state_expr(r);
                self.block(&format!("if ({cond})"), |s| s.element_inner(e, ctx));
            }
            _ => self.element_inner(e, ctx),
        }
    }

    fn element_inner(&mut self, e: &Element, ctx: Ctx) {
        let origin = if e.origin.is_empty() {
            String::new()
        } else {
            format!(" <{}>", e.origin.join(">"))
        };
        let id = if self.ids {
            format!(" @{}", e.id)
        } else {
            String::new()
        };
        let origin = if !self.ids && !e.origin.is_empty() {
            format!(" (from {})", e.origin.join(" > "))
        } else {
            origin
        };
        self.line(&format!("// {}{id}{origin}", e.kind.name()));
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
                    overlay: false,
                    ..ctx
                };
                let (flow, absolute) = self.partition(&e.children);
                if absolute.is_empty() || Self::padding(e).is_none() {
                    self.block(
                        &format!("Box({m}, contentAlignment = Alignment.{align})"),
                        |s| {
                            for c in &flow {
                                s.element(c, child);
                            }
                            s.overlay(&absolute, ctx);
                        },
                    );
                } else {
                    // Padding moves inside so the overlay covers the padding box.
                    let outer = Self::modifier(&without_padding(e), ctx, None, None);
                    let pad = Self::padding(e).unwrap();
                    self.block(&format!("Box({outer})"), |s| {
                        s.block(
                            &format!("Box(Modifier.{pad}, contentAlignment = Alignment.{align})"),
                            |s| {
                                for c in &flow {
                                    s.element(c, child);
                                }
                            },
                        );
                        s.overlay(&absolute, ctx);
                    });
                }
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
                    overlay: false,
                };
                self.block(&format!("Box({m})"), |s| {
                    for c in &e.children {
                        s.element(c, child);
                    }
                });
            }
            ElementKind::Grid => self.grid(e, ctx),
            ElementKind::Text => {
                let Some(text) = content_text(&e.content) else {
                    return;
                };
                let m = Self::modifier(e, ctx, None, None);
                let mut args = vec![
                    text,
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
                let label = content_text(&e.content).unwrap_or_else(|| string(""));
                let m = Self::modifier(e, ctx, None, None);
                let style = Self::text_style(e, "text_color");
                self.block(
                    &format!("Box({m}, contentAlignment = Alignment.Center)"),
                    |s| {
                        s.line(&format!("BasicText({label}, style = {style})"));
                    },
                );
            }
            ElementKind::Input => {
                let m = Self::modifier(e, ctx, None, None);
                let style = Self::text_style(e, "text_color");
                let placeholder = e
                    .props
                    .get("placeholder")
                    .and_then(value_text)
                    .unwrap_or_else(|| string(""));
                let bind = match e.props.get("bind") {
                    Some(Value::State(r)) => {
                        let v = state_expr(r);
                        format!(", value = {v}, onValueChange = {{ {v} = it }}")
                    }
                    _ => String::new(),
                };
                let field = format!("EraInput({placeholder}, {style}, c.muted, {m}{bind})");
                match e.props.get("label").and_then(value_text) {
                    Some(label) => self.block(
                        "Column(verticalArrangement = Arrangement.spacedBy(4.dp))",
                        |s| {
                            s.line(&format!(
                                "BasicText({label}, style = EraType.body.copy(color = c.muted))"
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

    /// Split children into normal flow and `position: absolute` ones;
    /// `position: fixed` ones are kept for the screen-level overlay.
    fn partition(&mut self, children: &[Element]) -> (Vec<Element>, Vec<Element>) {
        let mut flow = Vec::new();
        let mut absolute = Vec::new();
        for c in children {
            match keyword(c, "position") {
                Some("absolute") => absolute.push(c.clone()),
                Some("fixed") => self.fixed.push(c.clone()),
                _ => flow.push(c.clone()),
            }
        }
        (flow, absolute)
    }

    /// Absolutely positioned children, in a box that matches the parent
    /// without changing its size (CSS: out of flow).
    fn overlay(&mut self, absolute: &[Element], ctx: Ctx) {
        if absolute.is_empty() {
            return;
        }
        let child = Ctx {
            parent: Parent::Box,
            stretch: false,
            root: false,
            overlay: true,
            v_unbounded: false,
            h_unbounded: false,
        };
        let _ = ctx;
        self.block("Box(Modifier.matchParentSize())", |s| {
            for c in absolute {
                s.element(c, child);
            }
        });
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
        // `overflow: scroll/auto` scrolls along the main axis.
        let scrolls = matches!(keyword(e, "overflow"), Some("scroll" | "auto"));
        let scroll = scrolls.then_some(if row {
            "horizontalScroll(rememberScrollState())"
        } else {
            "verticalScroll(rememberScrollState())"
        });
        let (flow, absolute) = self.partition(&e.children);
        // A row that distributes free space acts like a block-level flex row
        // in a column: it is as wide as its parent (otherwise there is no space
        // to distribute).
        let distributes = matches!(
            keyword(e, "distribute"),
            Some("between" | "around" | "evenly")
        );
        let block_row =
            row && distributes && !e.props.contains_key("width") && ctx.parent == Parent::Column;
        let owned;
        let e = if block_row {
            let mut c = e.clone();
            c.props.insert("width", Value::Size(Size::Fill));
            owned = c;
            &owned
        } else {
            e
        };
        let m = if absolute.is_empty() {
            Self::modifier(e, ctx, None, scroll)
        } else {
            // The inner row/column fills whatever size the outer box was given.
            let mut inner = Vec::new();
            if e.props
                .get("width")
                .is_some_and(|w| *w != Value::Size(Size::Fit))
            {
                inner.push("fillMaxWidth()");
            }
            if e.props
                .get("height")
                .is_some_and(|h| *h != Value::Size(Size::Fit))
            {
                inner.push("fillMaxHeight()");
            }
            // Absolute children are placed against the padding box (CSS), so the
            // padding belongs to the inner row/column, not the outer box.
            let pad = Self::padding(e);
            let mut parts: Vec<String> = inner.iter().map(|s| s.to_string()).collect();
            parts.extend(pad);
            if parts.is_empty() {
                "Modifier".to_string()
            } else {
                format!("Modifier.{}", parts.join("."))
            }
        };
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
            overlay: false,
            v_unbounded: ctx.v_unbounded || (scrolls && !row),
            h_unbounded: ctx.h_unbounded || (scrolls && row),
        };
        let wraps = keyword(e, "flex_wrap") == Some("wrap");
        if wraps {
            // FlowRow/FlowColumn: the cross axis uses the same gap.
            let gap = e
                .props
                .get("gap")
                .and_then(|v| dim_expr("gap", v))
                .unwrap_or_else(|| "0.dp".into());
            args.push(format!(
                "{} = Arrangement.spacedBy({gap})",
                if row {
                    "verticalArrangement"
                } else {
                    "horizontalArrangement"
                }
            ));
            args.retain(|a| {
                !a.starts_with(if row {
                    "verticalAlignment"
                } else {
                    "horizontalAlignment"
                })
            });
        }
        let name = match (row, wraps) {
            (true, false) => "Row",
            (false, false) => "Column",
            (true, true) => "FlowRow",
            (false, true) => "FlowColumn",
        };
        let head = format!("{name}({})", args.join(", "));
        if absolute.is_empty() {
            self.block(&head, |s| {
                for c in &flow {
                    s.element(c, child);
                }
            });
        } else {
            let outer = Self::modifier(&without_padding(e), ctx, None, scroll);
            self.block(&format!("Box({outer})"), |s| {
                s.block(&head, |s| {
                    for c in &flow {
                        s.element(c, child);
                    }
                });
                s.overlay(&absolute, ctx);
            });
        }
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
            overlay: false,
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

/// Kotlin expression for a piece of state: `Session.code`.
pub(crate) fn state_expr(r: &StateRef) -> String {
    format!("{}.{}", r.store, ident(&r.field))
}

/// A Kotlin string expression; state is interpolated with `${...}`.
fn template_expr(parts: &[TextPart]) -> String {
    let mut out = String::from("\"");
    for p in parts {
        match p {
            TextPart::Lit(t) => {
                let quoted = string(t);
                out.push_str(&quoted[1..quoted.len() - 1]);
            }
            TextPart::State(r) => match r.ty {
                StateType::Number => out.push_str(&format!("${{eraText({})}}", state_expr(r))),
                _ => out.push_str(&format!("${{{}}}", state_expr(r))),
            },
        }
    }
    out.push('"');
    out
}

fn content_text(c: &Option<Content>) -> Option<String> {
    match c {
        Some(Content::Text(t)) => Some(string(t)),
        Some(Content::Template(p)) => Some(template_expr(p)),
        _ => None,
    }
}

fn value_text(v: &Value) -> Option<String> {
    match v {
        Value::Text(t) => Some(string(t)),
        Value::Template(p) => Some(template_expr(p)),
        _ => None,
    }
}

/// `Session.verify(); EraNav.go("Home")`
fn effects_code(effects: &[Effect]) -> String {
    effects
        .iter()
        .map(|e| match e {
            Effect::Call { store, action } => format!("{store}.{}()", ident(action)),
            Effect::Go(screen) => format!("EraNav.go({})", string(screen)),
            Effect::Back => "EraNav.back()".into(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The element without its padding (moved to an inner container).
fn without_padding(e: &Element) -> Element {
    let mut c = e.clone();
    c.props.retain(|k, _| !k.starts_with("padding"));
    c
}
