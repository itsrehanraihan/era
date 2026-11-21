//! Diagnostics shared by every ERA compiler stage.
//!
//! A [`Diagnostic`] has a severity, a stable code (`E0102`, `W0204`), a
//! message, an optional source [`Location`] and optional help text. Rendering
//! follows the familiar rustc layout so errors point at the exact file, line
//! and column.

use std::fmt::Write as _;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// Where a diagnostic points. `path` is a display path (project-relative,
/// forward slashes). `span` is a byte range into that file's source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    pub span: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub location: Option<Location>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    pub fn warning(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    fn new(severity: Severity, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            location: None,
            help: None,
        }
    }

    /// Attach a file location without a span.
    pub fn in_file(mut self, path: impl Into<String>) -> Self {
        self.location = Some(Location {
            path: path.into(),
            span: None,
        });
        self
    }

    /// Attach a file location with a byte span.
    pub fn at(mut self, path: impl Into<String>, span: Range<usize>) -> Self {
        self.location = Some(Location {
            path: path.into(),
            span: Some(span),
        });
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Render for a terminal. Pass the file's source text to get a snippet
    /// with a caret underline; without it only `path` (and no line) is shown.
    pub fn render(&self, source: Option<&str>) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}[{}]: {}",
            self.severity.label(),
            self.code,
            self.message
        );

        if let Some(loc) = &self.location {
            match (&loc.span, source) {
                (Some(span), Some(src)) => render_snippet(&mut out, &loc.path, span, src),
                _ => {
                    let _ = writeln!(out, "  --> {}", loc.path);
                }
            }
        }

        if let Some(help) = &self.help {
            let _ = writeln!(out, "   = help: {help}");
        }
        out
    }
}

fn render_snippet(out: &mut String, path: &str, span: &Range<usize>, src: &str) {
    let start = clamp_to_char_boundary(src, span.start);
    let end = clamp_to_char_boundary(src, span.end.max(start));
    let (line, col) = line_col(src, start);

    let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[start..].find('\n').map_or(src.len(), |i| start + i);
    let line_text = src[line_start..line_end].trim_end_matches('\r');

    // Underline only the part of the span on the first line, at least one caret.
    let underline_end = end.min(line_start + line_text.len());
    let width = src[start..underline_end.max(start)].chars().count().max(1);

    let gutter = line.to_string().len();
    let pad = " ".repeat(gutter);
    let _ = writeln!(out, "{pad}--> {path}:{line}:{col}");
    let _ = writeln!(out, "{pad} |");
    let _ = writeln!(out, "{line} | {line_text}");
    let _ = writeln!(out, "{pad} | {}{}", " ".repeat(col - 1), "^".repeat(width));
}

fn clamp_to_char_boundary(src: &str, mut offset: usize) -> usize {
    offset = offset.min(src.len());
    while !src.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// 1-based line and column (columns count characters, not bytes).
pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let offset = clamp_to_char_boundary(src, offset);
    let before = &src[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let col = before[line_start..].chars().count() + 1;
    (line, col)
}

/// Edit distance between two short ASCII-ish strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The closest candidate to `name`, if it is plausibly a typo
/// (distance at most 1 for short names, 2 otherwise). Ties keep the first.
pub fn suggest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let limit = if name.chars().count() <= 3 { 1 } else { 2 };
    candidates
        .into_iter()
        .map(|c| (levenshtein(name, c), c))
        .filter(|(d, _)| *d > 0 && *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions() {
        let c = ["gap", "padding", "background", "md", "lg"];
        assert_eq!(suggest("gapp", c), Some("gap"));
        assert_eq!(suggest("backgrond", c), Some("background"));
        assert_eq!(suggest("mdd", c), Some("md"));
        assert_eq!(suggest("xl", c), None); // 2 edits away from a 2-letter name
        assert_eq!(suggest("gap", c), None); // exact match is not a suggestion
        assert_eq!(suggest("zzzzzz", c), None);
    }

    #[test]
    fn line_col_counts_from_one() {
        let src = "ab\ncdé\nf";
        assert_eq!(line_col(src, 0), (1, 1));
        assert_eq!(line_col(src, 3), (2, 1));
        assert_eq!(line_col(src, 5), (2, 3)); // 'é' starts at byte 5
        assert_eq!(line_col(src, 8), (3, 1));
    }

    #[test]
    fn line_col_clamps_out_of_range_offsets() {
        assert_eq!(line_col("ab", 99), (1, 3));
    }

    #[test]
    fn renders_snippet_with_caret() {
        let src = "[project]\nid = \"bad\"\n";
        let d = Diagnostic::error("E0103", "invalid project id `bad`")
            .at("era.toml", 15..20)
            .with_help("use reverse-DNS form");
        let expected = "\
error[E0103]: invalid project id `bad`
 --> era.toml:2:6
  |
2 | id = \"bad\"
  |      ^^^^^
   = help: use reverse-DNS form
";
        assert_eq!(d.render(Some(src)), expected);
    }

    #[test]
    fn renders_path_only_without_span() {
        let d = Diagnostic::warning("W0204", "unexpected file").in_file("src/notes.txt");
        assert_eq!(
            d.render(None),
            "warning[W0204]: unexpected file\n  --> src/notes.txt\n"
        );
    }

    #[test]
    fn empty_span_still_shows_one_caret() {
        let d = Diagnostic::error("E0000", "x").at("f", 1..1);
        assert!(d.render(Some("abc")).contains("  |  ^\n"));
    }
}
