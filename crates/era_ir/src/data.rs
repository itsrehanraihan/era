//! Mock data lists (D-091): every `Data/<name>.json` file is a JSON array of
//! flat objects and becomes a list named in PascalCase (`products.json` ->
//! `Products`), shown with `for p in Products { ... }`.

use std::path::Path;

use era_diagnostics::Diagnostic;

use crate::ir::{DataList, Literal, StateType};

/// `menu_items` / `menu-items` -> `MenuItems`.
pub fn list_name(stem: &str) -> String {
    stem.split(['_', '-', ' ', '.'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn valid_field(name: &str) -> bool {
    let mut c = name.chars();
    c.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

/// Load every `Data/*.json` file, sorted by name.
pub fn load(root: &Path) -> (Vec<DataList>, Vec<Diagnostic>) {
    let mut lists = Vec::new();
    let mut diags = Vec::new();
    let Ok(dir) = std::fs::read_dir(root.join("Data")) else {
        return (lists, diags);
    };
    let mut files: Vec<_> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    for path in files {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let rel = format!("Data/{}", path.file_name().unwrap().to_string_lossy());
        let bad = |msg: String| {
            Diagnostic::error("E2180", msg)
                .in_file(rel.clone())
                .with_help("a data file is a JSON array of flat objects, like `[{\"name\": \"Tea\", \"price\": 3}]`")
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                diags.push(bad(format!("cannot read `{rel}`: {e}")));
                continue;
            }
        };
        let json: serde_json::Value = match serde_json::from_str(&text) {
            Ok(j) => j,
            Err(e) => {
                diags.push(bad(format!(
                    "`{rel}` is not valid JSON: {e} (line {}, column {})",
                    e.line(),
                    e.column()
                )));
                continue;
            }
        };
        let Some(items) = json.as_array() else {
            diags.push(bad(format!("`{rel}` must be a JSON array")));
            continue;
        };
        let mut fields: Vec<(String, StateType)> = Vec::new();
        let mut ok = true;
        for (i, item) in items.iter().enumerate() {
            let Some(obj) = item.as_object() else {
                diags.push(bad(format!("item {} in `{rel}` is not an object", i + 1)));
                ok = false;
                break;
            };
            for (k, v) in obj {
                let ty = match v {
                    serde_json::Value::String(_) => StateType::Text,
                    serde_json::Value::Number(_) => StateType::Number,
                    serde_json::Value::Bool(_) => StateType::Bool,
                    serde_json::Value::Null => continue,
                    _ => {
                        diags.push(bad(format!(
                            "`{k}` in item {} of `{rel}` is a nested object or array; only text, numbers and true/false are supported",
                            i + 1
                        )));
                        ok = false;
                        continue;
                    }
                };
                if !valid_field(k) {
                    diags.push(bad(format!(
                        "`{k}` in `{rel}` is not a valid field name (letters, digits and `_`)"
                    )));
                    ok = false;
                    continue;
                }
                match fields.iter().find(|(n, _)| n == k) {
                    Some((_, t)) if *t != ty => {
                        diags.push(bad(format!(
                            "`{k}` is {} in one item of `{rel}` and {} in item {}",
                            t.name(),
                            ty.name(),
                            i + 1
                        )));
                        ok = false;
                    }
                    Some(_) => {}
                    None => fields.push((k.clone(), ty)),
                }
            }
        }
        if !ok {
            continue;
        }
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        let rows = items
            .iter()
            .map(|item| {
                let obj = item.as_object().unwrap();
                fields
                    .iter()
                    .map(|(k, ty)| match (obj.get(k), ty) {
                        (Some(serde_json::Value::String(s)), _) => Literal::Text(s.clone()),
                        (Some(serde_json::Value::Number(n)), _) => {
                            Literal::Number(n.as_f64().unwrap_or(0.0))
                        }
                        (Some(serde_json::Value::Bool(b)), _) => Literal::Bool(*b),
                        (_, StateType::Text) => Literal::Text(String::new()),
                        (_, StateType::Number) => Literal::Number(0.0),
                        (_, StateType::Bool) => Literal::Bool(false),
                    })
                    .collect()
            })
            .collect();
        lists.push(DataList {
            name: list_name(&stem),
            file: rel,
            fields,
            rows,
        });
    }
    (lists, diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(list_name("products"), "Products");
        assert_eq!(list_name("menu_items"), "MenuItems");
        assert_eq!(list_name("top-picks"), "TopPicks");
    }

    #[test]
    fn loads_typed_rows_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Data")).unwrap();
        std::fs::write(
            dir.path().join("Data/products.json"),
            r#"[{"name": "Tea", "price": 3, "hot": true}, {"name": "Juice", "price": 4.5}]"#,
        )
        .unwrap();
        let (lists, diags) = load(dir.path());
        assert!(diags.is_empty(), "{diags:?}");
        let l = &lists[0];
        assert_eq!(l.name, "Products");
        let names: Vec<&str> = l.fields.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["hot", "name", "price"]);
        assert_eq!(l.rows[1][0], Literal::Bool(false));
        assert_eq!(l.rows[1][2], Literal::Number(4.5));
    }

    #[test]
    fn reports_bad_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Data")).unwrap();
        std::fs::write(dir.path().join("Data/a.json"), "{}").unwrap();
        std::fs::write(
            dir.path().join("Data/b.json"),
            r#"[{"x": 1}, {"x": "one"}]"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("Data/c.json"), "[1,").unwrap();
        let (lists, diags) = load(dir.path());
        assert!(lists.is_empty());
        assert_eq!(diags.len(), 3);
        assert!(diags.iter().all(|d| d.code == "E2180"));
    }
}
