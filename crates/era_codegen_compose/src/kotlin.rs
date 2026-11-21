//! Kotlin source helpers: identifiers, literals and number formatting.

use era_ir::ir::Color;

const KEYWORDS: &[&str] = &[
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
    "while",
];

/// `on_primary` -> `onPrimary`; Kotlin keywords are escaped with backticks.
pub fn ident(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in snake.chars() {
        if ch == '_' {
            upper = !out.is_empty();
        } else if upper {
            out.push(ch.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    if KEYWORDS.contains(&out.as_str()) {
        format!("`{out}`")
    } else {
        out
    }
}

/// A Kotlin string literal.
pub fn string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `Color(0xAARRGGBB)`
pub fn color(c: &Color) -> String {
    format!("Color(0x{:02X}{:02X}{:02X}{:02X})", c.a, c.r, c.g, c.b)
}

/// A number with no trailing `.0`; negative numbers are parenthesised so
/// `(-0.5).sp` parses.
pub fn num(x: f64) -> String {
    let s = if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    };
    if x < 0.0 { format!("({s})") } else { s }
}

pub fn dp(x: f64) -> String {
    format!("{}.dp", num(x))
}

pub fn sp(x: f64) -> String {
    format!("{}.sp", num(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers() {
        assert_eq!(ident("on_primary"), "onPrimary");
        assert_eq!(ident("body"), "body");
        assert_eq!(ident("title_lg"), "titleLg");
        assert_eq!(ident("x2_large"), "x2Large");
        assert_eq!(ident("object"), "`object`");
    }

    #[test]
    fn strings_escape_kotlin_specials() {
        assert_eq!(string("Hi"), "\"Hi\"");
        assert_eq!(string("a \"b\" \\ $5\nnext"), r#""a \"b\" \\ \$5\nnext""#);
        assert_eq!(string("Didn’t · é"), "\"Didn’t · é\"");
    }

    #[test]
    fn numbers_and_units() {
        assert_eq!(dp(16.0), "16.dp");
        assert_eq!(dp(1.5), "1.5.dp");
        assert_eq!(sp(-0.6), "(-0.6).sp");
        assert_eq!(
            color(&Color {
                r: 255,
                g: 107,
                b: 53,
                a: 255
            }),
            "Color(0xFFFF6B35)"
        );
        assert_eq!(
            color(&Color {
                r: 0,
                g: 0,
                b: 0,
                a: 128
            }),
            "Color(0x80000000)"
        );
    }
}
