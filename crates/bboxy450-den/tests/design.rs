// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `tests/design`: three machines that share one enum through an import and
//! use `define`, `table`, `open` and `source`, checked the way an author
//! does it, through the CLI; and their broken copies in `tests/unsound/design`.
#![expect(
    clippy::disallowed_methods,
    reason = "Standalone checker fixtures perform synchronous filesystem and process I/O; no UI thread exists here."
)]
use bboxy450_den::{check, check_file, check_with, file_resolver, lean_with, render};
use std::path::PathBuf;
use std::process::Command;

fn dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
}
fn den(args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-den"))
        .args(args)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn the_design_directory_is_sound_and_says_so_in_one_line() {
    let design = dir("design");
    let (code, stdout, stderr) = den(&["check", design.to_str().unwrap()]);
    assert_eq!((code, stderr.as_str()), (Some(0), ""), "{stdout}");
    let mut states = 0;
    let mut transitions = 0;
    let mut open = Vec::new();
    for file in ["elements.den", "gather.den", "seal.den", "speak.den"] {
        let report = check_file(&design.join(file), None).unwrap();
        assert!(report.is_sound(), "{file}: {}", report.to_text());
        states += report.records[0].states;
        transitions += report.records[0].transitions;
        open.extend(report.records[0].open.iter().map(|(id, _)| id.clone()));
    }
    assert_eq!(
        stdout,
        format!("4 specs sound ({states} states, {transitions} transitions)\n")
    );
    assert_eq!(open, ["GATHER_Q1", "SEAL_Q1", "SPEAK_Q1"]);
}

#[test]
fn broken_copies_are_reported_per_file_with_names_in_the_trace() {
    let (design, broken) = (dir("design"), dir("unsound").join("design"));
    let (code, stdout, stderr) =
        den(&["check", broken.to_str().unwrap(), design.to_str().unwrap()]);
    assert_eq!((code, stderr.as_str()), (Some(1), ""), "{stdout}");
    let lines: Vec<&str> = stdout
        .lines()
        .map(|line| {
            line.strip_prefix(&format!("{}/", broken.display()))
                .unwrap_or(line)
        })
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();
    let root = root.ancestors().find(|d| d.join(".git").exists()).unwrap();
    assert_eq!(
        lines,
        [
            "X202-imported-enum-redeclared.den:8:8 X202 Element is declared twice in this scope. Repair: give each declaration a unique name".to_string(),
            "X203-unused-import.den:4:15 X203 Gather is declared but never used. Repair: use the declaration or remove it".to_string(),
            format!(
                "X351-unresolved-import.den:3:8 X351 import design.Elemental does not resolve: no .den file declares module design.Elemental in {} or its parents up to {}. Repair: correct the module path, or pass the directory that holds it with --root",
                broken.canonicalize().unwrap().display(),
                root.display()
            ),
            "X601-any-word-answers.den:28:13 X601 invariant SPEAK_001 is false at {said = Element.Fire, turns = 1}; shortest trace: Speak(Element.Fire). Repair: repair the transition or the invariant".to_string(),
            "X601-any-word-answers.den:29:13 X601 invariant SPEAK_002 is false at {said = Element.Water, turns = 2}; shortest trace: Speak(Element.Fire) -> Speak(Element.Water). Repair: repair the transition or the invariant".to_string(),
            "4 sound, 4 unsound".to_string(),
        ]
    );
    // Each broken copy alone: exactly the code in its name.
    for entry in std::fs::read_dir(&broken).unwrap() {
        let path = entry.unwrap().path();
        let report = check_file(&path, None).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(!report.diagnostics.is_empty(), "{name}");
        assert!(
            report.diagnostics.iter().all(|d| d.code == &name[..4]),
            "{name}: {}",
            report.to_text()
        );
    }
}

#[test]
fn imports_resolve_by_module_line_up_to_the_root() {
    let design = dir("design");
    let seal = design.join("seal.den");
    let source = std::fs::read_to_string(&seal).unwrap();
    // The string API has no files: the imported enum is not declared there.
    let alone = check(&source);
    assert_eq!(alone.diagnostics[0].code, "X201", "{}", alone.to_text());
    assert_eq!(alone.diagnostics[0].message, "Element is not declared");
    // A resolver is any function from a module path to its source.
    let elements = std::fs::read_to_string(design.join("elements.den")).unwrap();
    let from_memory = |module: &[String]| {
        if module.join(".") == "design.Elements" {
            Ok(elements.clone())
        } else {
            Err("this resolver knows only design.Elements".to_string())
        }
    };
    let report = check_with(&source, &from_memory);
    assert!(report.is_sound(), "{}", report.to_text());
    let lost = check_with(
        &source.replace("design.Elements", "design.Lost"),
        &from_memory,
    );
    assert_eq!(
        (
            lost.diagnostics[0].code,
            lost.diagnostics[0].line,
            lost.diagnostics[0].message.as_str()
        ),
        (
            "X351",
            3,
            "import design.Lost does not resolve: this resolver knows only design.Elements"
        )
    );
    let broken = |_: &[String]| Ok("module design.Elements\nspec Elements { enum".to_string());
    let unparsed = check_with(&source, &broken);
    assert_eq!(unparsed.diagnostics[0].code, "X351");
    assert!(
        unparsed.diagnostics[0]
            .message
            .contains("its file does not parse (2:"),
        "{}",
        unparsed.to_text()
    );
    // An import of a header record brings no enum, and is unused like any other.
    let record = |_: &[String]| Ok("module design.Elements\ntask Elements {}".to_string());
    assert_eq!(check_with(&source, &record).diagnostics[0].code, "X201");

    // Files: found beside the importing file, or from a parent by folder name;
    // never above --root.
    assert!(check_file(&seal, Some(&design)).unwrap().is_sound());
    let temp = std::env::temp_dir().join(format!("bboxy450-den-design-{}", std::process::id()));
    let nested = temp.join("kb").join("machines");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir_all(temp.join("design")).unwrap();
    std::fs::copy(
        design.join("elements.den"),
        temp.join("design").join("shared.den"),
    )
    .unwrap();
    std::fs::copy(&seal, nested.join("seal.den")).unwrap();
    assert!(
        check_file(&nested.join("seal.den"), Some(&temp))
            .unwrap()
            .is_sound()
    );
    let below = check_file(&nested.join("seal.den"), Some(&temp.join("kb"))).unwrap();
    assert_eq!(below.diagnostics[0].code, "X351");
    let (code, stdout, _) = den(&[
        "check",
        "--root",
        temp.to_str().unwrap(),
        nested.to_str().unwrap(),
    ]);
    assert_eq!(
        (code, stdout.as_str()),
        (Some(0), "1 spec sound (6 states, 30 transitions)\n")
    );
    // Lowering and Lean see the imported enum as the spec's own.
    let resolver = file_resolver(&seal, None);
    let markdown = render::render_resolved(&source, true, &resolver)
        .unwrap()
        .markdown;
    assert!(
        markdown
            .contains("- `Element`: `Fire`, `Water`, `Air`, `Earth` (from `design.Elements`)\n"),
        "{markdown}"
    );
    assert!(markdown.contains("## Table Volatile\n\n| Element | Bool |"));
    assert!(
        markdown
            .contains("## Open questions\n\n- **SEAL_Q1**: Can a seal be broken, and by whom?\n")
    );
    assert!(
        markdown.contains("## Provenance\n\n- `SEAL_001` from decisions/d0009-stable-cores.md\n")
    );
    let theorem = source.replace("invariant SEAL_001", "theorem SEAL_001");
    let statements = lean_with(&theorem, &resolver)
        .unwrap()
        .unwrap()
        .statements()
        .unwrap();
    assert!(
        statements.contains("Element") && statements.contains("SEAL_001"),
        "{statements}"
    );
    let (code, stdout, _) = den(&[
        "render",
        "--markdown",
        "--out",
        temp.to_str().unwrap(),
        seal.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "{stdout}");
    assert!(stdout.contains(" <- design.Seal "));
    std::fs::remove_dir_all(&temp).unwrap();
}

/// A vocabulary is sound without an invariant or a scenario; anything with a
/// machine still owes both.
#[test]
fn a_vocabulary_spec_is_enums_only() {
    let elements = std::fs::read_to_string(dir("design").join("elements.den")).unwrap();
    let report = check(&elements);
    assert!(report.is_sound(), "{}", report.to_text());
    let machine = elements.replace(
        "  enum Element",
        "  state { on: Bool = false }\n  enum Element",
    );
    assert_eq!(check(&machine).diagnostics[0].code, "X203");
    let empty = "module design.Nothing\nspec Nothing {\n  intent \"Nothing.\"\n  event Go\n  on Go { case go otherwise => ignore Never }\n}\n";
    let codes: Vec<_> = check(empty).diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(codes, ["X902", "X903"]);
}
