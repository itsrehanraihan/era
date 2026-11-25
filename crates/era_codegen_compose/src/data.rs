//! `EraData.kt`: mock data lists from `Data/*.json` (D-091) as Kotlin data
//! classes and lists, read by `for (p in EraData.Products)`.

use std::fmt::Write as _;

use era_ir::ir::{App, StateType};

use crate::GENERATED_HEADER;
use crate::kotlin::ident;
use crate::stores::literal;

/// `None` when the app has no data lists.
pub fn generate(app: &App, package: &str) -> Option<String> {
    if app.data.is_empty() {
        return None;
    }
    let mut o = format!("{GENERATED_HEADER}\npackage {package}\n");
    for list in &app.data {
        let fields: Vec<String> = list
            .fields
            .iter()
            .map(|(n, t)| {
                let ty = match t {
                    StateType::Text => "String",
                    StateType::Number => "Double",
                    StateType::Bool => "Boolean",
                };
                format!("val {}: {ty}", ident(n))
            })
            .collect();
        let _ = writeln!(
            o,
            "\n/** One item of `{}` ({}). */\ndata class {}Item({})",
            list.name,
            list.file,
            list.name,
            fields.join(", ")
        );
    }
    o.push_str("\nobject EraData {\n");
    for list in &app.data {
        let _ = writeln!(o, "    val {0}: List<{0}Item> = listOf(", list.name);
        for row in &list.rows {
            let args: Vec<String> = list
                .fields
                .iter()
                .zip(row)
                .map(|((n, _), v)| format!("{} = {}", ident(n), literal(v)))
                .collect();
            let _ = writeln!(o, "        {}Item({}),", list.name, args.join(", "));
        }
        o.push_str("    )\n");
    }
    o.push_str("}\n");
    Some(o)
}
