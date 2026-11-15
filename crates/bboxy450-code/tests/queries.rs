// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The queries against a small fixture tree, and the server over stdio.
#![expect(
    clippy::disallowed_methods,
    reason = "Fixture files and the server child are plain synchronous test IO."
)]
use bboxy450_code::{RESULT_BYTES, Root, call};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};

const LIB: &str = "pub struct Seat {\n    pub id: u64,\n}\n\nimpl Seat {\n    pub fn open(id: u64) -> Self {\n        Seat { id }\n    }\n    fn close(&self) {}\n}\n\npub enum State {\n    Open,\n    Closed,\n}\n\npub fn all() -> Vec<Seat> {\n    vec![Seat::open(1)]\n}\n";

fn fixture() -> (tempfile::TempDir, Root) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join("src/lib.rs"), LIB).unwrap();
    std::fs::write(dir.path().join("notes.md"), "Seat notes\nnothing here\n").unwrap();
    std::fs::write(
        dir.path().join("blob.bin"),
        [0u8, 1, 2, b'S', b'e', b'a', b't'],
    )
    .unwrap();
    std::fs::write(dir.path().join(".gitignore"), "ignored.rs\n").unwrap();
    std::fs::write(dir.path().join("ignored.rs"), "pub struct Seat;\n").unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    let root = Root::new(dir.path()).unwrap();
    (dir, root)
}

#[test]
fn search_names_the_containing_item_counts_each_pattern_and_reports_missing_paths() {
    let (_dir, root) = fixture();
    let found = call(
        &root,
        "search",
        &json!({"patterns": ["Seat::open", "Closed", "absent"], "paths": ["src", "gone"]}),
    )
    .unwrap();
    assert_eq!(found["total"], 2);
    assert_eq!(found["missing"], json!(["gone"]));
    assert_eq!(
        found["patterns"][0],
        json!({"pattern": "Seat::open", "hits": 1})
    );
    assert_eq!(
        found["patterns"][2],
        json!({"pattern": "absent", "hits": 0})
    );
    assert_eq!(
        found["hits"][0],
        json!({"path": "src/lib.rs", "line": 14, "text": "    Closed,", "item": "enum State :: variant Closed"})
    );
    assert_eq!(found["hits"][1]["item"], "fn all");
    // Ignored and binary files are never searched.
    let everywhere = call(
        &root,
        "search",
        &json!({"patterns": ["Seat"], "mode": "files"}),
    )
    .unwrap();
    assert_eq!(
        everywhere["files"],
        json!([{"path": "notes.md", "hits": 1}, {"path": "src/lib.rs", "hits": 5}])
    );
    let counted = call(
        &root,
        "search",
        &json!({"patterns": ["Seat"], "mode": "count", "glob": "*.rs"}),
    )
    .unwrap();
    assert_eq!(counted["total"], 5);
    assert!(counted.get("hits").is_none());
    let after = call(
        &root,
        "search",
        &json!({"patterns": ["pub enum"], "context": 2}),
    )
    .unwrap();
    assert_eq!(
        after["hits"][0]["after"],
        json!(["    Open,", "    Closed,"])
    );
    let literal = call(
        &root,
        "search",
        &json!({"patterns": ["open(1)"], "fixed": true}),
    )
    .unwrap();
    assert_eq!(literal["total"], 1);
}

#[test]
fn a_bad_pattern_is_refused_with_the_correction() {
    let (_dir, root) = fixture();
    let grep = call(&root, "search", &json!({"patterns": ["open\\|close"]})).unwrap_err();
    assert!(grep.contains("alternation here is |"), "{grep}");
    assert!(
        call(&root, "search", &json!({"patterns": ["("]}))
            .unwrap_err()
            .contains("invalid pattern")
    );
    assert!(call(&root, "search", &json!({"patterns": []})).is_err());
    assert!(
        call(&root, "search", &json!({"patterns": ["x"], "unknown": 1}))
            .unwrap_err()
            .contains("invalid arguments")
    );
    assert!(call(&root, "nope", &json!({})).is_err());
}

#[test]
fn read_returns_ranges_whole_items_and_an_error_beside_good_requests() {
    let (_dir, root) = fixture();
    let got = call(
        &root,
        "read",
        &json!({"requests": [
            {"path": "src/lib.rs", "start": 12, "end": 13},
            {"path": "src/lib.rs", "around": 7},
            {"path": "missing.rs"},
            {"path": "../outside"},
            {"path": "blob.bin"},
            {"path": "notes.md"}
        ]}),
    )
    .unwrap();
    let files = got["files"].as_array().unwrap();
    assert_eq!(
        files[0],
        json!({"path": "src/lib.rs", "lines": 19, "start": 12, "end": 13, "text": "12\tpub enum State {\n13\t    Open,\n"})
    );
    assert_eq!(files[1]["item"], "impl Seat :: fn open");
    assert_eq!(
        (files[1]["start"].as_u64(), files[1]["end"].as_u64()),
        (Some(6), Some(8))
    );
    assert_eq!(
        files[2],
        json!({"path": "missing.rs", "error": "no such file"})
    );
    assert_eq!(files[3]["error"], "path is outside the workspace");
    assert_eq!(files[4]["error"], "not a text file");
    assert_eq!(files[5]["text"], "1\tSeat notes\n2\tnothing here\n");
    assert!(files[5].get("next_start").is_none());
}

#[test]
fn symbol_finds_definitions_before_uses_and_an_impl_answers_to_its_type() {
    let (_dir, root) = fixture();
    let got = call(
        &root,
        "symbol",
        &json!({"names": ["Seat", "open", "Nothing"], "refs": true}),
    )
    .unwrap();
    let seat = &got["symbols"][0];
    let kinds: Vec<&str> = seat["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["struct", "impl"]);
    assert_eq!(seat["definitions"][0]["signature"], "pub struct Seat");
    assert_eq!(seat["use_count"], 3);
    let open = &got["symbols"][1];
    assert_eq!(open["definitions"][0]["in"], "impl Seat");
    assert_eq!(open["uses"][0]["item"], "fn all");
    assert_eq!(got["symbols"][2]["definitions"], json!([]));
    let only = call(
        &root,
        "symbol",
        &json!({"names": ["Open"], "kind": "variant"}),
    )
    .unwrap();
    assert_eq!(only["symbols"][0]["definitions"][0]["in"], "enum State");
    assert!(only["symbols"][0].get("uses").is_none());
}

#[test]
fn files_and_outline_list_in_path_and_source_order() {
    let (_dir, root) = fixture();
    let listed = call(&root, "files", &json!({})).unwrap();
    assert_eq!(
        listed["files"],
        json!([
        {"path": "blob.bin", "lines": null}, {"path": "notes.md", "lines": 2}, {"path": "src/lib.rs", "lines": 19}])
    );
    assert_eq!(listed["lines"], 21);
    let rust = call(
        &root,
        "files",
        &json!({"glob": "*.rs", "paths": ["src", "gone"]}),
    )
    .unwrap();
    assert_eq!(
        (rust["total"].as_u64(), &rust["missing"]),
        (Some(1), &json!(["gone"]))
    );
    let outline = call(
        &root,
        "outline",
        &json!({"paths": ["src/lib.rs", "notes.md"]}),
    )
    .unwrap();
    let names: Vec<String> = outline["files"][0]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            format!(
                "{}{} {}",
                "  ".repeat(i["depth"].as_u64().unwrap() as usize),
                i["kind"].as_str().unwrap(),
                i["name"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            "struct Seat",
            "impl Seat",
            "  fn open",
            "  fn close",
            "enum State",
            "  variant Open",
            "  variant Closed",
            "fn all"
        ]
    );
    assert!(
        outline["files"][1]["error"]
            .as_str()
            .unwrap()
            .contains("no outline for this file type")
    );
}

// The three tiers: Rust in full, Den through its own parser, and Python,
// TypeScript and Go by syntax so a port can list what there is to rebuild.
#[test]
fn den_lean_and_source_tier_files_have_outlines_and_enclosing_items() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, body: &str| std::fs::write(dir.path().join(name), body).unwrap();
    write(
        "turnstile.den",
        include_str!("../../bboxy450-den/tests/sound/turnstile.den"),
    );
    write(
        "proof.lean",
        "namespace Gate\n\ntheorem locked_stays (n : Nat) : n = n := by\n  rfl\n\ndef twice (n : Nat) := n + n\n",
    );
    write(
        "app.py",
        "class Store:\n    def get(self, key):\n        return key\n\ndef main():\n    return Store()\n",
    );
    write(
        "app.ts",
        "interface Seat { id: number }\nexport class Hub {\n  open(id: number): Seat {\n    return { id };\n  }\n}\nfunction main(): void {}\n",
    );
    write(
        "app.go",
        "package main\n\ntype Seat struct{ ID int }\n\nfunc (s Seat) Open() int { return s.ID }\n\nfunc main() {}\n",
    );
    let root = Root::new(dir.path()).unwrap();
    let outline = call(
        &root,
        "outline",
        &json!({"paths": ["turnstile.den", "proof.lean", "app.py", "app.ts", "app.go"]}),
    )
    .unwrap();
    let kinds = |index: usize| -> Vec<String> {
        outline["files"][index]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                format!(
                    "{} {}",
                    i["kind"].as_str().unwrap(),
                    i["name"].as_str().unwrap()
                )
            })
            .collect()
    };
    let den = kinds(0);
    assert_eq!(den[0], "spec Turnstile", "{den:?}");
    assert!(
        den.iter().any(|d| d.starts_with("event "))
            && den.iter().any(|d| d.starts_with("handler ")),
        "{den:?}"
    );
    assert_eq!(
        outline["files"][0]["items"][1]["depth"], 1,
        "declarations sit inside the spec"
    );
    assert_eq!(
        kinds(1),
        ["namespace Gate", "theorem locked_stays", "def twice"]
    );
    assert_eq!(kinds(2), ["class Store", "fn get", "fn main"]);
    assert_eq!(
        kinds(3),
        ["interface Seat", "class Hub", "method open", "fn main"]
    );
    assert_eq!(kinds(4), ["type Seat", "method Open", "fn main"]);
    let hit = call(
        &root,
        "search",
        &json!({"patterns": ["return key", "return \\{ id"]}),
    )
    .unwrap();
    assert_eq!(hit["hits"][0]["item"], "class Store :: fn get");
    assert_eq!(hit["hits"][1]["item"], "class Hub :: method open");
    let found = call(&root, "symbol", &json!({"names": ["Turnstile", "main"]})).unwrap();
    assert_eq!(found["symbols"][0]["definitions"][0]["kind"], "spec");
    assert_eq!(
        found["symbols"][1]["definitions"].as_array().unwrap().len(),
        3
    );
}

#[test]
fn every_result_is_bounded_says_what_was_cut_and_is_the_same_twice() {
    let dir = tempfile::tempdir().unwrap();
    let long = "x".repeat(1000);
    let body: String = (0..3000)
        .map(|i| format!("fn f{i}() {{ let needle = \"{long}\"; }}\n"))
        .collect();
    for name in ["a.rs", "b.rs"] {
        std::fs::write(dir.path().join(name), &body).unwrap();
    }
    let root = Root::new(dir.path()).unwrap();
    for (tool, args) in [
        (
            "search",
            json!({"patterns": ["needle"], "limit": 500, "context": 20}),
        ),
        (
            "read",
            json!({"requests": [{"path": "a.rs"}, {"path": "b.rs"}, {"path": "a.rs", "start": 500}]}),
        ),
        ("symbol", json!({"names": ["needle", "f7"], "refs": true})),
        ("outline", json!({"paths": ["a.rs", "b.rs"]})),
        ("files", json!({})),
    ] {
        let first = call(&root, tool, &args).unwrap().to_string();
        assert!(first.len() <= RESULT_BYTES, "{tool}: {} bytes", first.len());
        assert_eq!(
            first,
            call(&root, tool, &args).unwrap().to_string(),
            "{tool} is deterministic"
        );
        if tool != "files" {
            let value: Value = serde_json::from_str(&first).unwrap();
            let continued = value.get("cut").is_some()
                || value.get("skipped").is_some()
                || value["files"]
                    .as_array()
                    .is_some_and(|f| f.iter().any(|f| f.get("next_start").is_some()));
            assert!(continued, "{tool} names its continuation: {first}");
        }
    }
    let searched = call(&root, "search", &json!({"patterns": ["needle"]})).unwrap();
    assert_eq!(searched["total"], 6000, "the total counts what was cut");
    let read = call(
        &root,
        "read",
        &json!({"requests": [{"path": "a.rs", "start": 10, "end": 12}]}),
    )
    .unwrap();
    assert!(
        read["files"][0]["text"]
            .as_str()
            .unwrap()
            .lines()
            .all(|l| l.chars().count() < 260),
        "long lines are clipped"
    );
}

/// A tree with a verification inventory: `src/a.rs` under a contract,
/// `src/b.rs` under its crate's coverage row only, `tools/c.rs` under nothing,
/// one sound Den spec that names `src/a.rs` and one unsound spec.
fn verified(contracts: usize) -> (tempfile::TempDir, Root) {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, body: &str| {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    write("Cargo.toml", "[package]\nname = \"fix\"\n");
    write("src/a.rs", "pub fn a() {}\n");
    write("src/b.rs", "pub fn b() {}\n");
    write("tools/Cargo.toml", "[package]\nname = \"tools\"\n");
    write("tools/c.rs", "pub fn c() {}\n");
    write(
        "specs/turnstile.den",
        &include_str!("../../bboxy450-den/tests/sound/turnstile.den")
            .replace("A coin opens", "src/a.rs: a coin opens"),
    );
    write(
        "specs/broken.den",
        include_str!("../../bboxy450-den/tests/unsound/X201-unknown-name.den"),
    );
    let mut rows = String::from(
        "[[contract]]\nname = \"Turnstile\"\ntitle = \"A paid gate\"\nmodels = [\"Turnstile\", \"BrokenTurnstile\"]\n\
         sources = [\"src/a.rs\"]\nimplementation_tests = [\"tests/gate.rs\"]\n\
         assumptions = \"One gate.\"\nproperty = \"TURN_001 holds.\"\nchecks = [\"make test-formal\"]\n",
    );
    for index in 0..contracts {
        rows.push_str(&format!(
            "[[contract]]\nname = \"Many{index}\"\nmodels = [\"Many{index}\"]\ncontrols = [{{name = \"BrokenMany{index}\", expected = \"safety\"}}]\n\
             sources = [\"src\"]\nproperty = \"{}\"\n",
            "p".repeat(300)
        ));
    }
    write("verification/contracts.toml", &rows);
    write(
        "verification/coverage.toml",
        "[[crate]]\nname = \"fix\"\nstatus = \"contract-model\"\nmodels = [\"Turnstile\", \"BrokenTurnstile\"]\nscope = \"s\"\nsources = [\"src/a.rs\"]\n\
         [[implementation_only]]\nname = \"Fixtures\"\nsources = [\"src/b.rs\"]\ntests = []\nmodels = []\n",
    );
    write(
        "verification/corpus.toml",
        "[[model]]\nname = \"Turnstile\"\nexpected = \"pass\"\nfrontend = \"den\"\n\
         [[model]]\nname = \"BrokenTurnstile\"\nexpected = \"safety\"\nfrontend = \"den\"\n",
    );
    let root = Root::new(dir.path()).unwrap();
    (dir, root)
}

#[test]
fn den_check_reports_sound_and_unsound_files_with_totals_and_missing_paths() {
    let (_dir, root) = verified(0);
    let got = call(&root, "den_check", &json!({"paths": ["specs", "gone"]})).unwrap();
    assert_eq!(got["totals"], json!({"files": 2, "sound": 1, "unsound": 1}));
    assert_eq!(got["missing"], json!(["gone"]));
    let broken = &got["files"][0];
    assert_eq!(
        (&broken["path"], &broken["sound"]),
        (&json!("specs/broken.den"), &json!(false))
    );
    let diagnostic = &broken["diagnostics"][0];
    assert_eq!(diagnostic["code"], "X201");
    assert_eq!(diagnostic["line"], 13);
    for field in ["column", "record", "message", "repair"] {
        assert!(diagnostic.get(field).is_some(), "{field}");
    }
    let sound = &got["files"][1];
    assert_eq!(
        (&sound["sound"], &sound["diagnostics"]),
        (&json!(true), &json!([]))
    );
    let record = &sound["records"][0];
    assert_eq!(
        (&record["kind"], &record["name"], &record["theorems"]),
        (&json!("spec"), &json!("Turnstile"), &json!(["TURN_001"]))
    );
    assert!(record["states"].as_u64().unwrap() > 0 && record["transitions"].as_u64().unwrap() > 0);
    assert!(record["invariants"].is_u64() && record["scenarios"].is_u64());
    let one = call(
        &root,
        "den_check",
        &json!({"paths": ["specs/turnstile.den"]}),
    )
    .unwrap();
    assert_eq!(one["totals"]["files"], 1);
    assert!(call(&root, "den_check", &json!({"paths": []})).is_err());
}

#[test]
fn contracts_link_paths_and_names_to_what_specifies_them() {
    let (_dir, root) = verified(0);
    let got = call(
        &root,
        "contracts",
        &json!({"paths": ["src/a.rs", "src/b.rs", "tools/c.rs", "gone.rs"], "names": ["Turnstile", "BrokenTurnstile", "Nope"]}),
    )
    .unwrap();
    assert_eq!(got["missing"], json!(["gone.rs"]));
    let a = &got["paths"][0];
    assert_eq!(
        a["contracts"],
        json!([{"name": "Turnstile", "title": "A paid gate", "property": "TURN_001 holds.",
            "models": [{"name": "Turnstile", "expected": "pass"}, {"name": "BrokenTurnstile", "expected": "safety"}],
            "controls": ["BrokenTurnstile"], "tests": ["tests/gate.rs"], "den": "specs/turnstile.den"}])
    );
    assert_eq!(
        a["coverage"],
        json!({"crate": "fix", "status": "contract-model", "models": 2})
    );
    assert_eq!(
        a["den"],
        json!([{"path": "specs/turnstile.den", "spec": "Turnstile", "sound": true, "theorems": ["TURN_001"]}])
    );
    assert!(a.get("uncovered").is_none());
    // No contract, but the crate's coverage row and an implementation-only row answer.
    let b = &got["paths"][1];
    assert_eq!(
        (
            &b["contracts"],
            &b["coverage"]["crate"],
            &b["implementation_only"]
        ),
        (&json!([]), &json!("fix"), &json!(["Fixtures"]))
    );
    assert!(b.get("uncovered").is_none());
    assert_eq!(
        got["paths"][2],
        json!({"path": "tools/c.rs", "contracts": [], "den": [], "uncovered": true}),
        "nothing specifies this file"
    );
    let named = &got["names"][0];
    assert_eq!(named["contracts"][0]["sources"], json!(["src/a.rs"]));
    assert_eq!(named["contracts"][0]["tests"], json!(["tests/gate.rs"]));
    assert_eq!(
        named["model"],
        json!({"expected": "pass", "frontend": "den"})
    );
    assert_eq!(named["den"]["path"], "specs/turnstile.den");
    assert_eq!(named["crates"], json!(["fix"]));
    assert!(named["contracts"][0].get("assumptions").is_none());
    // A model name leads to the contract that lists it.
    assert_eq!(got["names"][1]["contracts"][0]["name"], "Turnstile");
    assert_eq!(got["names"][1]["model"]["expected"], "safety");
    assert_eq!(
        got["names"][2],
        json!({"name": "Nope", "contracts": [], "unknown": true})
    );
    let detail = call(
        &root,
        "contracts",
        &json!({"paths": ["src"], "detail": true}),
    )
    .unwrap();
    assert_eq!(
        detail["paths"][0]["contracts"][0]["assumptions"],
        "One gate."
    );
    assert_eq!(
        detail["paths"][0]["contracts"][0]["sources"],
        json!(["src/a.rs"])
    );
    assert!(call(&root, "contracts", &json!({})).is_err());
    let (_plain, plain) = fixture();
    assert_eq!(
        call(&plain, "contracts", &json!({"paths": ["src/lib.rs"]})).unwrap(),
        json!({"unavailable": "no verification inventory under this root"})
    );
}

#[test]
fn contracts_and_den_check_are_bounded_say_what_was_cut_and_are_the_same_twice() {
    let (dir, root) = verified(400);
    let den = include_str!("../../bboxy450-den/tests/unsound/X201-unknown-name.den");
    for index in 0..200 {
        std::fs::write(dir.path().join(format!("specs/more{index:03}.den")), den).unwrap();
    }
    for (tool, args) in [
        ("contracts", json!({"paths": ["src/a.rs", "src"]})),
        (
            "contracts",
            json!({"names": ["Many7", "BrokenMany8"], "paths": ["."], "detail": true}),
        ),
        ("den_check", json!({"paths": ["specs"]})),
    ] {
        let first = call(&root, tool, &args).unwrap();
        let bytes = first.to_string();
        assert!(bytes.len() <= RESULT_BYTES, "{tool}: {} bytes", bytes.len());
        assert!(
            first["cut"]["omitted"].as_u64().unwrap() > 0 && first["cut"]["continue"].is_string(),
            "{tool} names its continuation"
        );
        assert_eq!(
            bytes,
            call(&root, tool, &args).unwrap().to_string(),
            "{tool} is deterministic"
        );
    }
    let checked = call(&root, "den_check", &json!({"paths": ["specs"]})).unwrap();
    assert_eq!(
        checked["totals"],
        json!({"files": 202, "sound": 1, "unsound": 201}),
        "the totals count what was cut"
    );
    let named = call(&root, "contracts", &json!({"names": ["BrokenMany8"]})).unwrap();
    assert_eq!(named["names"][0]["contracts"][0]["name"], "Many8");
    assert_eq!(
        named["names"][0]["contracts"][0]["controls"],
        json!(["BrokenMany8"])
    );
}

#[test]
fn the_server_answers_initialize_list_and_call_on_stdio() {
    let (dir, _root) = fixture();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_bboxy450-code"))
        .env("BBOXY_CODE_ROOT", dir.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut ask = |message: Value| {
        writeln!(stdin, "{message}").unwrap();
        serde_json::from_str::<Value>(&lines.next().unwrap().unwrap()).unwrap()
    };
    let hello = ask(
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-11-25"}}),
    );
    assert_eq!(hello["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(hello["result"]["serverInfo"]["name"], "bboxy450-code");
    let listed = ask(json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "read",
            "search",
            "symbol",
            "files",
            "outline",
            "den_check",
            "contracts"
        ]
    );
    let called = ask(json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {"name": "search", "arguments": {"patterns": ["pub enum"]}}}));
    assert_eq!(called["result"]["isError"], false);
    let text: Value =
        serde_json::from_str(called["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text["hits"][0]["item"], "enum State");
    let stated = ask(json!({"jsonrpc": "2.0", "id": 9, "method": "tools/call",
        "params": {"name": "files", "arguments": {"glob": "*.rs", "intent": "count the Rust files"}}}));
    assert_eq!(
        stated["result"]["isError"], false,
        "intent is accepted and ignored"
    );
    assert!(
        listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["inputSchema"]["properties"]["intent"]["type"] == "string")
    );
    let refused = ask(
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "search", "arguments": {"patterns": ["a\\|b"]}}}),
    );
    assert_eq!(refused["result"]["isError"], true);
    let unknown = ask(json!({"jsonrpc": "2.0", "id": 5, "method": "resources/list"}));
    assert_eq!(unknown["error"]["code"], -32601);
    drop(stdin);
    assert!(child.wait().unwrap().success());
}
