//! SVG -> Android VectorDrawable (DECISIONS Q-03 / D-051).
//!
//! Supports the subset icon sets use: `path`, `circle`, `ellipse`, `rect`,
//! `line`, `polyline`, `polygon` and untransformed `g`, with fill/stroke
//! presentation attributes (also via `style`). Anything else is an error that
//! names the unsupported feature, rather than a silently wrong icon.

use std::fmt::Write as _;

#[derive(Clone)]
struct Style {
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: Option<String>,
    linecap: Option<String>,
    linejoin: Option<String>,
    fill_rule: Option<String>,
    fill_opacity: Option<String>,
    stroke_opacity: Option<String>,
    opacity: f64,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            fill: Some("black".into()),
            stroke: None,
            stroke_width: None,
            linecap: None,
            linejoin: None,
            fill_rule: None,
            fill_opacity: None,
            stroke_opacity: None,
            opacity: 1.0,
        }
    }
}

fn num(s: &str) -> Result<f64, String> {
    s.trim()
        .trim_end_matches("px")
        .parse::<f64>()
        .map_err(|_| format!("`{s}` is not a number"))
}

fn fmt(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

/// SVG color -> `#AARRGGBB`; `None` for `none`/`transparent`.
fn color(s: &str) -> Result<Option<String>, String> {
    let s = s.trim();
    let hex = |h: &str| -> Result<Option<String>, String> {
        let full = match h.len() {
            3 => h.chars().flat_map(|c| [c, c]).collect(),
            6 => h.to_string(),
            _ => return Err(format!("unsupported color `#{h}`")),
        };
        if !full.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("unsupported color `#{h}`"));
        }
        Ok(Some(format!("#FF{}", full.to_ascii_uppercase())))
    };
    match s {
        "none" | "transparent" => Ok(None),
        "black" | "currentColor" | "currentcolor" => Ok(Some("#FF000000".into())),
        "white" => Ok(Some("#FFFFFFFF".into())),
        _ if s.starts_with("url(") => {
            Err("gradients and patterns (`url(...)`) are not supported".into())
        }
        _ if s.starts_with('#') => hex(&s[1..]),
        _ => Err(format!("unsupported color `{s}`; use #RRGGBB")),
    }
}

fn apply(style: &mut Style, node: roxmltree::Node) -> Result<(), String> {
    let mut set = |k: &str, v: &str| -> Result<(), String> {
        let v = v.trim().to_string();
        match k {
            "fill" => style.fill = Some(v),
            "stroke" => style.stroke = Some(v),
            "stroke-width" => style.stroke_width = Some(fmt(num(&v)?)),
            "stroke-linecap" => style.linecap = Some(v),
            "stroke-linejoin" => style.linejoin = Some(v),
            "fill-rule" => style.fill_rule = Some(v),
            "fill-opacity" => style.fill_opacity = Some(v),
            "stroke-opacity" => style.stroke_opacity = Some(v),
            "opacity" => style.opacity *= num(&v)?,
            "transform" => {
                return Err(
                    "`transform` is not supported; flatten transforms in your editor".into(),
                );
            }
            "clip-path" | "mask" | "filter" => return Err(format!("`{k}` is not supported")),
            _ => {}
        }
        Ok(())
    };
    for a in node.attributes() {
        set(a.name(), a.value())?;
    }
    if let Some(st) = node.attribute("style") {
        for decl in st.split(';') {
            if let Some((k, v)) = decl.split_once(':') {
                set(k.trim(), v)?;
            }
        }
    }
    Ok(())
}

fn attr(node: roxmltree::Node, name: &str) -> Result<f64, String> {
    node.attribute(name).map(num).unwrap_or(Ok(0.0))
}

fn points_path(node: roxmltree::Node, close: bool) -> Result<String, String> {
    let pts = node.attribute("points").unwrap_or("");
    let nums: Vec<&str> = pts
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    if nums.len() < 4 || !nums.len().is_multiple_of(2) {
        return Err("invalid `points`".into());
    }
    let mut d = String::new();
    for (i, pair) in nums.chunks(2).enumerate() {
        let _ = write!(
            d,
            "{}{},{} ",
            if i == 0 { "M" } else { "L" },
            fmt(num(pair[0])?),
            fmt(num(pair[1])?)
        );
    }
    if close {
        d.push('Z');
    }
    Ok(d.trim_end().to_string())
}

/// Path data for a shape element, or `None` for non-drawing elements.
fn shape_path(node: roxmltree::Node) -> Result<Option<String>, String> {
    let ellipse = |cx: f64, cy: f64, rx: f64, ry: f64| {
        format!(
            "M{},{} a{},{} 0 1,0 {},0 a{},{} 0 1,0 {},0 Z",
            fmt(cx - rx),
            fmt(cy),
            fmt(rx),
            fmt(ry),
            fmt(2.0 * rx),
            fmt(rx),
            fmt(ry),
            fmt(-2.0 * rx)
        )
    };
    Ok(Some(match node.tag_name().name() {
        "path" => node.attribute("d").ok_or("`path` without `d`")?.to_string(),
        "circle" => {
            let r = attr(node, "r")?;
            ellipse(attr(node, "cx")?, attr(node, "cy")?, r, r)
        }
        "ellipse" => ellipse(
            attr(node, "cx")?,
            attr(node, "cy")?,
            attr(node, "rx")?,
            attr(node, "ry")?,
        ),
        "rect" => {
            let (x, y, w, h) = (
                attr(node, "x")?,
                attr(node, "y")?,
                attr(node, "width")?,
                attr(node, "height")?,
            );
            let rx = node
                .attribute("rx")
                .or(node.attribute("ry"))
                .map(num)
                .transpose()?
                .unwrap_or(0.0)
                .min(w / 2.0);
            let ry = node
                .attribute("ry")
                .or(node.attribute("rx"))
                .map(num)
                .transpose()?
                .unwrap_or(0.0)
                .min(h / 2.0);
            if rx == 0.0 && ry == 0.0 {
                format!(
                    "M{},{} h{} v{} h{} Z",
                    fmt(x),
                    fmt(y),
                    fmt(w),
                    fmt(h),
                    fmt(-w)
                )
            } else {
                format!(
                    "M{},{} h{} a{},{} 0 0,1 {},{} v{} a{},{} 0 0,1 {},{} h{} a{},{} 0 0,1 {},{} v{} a{},{} 0 0,1 {},{} Z",
                    fmt(x + rx),
                    fmt(y),
                    fmt(w - 2.0 * rx),
                    fmt(rx),
                    fmt(ry),
                    fmt(rx),
                    fmt(ry),
                    fmt(h - 2.0 * ry),
                    fmt(rx),
                    fmt(ry),
                    fmt(-rx),
                    fmt(ry),
                    fmt(-(w - 2.0 * rx)),
                    fmt(rx),
                    fmt(ry),
                    fmt(-rx),
                    fmt(-ry),
                    fmt(-(h - 2.0 * ry)),
                    fmt(rx),
                    fmt(ry),
                    fmt(rx),
                    fmt(-ry)
                )
            }
        }
        "line" => format!(
            "M{},{} L{},{}",
            fmt(attr(node, "x1")?),
            fmt(attr(node, "y1")?),
            fmt(attr(node, "x2")?),
            fmt(attr(node, "y2")?)
        ),
        "polyline" => points_path(node, false)?,
        "polygon" => points_path(node, true)?,
        "title" | "desc" | "metadata" => return Ok(None),
        other => return Err(format!("`<{other}>` is not supported")),
    }))
}

fn walk(node: roxmltree::Node, inherited: &Style, out: &mut String) -> Result<(), String> {
    for child in node.children().filter(|n| n.is_element()) {
        let mut style = inherited.clone();
        apply(&mut style, child)?;
        match child.tag_name().name() {
            "g" => walk(child, &style, out)?,
            "defs" => continue,
            _ => {
                let Some(d) = shape_path(child)? else {
                    continue;
                };
                let mut attrs = vec![format!("android:pathData=\"{}\"", xml_escape(&d))];
                let alpha = |o: &Option<String>| -> Result<f64, String> {
                    Ok(o.as_deref().map(num).transpose()?.unwrap_or(1.0) * style.opacity)
                };
                if let Some(fill) = style.fill.as_deref().map(color).transpose()?.flatten() {
                    attrs.push(format!("android:fillColor=\"{fill}\""));
                    let a = alpha(&style.fill_opacity)?;
                    if a < 1.0 {
                        attrs.push(format!("android:fillAlpha=\"{}\"", fmt(a)));
                    }
                    if style.fill_rule.as_deref() == Some("evenodd") {
                        attrs.push("android:fillType=\"evenOdd\"".into());
                    }
                }
                if let Some(stroke) = style.stroke.as_deref().map(color).transpose()?.flatten() {
                    attrs.push(format!("android:strokeColor=\"{stroke}\""));
                    attrs.push(format!(
                        "android:strokeWidth=\"{}\"",
                        style.stroke_width.as_deref().unwrap_or("1")
                    ));
                    let a = alpha(&style.stroke_opacity)?;
                    if a < 1.0 {
                        attrs.push(format!("android:strokeAlpha=\"{}\"", fmt(a)));
                    }
                    if let Some(c) = &style.linecap {
                        attrs.push(format!("android:strokeLineCap=\"{c}\""));
                    }
                    if let Some(j) = &style.linejoin {
                        attrs.push(format!("android:strokeLineJoin=\"{j}\""));
                    }
                }
                let _ = writeln!(out, "    <path\n        {} />", attrs.join("\n        "));
            }
        }
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Convert SVG source to VectorDrawable XML.
pub fn to_vector_drawable(svg: &str) -> Result<String, String> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| format!("invalid SVG: {e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err("the root element is not `<svg>`".into());
    }
    let (min_x, min_y, vw, vh) = match root.attribute("viewBox") {
        Some(vb) => {
            let v: Vec<f64> = vb
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .map(num)
                .collect::<Result<_, _>>()?;
            if v.len() != 4 || v[2] <= 0.0 || v[3] <= 0.0 {
                return Err(format!("invalid viewBox `{vb}`"));
            }
            (v[0], v[1], v[2], v[3])
        }
        None => (0.0, 0.0, attr(root, "width")?, attr(root, "height")?),
    };
    if vw <= 0.0 || vh <= 0.0 {
        return Err("the SVG needs a `viewBox` (or `width` and `height`)".into());
    }
    let mut style = Style::default();
    apply(&mut style, root)?;
    let mut paths = String::new();
    walk(root, &style, &mut paths)?;
    if paths.is_empty() {
        return Err("the SVG has no shapes".into());
    }
    // A viewBox that does not start at 0,0 is shifted with a group.
    let body = if min_x != 0.0 || min_y != 0.0 {
        let indented: String = paths.lines().map(|l| format!("    {l}\n")).collect();
        format!(
            "    <group android:translateX=\"{}\" android:translateY=\"{}\">\n{indented}    </group>\n",
            fmt(-min_x),
            fmt(-min_y)
        )
    } else {
        paths
    };
    Ok(format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!-- Generated by ERA from SVG. Do not edit. -->\n<vector xmlns:android=\"http://schemas.android.com/apk/res/android\"\n    android:width=\"{w}dp\"\n    android:height=\"{h}dp\"\n    android:viewportWidth=\"{w}\"\n    android:viewportHeight=\"{h}\">\n{body}</vector>\n",
        w = fmt(vw),
        h = fmt(vh),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="8" r="4"/><path d="M4 21c0-4 4-6 8-6"/></svg>"##;

    #[test]
    fn converts_stroked_icon() {
        let v = to_vector_drawable(ICON).unwrap();
        assert!(v.contains("android:viewportWidth=\"24\""), "{v}");
        assert!(
            v.contains("android:pathData=\"M8,8 a4,4 0 1,0 8,0 a4,4 0 1,0 -8,0 Z\""),
            "{v}"
        );
        assert!(v.contains("android:pathData=\"M4 21c0-4 4-6 8-6\""), "{v}");
        assert!(v.contains("android:strokeColor=\"#FF000000\""), "{v}");
        assert!(v.contains("android:strokeWidth=\"1.8\""), "{v}");
        assert!(v.contains("android:strokeLineCap=\"round\""), "{v}");
        assert!(!v.contains("fillColor"), "fill=none: {v}");
    }

    #[test]
    fn filled_shapes_groups_and_style_attribute() {
        let svg = r##"<svg viewBox="0 0 10 10"><g style="fill:#f00;fill-opacity:0.5"><rect x="1" y="1" width="4" height="2"/><polygon points="0,0 5,0 5,5" fill-rule="evenodd"/></g><line x1="0" y1="0" x2="10" y2="10" stroke="white"/></svg>"##;
        let v = to_vector_drawable(svg).unwrap();
        assert!(v.contains("M1,1 h4 v2 h-4 Z"), "{v}");
        assert!(v.contains("android:fillColor=\"#FFFF0000\""), "{v}");
        assert!(v.contains("android:fillAlpha=\"0.5\""), "{v}");
        assert!(v.contains("M0,0 L5,0 L5,5 Z"), "{v}");
        assert!(v.contains("android:fillType=\"evenOdd\""), "{v}");
        assert!(v.contains("M0,0 L10,10"), "{v}");
        assert!(v.contains("android:strokeColor=\"#FFFFFFFF\""), "{v}");
    }

    #[test]
    fn rounded_rect_and_offset_viewbox() {
        let v = to_vector_drawable(
            r#"<svg viewBox="2 2 20 20"><rect x="2" y="2" width="20" height="10" rx="3"/></svg>"#,
        )
        .unwrap();
        assert!(
            v.contains("<group android:translateX=\"-2\" android:translateY=\"-2\">"),
            "{v}"
        );
        assert!(v.contains("M5,2 h14 a3,3 0 0,1 3,3"), "{v}");
    }

    #[test]
    fn unsupported_features_are_errors() {
        let cases = [
            (
                r#"<svg viewBox="0 0 1 1"><path d="M0 0" transform="rotate(4)"/></svg>"#,
                "transform",
            ),
            (
                r#"<svg viewBox="0 0 1 1"><path d="M0 0" fill="url(#g)"/></svg>"#,
                "gradients",
            ),
            (r#"<svg viewBox="0 0 1 1"><text>hi</text></svg>"#, "<text>"),
            (
                r#"<svg viewBox="0 0 1 1"><path d="M0 0" fill="rebeccapurple"/></svg>"#,
                "rebeccapurple",
            ),
            (r#"<svg viewBox="0 0 1 1"></svg>"#, "no shapes"),
            (r#"<svg><path d="M0 0"/></svg>"#, "viewBox"),
            ("<svg", "invalid SVG"),
        ];
        for (svg, want) in cases {
            let e = to_vector_drawable(svg).unwrap_err();
            assert!(e.contains(want), "{svg}: {e}");
        }
    }
}
