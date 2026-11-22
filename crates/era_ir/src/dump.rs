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
    if let Some(icon) = &app.icon {
        let bg = icon
            .background
            .as_ref()
            .map(color)
            .unwrap_or("white".into());
        let _ = writeln!(o, "icon {:?} background={bg}", icon.path);
    }
    for d in &app.data {
        let fields: Vec<String> = d
            .fields
            .iter()
            .map(|(n, t)| format!("{n}:{}", t.name()))
            .collect();
        let _ = writeln!(
            o,
            "data {} ({}) {} rows [{}]",
            d.name,
            d.file,
            d.rows.len(),
            fields.join(" ")
        );
    }
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
    if !app.stores.is_empty() {
        let _ = writeln!(o, "stores");
    }
    for st in &app.stores {
        let _ = writeln!(o, "  store {}", st.name);
        for f in &st.fields {
            let _ = writeln!(
                o,
                "    {}state {}: {} = {}",
                if f.persist { "persist " } else { "" },
                f.name,
                f.ty.name(),
                literal(&f.initial)
            );
        }
        for a in &st.actions {
            let steps: Vec<String> = a
                .steps
                .iter()
                .map(|s| match s {
                    ActionStep::Set { field, value } => format!(
                        "set {field} = {}",
                        match value {
                            StepValue::Literal(l) => literal(l),
                            StepValue::Field(f) => f.clone(),
                        }
                    ),
                    ActionStep::Toggle { field } => format!("toggle {field}"),
                    ActionStep::Add { field, amount } => format!("add {field} {}", num(*amount)),
                })
                .collect();
            let _ = writeln!(o, "    action {} [{}]", a.name, steps.join("; "));
        }
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
        Value::Size(Size::Percent(p)) => format!("{}%", num(*p)),
        Value::Size(Size::Fr(f)) => format!("{}fr", num(*f)),
        Value::Size(Size::Viewport { vertical, percent }) => {
            format!("{}{}", num(*percent), if *vertical { "vh" } else { "vw" })
        }
        Value::Size(Size::MinContent) => "min-content".into(),
        Value::Size(Size::MaxContent) => "max-content".into(),
        Value::Number(x) => num(*x),
        Value::Bool(b) => b.to_string(),
        Value::Keyword(k) => (*k).into(),
        Value::Text(t) => format!("{t:?}"),
        Value::TextStyle(s) => s.clone(),
        Value::Template(parts) => template(parts),
        Value::Effects(e) => effects(e),
        Value::State(r) => format!("{}.{}", r.store, r.field),
        Value::Gradient(g) => {
            let kind = match &g.kind {
                GradientKind::Linear(a) => format!("linear({}deg", num(*a)),
                GradientKind::LinearToCorner { right, bottom } => format!(
                    "linear(to {} {}",
                    if *bottom { "bottom" } else { "top" },
                    if *right { "right" } else { "left" }
                ),
                GradientKind::Radial => "radial(circle".into(),
            };
            let stops: Vec<String> = g
                .stops
                .iter()
                .map(|s| format!("{} {}%", color(&s.color), num(s.position * 100.0)))
                .collect();
            format!("{kind}, {})", stops.join(", "))
        }
        Value::Layers(l) => l.iter().map(value).collect::<Vec<_>>().join(" | "),
        Value::Shadows(list) => list
            .iter()
            .map(|s| {
                format!(
                    "{}{} {} {} {} {}",
                    if s.inset { "inset " } else { "" },
                    num(s.x),
                    num(s.y),
                    num(s.blur),
                    num(s.spread),
                    color(&s.color)
                )
            })
            .collect::<Vec<_>>()
            .join(", "),
        Value::Filters(list) => list
            .iter()
            .map(|f| format!("{}({})", f.name, num(f.amount)))
            .collect::<Vec<_>>()
            .join(" "),
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
        Some(Content::Template(p)) => format!(" {}", template(p)),
        Some(Content::Asset(a)) => format!(" {:?}", a.path),
        Some(Content::Repeat { var, list }) => format!(" {var} in {list}"),
        Some(Content::ItemAsset { field, .. }) => format!(" {}.{}", field.store, field.field),
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

fn literal(l: &Literal) -> String {
    match l {
        Literal::Text(t) => format!("{t:?}"),
        Literal::Number(n) => num(*n),
        Literal::Bool(b) => b.to_string(),
    }
}

/// `"Code: {Session.code}"`
fn template(parts: &[TextPart]) -> String {
    let mut s = String::new();
    for p in parts {
        match p {
            TextPart::Lit(t) => s.push_str(&t.replace('{', "{{").replace('}', "}}")),
            TextPart::State(r) => s.push_str(&format!("{{{}.{}}}", r.store, r.field)),
        }
    }
    format!("{s:?}")
}

fn effects(e: &[Effect]) -> String {
    let parts: Vec<String> = e
        .iter()
        .map(|e| match e {
            Effect::Call { store, action } => format!("{store}.{action}"),
            Effect::Go(s) => format!("go {s}"),
            Effect::Back => "back".into(),
        })
        .collect();
    if parts.is_empty() {
        "nothing".into()
    } else {
        format!("[{}]", parts.join(" then "))
    }
}
