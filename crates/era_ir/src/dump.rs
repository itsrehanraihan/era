//! Deterministic, human-readable text form of the IR. Used for snapshot
//! tests and `era check --ir`. The format is for people, not a stable API.

use std::fmt::Write as _;

use crate::ir::*;
use crate::skin::Skin;

pub fn dump(app: &App) -> String {
    let mut o = String::new();
    let _ = writeln!(
        o,
        "app {:?} id={} version={} start={}",
        app.name, app.id, app.version, app.start
    );
    skin(&mut o, &app.skin);
    for s in &app.screens {
        let title = s
            .title
            .as_ref()
            .map(|t| format!(" title={t:?}"))
            .unwrap_or_default();
        let _ = writeln!(
            o,
            "screen {}{title} background={}",
            s.name,
            color(&s.background)
        );
        element(&mut o, &s.root, 1);
    }
    let _ = writeln!(o, "assets");
    for a in &app.assets {
        let _ = writeln!(o, "  {:?} {}", a.kind, a.path);
    }
    o
}

fn num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

fn hex(c: &Color) -> String {
    if c.a == 255 {
        format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", c.r, c.g, c.b, c.a)
    }
}

fn color(c: &ColorValue) -> String {
    match &c.token {
        Some(t) => format!("{t}({})", hex(&c.rgba)),
        None => hex(&c.rgba),
    }
}

fn value(v: &Value) -> String {
    match v {
        Value::Color(c) => color(c),
        Value::Transparent => "none".into(),
        Value::Dimension(d) => match &d.token {
            Some(t) => format!("{t}({})", num(d.dp)),
            None => num(d.dp),
        },
        Value::Size(Size::Fill) => "fill".into(),
        Value::Size(Size::Fit) => "fit".into(),
        Value::Size(Size::Dp(x)) => num(*x),
        Value::Number(x) => num(*x),
        Value::Bool(b) => b.to_string(),
        Value::Keyword(k) => (*k).into(),
        Value::Text(t) => format!("{t:?}"),
        Value::TextStyle(s) => s.clone(),
    }
}

fn skin(o: &mut String, s: &Skin) {
    let builtin = if s.builtin { " (built-in)" } else { "" };
    let _ = writeln!(o, "skin {}{builtin}", s.name);
    let colors = |m: &std::collections::BTreeMap<String, Color>| {
        m.iter()
            .map(|(k, c)| format!("{k}={}", hex(c)))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let _ = writeln!(o, "  colors {}", colors(&s.colors));
    if let Some(d) = &s.dark_colors {
        let _ = writeln!(o, "  colors dark {}", colors(d));
    }
    for (name, m) in [
        ("spacing", &s.spacing),
        ("radius", &s.radius),
        ("border", &s.border),
        ("elevation", &s.elevation),
    ] {
        let entries: Vec<String> = m.iter().map(|(k, v)| format!("{k}={}", num(*v))).collect();
        let _ = writeln!(o, "  {name} {}", entries.join(" "));
    }
    let t = &s.typography;
    let _ = writeln!(o, "  font {:?} files=[{}]", t.font, t.font_files.join(", "));
    for (name, st) in &t.styles {
        let mut parts = vec![
            format!("size={}", num(st.size)),
            format!("weight={}", st.weight),
        ];
        if let Some(lh) = st.line_height {
            parts.push(format!("line_height={}", num(lh)));
        }
        if let Some(tr) = st.tracking {
            parts.push(format!("tracking={}", num(tr)));
        }
        let _ = writeln!(o, "  style {name} {}", parts.join(" "));
    }
}

fn element(o: &mut String, e: &Element, depth: usize) {
    let pad = "  ".repeat(depth);
    let content = match &e.content {
        Some(Content::Text(t)) => format!(" {t:?}"),
        Some(Content::Asset(a)) => format!(" {:?}", a.path),
        None => String::new(),
    };
    let props: Vec<String> = e
        .props
        .iter()
        .map(|(k, v)| format!("{k}={}", value(v)))
        .collect();
    let props = if props.is_empty() {
        String::new()
    } else {
        format!(" {{{}}}", props.join(" "))
    };
    let origin = if e.origin.is_empty() {
        String::new()
    } else {
        format!(" <{}>", e.origin.join(">"))
    };
    let _ = writeln!(
        o,
        "{pad}{}{content}{props} @{}{origin}",
        e.kind.name(),
        e.id
    );
    for c in &e.children {
        element(o, c, depth + 1);
    }
}
