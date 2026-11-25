// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A `.den` record is written, read, searched, verified and run through the
//! task lifecycle by the same tools as a Markdown one, and keeps its author's text.
use super::*;

const PATH: &str = "tasks/2026-11-15-den-task.den";
/// Everything the author wrote before the brief, spacing and comment included.
const HEAD: &str = "module tasks.`2026-11-15-den-task`\n\ntask DenTask {\n  title  \"Den task: written as source\"   // the author's spacing stays\n  verify $ true\n  paths  \"fixture/**\", \"docs/\"\n  brief  \"\"\"\n    Build the thing.\n\n    - keep this list\n";
const TAIL: &str = "  priority 2\n";

fn den_text(f: &Fixture, path: &str) -> String {
    std::fs::read_to_string(f.config.repository.join(".bboxy").join(path)).unwrap()
}
/// The file still loads, the author's text is where it was, and `managed.status` is `status`.
fn assert_den_task(f: &Fixture, status: &str) -> bbt_mux::index::Document {
    let text = den_text(f, PATH);
    let record = bboxy450_den::load(&text)
        .unwrap_or_else(|d| panic!("{d:?}\n{text}"))
        .remove(0);
    assert!(text.starts_with(HEAD), "{text}");
    assert!(text.contains(TAIL), "{text}");
    let managed = record
        .managed
        .iter()
        .find(|field| field.name == "status")
        .unwrap_or_else(|| panic!("no managed status\n{text}"));
    assert_eq!(managed.value.text(), status, "{text}");
    let document = bbt_mux::index::Document::parse(PATH, &text);
    assert_eq!(document.error, None, "{text}");
    assert_eq!(document.metadata["status"], status.to_ascii_lowercase());
    document
}

#[test]
fn den_task_is_written_read_searched_verified_and_run_through_its_lifecycle() {
    let f = Fixture::new();
    let source = format!("{HEAD}  \"\"\"\n{TAIL}}}\n");
    let task_path = format!(".bboxy/{PATH}");
    let created = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "knowledge_write",
        json!({"path":PATH,"frontmatter":{"type":"task"},"body":source}),
    ))["record"]
        .clone();
    let id = created["id"].as_str().unwrap().to_owned();
    let written = den_text(&f, PATH);
    println!("--- {PATH} as written by the host ---\n{written}");
    assert!(written.starts_with(source.strip_suffix("}\n").unwrap()), "{written}");
    let document = assert_den_task(&f, "Open");
    assert_item_signature(&document, f.oracle.id);

    // Read: the header is the metadata, the brief is the body.
    for locator in [task_path.as_str(), "tasks/2026-11-15-den-task", id.as_str()] {
        let fetched = data(f.call(
            f.oracle.id,
            Ulid::new(),
            "knowledge_fetch",
            json!({"locator":locator}),
        ));
        let front = &fetched["frontmatter"];
        assert_eq!(front["title"], "Den task: written as source", "{fetched}");
        assert_eq!(front["type"], "task");
        assert_eq!(front["status"], "open");
        assert_eq!(front["verify"], "true");
        assert_eq!(front["allowed_paths"], json!(["fixture/**", "docs/"]));
        assert_eq!(front["priority"], 2);
        assert_eq!(front["delegated_by"], format!("seat:{}", f.oracle.id));
        assert_eq!(fetched["body"], "Build the thing.\n\n- keep this list");
        assert_eq!(fetched["parse_error"], Value::Null);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let found = data(f.call(
            f.oracle.id,
            Ulid::new(),
            "knowledge_search",
            json!({"query":"type:task Den task written as source"}),
        ));
        if found.to_string().contains(&id) {
            break;
        }
        assert!(Instant::now() < deadline, "search never found it: {found}");
        std::thread::sleep(Duration::from_millis(20));
    }

    // Lifecycle: every step rewrites the managed block and nothing the author wrote.
    let step = |seat: Ulid, tool: &str, mut args: Value| {
        args["task_path"] = json!(task_path);
        data(f.mutate(seat, Ulid::new(), tool, args))
    };
    let delegated = step(
        f.oracle.id,
        "task_delegate",
        json!({"seat_id":f.own.id,"role":"executor"}),
    );
    assert_eq!(delegated["task_status"], "claimed");
    let claimed = assert_den_task(&f, "Claimed");
    assert_eq!(claimed.metadata["assignee"], format!("seat:{}", f.own.id));
    assert_eq!(claimed.body, "Build the thing.\n\n- keep this list");
    let reported = step(
        f.own.id,
        "task_report",
        json!({"summary":"Den fixture report: verified","verify_result":verification("true", 0, json!([]))}),
    );
    assert_eq!(reported["task_status"], "reported");
    let document = assert_den_task(&f, "Reported");
    assert!(document.body.starts_with("Build the thing.\n\n- keep this list\n\n## Report from "), "{}", document.body);
    assert!(den_text(&f, PATH).contains("\n    ## Report from seat:"), "{}", den_text(&f, PATH));
    assert_item_signature(&document, f.own.id);
    assert_eq!(step(f.oracle.id, "task_accept", json!({}))["task_status"], "accepted");
    assert_den_task(&f, "Accepted");
    assert_eq!(step(f.oracle.id, "task_close", json!({}))["task_status"], "done");
    let done = assert_den_task(&f, "Done");
    assert_item_signature(&done, f.oracle.id);
    println!("--- {PATH} after close ---\n{}", den_text(&f, PATH));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let verified = f.call(
            f.oracle.id,
            Ulid::new(),
            "provenance_verify",
            json!({"locator":id}),
        );
        if verified["isError"] == false
            && verified["structuredContent"]["document_signature"]["status"] == "valid"
            && verified["structuredContent"]["event"]["status"] == "valid"
        {
            break;
        }
        assert!(Instant::now() < deadline, "signature never verified: {verified}");
        std::thread::sleep(Duration::from_millis(20));
    }

    // An unsound source is refused at the door with its code, line and repair.
    let unsound = "module tasks.`2026-11-15-unsound`\n\ntask Unsound {\n  title \"No verify\"\n  brief \"x\"\n}\n";
    let refusal = refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "knowledge_write",
            json!({"path":"tasks/2026-11-15-unsound.den","frontmatter":{"type":"task"},"body":unsound}),
        ),
        "X354 line 3:1: a task needs a verify. Repair: add a `verify` field to Unsound",
    );
    assert_eq!(refusal["error"]["code"], "den_unsound", "{refusal}");
    assert!(!f.config.repository.join(".bboxy/tasks/2026-11-15-unsound.den").exists());

    // One record, one format.
    refused(
        f.mutate(
            f.oracle.id,
            Ulid::new(),
            "knowledge_write",
            json!({"path":"tasks/2026-11-15-den-task.md","frontmatter":{"type":"task","title":"Twin","verify":"true","allowed_paths":["fixture/**"]},"body":"Twin.\n"}),
        ),
        &format!("record already exists as {PATH}"),
    );
    assert!(!f.config.repository.join(".bboxy/tasks/2026-11-15-den-task.md").exists());

    // Frontmatter and body to a `.den` path: the host generates the Den.
    let args = |path: &str| json!({"path":path,"frontmatter":{"type":"task","title":"From \"frontmatter\"","verify":"true","allowed_paths":["fixture/**"],"custom":{"nested":["kept",42]}},"body":"Brief: keep this body.\n"});
    let generated = "tasks/2026-11-15-from-frontmatter.den";
    let record = data(f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", args(generated)))["record"].clone();
    assert_eq!(record["body"], "Brief: keep this body.\n");
    assert_eq!(record["frontmatter"]["custom"], json!({"nested":["kept",42]}));
    let text = den_text(&f, generated);
    println!("--- {generated} as generated by the host ---\n{text}");
    assert!(text.starts_with("module tasks.`2026-11-15-from-frontmatter`\n\ntask FromFrontmatter {\n  title \"From \\\"frontmatter\\\"\"\n  verify $ true\n  paths \"fixture/**\"\n  brief \"\"\"\n    Brief: keep this body.\n    \"\"\"\n\n  managed {\n"), "{text}");
    assert_item_signature(&bbt_mux::index::Document::parse(generated, &text), f.oracle.id);

    // A Markdown task is what it was.
    let legacy = "tasks/2026-11-15-legacy.md";
    data(f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", args(legacy)));
    assert!(den_text(&f, legacy).starts_with("---\n"));
    let delegated = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({"task_path":format!(".bboxy/{legacy}"),"seat_id":f.own.id,"role":"executor"}),
    ));
    assert_eq!(delegated["task_status"], "claimed");
    let document = bbt_mux::index::Document::parse(legacy, &den_text(&f, legacy));
    assert_eq!(document.metadata["status"], "claimed");
    assert_eq!(document.body, "Brief: keep this body.\n");
    assert_item_signature(&document, f.oracle.id);
}

#[test]
fn knowledge_index_follows_external_git_checkout_and_removal() {
    const LOCATOR: &str = "specs/external-git";
    const RECORD: &str = ".bboxy/specs/external-git.den";
    let git = |repository: &std::path::Path, args: &[&str]| {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repository)
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Knowledge Fixture")
            .env("GIT_AUTHOR_EMAIL", "knowledge@example.invalid")
            .env("GIT_COMMITTER_NAME", "Knowledge Fixture")
            .env("GIT_COMMITTER_EMAIL", "knowledge@example.invalid")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    let f = Fixture::open_seeded(false, false, |repository, _| {
        git(
            repository,
            &["init", "--quiet", "--initial-branch=main", "--template="],
        );
        std::fs::create_dir(repository.join(".bboxy/specs")).unwrap();
        std::fs::write(
            repository.join(".bboxy/specs/keep.md"),
            "---\ntype: spec\nnumber: 8764\n---\nExisting spec\n",
        )
        .unwrap();
        // Machine-local artifacts can claim the same number as an incoming tracked spec.
        std::fs::create_dir(repository.join(".bboxy/artifacts")).unwrap();
        std::fs::write(
            repository.join(".bboxy/artifacts/local.md"),
            "---\ntype: artifact\nnumber: 8765\n---\nLocal artifact\n",
        )
        .unwrap();
        git(repository, &["add", "-f", ".bboxy/specs/keep.md"]);
        git(repository, &["commit", "--quiet", "-m", "Seed"]);
        std::fs::write(
            repository.join(RECORD),
            r#"module specs.`external-git`

spec ExternalGit {
  title "Git checkout witness"
  intent "Observe Git writing a record to a mounted root."
  body "Arrived from Git."
  managed {
    id = json"\"01M00000000000000000000001\""
    number = json"8765"
  }
}
"#,
        )
        .unwrap();
        git(repository, &["add", "-f", RECORD]);
        git(repository, &["commit", "--quiet", "-m", "Add the Den spec"]);
        git(repository, &["branch", "with-record"]);
        git(repository, &["checkout", "--quiet", "--detach", "HEAD^"]);
    });
    let fetch = |locator: &str| {
        f.call(
            f.oracle.id,
            Ulid::new(),
            "knowledge_fetch",
            json!({"locator":locator}),
        )
    };
    let wait_for = |present: bool| {
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            let response = fetch(LOCATOR);
            let value = &response["structuredContent"];
            if present && response["isError"] == false {
                assert_eq!(value["body"], "Arrived from Git.");
                assert_eq!(value["parse_error"], Value::Null);
                return;
            }
            if !present && value["error"]["code"] == "not_found" {
                return;
            }
            assert_eq!(value["error"]["code"], "not_found", "{response}");
            assert!(Instant::now() < deadline, "present={present}: {response}");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    data(fetch("specs/keep"));
    let status = data(f.call(f.oracle.id, Ulid::new(), "knowledge_status", json!({})));
    let freshness = &status["index_freshness"];
    assert!(freshness["last_rescan_ms"].as_u64().unwrap() > 0);
    let event = freshness.get("last_event_ms").unwrap();
    assert!(event.is_null() || event.as_u64().is_some_and(|ms| ms > 0));
    wait_for(false);
    git(
        &f.config.repository,
        &["checkout", "--quiet", "with-record"],
    );
    assert!(f.config.repository.join(RECORD).is_file());
    wait_for(true);
    let search = || {
        data(f.call(
            f.oracle.id,
            Ulid::new(),
            "knowledge_search",
            json!({"query":"Git checkout witness"}),
        ))
    };
    assert_eq!(search()["items"][0]["ref"]["path"], "specs/external-git.den");
    knowledge_denied(fetch("#8765"), "number_conflict");
    git(
        &f.config.repository,
        &["checkout", "--quiet", "--detach", "HEAD^"],
    );
    assert!(!f.config.repository.join(RECORD).exists());
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        let hits = search();
        if hits["items"] == json!([]) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "deleted spec still indexed: {hits}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    wait_for(false);
    assert_eq!(data(fetch("#8765"))["path"], "artifacts/local.md");
}
