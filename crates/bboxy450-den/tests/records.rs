// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Header records: every sound fixture loads, lowers and keeps its fields;
//! every broken control reports exactly its code and line.
#![expect(
    clippy::disallowed_methods,
    reason = "Standalone checker fixtures perform synchronous filesystem and process I/O; no UI thread exists here."
)]
use bboxy450_den::{Record, Value, body, check, declarations, load, metadata, render, set_managed};
use std::path::PathBuf;
use std::process::Command;

fn dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name)
}
fn read(name: &str) -> String {
    std::fs::read_to_string(dir("records").join(name)).unwrap()
}
fn one(source: &str) -> Record {
    let mut records = load(source).unwrap_or_else(|d| panic!("{d:?}"));
    assert_eq!(records.len(), 1);
    records.remove(0)
}
fn value<'a>(record: &'a Record, name: &str) -> &'a Value {
    &record
        .fields
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("no field {name}"))
        .value
}
fn get(metadata: &[(String, String)], key: &str) -> String {
    let found: Vec<_> = metadata.iter().filter(|(k, _)| k == key).collect();
    assert_eq!(found.len(), 1, "{key} in {metadata:?}");
    found[0].1.clone()
}
fn reference(path: &[&str]) -> Value {
    Value::Reference(path.iter().map(|s| s.to_string()).collect())
}

/// Every sound fixture, by file: its kind, and that the whole record survives
/// lowering: each header pair, the body, and each field name or its header key.
#[test]
fn every_kind_loads_and_lowers_with_every_field() {
    let expected = [
        ("bug.den", "bug"),
        ("code_query.den", "spec"),
        ("decision.den", "decision"),
        ("e2e.den", "e2e"),
        ("epic.den", "epic"),
        ("interface.den", "interface"),
        ("invariant.den", "invariant"),
        ("proposal.den", "proposal"),
        ("recipe.den", "recipe"),
        ("report.den", "report"),
        ("task.den", "task"),
    ];
    let mut files: Vec<_> = std::fs::read_dir(dir("records"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(files, expected.map(|(file, _)| file.to_string()));
    for (file, kind) in expected {
        let source = read(file);
        let record = one(&source);
        assert_eq!(record.kind, kind, "{file}");
        let report = check(&source);
        assert!(report.is_sound(), "{file}: {}", report.to_text());
        assert_eq!(report.records[0].kind, kind);
        assert_eq!(report.records[0].name, record.name);

        let outline = declarations(&source).unwrap();
        assert_eq!((outline[0].kind, &outline[0].name), (kind, &record.name));
        assert_eq!(outline[0].line, record.line);
        let managed = usize::from(!record.managed.is_empty());
        assert_eq!(outline.len(), 1 + record.fields.len() + managed, "{file}");
        for field in &record.fields {
            assert!(
                outline
                    .iter()
                    .any(|d| d.kind == "field" && d.name == field.name && d.line == field.line),
                "{file}: {}",
                field.name
            );
        }

        let rendered = render::render(&source).unwrap();
        let markdown = &rendered.markdown;
        assert_eq!(rendered.module, record.module.join("."));
        assert_eq!(
            render::render(&source).unwrap(),
            rendered,
            "{file}: deterministic"
        );
        let header = markdown.split("\n---\n").next().unwrap();
        assert!(
            header.starts_with(&format!("---\ntype: {kind}\ntitle: ")),
            "{markdown}"
        );
        assert!(header.contains(&format!("generated_from:\n  module: {}\n", rendered.module)));
        let pairs = metadata(&record);
        for (key, value) in &pairs[1..] {
            let line = format!("{key}: '{}'", value.replace('\'', "''").replace('\n', " "));
            assert!(
                header.lines().any(|l| l == line),
                "{file}: {line}\n{header}"
            );
        }
        assert!(markdown.contains(&body(&record)), "{file}");
        // Round trip: a field is a header key, the body, or named as not lowered.
        let not_lowered = header
            .lines()
            .find_map(|line| line.strip_prefix("not_lowered: "))
            .unwrap_or("");
        for field in record.fields.iter().chain(&record.managed) {
            let key = match field.name.as_str() {
                "serves" | "realizes" | "decides" | "amends" => "links",
                "paths" => "allowed_paths",
                "delegator" => "delegated_by",
                name => name,
            };
            let lowered = pairs.iter().any(|(k, _)| k == key)
                || (matches!(key, "brief" | "body" | "steps") && !body(&record).is_empty())
                || not_lowered.split(", ").any(|name| name == key);
            assert!(lowered, "{file}: {} is lost", field.name);
        }
        let (embedded, sha) = render::embedded(markdown).unwrap();
        assert_eq!(embedded, source, "{file}");
        assert_eq!(sha, render::source_sha256(&source));
    }
}

#[test]
fn a_task_is_typed_fields_a_header_and_a_body() {
    let record = one(&read("task.den"));
    assert_eq!(record.name, "DenCheckerCrate");
    assert_eq!(record.module, ["tasks", "2026-11-09-den-checker-crate"]);
    assert_eq!(record.imports.len(), 2);
    assert_eq!(record.line, 6);
    let names: Vec<_> = record.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "title", "serves", "role", "paths", "verify", "brief", "priority"
        ]
    );
    assert_eq!((record.fields[1].line, record.fields[1].column), (8, 3));
    assert_eq!(
        value(&record, "title"),
        &Value::Text("Den v1: the checker crate and make check-kb".into())
    );
    assert_eq!(
        value(&record, "serves"),
        &Value::List(vec![
            reference(&["SpecLanguage"]),
            reference(&["SpecFirstOs"])
        ])
    );
    assert_eq!(value(&record, "role"), &reference(&["Executor"]));
    assert!(matches!(value(&record, "paths"), Value::List(paths) if paths.len() == 4));
    assert_eq!(
        value(&record, "verify"),
        &Value::Command {
            command: "cargo nextest run -p bboxy450-den && make check-kb".into(),
            exit: None
        }
    );
    // An unknown field is kept, and its trailing comment is not part of it.
    assert_eq!(value(&record, "priority"), &Value::Integer(2));
    let managed: Vec<_> = record
        .managed
        .iter()
        .map(|f| (f.name.as_str(), &f.value))
        .collect();
    assert_eq!(managed[0].0, "id");
    assert_eq!(
        managed[0].1,
        &Value::Tagged {
            tag: "id".into(),
            value: "01M71JZAEJPC72B5QT15N0E09Q".into()
        }
    );
    assert_eq!(managed[1], ("status", &reference(&["Open"])));

    let brief = "Build the parser and soundness checker defined in SpecLanguage.\n\n- One crate, no parser runtime.\n- Every diagnostic carries a code, a line and a repair.";
    assert_eq!(value(&record, "brief"), &Value::Block(brief.into()));
    assert_eq!(body(&record), brief);

    let pairs = metadata(&record);
    let keys: Vec<_> = pairs.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(
        keys,
        [
            "type",
            "title",
            "links",
            "role",
            "allowed_paths",
            "verify",
            "priority",
            "id",
            "status",
            "delegated_by"
        ]
    );
    assert_eq!(get(&pairs, "type"), "task");
    assert_eq!(
        get(&pairs, "links"),
        "[[specs.SpecLanguage]], [[specs.SpecFirstOs]]"
    );
    assert_eq!(
        get(&pairs, "allowed_paths"),
        "crates/bboxy450-den/, Cargo.toml, Makefile, verification/coverage.toml"
    );
    assert_eq!(get(&pairs, "status"), "open");
    assert_eq!(
        get(&pairs, "delegated_by"),
        "seat:01M71GNVXV7ZHXMGM50M5SB2RA"
    );
}

#[test]
fn the_other_kinds_keep_their_shapes() {
    let report = one(&read("report.den"));
    let pairs = metadata(&report);
    assert_eq!(
        get(&pairs, "task"),
        "2026-11-09-remote-delivery-conformance-and-card"
    );
    assert_eq!(get(&pairs, "author"), "seat:01M71H0S7MDHJ0QTCQFPQYXJHW");
    assert_eq!(get(&pairs, "commits"), "cd1f193b9, ae541605f");
    assert_eq!(
        get(&pairs, "verify"),
        "cargo nextest run -p bboxy450-term-remote"
    );
    assert_eq!(get(&pairs, "verify_exit"), "0");
    assert!(body(&report).starts_with("The conformance test replays"));

    let decision = one(&read("decision.den"));
    assert_eq!(
        value(&decision, "status"),
        &Value::Raw("Decided on date\"2026-10-29\" by Operator".into())
    );
    assert_eq!(
        value(&decision, "decides"),
        &reference(&["specs", "TestPolicy"])
    );
    let pairs = metadata(&decision);
    assert_eq!(get(&pairs, "status"), "decided");
    assert_eq!(get(&pairs, "title"), "D365MandatoryFormalVerification");
    assert_eq!(
        get(&pairs, "links"),
        "[[specs.TestPolicy]], [[decisions.D220SignedMain]]"
    );

    let e2e = one(&read("e2e.den"));
    assert_eq!(
        body(&e2e),
        "1. Install both signed apps.\n2. Restart the GUI and keep the session."
    );
    assert!(matches!(value(&e2e, "evidence"), Value::Tagged { tag, .. } if tag == "path"));
    assert_eq!(get(&metadata(&e2e), "coverage"), "Full");

    let invariant = one(&read("invariant.den"));
    assert_eq!(
        get(&metadata(&invariant), "check"),
        "python3 -I scripts/verify-signatures.py \"${SIGNED_RANGE:-HEAD}\""
    );
    assert_eq!(get(&metadata(&invariant), "status"), "active");

    let epic = one(&read("epic.den"));
    assert_eq!(
        epic.fields.iter().filter(|f| f.name == "channel").count(),
        7
    );
    let Value::Nested { head, fields } = value(&epic, "staff") else {
        panic!("staff")
    };
    assert_eq!((head.as_str(), fields.len()), ("", 4));
    assert_eq!(fields[2].name, "Executor");
    assert_eq!(
        fields[2].value,
        Value::Raw("x 1..1  owns \"one task at a time, in its own worktree\"".into())
    );
    let Value::Nested { fields: work, .. } = value(&epic, "work") else {
        panic!("work")
    };
    assert_eq!(work.len(), 6);
    let Value::Nested { head, fields } = &work[5].value else {
        panic!("a work item with its own block")
    };
    assert_eq!(head, "Graph by Executor needs Parser");
    assert_eq!(fields[2].value, reference(&["reports", "DenGraph"]));
    let Value::Nested { fields, .. } = &work[4].value else {
        panic!("research")
    };
    assert_eq!(
        fields[0].value,
        Value::Block(
            "Read every record kind under .bboxy. For each: its fields, its links,\nhow many exist, and what a checker would reject today. Evidence as file:line.".into()
        )
    );
    let Value::Nested { head, fields } = value(&epic, "budget") else {
        panic!("budget")
    };
    assert_eq!((head.as_str(), fields.len()), ("", 2));
    assert_eq!(fields[1].value, Value::Raw("<= 1 at a time".into()));
    let lowered = render::render(&read("epic.den")).unwrap().markdown;
    assert!(lowered.contains("\nnot_lowered: staff, channel, work, gate, budget\n"));
}

#[test]
fn code_query_is_a_real_record_in_den() {
    let source = read("code_query.den");
    let record = one(&source);
    assert_eq!((record.kind, record.name.as_str()), ("spec", "CodeQuery"));
    let pairs = metadata(&record);
    assert_eq!(
        get(&pairs, "title"),
        "Code query: one MCP server for reading and searching code"
    );
    assert_eq!(get(&pairs, "status"), "draft");
    assert_eq!(get(&pairs, "links"), "[[specs.GapsAndImprovements]]");
    assert_eq!(get(&pairs, "id"), "01M7B58R972307J7YT0GG738VY");
    assert_eq!(get(&pairs, "number"), "1794292861627504");
    assert!(matches!(value(&record, "acceptance"), Value::List(items) if items.len() == 6));
    let body = body(&record);
    for fact in [
        "152,152 tool calls",
        "| Read a line range, whole file, head or tail, or a file at a revision | 58.0% |",
        "**No rust-analyzer in the first version.**",
        "a pattern containing `\\|` is refused",
        "| `outline` | `paths[]` |",
        "## Not built",
    ] {
        assert!(body.contains(fact), "{fact}");
    }
    assert!(
        body.lines().all(|line| !line.starts_with("    ")),
        "indentation is stripped"
    );
    let markdown = render::render(&source).unwrap().markdown;
    assert!(markdown.contains(&body));
    assert!(markdown.contains("\n# Code query: one MCP server for reading and searching code\n"));
}

#[test]
fn set_managed_changes_one_value_and_nothing_else() {
    let source = read("task.den");
    let claimed = set_managed(&source, "DenCheckerCrate", "status", "Claimed").unwrap();
    assert_eq!(
        claimed,
        source.replace("status    = Open", "status    = Claimed")
    );
    assert_eq!(get(&metadata(&one(&claimed)), "status"), "claimed");
    assert_eq!(
        set_managed(&claimed, "DenCheckerCrate", "status", "Claimed").unwrap(),
        claimed,
        "idempotent"
    );

    let seat = "seat\"01M71H0S7MDHJ0QTCQFPQYXJHW\"";
    let assigned = set_managed(&claimed, "DenCheckerCrate", "assignee", seat).unwrap();
    assert_eq!(
        assigned,
        claimed.replace(
            "    delegator = seat\"01M71GNVXV7ZHXMGM50M5SB2RA\"\n",
            "    delegator = seat\"01M71GNVXV7ZHXMGM50M5SB2RA\"\n    assignee = seat\"01M71H0S7MDHJ0QTCQFPQYXJHW\"\n"
        )
    );
    assert_eq!(
        get(&metadata(&one(&assigned)), "assignee"),
        "seat:01M71H0S7MDHJ0QTCQFPQYXJHW"
    );
    assert_eq!(
        set_managed(&assigned, "DenCheckerCrate", "assignee", seat).unwrap(),
        assigned
    );
    // The author's part is the same record throughout.
    assert_eq!(one(&assigned).fields, one(&source).fields);

    // No managed block yet: it is added before the closing brace.
    let bare = "module tasks.T\n\ntask T {\n  title  \"t\"\n  verify $ true\n  brief  \"b\"\n}\n";
    let opened = set_managed(bare, "T", "status", "Open").unwrap();
    assert_eq!(
        opened,
        "module tasks.T\n\ntask T {\n  title  \"t\"\n  verify $ true\n  brief  \"b\"\n  managed {\n    status = Open\n  }\n}\n"
    );
    assert_eq!(set_managed(&opened, "T", "status", "Open").unwrap(), opened);
    let one_line = "module tasks.T\ntask T { title \"t\"; brief \"b\"; verify $ true\n managed { id = id\"1\" } }";
    let next = set_managed(one_line, "T", "status", "Done").unwrap();
    assert_eq!(
        next,
        one_line.replace("id\"1\" }", "id\"1\"; status = Done }")
    );

    // Refused: the wrong record, a status the host does not know, a value
    // that is more than a value, and a source that does not load.
    let code = |result: Result<String, Vec<bboxy450_den::Diagnostic>>| result.unwrap_err()[0].code;
    assert_eq!(
        code(set_managed(&source, "Other", "status", "Claimed")),
        "X201"
    );
    assert_eq!(
        code(set_managed(
            &source,
            "DenCheckerCrate",
            "status",
            "Finished"
        )),
        "X357"
    );
    assert_eq!(
        code(set_managed(
            &source,
            "DenCheckerCrate",
            "status",
            "Open }\n  title \"x\""
        )),
        "X101"
    );
    assert_eq!(
        code(set_managed(
            &source,
            "DenCheckerCrate",
            "note",
            "Open; status = Done"
        )),
        "X356"
    );
    assert_eq!(
        code(set_managed(&source, "DenCheckerCrate", "note", "a; b = c")),
        "X101"
    );
    assert_eq!(
        code(set_managed("module T\ntsak T {}", "T", "status", "Open")),
        "X152"
    );
}

/// One broken control per record code: exactly that code, at that line.
#[test]
fn broken_records_report_exactly_their_code_and_line() {
    let expected = [
        ("X102-unterminated-text-block.den", 6),
        ("X152-unknown-record-kind.den", 3),
        ("X205-unknown-reference.den", 7),
        ("X206-duplicate-field.den", 7),
        ("X354-missing-required-field.den", 3),
        ("X356-managed-field-twice.den", 10),
        ("X357-invalid-task-status.den", 9),
    ];
    let unsound = dir("unsound").join("records");
    let mut files: Vec<_> = std::fs::read_dir(&unsound)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(files, expected.map(|(file, _)| file.to_string()));
    for (file, line) in expected {
        let source = std::fs::read_to_string(unsound.join(file)).unwrap();
        let diagnostics = load(&source).unwrap_err();
        let found: Vec<_> = diagnostics.iter().map(|d| (d.code, d.line)).collect();
        assert_eq!(found, [(&file[..4], line)], "{file}");
        assert!(diagnostics[0].column > 0 && !diagnostics[0].repair.is_empty());
        // `check` and `render` say the same; nothing is lowered.
        let report = check(&source);
        assert_eq!(report.diagnostics.len(), 1, "{file}");
        assert_eq!(
            (report.diagnostics[0].code, report.diagnostics[0].line),
            found[0]
        );
        assert!(render::render(&source).is_err());
    }
}

#[test]
fn required_fields_and_edges_per_kind() {
    let missing = |source: &str| -> Vec<String> {
        load(source)
            .unwrap_err()
            .iter()
            .map(|d| format!("{} {}", d.code, d.message))
            .collect()
    };
    assert_eq!(
        missing("module t.T\ntask T {}"),
        [
            "X354 a task needs a title",
            "X354 a task needs a verify",
            "X354 a task needs a brief"
        ]
    );
    assert_eq!(
        missing("module e.E\nepic E {}"),
        ["X354 a epic needs a intent"]
    );
    assert_eq!(
        missing("module r.R\nreport R {}"),
        [
            "X354 a report needs a task",
            "X354 a report needs a author",
            "X354 a report needs a verify"
        ]
    );
    assert_eq!(
        missing("module d.D\ndecision D {}"),
        ["X354 a decision needs a status"]
    );
    assert_eq!(
        missing("module e.E\ne2e E {}"),
        ["X354 a e2e needs a function", "X354 a e2e needs a expected"]
    );
    assert_eq!(
        missing("module s.S\nspec S { title \"t\" }"),
        ["X354 a spec needs a intent"]
    );
    assert_eq!(
        missing("module s.S\nspec S { intent \"  \" }"),
        ["X354 a spec needs a intent"]
    );
    for kind in ["proposal", "bug", "recipe", "invariant", "interface"] {
        assert_eq!(one(&format!("module k.K\n{kind} K {{}}")).kind, kind);
    }
    let code = |source: &str| load(source).unwrap_err()[0].code;
    assert_eq!(code("module t.T\ntask T { title \"open }"), "X102");
    assert_eq!(code("module t.`T\ntask T {}"), "X102");
    assert_eq!(
        code("module t.T\ntask T { managed { a = 1 }; managed { b = 2 } }"),
        "X356"
    );
    assert_eq!(code("module t.T\ntask T { title }"), "X101");
    assert_eq!(code("module t.T\ntask T {"), "X101");
    assert_eq!(code("module t.T\ntask T {}\ntask U {}"), "X101");
    assert_eq!(code("task T {}"), "X101");
    assert_eq!(code("module t.T\nbug T { serves \"prose\" }"), "X205");
    // Every host status, in any case; a record may link to itself.
    for status in [
        "open", "Claimed", "RUNNING", "Reported", "Accepted", "Rejected", "Blocked", "Done",
    ] {
        let source = format!(
            "module t.T\ntask T {{ title \"t\"; brief \"b\"; serves T; status {status}\n verify $ true\n}}"
        );
        assert_eq!(
            get(&metadata(&one(&source)), "status"),
            status.to_lowercase()
        );
    }
    // A machine is not a header record, and still checks as before.
    let machine = std::fs::read_to_string(dir("sound").join("turnstile.den")).unwrap();
    assert_eq!(code(&machine), "X190");
    assert!(check(&machine).is_sound());
    assert!(check(&machine).records[0].states > 0);
}

#[test]
fn the_cli_checks_and_lowers_records() {
    let den = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_bboxy450-den"))
            .args(args)
            .output()
            .unwrap()
    };
    let sound = den(&["check", dir("records").to_str().unwrap()]);
    assert_eq!(sound.status.code(), Some(0), "{sound:?}");
    assert_eq!(
        sound.stdout,
        b"11 records sound (0 states, 0 transitions)\n"
    );
    let unsound = den(&["check", dir("unsound").join("records").to_str().unwrap()]);
    assert_eq!(unsound.status.code(), Some(1));
    let text = String::from_utf8(unsound.stdout).unwrap();
    assert_eq!(text.lines().count(), 8, "{text}");
    assert!(text.ends_with("\n0 sound, 7 unsound\n"));
    assert!(
        text.contains(
            "X357-invalid-task-status.den:9:5 X357 Finished is not a task status. Repair: "
        )
    );

    let out = std::env::temp_dir().join(format!("bboxy450-den-records-{}", std::process::id()));
    std::fs::create_dir(&out).unwrap();
    let source = out.join("task.den");
    std::fs::copy(dir("records").join("task.den"), &source).unwrap();
    for expected in [" <- tasks.2026-11-09-den-checker-crate ", " unchanged"] {
        let rendered = den(&["render", "--markdown", source.to_str().unwrap()]);
        assert_eq!(rendered.status.code(), Some(0), "{rendered:?}");
        assert!(
            String::from_utf8(rendered.stdout)
                .unwrap()
                .contains(expected)
        );
    }
    let markdown = std::fs::read_to_string(out.join("task.md")).unwrap();
    assert_eq!(
        markdown,
        render::render(&read("task.den")).unwrap().markdown
    );
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn layout_says_where_the_host_edits() {
    let source = read("task.den");
    let (record, layout) = bboxy450_den::layout(&source).unwrap();
    assert_eq!(record, one(&source));
    assert_eq!(layout.values.len(), record.fields.len());
    let slice = |(start, end): (usize, usize)| &source[start..end];
    assert_eq!(
        slice(layout.values[0]),
        "\"Den v1: the checker crate and make check-kb\""
    );
    let brief = record
        .fields
        .iter()
        .position(|f| f.name == "brief")
        .unwrap();
    let text = slice(layout.values[brief]);
    assert!(
        text.starts_with("\"\"\"\n") && text.ends_with("\"\"\""),
        "{text}"
    );
    let managed = slice(layout.managed.unwrap());
    assert!(
        managed.starts_with("managed {") && managed.ends_with("\n  }"),
        "{managed}"
    );
    assert_eq!(&source[layout.close..], "}\n");
    let bare = "module a.B\nepic B { intent \"x\" }\n";
    let (_, layout) = bboxy450_den::layout(bare).unwrap();
    assert_eq!((layout.managed, &bare[layout.close..]), (None, "}\n"));
    assert_eq!(
        bboxy450_den::layout("module a.B\ntask B {}\n").unwrap_err()[0].code,
        "X354"
    );
}
