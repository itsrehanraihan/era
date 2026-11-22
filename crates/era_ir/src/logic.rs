//! Stores (app state and actions, M6) and the UI-side lookups into them:
//! `Store.field` references, `Store.action` calls and `{Store.field}` text
//! interpolation. Logic never refers to screens; navigation stays in the UI.

use std::collections::BTreeMap;

use era_diagnostics::{Diagnostic, suggest};
use era_syntax::{Decl, Ident, Item, Span, StepKind, Value as AstValue, ValueKind};

use crate::ir::*;

pub struct StoreInfo {
    pub fields: BTreeMap<String, StateType>,
    pub actions: Vec<String>,
}

#[derive(Default)]
pub struct Stores {
    pub map: BTreeMap<String, StoreInfo>,
}

fn hint(name: &str, candidates: &[&str], what: &str) -> Option<String> {
    match suggest(name, candidates.iter().copied()) {
        Some(s) => Some(format!("did you mean `{s}`?")),
        None if candidates.is_empty() => None,
        None => Some(format!("{what}: {}", candidates.join(", "))),
    }
}

fn err(
    path: &str,
    code: &'static str,
    msg: String,
    span: Span,
    help: Option<String>,
) -> Diagnostic {
    let mut d = Diagnostic::error(code, msg).at(path, span.range());
    d.help = help;
    d
}

fn state_type(name: &str) -> Option<StateType> {
    match name {
        "text" => Some(StateType::Text),
        "number" => Some(StateType::Number),
        "bool" => Some(StateType::Bool),
        _ => None,
    }
}

fn literal(v: &AstValue) -> Option<Literal> {
    match &v.kind {
        ValueKind::String(s) => Some(Literal::Text(s.clone())),
        ValueKind::Number(n) => Some(Literal::Number(*n)),
        ValueKind::Bool(b) => Some(Literal::Bool(*b)),
        _ => None,
    }
}

fn literal_type(l: &Literal) -> StateType {
    match l {
        Literal::Text(_) => StateType::Text,
        Literal::Number(_) => StateType::Number,
        Literal::Bool(_) => StateType::Bool,
    }
}

/// Build every store. `decls` are `(file path, store declaration)`.
pub fn build_stores(decls: &[(&str, &Decl)]) -> (Vec<Store>, Stores, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut stores = Vec::new();
    let mut table = Stores::default();

    for (path, decl) in decls {
        let mut fields = Vec::new();
        let mut types = BTreeMap::new();
        for item in &decl.body.items {
            let Item::State(s) = item else { continue };
            let Some(ty) = state_type(&s.ty.name) else {
                diags.push(err(
                    path,
                    "E2170",
                    format!("unknown state type `{}`", s.ty.name),
                    s.ty.span,
                    hint(&s.ty.name, &["text", "number", "bool"], "state types"),
                ));
                continue;
            };
            let initial = match &s.initial {
                None => match ty {
                    StateType::Text => Literal::Text(String::new()),
                    StateType::Number => Literal::Number(0.0),
                    StateType::Bool => Literal::Bool(false),
                },
                Some(v) => match literal(v) {
                    Some(l) if literal_type(&l) == ty => l,
                    _ => {
                        diags.push(err(
                            path,
                            "E2171",
                            format!(
                                "`{}` is {} state, so its initial value must be {}",
                                s.name.name,
                                ty.name(),
                                example(ty)
                            ),
                            v.span,
                            None,
                        ));
                        continue;
                    }
                },
            };
            types.insert(s.name.name.clone(), ty);
            fields.push(StateField {
                name: s.name.name.clone(),
                ty,
                initial,
                persist: s.persist,
            });
        }

        let mut actions = Vec::new();
        for item in &decl.body.items {
            let Item::Action(a) = item else { continue };
            let mut steps = Vec::new();
            for step in &a.steps {
                let field_of = |f: &Ident, diags: &mut Vec<Diagnostic>| -> Option<StateType> {
                    let t = types.get(&f.name).copied();
                    if t.is_none() {
                        let names: Vec<&str> = types.keys().map(String::as_str).collect();
                        diags.push(err(
                            path,
                            "E2172",
                            format!("`{}` has no state `{}`", decl.name.name, f.name),
                            f.span,
                            hint(&f.name, &names, "its state"),
                        ));
                    }
                    t
                };
                match &step.kind {
                    StepKind::Set { field, value } => {
                        let Some(ty) = field_of(field, &mut diags) else {
                            continue;
                        };
                        let v = match (&value.kind, literal(value)) {
                            (_, Some(l)) if literal_type(&l) == ty => StepValue::Literal(l),
                            (ValueKind::Ident(other), _) if types.get(other) == Some(&ty) => {
                                StepValue::Field(other.clone())
                            }
                            _ => {
                                diags.push(err(
                                    path,
                                    "E2173",
                                    format!(
                                        "`{}` is {} state; set it to {} or another {} state",
                                        field.name,
                                        ty.name(),
                                        example(ty),
                                        ty.name()
                                    ),
                                    value.span,
                                    None,
                                ));
                                continue;
                            }
                        };
                        steps.push(ActionStep::Set {
                            field: field.name.clone(),
                            value: v,
                        });
                    }
                    StepKind::Toggle { field } => {
                        let Some(ty) = field_of(field, &mut diags) else {
                            continue;
                        };
                        if ty != StateType::Bool {
                            diags.push(err(
                                path,
                                "E2173",
                                format!(
                                    "`toggle` needs bool state, but `{}` is {}",
                                    field.name,
                                    ty.name()
                                ),
                                step.span,
                                None,
                            ));
                            continue;
                        }
                        steps.push(ActionStep::Toggle {
                            field: field.name.clone(),
                        });
                    }
                    StepKind::Add { field, amount } => {
                        let Some(ty) = field_of(field, &mut diags) else {
                            continue;
                        };
                        let ValueKind::Number(n) = amount.kind else {
                            diags.push(err(
                                path,
                                "E2173",
                                "`add` needs a number, like `add count 1`".into(),
                                amount.span,
                                None,
                            ));
                            continue;
                        };
                        if ty != StateType::Number {
                            diags.push(err(
                                path,
                                "E2173",
                                format!(
                                    "`add` needs number state, but `{}` is {}",
                                    field.name,
                                    ty.name()
                                ),
                                step.span,
                                None,
                            ));
                            continue;
                        }
                        steps.push(ActionStep::Add {
                            field: field.name.clone(),
                            amount: n,
                        });
                    }
                }
            }
            actions.push(StoreAction {
                name: a.name.name.clone(),
                steps,
            });
        }

        table.map.insert(
            decl.name.name.clone(),
            StoreInfo {
                fields: types,
                actions: actions.iter().map(|a| a.name.clone()).collect(),
            },
        );
        stores.push(Store {
            name: decl.name.name.clone(),
            fields,
            actions,
        });
    }
    stores.sort_by(|a, b| a.name.cmp(&b.name));
    (stores, table, diags)
}

fn example(ty: StateType) -> &'static str {
    match ty {
        StateType::Text => "text in quotes",
        StateType::Number => "a number",
        StateType::Bool => "`true` or `false`",
    }
}

impl Stores {
    fn store<'a>(&'a self, name: &Ident, path: &str) -> Result<&'a StoreInfo, Diagnostic> {
        self.map.get(&name.name).ok_or_else(|| {
            let names: Vec<&str> = self.map.keys().map(String::as_str).collect();
            let help = hint(&name.name, &names, "stores").or_else(|| {
                Some("declare it in a logic file, e.g. `src/Logic/session.er`: `store Session { ... }`".into())
            });
            err(path, "E2161", format!("unknown store `{}`", name.name), name.span, help)
        })
    }

    /// `Store.field`
    pub fn state_ref(&self, parts: &[Ident], path: &str) -> Result<StateRef, Diagnostic> {
        let [store, field] = parts else {
            let span = parts[0].span.to(parts.last().unwrap().span);
            return Err(err(
                path,
                "E2162",
                "state is referenced as `Store.field`".into(),
                span,
                None,
            ));
        };
        let info = self.store(store, path)?;
        match info.fields.get(&field.name) {
            Some(ty) => Ok(StateRef {
                store: store.name.clone(),
                field: field.name.clone(),
                ty: *ty,
            }),
            None if info.actions.contains(&field.name) => Err(err(
                path,
                "E2162",
                format!("`{}.{}` is an action, not state", store.name, field.name),
                field.span,
                Some("actions go in `on_press`".into()),
            )),
            None => {
                let names: Vec<&str> = info.fields.keys().map(String::as_str).collect();
                Err(err(
                    path,
                    "E2162",
                    format!("`{}` has no state `{}`", store.name, field.name),
                    field.span,
                    hint(&field.name, &names, "its state"),
                ))
            }
        }
    }

    /// `Store.action`
    pub fn action(&self, parts: &[Ident], path: &str) -> Result<Effect, Diagnostic> {
        let [store, action] = parts else {
            let span = parts[0].span.to(parts.last().unwrap().span);
            return Err(err(
                path,
                "E2163",
                "actions are called as `Store.action`".into(),
                span,
                None,
            ));
        };
        let info = self.store(store, path)?;
        if info.actions.contains(&action.name) {
            return Ok(Effect::Call {
                store: store.name.clone(),
                action: action.name.clone(),
            });
        }
        let names: Vec<&str> = info.actions.iter().map(String::as_str).collect();
        let help = if info.fields.contains_key(&action.name) {
            Some(format!(
                "`{}` is state; add an action that changes it",
                action.name
            ))
        } else {
            hint(&action.name, &names, "its actions")
        };
        Err(err(
            path,
            "E2163",
            format!("`{}` has no action `{}`", store.name, action.name),
            action.span,
            help,
        ))
    }

    /// Split `"Code: {Session.code}"` into parts. `{{` and `}}` are literal braces.
    pub fn template(
        &self,
        text: &str,
        span: Span,
        path: &str,
    ) -> Result<Vec<TextPart>, Diagnostic> {
        let mut parts = Vec::new();
        let mut lit = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.peek() == Some(&'{') => {
                    chars.next();
                    lit.push('{');
                }
                '}' if chars.peek() == Some(&'}') => {
                    chars.next();
                    lit.push('}');
                }
                '{' => {
                    let mut inner = String::new();
                    let mut closed = false;
                    for c in chars.by_ref() {
                        if c == '}' {
                            closed = true;
                            break;
                        }
                        inner.push(c);
                    }
                    if !closed {
                        return Err(err(
                            path,
                            "E2160",
                            "unclosed `{` in text".into(),
                            span,
                            Some(
                                "write `{Store.field}` to show state, or `{{` for a literal brace"
                                    .into(),
                            ),
                        ));
                    }
                    let names: Vec<Ident> = inner
                        .trim()
                        .split('.')
                        .map(|n| Ident {
                            name: n.trim().to_string(),
                            span,
                        })
                        .collect();
                    if names.iter().any(|n| n.name.is_empty()) {
                        return Err(err(
                            path,
                            "E2160",
                            format!("`{{{inner}}}` is not a state reference"),
                            span,
                            Some("write `{Store.field}`, or `{{` for a literal brace".into()),
                        ));
                    }
                    let r = self.state_ref(&names, path)?;
                    if !lit.is_empty() {
                        parts.push(TextPart::Lit(std::mem::take(&mut lit)));
                    }
                    parts.push(TextPart::State(r));
                }
                c => lit.push(c),
            }
        }
        if !lit.is_empty() || parts.is_empty() {
            parts.push(TextPart::Lit(lit));
        }
        Ok(parts)
    }
}

/// `[Lit("a")]` -> `Some("a")`: text with no state is plain text.
pub fn plain(parts: &[TextPart]) -> Option<String> {
    match parts {
        [TextPart::Lit(s)] => Some(s.clone()),
        _ => None,
    }
}
