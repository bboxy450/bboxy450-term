// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Code and what specifies, models or proves it: [`den_check`] runs Den's own
//! checker over files, [`contracts`] reads the inventory under `verification/`
//! and the Den specs of the tree. Both only read.
use super::{Budget, Root, parse, text, with_cut};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;

// ---------- den_check ----------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DenCheck {
    paths: Vec<String>,
}

/// Den's verdict on `*.den` files: records, bounded results and diagnostics.
pub fn den_check(root: &Root, args: &Value) -> Result<Value, String> {
    let args: DenCheck = parse(args)?;
    if args.paths.is_empty() {
        return Err("give at least one path".into());
    }
    let (found, missing) = root.walk(&args.paths, Some("*.den"), None)?;
    let mut budget = Budget::new();
    let (mut out, mut sound, mut cut) = (vec![], 0usize, 0usize);
    for path in &found {
        let one = match text(path) {
            Some(body) => {
                let report = bboxy450_den::check(&body);
                sound += usize::from(report.is_sound());
                let records: Vec<Value> = report
                    .records
                    .iter()
                    .map(|r| {
                        json!({"kind": r.kind, "name": r.name, "intent": r.intent,
                            "states": r.states, "transitions": r.transitions,
                            "invariants": r.invariants.len(), "properties": r.properties.len(),
                            "theorems": r.theorems, "scenarios": r.scenarios})
                    })
                    .collect();
                let diagnostics: Vec<Value> = report
                    .diagnostics
                    .iter()
                    .map(|d| {
                        json!({"code": d.code, "line": d.line, "column": d.column,
                            "record": d.record, "message": d.message, "repair": d.repair})
                    })
                    .collect();
                json!({"path": root.show(path), "sound": report.is_sound(),
                    "records": records, "diagnostics": diagnostics})
            }
            None => json!({"path": root.show(path), "sound": false, "error": "not a text file"}),
        };
        if budget.spend(&one) {
            out.push(one);
        } else {
            cut += 1;
        }
    }
    // The totals count every file, also those whose detail was cut.
    let mut value = json!({"files": out,
        "totals": {"files": found.len(), "sound": sound, "unsound": found.len() - sound}});
    if !missing.is_empty() {
        value["missing"] = json!(missing);
    }
    Ok(with_cut(
        value,
        cut,
        "files did not fit; ask for fewer files or one directory at a time",
    ))
}

// ---------- contracts ----------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contracts {
    #[serde(default)]
    paths: Vec<String>,
    #[serde(default)]
    names: Vec<String>,
    /// Also the assumptions text and, for paths, each contract's sources.
    #[serde(default)]
    detail: bool,
}

// The three inventory files, read for the fields answered here; the rest of
// each row is ignored.
#[derive(Deserialize, Default)]
struct ContractsFile {
    #[serde(default)]
    contract: Vec<Contract>,
}
#[derive(Deserialize)]
struct Contract {
    name: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    models: Vec<String>,
    #[serde(default)]
    controls: Vec<Named>,
    #[serde(default)]
    sources: Vec<String>,
    #[serde(default)]
    implementation_tests: Vec<String>,
    #[serde(default)]
    assumptions: String,
    #[serde(default)]
    property: String,
}
#[derive(Deserialize)]
struct Named {
    name: String,
}
#[derive(Deserialize, Default)]
struct CoverageFile {
    #[serde(default, rename = "crate")]
    crates: Vec<Covered>,
    #[serde(default)]
    implementation_only: Vec<Covered>,
}
#[derive(Deserialize)]
struct Covered {
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    models: Vec<String>,
    #[serde(default)]
    sources: Vec<String>,
}
#[derive(Deserialize, Default)]
struct CorpusFile {
    #[serde(default)]
    model: Vec<Model>,
}
#[derive(Deserialize)]
struct Model {
    name: String,
    #[serde(default)]
    expected: String,
    #[serde(default)]
    frontend: String,
}
#[derive(Deserialize)]
struct Manifest {
    package: Option<Named>,
}

/// One inventory file; an absent one is empty, a malformed one is an error.
fn load<T: Default + for<'a> Deserialize<'a>>(root: &Root, name: &str) -> Result<T, String> {
    match std::fs::read_to_string(root.0.join("verification").join(name)) {
        Ok(body) => {
            toml::from_str(&body).map_err(|e| format!("verification/{name}: {}", e.message()))
        }
        Err(_) => Ok(T::default()),
    }
}

/// A Den file of the tree and the spec it declares, if it parses.
struct Den {
    path: String,
    body: String,
    spec: Option<String>,
}

impl Den {
    fn all(root: &Root) -> Vec<Self> {
        // Records live under .bboxy, which a walk of the root skips as hidden.
        let roots = [".".to_owned(), ".bboxy".to_owned()];
        let (found, _) = root.walk(&roots, Some("*.den"), None).unwrap_or_default();
        found
            .iter()
            .filter_map(|path| {
                let body = text(path)?;
                let spec = bboxy450_den::declarations(&body)
                    .ok()
                    .and_then(|all| all.into_iter().find(|d| d.kind == "spec"))
                    .map(|d| d.name);
                Some(Self {
                    path: root.show(path),
                    body,
                    spec,
                })
            })
            .collect()
    }
    fn row(&self) -> Value {
        let report = bboxy450_den::check(&self.body);
        let theorems: Vec<&String> = report.records.iter().flat_map(|r| &r.theorems).collect();
        json!({"path": self.path, "spec": self.spec, "sound": report.is_sound(), "theorems": theorems})
    }
}

/// `path` is `prefix` or lies under it.
fn under(path: &str, prefix: &str) -> bool {
    prefix.is_empty()
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The package a path belongs to: the nearest `Cargo.toml` with a `[package]`.
fn package(root: &Root, full: &Path) -> Option<String> {
    full.ancestors()
        .take_while(|dir| dir.starts_with(&root.0))
        .find_map(|dir| {
            let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
            Some(toml::from_str::<Manifest>(&manifest).ok()?.package?.name)
        })
}

/// What specifies, models or proves Rust paths, and what a contract, model or
/// Den spec name speaks for.
pub fn contracts(root: &Root, args: &Value) -> Result<Value, String> {
    let args: Contracts = parse(args)?;
    if args.paths.is_empty() && args.names.is_empty() {
        return Err("give at least one path or name".into());
    }
    if !root.0.join("verification").is_dir() {
        return Ok(json!({"unavailable": "no verification inventory under this root"}));
    }
    let inventory: ContractsFile = load(root, "contracts.toml")?;
    let coverage: CoverageFile = load(root, "coverage.toml")?;
    let corpus: CorpusFile = load(root, "corpus.toml")?;
    let dens = Den::all(root);
    let expected = |name: &str| {
        corpus
            .model
            .iter()
            .find(|m| m.name == name)
            .map(|m| m.expected.as_str())
    };
    let row = |contract: &Contract, sources: bool| {
        let models: Vec<Value> = contract
            .models
            .iter()
            .map(|m| json!({"name": m, "expected": expected(m)}))
            .collect();
        // Rows without a controls table list their broken controls among the
        // models: the ones the corpus expects to fail.
        let controls: Vec<&str> = if contract.controls.is_empty() {
            contract
                .models
                .iter()
                .map(String::as_str)
                .filter(|m| expected(m).is_some_and(|e| e != "pass"))
                .collect()
        } else {
            contract.controls.iter().map(|c| c.name.as_str()).collect()
        };
        let mut value = json!({"name": contract.name, "title": contract.title,
            "property": contract.property, "models": models, "controls": controls,
            "tests": contract.implementation_tests});
        if sources || args.detail {
            value["sources"] = json!(contract.sources);
        }
        if args.detail {
            value["assumptions"] = json!(contract.assumptions);
        }
        if let Some(den) = dens
            .iter()
            .find(|d| d.spec.as_deref() == Some(&contract.name))
        {
            value["den"] = json!(den.path);
        }
        value
    };
    let mut budget = Budget::new();
    let (mut paths, mut names, mut missing, mut cut) = (vec![], vec![], vec![], 0usize);
    // Spend the answer's frame, then as many of its contracts as still fit.
    let mut fill = |mut one: Value, rows: Vec<Value>, into: &mut Vec<Value>| {
        if !budget.spend(&one) {
            cut += 1 + rows.len();
            return;
        }
        for row in rows {
            if budget.spend(&row) {
                one["contracts"].as_array_mut().unwrap().push(row);
            } else {
                cut += 1;
            }
        }
        into.push(one);
    };
    for given in &args.paths {
        let Some(full) = root.resolve(given).ok().filter(|full| full.exists()) else {
            missing.push(given);
            continue;
        };
        let path = root.show(&full);
        // A source may be the path, a file under it, or a directory holding it.
        let touches =
            |sources: &[String]| sources.iter().any(|s| under(s, &path) || under(&path, s));
        let rows: Vec<Value> = inventory
            .contract
            .iter()
            .filter(|c| touches(&c.sources))
            .map(|c| row(c, false))
            .collect();
        let only: Vec<&str> = coverage
            .implementation_only
            .iter()
            .filter(|c| touches(&c.sources))
            .map(|c| c.name.as_str())
            .collect();
        let mentions: Vec<Value> = dens
            .iter()
            .filter(|d| !path.is_empty() && d.body.contains(&path))
            .map(Den::row)
            .collect();
        let krate = package(root, &full);
        let covered = coverage
            .crates
            .iter()
            .find(|c| Some(&c.name) == krate.as_ref());
        let mut one = json!({"path": path, "contracts": [], "den": mentions});
        if let Some(c) = covered {
            one["coverage"] =
                json!({"crate": c.name, "status": c.status, "models": c.models.len()});
        }
        if !only.is_empty() {
            one["implementation_only"] = json!(only);
        }
        if rows.is_empty() && covered.is_none() && only.is_empty() {
            one["uncovered"] = json!(true);
        }
        fill(one, rows, &mut paths);
    }
    for name in &args.names {
        let rows: Vec<Value> = inventory
            .contract
            .iter()
            .filter(|c| {
                c.name == *name
                    || c.models.contains(name)
                    || c.controls.iter().any(|k| k.name == *name)
            })
            .map(|c| row(c, true))
            .collect();
        let crates: Vec<&str> = coverage
            .crates
            .iter()
            .filter(|c| c.models.contains(name))
            .map(|c| c.name.as_str())
            .collect();
        let model = corpus.model.iter().find(|m| m.name == *name);
        let den = dens.iter().find(|d| d.spec.as_ref() == Some(name));
        let mut one = json!({"name": name, "contracts": []});
        if let Some(m) = model {
            one["model"] = json!({"expected": m.expected, "frontend": m.frontend});
        }
        if !crates.is_empty() {
            one["crates"] = json!(crates);
        }
        if let Some(den) = den {
            one["den"] = den.row();
        }
        if rows.is_empty() && model.is_none() && crates.is_empty() && den.is_none() {
            one["unknown"] = json!(true);
        }
        fill(one, rows, &mut names);
    }
    let mut value = json!({"paths": paths, "names": names});
    if !missing.is_empty() {
        value["missing"] = json!(missing);
    }
    Ok(with_cut(
        value,
        cut,
        "more contracts than fit; ask for one file instead of a directory, fewer paths, or a contract by name",
    ))
}
