//! Lexer: source text to tokens. Newlines are significant (they separate
//! items), so they are emitted as tokens; runs of blank lines collapse to one.

use era_diagnostics::Diagnostic;

use crate::ast::{Color, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    App,
    Skin,
    Screen,
    Layout,
    Component,
    Store,
    Prop,
    Variant,
    When,
    Is,
    Slot,
    True,
    False,
    None,
}

impl Keyword {
    pub const ALL: &[(&str, Keyword)] = &[
        ("app", Keyword::App),
        ("skin", Keyword::Skin),
        ("screen", Keyword::Screen),
        ("layout", Keyword::Layout),
        ("component", Keyword::Component),
        ("store", Keyword::Store),
        ("prop", Keyword::Prop),
        ("variant", Keyword::Variant),
        ("when", Keyword::When),
        ("is", Keyword::Is),
        ("slot", Keyword::Slot),
        ("true", Keyword::True),
        ("false", Keyword::False),
        ("none", Keyword::None),
    ];

    pub fn as_str(self) -> &'static str {
        Self::ALL.iter().find(|(_, k)| *k == self).unwrap().0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Keyword(Keyword),
    String(String),
    Number(f64),
    Color(Color),
    LBrace,
    RBrace,
    Colon,
    Comma,
    Pipe,
    Dot,
    Eq,
    Question,
    Newline,
    Eof,
    /// Invalid input already reported by the lexer. The parser accepts it
    /// silently so one mistake does not produce a cascade of errors.
    Error,
}

impl TokenKind {
    /// How a token is named in "expected X, found Y" messages.
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(n) => format!("`{n}`"),
            TokenKind::Keyword(k) => format!("keyword `{}`", k.as_str()),
            TokenKind::String(_) => "a string".into(),
            TokenKind::Number(_) => "a number".into(),
            TokenKind::Color(_) => "a color".into(),
            TokenKind::LBrace => "`{`".into(),
            TokenKind::RBrace => "`}`".into(),
            TokenKind::Colon => "`:`".into(),
            TokenKind::Comma => "`,`".into(),
            TokenKind::Pipe => "`|`".into(),
            TokenKind::Dot => "`.`".into(),
            TokenKind::Eq => "`=`".into(),
            TokenKind::Question => "`?`".into(),
            TokenKind::Newline => "end of line".into(),
            TokenKind::Eof => "end of file".into(),
            TokenKind::Error => "invalid input".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    path: &'a str,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
}

/// Tokenize `src`. Always returns a token list ending in `Eof`; invalid input
/// produces diagnostics and is skipped so parsing can continue.
pub fn lex(src: &str, path: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lx = Lexer {
        src,
        pos: 0,
        path,
        tokens: Vec::new(),
        diags: Vec::new(),
    };
    lx.run();
    (lx.tokens, lx.diags)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut it = self.src[self.pos..].chars();
        it.next();
        it.next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn eat_while(&mut self, f: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&f) {
            self.bump();
        }
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start, self.pos),
        });
    }

    fn error(&mut self, code: &'static str, msg: impl Into<String>, span: Span) -> &mut Diagnostic {
        self.tokens.push(Token {
            kind: TokenKind::Error,
            span,
        });
        self.diags
            .push(Diagnostic::error(code, msg).at(self.path, span.range()));
        self.diags.last_mut().unwrap()
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                '\n' => {
                    self.bump();
                    let last_is_sep = matches!(
                        self.tokens.last().map(|t| &t.kind),
                        None | Some(TokenKind::Newline)
                    );
                    if !last_is_sep {
                        self.push(TokenKind::Newline, start);
                    }
                }
                ' ' | '\t' | '\r' => {
                    self.bump();
                }
                '/' if self.peek2() == Some('/') => self.eat_while(|c| c != '\n'),
                '{' => self.single(TokenKind::LBrace),
                '}' => self.single(TokenKind::RBrace),
                ':' => self.single(TokenKind::Colon),
                ',' => self.single(TokenKind::Comma),
                '|' => self.single(TokenKind::Pipe),
                '=' => self.single(TokenKind::Eq),
                '?' => self.single(TokenKind::Question),
                '"' => self.string(),
                '#' => self.color(),
                '-' if self.peek2().is_some_and(|c| c.is_ascii_digit()) => self.number(),
                c if c.is_ascii_digit() => self.number(),
                c if is_ident_start(c) => self.ident(),
                '.' if self.peek2().is_some_and(|c| c.is_ascii_digit()) => {
                    self.bump();
                    self.eat_while(|c| c.is_ascii_digit());
                    let text = &self.src[start + 1..self.pos];
                    let help = format!("write `0.{text}`");
                    self.error(
                        "E1005",
                        "a number cannot start with `.`",
                        Span::new(start, self.pos),
                    )
                    .help = Some(help);
                }
                '.' => self.single(TokenKind::Dot),
                other => {
                    self.bump();
                    let span = Span::new(start, self.pos);
                    let d = self.error("E1004", format!("unexpected character `{other}`"), span);
                    if !other.is_ascii() {
                        d.help = Some(
                            "names may only use ASCII letters, digits and `_`; put other text in quotes".into(),
                        );
                    }
                }
            }
        }
        let end = self.src.len();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(end, end),
        });
    }

    fn single(&mut self, kind: TokenKind) {
        let start = self.pos;
        self.bump();
        self.push(kind, start);
    }

    fn ident(&mut self) {
        let start = self.pos;
        self.eat_while(is_ident_continue);
        let text = &self.src[start..self.pos];
        let kind = match Keyword::ALL.iter().find(|(k, _)| *k == text) {
            Some((_, kw)) => TokenKind::Keyword(*kw),
            None => TokenKind::Ident(text.to_string()),
        };
        self.push(kind, start);
    }

    fn number(&mut self) {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        self.eat_while(|c| c.is_ascii_digit());
        if self.peek() == Some('.') {
            self.bump();
            let frac_start = self.pos;
            self.eat_while(|c| c.is_ascii_digit());
            if self.pos == frac_start {
                let text = self.src[start..self.pos].to_string();
                let help = format!("write `{text}0` or `{}`", text.trim_end_matches('.'));
                self.error(
                    "E1005",
                    "a number cannot end with `.`",
                    Span::new(start, self.pos),
                )
                .help = Some(help);
                return;
            }
        }
        // A number glued to letters (`16px`, `2x`) is almost certainly a unit.
        if self.peek().is_some_and(is_ident_start) {
            let unit_start = self.pos;
            self.eat_while(is_ident_continue);
            let unit = self.src[unit_start..self.pos].to_string();
            self.error(
                "E1006",
                format!("unexpected `{unit}` after number"),
                Span::new(start, self.pos),
            )
            .help = Some("ERA numbers have no units: sizes are dp, font sizes are sp".into());
            return;
        }
        let text = &self.src[start..self.pos];
        match text.parse::<f64>() {
            Ok(v) => self.push(TokenKind::Number(v), start),
            Err(_) => {
                self.error(
                    "E1005",
                    format!("invalid number `{text}`"),
                    Span::new(start, self.pos),
                );
            }
        }
    }

    fn color(&mut self) {
        let start = self.pos;
        self.bump(); // '#'
        self.eat_while(is_ident_continue);
        let digits = &self.src[start + 1..self.pos];
        let span = Span::new(start, self.pos);
        match parse_hex_color(digits) {
            Some(c) => self.push(TokenKind::Color(c), start),
            None => {
                let msg = format!("invalid color `#{digits}`");
                self.error("E1003", msg, span).help =
                    Some("colors are `#RGB`, `#RRGGBB` or `#RRGGBBAA` using 0-9 and A-F".into());
            }
        }
    }

    fn string(&mut self) {
        let start = self.pos;
        self.bump(); // opening quote
        let mut value = String::new();
        loop {
            let Some(c) = self.peek() else {
                return self.unterminated(start);
            };
            match c {
                '"' => {
                    self.bump();
                    break;
                }
                '\n' => return self.unterminated(start),
                '\\' => {
                    let esc_start = self.pos;
                    self.bump();
                    match self.bump() {
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('u') => match self.unicode_escape() {
                            Some(ch) => value.push(ch),
                            None => {
                                let span = Span::new(esc_start, self.pos);
                                self.error("E1002", "invalid unicode escape", span).help =
                                    Some("write it like `\\u{1F600}`".into());
                            }
                        },
                        other => {
                            let span = Span::new(esc_start, self.pos);
                            let shown = other.map(String::from).unwrap_or_default();
                            self.error("E1002", format!("unknown escape `\\{shown}`"), span)
                                .help = Some(r#"valid escapes: \" \\ \n \t \u{...}"#.into());
                        }
                    }
                }
                _ => {
                    value.push(c);
                    self.bump();
                }
            }
        }
        self.push(TokenKind::String(value), start);
    }

    fn unicode_escape(&mut self) -> Option<char> {
        if self.peek() != Some('{') {
            return None;
        }
        self.bump();
        let hex_start = self.pos;
        self.eat_while(|c| c.is_ascii_hexdigit());
        let hex = &self.src[hex_start..self.pos];
        if self.peek() != Some('}') {
            return None;
        }
        self.bump();
        if hex.is_empty() || hex.len() > 6 {
            return None;
        }
        char::from_u32(u32::from_str_radix(hex, 16).ok()?)
    }

    fn unterminated(&mut self, start: usize) {
        let span = Span::new(start, self.pos);
        self.error("E1001", "unterminated string", span).help =
            Some("add a closing `\"` before the end of the line".into());
    }
}

fn parse_hex_color(digits: &str) -> Option<Color> {
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    let nib = |i: usize| byte(&digits[i..=i].repeat(2));
    match digits.len() {
        3 => Some(Color {
            r: nib(0)?,
            g: nib(1)?,
            b: nib(2)?,
            a: 255,
        }),
        6 | 8 => Some(Color {
            r: byte(&digits[0..2])?,
            g: byte(&digits[2..4])?,
            b: byte(&digits[4..6])?,
            a: if digits.len() == 8 {
                byte(&digits[6..8])?
            } else {
                255
            },
        }),
        _ => None,
    }
}
