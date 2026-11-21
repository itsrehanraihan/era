use era_syntax::ast::Color;
use era_syntax::lexer::Keyword;
use era_syntax::{TokenKind as T, lex};

fn kinds(src: &str) -> Vec<T> {
    let (tokens, diags) = lex(src, "t.era");
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    tokens.into_iter().map(|t| t.kind).collect()
}

fn error_codes(src: &str) -> Vec<&'static str> {
    lex(src, "t.era").1.iter().map(|d| d.code).collect()
}

fn id(s: &str) -> T {
    T::Ident(s.into())
}

#[test]
fn punctuation_identifiers_and_keywords() {
    assert_eq!(
        kinds("screen Home { title: \"Hi\" }"),
        [
            T::Keyword(Keyword::Screen),
            id("Home"),
            T::LBrace,
            id("title"),
            T::Colon,
            T::String("Hi".into()),
            T::RBrace,
            T::Eof
        ]
    );
    assert_eq!(
        kinds("variant size: a | b = a ?,"),
        [
            T::Keyword(Keyword::Variant),
            id("size"),
            T::Colon,
            id("a"),
            T::Pipe,
            id("b"),
            T::Eq,
            id("a"),
            T::Question,
            T::Comma,
            T::Eof
        ]
    );
}

#[test]
fn every_keyword_is_recognised() {
    for (text, kw) in Keyword::ALL {
        assert_eq!(kinds(text), [T::Keyword(*kw), T::Eof]);
    }
    // Keywords are case-sensitive; prefixes are identifiers.
    assert_eq!(
        kinds("Screen screens"),
        [id("Screen"), id("screens"), T::Eof]
    );
}

#[test]
fn newlines_collapse_and_comments_are_skipped() {
    assert_eq!(
        kinds("\n\n// header\na // trailing\n\n\r\n  b\n"),
        [id("a"), T::Newline, id("b"), T::Newline, T::Eof]
    );
}

#[test]
fn numbers() {
    assert_eq!(
        kinds("0 16 1.25 -0.5 999"),
        [
            T::Number(0.0),
            T::Number(16.0),
            T::Number(1.25),
            T::Number(-0.5),
            T::Number(999.0),
            T::Eof
        ]
    );
}

#[test]
fn colors() {
    let c = |r, g, b, a| T::Color(Color { r, g, b, a });
    assert_eq!(
        kinds("#FF6B35 #fff #00000080"),
        [
            c(255, 107, 53, 255),
            c(255, 255, 255, 255),
            c(0, 0, 0, 128),
            T::Eof
        ]
    );
}

#[test]
fn strings_and_escapes() {
    assert_eq!(
        kinds(r#""a \"b\" \\ \n \t \u{1F600} é""#),
        [T::String("a \"b\" \\ \n \t \u{1F600} é".into()), T::Eof]
    );
}

#[test]
fn spans_are_byte_ranges() {
    let (tokens, _) = lex("ab  \"é\"", "t.era");
    assert_eq!(tokens[0].span.range(), 0..2);
    assert_eq!(tokens[1].span.range(), 4..8); // é is 2 bytes
}

#[test]
fn invalid_input_reports_and_continues() {
    assert_eq!(error_codes("\"open"), ["E1001"]);
    assert_eq!(error_codes("\"line\nbreak\""), ["E1001", "E1001"]);
    assert_eq!(error_codes(r#""\q""#), ["E1002"]);
    assert_eq!(error_codes(r#""\u{110000}""#), ["E1002"]);
    assert_eq!(error_codes("#12345 #GGG #"), ["E1003", "E1003", "E1003"]);
    assert_eq!(error_codes("@ $"), ["E1004", "E1004"]);
    assert_eq!(error_codes("1. .5"), ["E1005", "E1005"]);
    assert_eq!(error_codes("16px"), ["E1006"]);
}

#[test]
fn errors_leave_an_error_token() {
    let (tokens, _) = lex("a 16px b", "t.era");
    let k: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(k, [id("a"), T::Error, id("b"), T::Eof]);
}

#[test]
fn non_ascii_identifier_gets_help() {
    let (_, diags) = lex("café", "t.era");
    assert_eq!(diags[0].code, "E1004");
    assert!(diags[0].help.as_ref().unwrap().contains("ASCII"));
}
