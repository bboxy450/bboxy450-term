use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

#[derive(Default)]
struct Generator {
    definitions: BTreeMap<String, Vec<(String, String)>>,
}

fn camel(path: &str) -> String {
    path.split(['.', '_'])
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            chars.next().unwrap().to_uppercase().collect::<String>() + chars.as_str()
        })
        .collect()
}

fn ident(key: &str) -> String {
    assert!(
        key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "invalid token key {key}"
    );
    match key {
        "type" | "match" | "where" | "ref" | "loop" | "move" | "mod" | "fn" | "const"
        | "static" | "use" | "struct" | "enum" | "impl" | "trait" | "pub" | "in" | "for" | "as"
        | "async" | "await" | "gen" => format!("r#{key}"),
        _ => key.to_owned(),
    }
}

impl Generator {
    fn value(&mut self, path: &str, value: &Value) -> (String, String) {
        match value {
            Value::Bool(v) => ("bool".into(), v.to_string()),
            Value::Number(v) => {
                let v = v
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .expect("finite numeric token");
                ("f64".into(), format!("{v:?}"))
            }
            Value::String(v) => ("&'static str".into(), format!("{v:?}")),
            Value::Array(values) => {
                assert!(
                    !values.is_empty(),
                    "empty token arrays need an explicit schema: {path}"
                );
                let items: Vec<_> = values.iter().map(|v| self.value(path, v)).collect();
                let kind = &items[0].0;
                assert!(
                    items.iter().all(|(t, _)| t == kind),
                    "mixed token array: {path}"
                );
                (
                    format!("&'static [{kind}]"),
                    format!(
                        "&[{}]",
                        items
                            .iter()
                            .map(|(_, v)| v.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                )
            }
            Value::Object(object) => {
                let name = if path.starts_with("Tokens.density.") {
                    "DensityTokens".to_owned()
                } else if matches!(path, "Tokens.color.dark" | "Tokens.color.light") {
                    "Palette".to_owned()
                } else {
                    camel(path)
                };
                let mut fields = Vec::new();
                let mut initializers = Vec::new();
                for (key, child) in object {
                    let (kind, value) = self.value(&format!("{path}.{key}"), child);
                    fields.push((key.clone(), kind));
                    initializers.push(format!("{}: {value}", ident(key)));
                }
                if let Some(previous) = self.definitions.get(&name) {
                    assert_eq!(previous, &fields, "token schemas disagree for {name}");
                } else {
                    self.definitions.insert(name.clone(), fields);
                }
                (
                    name.clone(),
                    format!("{name} {{ {} }}", initializers.join(",")),
                )
            }
            Value::Null => panic!("null token without a schema: {path}"),
        }
    }

    fn definitions(&self) -> String {
        let mut out = String::new();
        for (name, fields) in &self.definitions {
            out.push_str(&format!(
                "#[derive(Debug, Clone, Copy, PartialEq)]\npub struct {name} {{\n"
            ));
            for (field, kind) in fields {
                out.push_str(&format!("pub {}: {kind},\n", ident(field)));
            }
            out.push_str("}\n");
            // Reflection is compiled only in tests: it reads the generated
            // fields themselves, rather than comparing a copied JSON string.
            out.push_str(&format!("#[cfg(test)] impl super::Visit for {name} {{ fn visit(&self, path: &str, out: &mut std::collections::BTreeMap<String, super::Scalar>) {{\n"));
            let keys = fields
                .iter()
                .map(|(field, _)| format!("String::from({field:?})"))
                .collect::<Vec<_>>()
                .join(",");
            out.push_str(&format!(
                "out.insert(path.into(), super::Scalar::ObjectKeys(vec![{keys}]));\n"
            ));
            for (field, _) in fields {
                out.push_str(&format!(
                    "super::Visit::visit(&self.{}, &format!(\"{{}}.{field}\", path), out);\n",
                    ident(field)
                ));
            }
            out.push_str("}}\n");
        }
        out
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let tokens_path = root.join("design/tokens.toml");
    let style_path = root.join("design/style.json");
    println!("cargo:rerun-if-changed={}", tokens_path.display());
    println!("cargo:rerun-if-changed={}", style_path.display());
    let tokens: toml::Value = toml::from_str(&fs::read_to_string(tokens_path)?)?;
    let tokens = serde_json::to_value(tokens)?;
    let style: Value = serde_json::from_str(&fs::read_to_string(style_path)?)?;
    let mut generator = Generator::default();
    let (_, tokens) = generator.value("Tokens", &tokens);
    let (_, style) = generator.value("Style", &style);
    let out = format!(
        "// Generated from design/tokens.toml and design/style.json.\n{}\npub const TOKENS: Tokens = {tokens};\npub const STYLE: Style = {style};\n",
        generator.definitions()
    );
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("tokens.rs"),
        out,
    )?;
    Ok(())
}
