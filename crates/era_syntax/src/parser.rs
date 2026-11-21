//! Recursive-descent parser for the uniform node grammar (docs/GRAMMAR.md).
//!
//! The parser recovers from errors: a bad item is skipped up to the next
//! separator, and a bad declaration up to the next top-level keyword, so one
//! run reports as many independent problems as possible.

use era_diagnostics::Diagnostic;

use crate::ast::*;
use crate::lexer::{Keyword, Token, TokenKind, lex};

/// Parse a whole file. Always returns a (possibly partial) AST together with
/// every lexical and syntax diagnostic.
pub fn parse(src: &str, path: &str) -> (File, Vec<Diagnostic>) {
    let (tokens, mut diags) = lex(src, path);
    let mut p = Parser {
        tokens,
        pos: 0,
        path,
        diags: Vec::new(),
    };
    let file = p.file();
    diags.extend(p.diags);
    diags.sort_by_key(|d| {
        d.location
            .as_ref()
            .and_then(|l| l.span.clone())
            .map(|s| s.start)
    });
    (file, diags)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    path: &'a str,
    diags: Vec<Diagnostic>,
}

const DECL_KEYWORDS: &[(Keyword, DeclKind)] = &[
    (Keyword::App, DeclKind::App),
    (Keyword::Skin, DeclKind::Skin),
    (Keyword::Screen, DeclKind::Screen),
    (Keyword::Layout, DeclKind::Layout),
    (Keyword::Component, DeclKind::Component),
    (Keyword::Store, DeclKind::Store),
];

fn decl_kind(kind: &TokenKind) -> Option<DeclKind> {
    match kind {
        TokenKind::Keyword(k) => DECL_KEYWORDS
            .iter()
            .find(|(kw, _)| kw == k)
            .map(|(_, d)| *d),
        _ => None,
    }
}

impl Parser<'_> {
    // ----- token helpers -------------------------------------------------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_kind_at(&self, n: usize) -> &TokenKind {
        let i = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[i].kind
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, kind: &TokenKind) -> Option<Token> {
        (self.peek_kind() == kind).then(|| self.bump())
    }

    fn prev_end(&self) -> usize {
        self.pos
            .checked_sub(1)
            .map_or(0, |i| self.tokens[i].span.end)
    }

    /// Report an error at the current token, unless the lexer already
    /// reported a problem there.
    fn error_here(&mut self, code: &'static str, msg: String, help: Option<String>) {
        if *self.peek_kind() == TokenKind::Error {
            return;
        }
        let span = self.peek().span;
        self.error_at(code, msg, span, help);
    }

    fn error_at(&mut self, code: &'static str, msg: String, span: Span, help: Option<String>) {
        let mut d = Diagnostic::error(code, msg).at(self.path, span.range());
        d.help = help;
        self.diags.push(d);
    }

    fn expected(&mut self, what: &str) {
        let found = self.peek_kind().describe();
        self.error_here("E1101", format!("expected {what}, found {found}"), None);
    }

    /// Identifier used as a name (not a keyword).
    fn expect_name(&mut self, what: &str) -> Option<Ident> {
        match self.peek_kind().clone() {
            TokenKind::Ident(name) => {
                let span = self.bump().span;
                Some(Ident { name, span })
            }
            TokenKind::Keyword(k) => {
                self.error_here(
                    "E1107",
                    format!(
                        "`{}` is a reserved word and cannot be used as {what}",
                        k.as_str()
                    ),
                    Some("choose a different name".into()),
                );
                None
            }
            _ => {
                self.expected(what);
                None
            }
        }
    }

    fn expect(&mut self, kind: TokenKind, what: &str) -> Option<Token> {
        if let Some(t) = self.eat(&kind) {
            return Some(t);
        }
        self.expected(what);
        None
    }

    fn skip_separators(&mut self) {
        while matches!(self.peek_kind(), TokenKind::Newline | TokenKind::Comma) {
            self.bump();
        }
    }

    /// Skip a balanced `{ ... }` starting at the current `{`.
    fn skip_braced(&mut self) {
        let mut depth = 0usize;
        loop {
            match self.peek_kind() {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        self.bump();
                        return;
                    }
                }
                TokenKind::Eof => return,
                _ => {}
            }
            self.bump();
        }
    }

    /// Recover inside a block: skip to the next separator or closing brace.
    fn recover_item(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Comma | TokenKind::RBrace | TokenKind::Eof => {
                    return;
                }
                TokenKind::LBrace => self.skip_braced(),
                _ => {
                    self.bump();
                }
            }
        }
    }

    /// Recover at file level: skip to the next declaration keyword.
    fn recover_decl(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Eof => return,
                k if decl_kind(k).is_some() => return,
                TokenKind::LBrace => self.skip_braced(),
                _ => {
                    self.bump();
                }
            }
        }
    }

    // ----- grammar ---------------------------------------------------------

    fn file(&mut self) -> File {
        let mut decls = Vec::new();
        loop {
            while matches!(self.peek_kind(), TokenKind::Newline) {
                self.bump();
            }
            match self.peek_kind().clone() {
                TokenKind::Eof => break,
                k if decl_kind(&k).is_some() => {
                    if let Some(d) = self.decl() {
                        decls.push(d);
                    }
                }
                TokenKind::Ident(name) => {
                    let help = if name == "use" {
                        "skins are selected by name inside `app`, e.g. `skin: Studio`".to_string()
                    } else {
                        "UI elements belong inside a `screen`, `layout` or `component`".to_string()
                    };
                    self.error_here(
                        "E1101",
                        format!("expected a declaration, found `{name}`"),
                        Some(help),
                    );
                    self.bump();
                    self.recover_decl();
                }
                _ => {
                    self.expected(
                        "a declaration (`app`, `skin`, `screen`, `layout` or `component`)",
                    );
                    self.bump();
                    self.recover_decl();
                }
            }
        }
        File { decls }
    }

    fn decl(&mut self) -> Option<Decl> {
        let kw = self.bump();
        let kind = decl_kind(&kw.kind).expect("called on a declaration keyword");

        let name = if kind == DeclKind::App {
            match self.peek_kind().clone() {
                TokenKind::String(name) => {
                    let span = self.bump().span;
                    Some(Ident { name, span })
                }
                _ => {
                    let found = self.peek_kind().describe();
                    self.error_here(
                        "E1101",
                        format!("expected the app's display name in quotes, found {found}"),
                        Some("write it like `app \"My App\" { ... }`".into()),
                    );
                    None
                }
            }
        } else {
            self.expect_name(&format!("a {} name", kind.keyword()))
        };
        let Some(name) = name else {
            self.recover_decl();
            return None;
        };

        let Some(open) = self.eat(&TokenKind::LBrace) else {
            let found = self.peek_kind().describe();
            self.error_here(
                "E1101",
                format!(
                    "expected `{{` after `{} {}`, found {found}",
                    kind.keyword(),
                    name.name
                ),
                None,
            );
            self.recover_decl();
            return None;
        };
        let body = if kind == DeclKind::Store {
            self.store_body(open)
        } else {
            self.block_body(open)
        };
        Some(Decl {
            kind,
            span: kw.span.to(body.span),
            name,
            body,
        })
    }

    /// Parse items after an already-consumed `{`, through the closing `}`.
    fn block_body(&mut self, open: Token) -> Block {
        let mut items = Vec::new();
        loop {
            self.skip_separators();
            match self.peek_kind() {
                TokenKind::RBrace => {
                    let close = self.bump();
                    return Block {
                        items,
                        span: open.span.to(close.span),
                    };
                }
                TokenKind::Eof => {
                    self.error_at(
                        "E1102",
                        "unclosed `{`".into(),
                        open.span,
                        Some("add a matching `}`".into()),
                    );
                    return Block {
                        items,
                        span: Span::new(open.span.start, self.prev_end()),
                    };
                }
                _ => {}
            }

            match self.item() {
                Some(item) => {
                    let after = self.peek_kind();
                    if !matches!(
                        after,
                        TokenKind::Newline | TokenKind::Comma | TokenKind::RBrace | TokenKind::Eof
                    ) {
                        let found = after.describe();
                        self.error_here(
                            "E1103",
                            format!("expected a new line or `,` after {}, found {found}", item.describe()),
                            Some("put each property and element on its own line, or separate them with `,`".into()),
                        );
                        self.recover_item();
                    }
                    items.push(item);
                }
                None => self.recover_item(),
            }
        }
    }

    fn item(&mut self) -> Option<Item> {
        let tok = self.peek().clone();
        // Any word followed by `:` is a property, even a keyword (`skin: Studio`).
        let word = match &tok.kind {
            TokenKind::Ident(n) => Some(n.clone()),
            TokenKind::Keyword(k) => Some(k.as_str().to_string()),
            _ => None,
        };
        if let Some(name) = word.filter(|_| *self.peek_kind_at(1) == TokenKind::Colon) {
            self.bump();
            self.bump();
            let name = Ident {
                name,
                span: tok.span,
            };
            let value = self.value(&format!("a value for `{}`", name.name))?;
            return Some(Item::Property(Property {
                span: tok.span.to(value.span),
                name,
                value,
            }));
        }

        // Early drafts used `use skin "path"`; point to the approved form.
        if tok.kind == TokenKind::Ident("use".into())
            && *self.peek_kind_at(1) == TokenKind::Keyword(Keyword::Skin)
        {
            self.error_here(
                "E1108",
                "`use skin` is not ERA syntax".into(),
                Some("select a skin by name with `skin: Studio`; files in `Skins/` load automatically".into()),
            );
            return None;
        }

        match tok.kind {
            TokenKind::Keyword(Keyword::Prop) => self.prop_decl().map(Item::Prop),
            TokenKind::Keyword(Keyword::Variant) => self.variant_decl().map(Item::Variant),
            TokenKind::Keyword(Keyword::When) => self.when_block().map(Item::When),
            TokenKind::Keyword(Keyword::Slot) => {
                self.bump();
                Some(Item::Slot(tok.span))
            }
            TokenKind::Ident(_) => self.node().map(Item::Node),
            ref k if decl_kind(k).is_some() => {
                let kw = decl_kind(k).unwrap().keyword();
                self.error_here(
                    "E1104",
                    format!("`{kw}` declarations must be at the top level of a file"),
                    Some(format!(
                        "move this `{kw}` outside of the surrounding `{{ }}`"
                    )),
                );
                // Skip the whole nested declaration.
                while !matches!(
                    self.peek_kind(),
                    TokenKind::LBrace | TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
                ) {
                    self.bump();
                }
                if *self.peek_kind() == TokenKind::LBrace {
                    self.skip_braced();
                }
                None
            }
            _ => {
                self.expected("a property or element");
                None
            }
        }
    }

    fn node(&mut self) -> Option<Node> {
        let name_tok = self.bump();
        let TokenKind::Ident(name) = name_tok.kind else {
            unreachable!("node() is called on an identifier")
        };
        let name = Ident {
            name,
            span: name_tok.span,
        };

        let arg = match self.peek_kind().clone() {
            TokenKind::String(s) => Some(self.simple_value(ValueKind::String(s))),
            TokenKind::Number(n) => Some(self.simple_value(ValueKind::Number(n))),
            TokenKind::Color(c) => Some(self.simple_value(ValueKind::Color(c))),
            TokenKind::Ident(_) => self.path_or_ident(),
            TokenKind::Error => {
                self.bump();
                return None;
            }
            _ => None,
        };

        let block = self
            .eat(&TokenKind::LBrace)
            .map(|open| self.block_body(open));

        let end = block
            .as_ref()
            .map(|b| b.span)
            .or(arg.as_ref().map(|a| a.span))
            .unwrap_or(name.span);
        Some(Node {
            span: name.span.to(end),
            name,
            arg,
            block,
        })
    }

    fn simple_value(&mut self, kind: ValueKind) -> Value {
        let span = self.bump().span;
        Value { kind, span }
    }

    /// A value, optionally chained with `then` (M6 actions).
    fn value(&mut self, what: &str) -> Option<Value> {
        let first = self.single_value(what)?;
        if !self.at_word("then") {
            return Some(first);
        }
        let mut parts = vec![first];
        while self.at_word("then") {
            self.bump();
            parts.push(self.single_value("an action after `then`")?);
        }
        let span = parts[0].span.to(parts.last().unwrap().span);
        Some(Value {
            kind: ValueKind::Seq(parts),
            span,
        })
    }

    fn at_word(&self, word: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(w) if w == word)
    }

    /// `name` or `Store.field` (a dotted path).
    fn path_or_ident(&mut self) -> Option<Value> {
        let TokenKind::Ident(first) = self.peek_kind().clone() else {
            return None;
        };
        let first_tok = self.bump();
        if *self.peek_kind() != TokenKind::Dot {
            return Some(Value {
                kind: ValueKind::Ident(first),
                span: first_tok.span,
            });
        }
        let mut parts = vec![Ident {
            name: first,
            span: first_tok.span,
        }];
        while self.eat(&TokenKind::Dot).is_some() {
            parts.push(self.expect_name("a name after `.`")?);
        }
        let span = first_tok.span.to(parts.last().unwrap().span);
        Some(Value {
            kind: ValueKind::Path(parts),
            span,
        })
    }

    fn single_value(&mut self, what: &str) -> Option<Value> {
        // `go Screen`
        if self.at_word("go") && matches!(self.peek_kind_at(1), TokenKind::Ident(_)) {
            let start = self.bump().span;
            let target = self.expect_name("a screen name after `go`")?;
            let span = start.to(target.span);
            return Some(Value {
                kind: ValueKind::Go(target),
                span,
            });
        }
        if matches!(self.peek_kind(), TokenKind::Ident(_)) {
            return self.path_or_ident();
        }
        let kind = match self.peek_kind().clone() {
            TokenKind::String(s) => ValueKind::String(s),
            TokenKind::Number(n) => ValueKind::Number(n),
            TokenKind::Color(c) => ValueKind::Color(c),
            TokenKind::Ident(i) => ValueKind::Ident(i),
            TokenKind::Keyword(Keyword::True) => ValueKind::Bool(true),
            TokenKind::Keyword(Keyword::False) => ValueKind::Bool(false),
            TokenKind::Keyword(Keyword::None) => ValueKind::None,
            TokenKind::Error => {
                self.bump();
                return None;
            }
            TokenKind::LBrace => {
                self.error_here(
                    "E1105",
                    format!("expected {what}, found `{{`"),
                    Some("a property holds one value; for nested content drop the `:`".into()),
                );
                return None;
            }
            _ => {
                self.expected(what);
                return None;
            }
        };
        Some(self.simple_value(kind))
    }

    /// Body of a `store`: `state` and `action` declarations only.
    fn store_body(&mut self, open: Token) -> Block {
        let mut items = Vec::new();
        loop {
            self.skip_separators();
            match self.peek_kind().clone() {
                TokenKind::RBrace => {
                    let close = self.bump();
                    return Block {
                        items,
                        span: open.span.to(close.span),
                    };
                }
                TokenKind::Eof => {
                    self.error_at(
                        "E1102",
                        "unclosed `{`".into(),
                        open.span,
                        Some("add a matching `}`".into()),
                    );
                    return Block {
                        items,
                        span: Span::new(open.span.start, self.prev_end()),
                    };
                }
                TokenKind::Ident(w) if w == "state" => match self.state_decl() {
                    Some(s) => items.push(Item::State(s)),
                    None => self.recover_item(),
                },
                TokenKind::Ident(w) if w == "action" => match self.action_decl() {
                    Some(a) => items.push(Item::Action(a)),
                    None => self.recover_item(),
                },
                _ => {
                    let found = self.peek_kind().describe();
                    self.error_here(
                        "E1109",
                        format!("expected `state` or `action` in a store, found {found}"),
                        Some(
                            "a store holds `state name: type = value` and `action name { ... }`"
                                .into(),
                        ),
                    );
                    self.bump();
                    self.recover_item();
                }
            }
        }
    }

    fn state_decl(&mut self) -> Option<StateDecl> {
        let start = self.bump().span;
        let name = self.expect_name("a state name")?;
        self.expect(
            TokenKind::Colon,
            &format!("`:` and a type after `state {}`", name.name),
        )?;
        let ty = self.expect_name("a state type (`text`, `number` or `bool`)")?;
        let mut end = ty.span;
        let initial = if self.eat(&TokenKind::Eq).is_some() {
            let v = self.single_value("an initial value")?;
            end = v.span;
            Some(v)
        } else {
            None
        };
        Some(StateDecl {
            name,
            ty,
            initial,
            span: start.to(end),
        })
    }

    fn action_decl(&mut self) -> Option<ActionDecl> {
        let start = self.bump().span;
        let name = self.expect_name("an action name")?;
        let open = self.expect(
            TokenKind::LBrace,
            &format!("`{{` after `action {}`", name.name),
        )?;
        let mut steps = Vec::new();
        loop {
            self.skip_separators();
            match self.peek_kind().clone() {
                TokenKind::RBrace => {
                    let close = self.bump();
                    return Some(ActionDecl {
                        name,
                        steps,
                        span: start.to(close.span),
                    });
                }
                TokenKind::Eof => {
                    self.error_at(
                        "E1102",
                        "unclosed `{`".into(),
                        open.span,
                        Some("add a matching `}`".into()),
                    );
                    return None;
                }
                _ => match self.step() {
                    Some(s) => steps.push(s),
                    None => self.recover_item(),
                },
            }
        }
    }

    /// `set x = v`, `toggle x` or `add x n`.
    fn step(&mut self) -> Option<Step> {
        let word = match self.peek_kind() {
            TokenKind::Ident(w) => w.clone(),
            _ => String::new(),
        };
        match word.as_str() {
            "set" => {
                let start = self.bump().span;
                let field = self.expect_name("a state name after `set`")?;
                self.expect(TokenKind::Eq, &format!("`=` after `set {}`", field.name))?;
                let value = self.single_value("a value")?;
                let span = start.to(value.span);
                Some(Step {
                    kind: StepKind::Set { field, value },
                    span,
                })
            }
            "toggle" => {
                let start = self.bump().span;
                let field = self.expect_name("a state name after `toggle`")?;
                let span = start.to(field.span);
                Some(Step {
                    kind: StepKind::Toggle { field },
                    span,
                })
            }
            "add" => {
                let start = self.bump().span;
                let field = self.expect_name("a state name after `add`")?;
                let amount = self.single_value("an amount")?;
                let span = start.to(amount.span);
                Some(Step {
                    kind: StepKind::Add { field, amount },
                    span,
                })
            }
            _ => {
                let found = self.peek_kind().describe();
                self.error_here(
                    "E1110",
                    format!("expected an action step, found {found}"),
                    Some("steps are `set name = value`, `toggle name` and `add name 1`".into()),
                );
                None
            }
        }
    }

    fn prop_decl(&mut self) -> Option<PropDecl> {
        let start = self.bump().span;
        let name = self.expect_name("a prop name")?;
        self.expect(
            TokenKind::Colon,
            &format!("`:` and a type after `prop {}`", name.name),
        )?;
        let ty_ident = self.expect_name("a prop type")?;
        let Some(ty) = PropType::from_name(&ty_ident.name) else {
            let types: Vec<_> = PropType::ALL.iter().map(|(n, _)| *n).collect();
            self.error_at(
                "E1106",
                format!("unknown prop type `{}`", ty_ident.name),
                ty_ident.span,
                Some(format!("prop types are: {}", types.join(", "))),
            );
            return None;
        };
        let mut end = ty_ident.span;
        let optional = match self.eat(&TokenKind::Question) {
            Some(q) => {
                end = q.span;
                true
            }
            None => false,
        };
        let default = if self.eat(&TokenKind::Eq).is_some() {
            let v = self.value("a default value")?;
            end = v.span;
            Some(v)
        } else {
            None
        };
        Some(PropDecl {
            name,
            ty,
            ty_span: ty_ident.span,
            optional,
            default,
            span: start.to(end),
        })
    }

    fn variant_decl(&mut self) -> Option<VariantDecl> {
        let start = self.bump().span;
        let name = self.expect_name("a variant name")?;
        self.expect(
            TokenKind::Colon,
            &format!("`:` and options after `variant {}`", name.name),
        )?;
        let mut options = vec![self.expect_name("a variant option")?];
        while self.eat(&TokenKind::Pipe).is_some() {
            options.push(self.expect_name("a variant option after `|`")?);
        }
        let mut end = options.last().unwrap().span;
        let default = if self.eat(&TokenKind::Eq).is_some() {
            let d = self.expect_name("the default option")?;
            end = d.span;
            Some(d)
        } else {
            None
        };
        Some(VariantDecl {
            name,
            options,
            default,
            span: start.to(end),
        })
    }

    fn when_block(&mut self) -> Option<When> {
        let start = self.bump().span;
        let variant = self.expect_name("a variant name after `when`")?;
        self.expect(
            TokenKind::Keyword(Keyword::Is),
            &format!("`is` after `when {}`", variant.name),
        )?;
        let option = self.expect_name("a variant option after `is`")?;
        let open = self.expect(TokenKind::LBrace, "`{` to start the `when` block")?;
        let block = self.block_body(open);
        Some(When {
            span: start.to(block.span),
            variant,
            option,
            block,
        })
    }
}
