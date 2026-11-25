//! Source edits behind every Era Builder action (D-095). Each function takes
//! a file's text and returns the new text, or `None` if the node was not
//! found. Edits are minimal text changes, so hand-written code keeps its
//! formatting and comments.

use era_ir::schema;
use era_syntax::{File, ForLoop, Item, Node, Value, ValueKind, parse};

/// A node found by its start offset.
pub enum Found<'a> {
    Node(&'a Node),
    For(&'a ForLoop),
}

fn walk<'a>(items: &'a [Item], start: usize) -> Option<Found<'a>> {
    for item in items {
        match item {
            Item::Node(n) => {
                if n.span.start == start {
                    return Some(Found::Node(n));
                }
                if let Some(b) = &n.block
                    && let Some(f) = walk(&b.items, start)
                {
                    return Some(f);
                }
            }
            Item::For(f) => {
                if f.span.start == start {
                    return Some(Found::For(f));
                }
                if let Some(x) = walk(&f.block.items, start) {
                    return Some(x);
                }
            }
            Item::When(w) => {
                if let Some(x) = walk(&w.block.items, start) {
                    return Some(x);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn find(file: &File, start: usize) -> Option<Found<'_>> {
    file.decls.iter().find_map(|d| walk(&d.body.items, start))
}

fn find_node(file: &File, start: usize) -> Option<&Node> {
    match find(file, start)? {
        Found::Node(n) => Some(n),
        Found::For(_) => None,
    }
}

fn line_start(src: &str, pos: usize) -> usize {
    src[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

fn line_end(src: &str, pos: usize) -> usize {
    src[pos..].find('\n').map(|i| pos + i).unwrap_or(src.len())
}

/// Leading whitespace of the line that holds `pos`.
fn indent_at(src: &str, pos: usize) -> String {
    let ls = line_start(src, pos);
    src[ls..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

fn same_line(src: &str, a: usize, b: usize) -> bool {
    !src[a.min(b)..a.max(b)].contains('\n')
}

/// The canonical property key for `name` on this node (`borderRadius` ==
/// `border-radius` == `radius`).
fn key(node: &Node, name: &str) -> String {
    match schema::primitive(&node.name.name).and_then(|p| p.key(name)) {
        Some(k) => k.to_string(),
        None => schema::normalize(name),
    }
}

fn splice(src: &str, start: usize, end: usize, with: &str) -> String {
    format!("{}{with}{}", &src[..start], &src[end..])
}

/// `(name, value as written)` for each property of a node.
pub fn properties(src: &str, path: &str, start: usize) -> Vec<(String, String)> {
    let (file, _) = parse(src, path);
    let Some(node) = find_node(&file, start) else {
        return Vec::new();
    };
    let Some(block) = &node.block else {
        return Vec::new();
    };
    block
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Property(p) => Some((
                p.name.name.clone(),
                src[p.value.span.start..p.value.span.end].to_string(),
            )),
            _ => None,
        })
        .collect()
}

/// The written value of one property, matched by any spelling.
pub fn property(src: &str, path: &str, start: usize, name: &str) -> Option<String> {
    let (file, _) = parse(src, path);
    let node = find_node(&file, start)?;
    let want = key(node, name);
    node.block.as_ref()?.items.iter().find_map(|i| match i {
        Item::Property(p) if key(node, &p.name.name) == want => {
            Some(src[p.value.span.start..p.value.span.end].to_string())
        }
        _ => None,
    })
}

/// Set (or add) `name: value` on the node at `start`.
pub fn set_property(
    src: &str,
    path: &str,
    start: usize,
    name: &str,
    value: &str,
) -> Option<String> {
    let (file, _) = parse(src, path);
    let node = find_node(&file, start)?;
    let want = key(node, name);
    if let Some(block) = &node.block {
        for i in &block.items {
            if let Item::Property(p) = i
                && key(node, &p.name.name) == want
            {
                return Some(splice(src, p.value.span.start, p.value.span.end, value));
            }
        }
        let open = block.span.start + 1;
        let close = block.span.end - 1;
        if same_line(src, block.span.start, close) {
            let inner = src[open..close].trim();
            let text = if inner.is_empty() {
                format!(" {name}: {value} ")
            } else {
                format!(" {name}: {value};")
            };
            return Some(splice(src, open, open, &text));
        }
        let child = format!("{}    ", indent_at(src, node.span.start));
        return Some(splice(
            src,
            open,
            open,
            &format!("\n{child}{name}: {value}"),
        ));
    }
    Some(splice(
        src,
        node.span.end,
        node.span.end,
        &format!(" {{ {name}: {value} }}"),
    ))
}

/// Remove a property (any spelling) from the node at `start`.
pub fn remove_property(src: &str, path: &str, start: usize, name: &str) -> Option<String> {
    let (file, _) = parse(src, path);
    let node = find_node(&file, start)?;
    let want = key(node, name);
    let p = node.block.as_ref()?.items.iter().find_map(|i| match i {
        Item::Property(p) if key(node, &p.name.name) == want => Some(p),
        _ => None,
    })?;
    let (s, e) = (p.span.start, p.span.end);
    let ls = line_start(src, s);
    let le = line_end(src, e);
    // The property is alone on its line: remove the whole line.
    if src[ls..s].trim().is_empty() && src[e..le].trim().trim_matches([';', ',']).is_empty() {
        let end = (le + 1).min(src.len());
        return Some(splice(src, ls, end, ""));
    }
    // Otherwise remove it and one following separator.
    let mut end = e;
    let rest = &src[e..];
    let skip = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    if rest[skip..].starts_with([';', ',']) {
        end = e + skip + 1;
    }
    let mut start = s;
    while start > 0 && src.as_bytes()[start - 1] == b' ' {
        start -= 1;
    }
    Some(splice(src, start, end, ""))
}

/// The node's argument as written (`"Hello"` for `text "Hello"`).
pub fn argument(src: &str, path: &str, start: usize) -> Option<Value> {
    let (file, _) = parse(src, path);
    find_node(&file, start)?.arg.clone()
}

/// Replace a `text`/`button` label, given the raw text (quoted for you).
pub fn set_label(src: &str, path: &str, start: usize, label: &str) -> Option<String> {
    let (file, _) = parse(src, path);
    let node = find_node(&file, start)?;
    let quoted = format!("\"{}\"", label.replace('\\', "\\\\").replace('"', "\\\""));
    match &node.arg {
        Some(v) if matches!(v.kind, ValueKind::String(_)) => {
            Some(splice(src, v.span.start, v.span.end, &quoted))
        }
        Some(_) => None,
        None => {
            let at = node.name.span.end;
            Some(splice(src, at, at, &format!(" {quoted}")))
        }
    }
}

/// Byte range of a node or loop, extended to whole lines when it sits
/// alone on its lines.
fn whole(src: &str, s: usize, e: usize) -> (usize, usize, bool) {
    let ls = line_start(src, s);
    let le = line_end(src, e);
    if src[ls..s].trim().is_empty() && src[e..le].trim().trim_matches([';', ',']).is_empty() {
        (ls, (le + 1).min(src.len()), true)
    } else {
        (s, e, false)
    }
}

fn span_of(file: &File, start: usize) -> Option<(usize, usize)> {
    Some(match find(file, start)? {
        Found::Node(n) => (n.span.start, n.span.end),
        Found::For(f) => (f.span.start, f.span.end),
    })
}

/// Delete the node (or `for`) at `start`.
pub fn delete(src: &str, path: &str, start: usize) -> Option<String> {
    let (file, _) = parse(src, path);
    let (s, e) = span_of(&file, start)?;
    let (a, b, lines) = whole(src, s, e);
    if lines {
        return Some(splice(src, a, b, ""));
    }
    let mut end = e;
    let rest = &src[e..];
    let skip = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    if rest[skip..].starts_with([';', ',']) {
        end = e + skip + 1;
    }
    Some(splice(src, s, end, ""))
}

/// Re-indent every line of `text` from `from` to `to`.
fn reindent(text: &str, from: &str, to: &str) -> String {
    text.lines()
        .map(|l| match l.strip_prefix(from) {
            Some(rest) => format!("{to}{rest}"),
            None if l.trim().is_empty() => String::new(),
            None => format!("{to}{}", l.trim_start()),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Add `snippet` as the last child of the container at `start`.
pub fn insert_child(src: &str, path: &str, start: usize, snippet: &str) -> Option<String> {
    let (file, _) = parse(src, path);
    let node = find_node(&file, start)?;
    let indent = indent_at(src, node.span.start);
    let child = format!("{indent}    ");
    let body = reindent(snippet, "", &child);
    match &node.block {
        Some(block) => {
            let close = block.span.end - 1;
            if same_line(src, block.span.start, close) {
                let inner = src[block.span.start + 1..close].trim();
                let sep = if inner.is_empty() { " " } else { "; " };
                let one_line = snippet.lines().count() == 1;
                if one_line {
                    let mut before = close;
                    while before > 0 && src.as_bytes()[before - 1] == b' ' {
                        before -= 1;
                    }
                    return Some(splice(
                        src,
                        before,
                        close,
                        &format!("{sep}{} ", snippet.trim()),
                    ));
                }
                return Some(splice(
                    src,
                    block.span.start + 1,
                    close,
                    &format!("\n{child}{inner}\n{body}\n{indent}"),
                ));
            }
            let ls = line_start(src, close);
            if src[ls..close].trim().is_empty() {
                Some(splice(src, ls, ls, &format!("{body}\n")))
            } else {
                Some(splice(src, close, close, &format!("\n{body}\n{indent}")))
            }
        }
        None => Some(splice(
            src,
            node.span.end,
            node.span.end,
            &format!(" {{\n{body}\n{indent}}}"),
        )),
    }
}

/// Where a moved node goes, relative to the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Before,
    After,
    Inside,
}

/// Insert `snippet` before or after the node at `target`.
pub fn insert_next_to(
    src: &str,
    path: &str,
    target: usize,
    snippet: &str,
    after: bool,
) -> Option<String> {
    let (file, _) = parse(src, path);
    let (s, e) = span_of(&file, target)?;
    let (a, b, lines) = whole(src, s, e);
    let indent = indent_at(src, s);
    if lines {
        let body = reindent(snippet, "", &indent);
        let at = if after { b } else { a };
        let text = if after && !src[..b].ends_with('\n') {
            format!("\n{body}")
        } else {
            format!("{body}\n")
        };
        return Some(splice(src, at, at, &text));
    }
    let one = snippet.trim().replace('\n', " ");
    Some(if after {
        splice(src, e, e, &format!("; {one}"))
    } else {
        splice(src, s, s, &format!("{one}; "))
    })
}

/// Move the node at `from` before/after/inside the node at `to` (same file).
pub fn move_node(src: &str, path: &str, from: usize, to: usize, place: Place) -> Option<String> {
    if from == to {
        return None;
    }
    let (file, _) = parse(src, path);
    let (s, e) = span_of(&file, from)?;
    let (ts, _) = span_of(&file, to)?;
    if ts > s && ts < e {
        return None; // into itself
    }
    let indent = indent_at(src, s);
    let (a, b, lines) = whole(src, s, e);
    let moved = if lines {
        src[a..b].trim_end_matches('\n').to_string()
    } else {
        src[s..e].to_string()
    };
    let moved = reindent(&moved, &indent, "");
    let without = delete(src, path, from)?;
    let removed = src.len() - without.len();
    let target = if ts > s { ts - removed } else { ts };
    match place {
        Place::Inside => insert_child(&without, path, target, &moved),
        Place::Before => insert_next_to(&without, path, target, &moved, false),
        Place::After => insert_next_to(&without, path, target, &moved, true),
    }
}

/// The starter code for each palette item.
pub fn snippet(kind: &str) -> String {
    match kind {
        "column" => "column {\n    gap: 8px\n    padding: 16px\n}".into(),
        "row" => "row {\n    gap: 8px\n    align-items: center\n}".into(),
        "stack" => "stack {\n    width: 120px\n    height: 120px\n}".into(),
        "card" => "card {\n    gap: 8px\n    text \"Card\"\n}".into(),
        "grid" => "grid {\n    columns: 2\n    gap: 8px\n}".into(),
        "text" => "text \"Text\"".into(),
        "button" => "button \"Button\"".into(),
        "input" => "input { placeholder: \"Type here\" }".into(),
        "divider" => "divider".into(),
        "spacer" => "spacer".into(),
        "box" => {
            "stack { width: 64px; height: 64px; border-radius: 12px; background: primary }".into()
        }
        other => format!("{other} {{ }}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "screen Home {
    column {
        padding: 16
        text \"Hi\" { color: primary }
        row { gap: 4 }
    }
}
";

    fn at(src: &str, needle: &str) -> usize {
        src.find(needle).unwrap()
    }

    #[test]
    fn set_add_and_remove_properties() {
        let col = at(SRC, "column");
        let s = set_property(SRC, "a.era", col, "padding", "24px").unwrap();
        assert!(s.contains("padding: 24px\n"));
        let s = set_property(&s, "a.era", col, "background", "#FFFFFF").unwrap();
        assert!(
            s.contains("column {\n        background: #FFFFFF\n        padding: 24px"),
            "{s}"
        );
        // Any spelling matches: radius == border-radius.
        let s = set_property(&s, "a.era", col, "border-radius", "8px").unwrap();
        let s = set_property(&s, "a.era", col, "radius", "12px").unwrap();
        assert_eq!(s.matches("12px").count(), 1, "{s}");
        assert!(!s.contains("8px"));
        let s = remove_property(&s, "a.era", col, "background").unwrap();
        assert!(!s.contains("background"));
        // One-line blocks stay on one line.
        let t = at(&s, "text");
        let s = set_property(&s, "a.era", t, "font-size", "20px").unwrap();
        assert!(
            s.contains("text \"Hi\" { font-size: 20px; color: primary }"),
            "{s}"
        );
        let s = remove_property(&s, "a.era", at(&s, "text"), "color").unwrap();
        assert!(s.contains("text \"Hi\" { font-size: 20px; }"), "{s}");
        // A node without a block gets one.
        let d = "screen A {\n    divider\n}\n";
        let s2 = set_property(d, "a.era", at(d, "divider"), "color", "red").unwrap();
        assert!(s2.contains("divider { color: red }"), "{s2}");
    }

    #[test]
    fn labels() {
        let s = set_label(SRC, "a.era", at(SRC, "text"), "Hello \"you\"").unwrap();
        assert!(
            s.contains(r#"text "Hello \"you\"" { color: primary }"#),
            "{s}"
        );
    }

    #[test]
    fn insert_delete_and_move() {
        let col = at(SRC, "column");
        let s = insert_child(SRC, "a.era", col, "button \"Go\"").unwrap();
        assert!(
            s.contains("        row { gap: 4 }\n        button \"Go\"\n    }"),
            "{s}"
        );
        // Into a one-line block.
        let s = insert_child(&s, "a.era", at(&s, "row"), "text \"x\"").unwrap();
        assert!(s.contains("row { gap: 4; text \"x\" }"), "{s}");
        // Multi-line snippets are indented.
        let s = insert_child(&s, "a.era", col, &snippet("card")).unwrap();
        assert!(
            s.contains(
                "        card {\n            gap: 8px\n            text \"Card\"\n        }\n    }"
            ),
            "{s}"
        );
        let s = delete(&s, "a.era", at(&s, "button")).unwrap();
        assert!(!s.contains("button"));
        // Move `text "Hi"` after the row.
        let s = move_node(
            &s,
            "a.era",
            at(&s, "text \"Hi\""),
            at(&s, "row"),
            Place::After,
        )
        .unwrap();
        let (t, r) = (at(&s, "text \"Hi\""), at(&s, "row"));
        assert!(r < t, "{s}");
        assert!(
            s.contains("\n        text \"Hi\" { color: primary }\n"),
            "{s}"
        );
        let (parsed, diags) = parse(&s, "a.era");
        assert!(diags.is_empty(), "{diags:?}\n{s}");
        assert_eq!(parsed.decls.len(), 1);
    }

    #[test]
    fn every_snippet_parses() {
        for k in [
            "column", "row", "stack", "card", "grid", "text", "button", "input", "divider",
            "spacer", "box",
        ] {
            let src = "screen A {\n    column {\n    }\n}\n".to_string();
            let s = insert_child(&src, "a.era", at(&src, "column"), &snippet(k)).unwrap();
            let (_, diags) = parse(&s, "a.era");
            assert!(diags.is_empty(), "{k}: {diags:?}\n{s}");
        }
    }
}
