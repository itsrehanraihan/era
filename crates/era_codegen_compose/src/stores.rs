//! `EraStores.kt`: each ERA `store` becomes a Kotlin `object` whose fields
//! are Compose state, so screens update when an action changes them (M6).

use std::fmt::Write as _;

use era_ir::ir::{ActionStep, App, Literal, StateType, StepValue, Store};

use crate::GENERATED_HEADER;
use crate::kotlin::{ident, num, string};

/// Numbers are `Double` in Kotlin: `3` -> `3.0`.
fn double(x: f64) -> String {
    let n = num(x);
    if n.contains('.') || n.contains('e') {
        n
    } else if let Some(inner) = n.strip_prefix('(').and_then(|v| v.strip_suffix(')')) {
        format!("({inner}.0)")
    } else {
        format!("{n}.0")
    }
}

fn literal(l: &Literal) -> String {
    match l {
        Literal::Text(t) => string(t),
        Literal::Number(n) => double(*n),
        Literal::Bool(b) => b.to_string(),
    }
}

fn store(o: &mut String, st: &Store) {
    let _ = writeln!(o, "/** `store {}` */\nobject {} {{", st.name, st.name);
    for f in &st.fields {
        let ty = match f.ty {
            StateType::Text => "String",
            StateType::Number => "Double",
            StateType::Bool => "Boolean",
        };
        let _ = writeln!(
            o,
            "    var {}: {ty} by mutableStateOf({})",
            ident(&f.name),
            literal(&f.initial)
        );
    }
    for a in &st.actions {
        let _ = writeln!(o, "\n    fun {}() {{", ident(&a.name));
        for step in &a.steps {
            let line = match step {
                ActionStep::Set { field, value } => format!(
                    "{} = {}",
                    ident(field),
                    match value {
                        StepValue::Literal(l) => literal(l),
                        StepValue::Field(f) => ident(f),
                    }
                ),
                ActionStep::Toggle { field } => format!("{0} = !{0}", ident(field)),
                ActionStep::Add { field, amount } => {
                    format!("{} += {}", ident(field), double(*amount))
                }
            };
            let _ = writeln!(o, "        {line}");
        }
        o.push_str("    }\n");
    }
    o.push_str("}\n");
}

/// `None` when the app has no stores.
pub fn generate(app: &App, package: &str) -> Option<String> {
    if app.stores.is_empty() {
        return None;
    }
    let mut o = format!(
        "{GENERATED_HEADER}
package {package}

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
"
    );
    for st in &app.stores {
        o.push('\n');
        store(&mut o, st);
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles() {
        assert_eq!(double(0.0), "0.0");
        assert_eq!(double(1.5), "1.5");
        assert_eq!(double(-2.0), "(-2.0)");
    }
}
