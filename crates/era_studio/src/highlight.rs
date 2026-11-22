//! ERA syntax colouring, built on ERA's own lexer so it always matches the
//! language. Colour literals are shown on a swatch of their own colour.

use era_syntax::{TokenKind, lex};

/// What a stretch of text is, for colouring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Plain,
    Comment,
    Keyword,
    /// PascalCase: your screens, components, layouts, skins and stores.
    Declared,
    /// snake_case words: elements, properties and tokens.
    Word,
    /// A word directly before `:` (a property or token name).
    Property,
    String,
    Number,
    /// A colour literal, with its RGBA value.
    Color(u8, u8, u8, u8),
    Punct,
    /// Invalid input the lexer rejected.
    Error,
}

/// Split `text` into `(byte range, class)` covering the whole text.
pub fn classify(text: &str) -> Vec<(std::ops::Range<usize>, Class)> {
    let (tokens, _) = lex(text, "");
    let mut out = Vec::new();
    let mut pos = 0;
    for (i, t) in tokens.iter().enumerate() {
        let span = t.span.range();
        if span.start > pos {
            gap(text, pos, span.start, &mut out);
        }
        if span.is_empty() {
            continue;
        }
        let next_is_colon = matches!(tokens.get(i + 1).map(|n| &n.kind), Some(TokenKind::Colon));
        let class = match &t.kind {
            TokenKind::Keyword(_) if next_is_colon => Class::Property,
            TokenKind::Keyword(_) => Class::Keyword,
            TokenKind::Ident(_) if next_is_colon => Class::Property,
            TokenKind::Ident(w) if w.starts_with(|c: char| c.is_ascii_uppercase()) => {
                Class::Declared
            }
            TokenKind::Ident(w) if CONTEXTUAL.contains(&w.as_str()) => Class::Keyword,
            TokenKind::Ident(_) => Class::Word,
            TokenKind::String(_) => Class::String,
            TokenKind::Number(_) | TokenKind::Dimension(..) => Class::Number,
            TokenKind::Color(c) => Class::Color(c.r, c.g, c.b, c.a),
            TokenKind::Error => Class::Error,
            TokenKind::Newline | TokenKind::Eof => Class::Plain,
            _ => Class::Punct,
        };
        out.push((span.clone(), class));
        pos = span.end;
    }
    if pos < text.len() {
        gap(text, pos, text.len(), &mut out);
    }
    out
}

/// Contextual words from M6 that read as keywords.
const CONTEXTUAL: &[&str] = &[
    "state", "action", "set", "toggle", "add", "go", "back", "then",
];

/// Text between tokens is whitespace or `//` comments.
fn gap(text: &str, start: usize, end: usize, out: &mut Vec<(std::ops::Range<usize>, Class)>) {
    let slice = &text[start..end];
    match slice.find("//") {
        Some(i) => {
            if i > 0 {
                out.push((start..start + i, Class::Plain));
            }
            let comment_end = slice[i..].find('\n').map_or(end, |n| start + i + n);
            out.push((start + i..comment_end, Class::Comment));
            if comment_end < end {
                gap(text, comment_end, end, out);
            }
        }
        None => out.push((start..end, Class::Plain)),
    }
}

/// Theme colours `(r, g, b)` for a class, for a dark or light editor.
pub fn color(class: Class, dark: bool) -> [u8; 3] {
    let (d, l) = match class {
        Class::Plain | Class::Punct => ([212, 212, 212], [36, 41, 46]),
        Class::Comment => ([106, 153, 85], [106, 115, 125]),
        Class::Keyword => ([197, 134, 192], [175, 0, 219]),
        Class::Declared => ([78, 201, 176], [38, 127, 153]),
        Class::Word => ([220, 220, 170], [121, 94, 38]),
        Class::Property => ([156, 220, 254], [0, 92, 197]),
        Class::String => ([206, 145, 120], [163, 21, 21]),
        Class::Number => ([181, 206, 168], [9, 134, 88]),
        Class::Color(r, g, b, _) => {
            // Readable text on a swatch of the colour itself.
            let lum = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            let c = if lum > 140.0 {
                [0, 0, 0]
            } else {
                [255, 255, 255]
            };
            (c, c)
        }
        Class::Error => ([244, 71, 71], [205, 49, 49]),
    };
    if dark { d } else { l }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(text: &str) -> Vec<(&str, Class)> {
        classify(text)
            .into_iter()
            .map(|(r, c)| (&text[r], c))
            .filter(|(s, _)| !s.trim().is_empty())
            .collect()
    }

    #[test]
    fn covers_text_without_gaps() {
        let text = "screen Home { // hi\n  text \"x\" { color: #FF0000 }\n}\n";
        let spans = classify(text);
        let mut pos = 0;
        for (r, _) in &spans {
            assert_eq!(r.start, pos, "{spans:?}");
            pos = r.end;
        }
        assert_eq!(pos, text.len());
    }

    #[test]
    fn classes_are_sensible() {
        let got = classes(
            "screen Home { gap: md\n AppButton { label: \"Go\" } // note\n on_press: go Menu then back, x: #FFF, n: 2, skin: S }",
        );
        assert!(got.contains(&("screen", Class::Keyword)));
        assert!(got.contains(&("Home", Class::Declared)));
        assert!(got.contains(&("gap", Class::Property)));
        assert!(got.contains(&("md", Class::Word)));
        assert!(got.contains(&("\"Go\"", Class::String)));
        assert!(got.contains(&("// note", Class::Comment)));
        assert!(got.contains(&("go", Class::Keyword)));
        assert!(got.contains(&("then", Class::Keyword)));
        assert!(got.contains(&("#FFF", Class::Color(255, 255, 255, 255))));
        assert!(got.contains(&("2", Class::Number)));
        assert!(got.contains(&("skin", Class::Property)));
    }

    #[test]
    fn errors_and_unicode() {
        let got = classes("text \"é\" { gap: 16qq; width: 50% }");
        assert!(got.contains(&("16qq", Class::Error)), "{got:?}");
        assert!(got.contains(&("50%", Class::Number)), "{got:?}");
        assert!(got.contains(&("\"é\"", Class::String)));
    }

    #[test]
    fn swatch_text_contrasts() {
        assert_eq!(color(Class::Color(255, 255, 255, 255), true), [0, 0, 0]);
        assert_eq!(color(Class::Color(0, 0, 0, 255), false), [255, 255, 255]);
    }
}
