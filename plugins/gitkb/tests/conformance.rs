// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real GitKB fixtures, never a mock Markdown store or the operator's KB.
//! Translated from bboxy450-term `scripts/test-gitkb-plugin.py`. Run through
//! `make check-gitkb-plugin`, which supplies:
//! - `BBOXY_GITKB_PLUGIN`: the built plugin executable;
//! - `BBOXY_GITKB_BIN`: the pinned native backend;
//! - `BBOXY_GITKB_EVIDENCE`: where to write the JSON evidence;
//! - `BBOXY_GITKB_TRACE_TARGET`: private Cargo target for the traced rebuild.
//!
//! `BBOXY_GITKB_RETAIN=1` keeps the private fixture directory.

// A test process with no UI thread.
#![allow(clippy::disallowed_methods)]

use std::collections::BTreeSet;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use bbt_plugin_sdk::testkit::{self, Client, Failure};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const WAIT: Duration = Duration::from_secs(8);
const LONG: Duration = Duration::from_secs(16);
const PIXEL: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xb5, 0x1c, 0x0c,
    0x02, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xfc, 0xff, 0x1f, 0x00,
    0x02, 0xeb, 0x01, 0xf5, 0x8f, 0x59, 0x42, 0x67, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

fn env_path(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| panic!("Make must supply {name}"));
    PathBuf::from(value)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn which(name: &Path) -> PathBuf {
    if name.components().count() > 1 {
        return name.canonicalize().expect("backend path");
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
        .and_then(|p| p.canonicalize().ok())
        .expect("backend on PATH")
}

/// `file://` URI as the plugin writes it.
fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut uri = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~') {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

struct Fixture {
    run: PathBuf,
    plugin: PathBuf,
    backend: PathBuf,
    env: Vec<(String, String)>,
    commands: Vec<Value>,
    clients: usize,
}

impl Fixture {
    fn new() -> Self {
        let plugin = env_path("BBOXY_GITKB_PLUGIN")
            .canonicalize()
            .expect("built plugin");
        let backend = which(&env_path("BBOXY_GITKB_BIN"));
        let base = std::env::temp_dir().canonicalize().unwrap();
        let run = base.join(format!(
            "bboxy-gitkb-fixture-{}-{}",
            std::process::id(),
            &sha256(format!("{:?}", std::time::SystemTime::now()).as_bytes())[..8]
        ));
        std::fs::create_dir(&run).unwrap();
        std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700)).unwrap();
        let home = run.join("home");
        std::fs::create_dir(&home).unwrap();
        std::fs::write(
            home.join(".gitconfig"),
            "[user]\nname = Private GitKB Fixture\nemail = fixture@example.invalid\n",
        )
        .unwrap();
        let env = vec![
            ("PATH".into(), std::env::var("PATH").unwrap_or_default()),
            ("HOME".into(), home.display().to_string()),
            ("TMPDIR".into(), run.display().to_string()),
            ("LANG".into(), "en_US.UTF-8".into()),
            ("TERM".into(), "dumb".into()),
        ];
        Self {
            run,
            plugin,
            backend,
            env,
            commands: Vec::new(),
            clients: 0,
        }
    }

    fn run(
        &mut self,
        command: &[&str],
        cwd: &Path,
        env: &[(String, String)],
        stdin: Option<&str>,
    ) -> String {
        let mut child = Command::new(command[0])
            .args(&command[1..])
            .current_dir(cwd)
            .env_clear()
            .envs(env.iter().cloned())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("{command:?}: {e}"));
        let mut input = child.stdin.take().unwrap();
        input.write_all(stdin.unwrap_or("").as_bytes()).unwrap();
        drop(input);
        let output = child.wait_with_output().unwrap();
        let exit = output.status.code().unwrap_or(-1);
        self.commands
            .push(json!({"command": command, "cwd": cwd.display().to_string(), "exit": exit}));
        assert!(
            output.status.success(),
            "{command:?} exit {exit}\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn cli_text(&mut self, root: &Path, args: &[&str], stdin: Option<&str>) -> String {
        let backend = self.backend.display().to_string();
        let mut command = vec![backend.as_str(), "--no-pager", "--color", "never"];
        command.extend(args);
        let mut env = self.env.clone();
        env.push(("GITKB_ROOT".into(), root.display().to_string()));
        env.push(("GITKB_WORKSPACE".into(), "main".into()));
        self.run(&command, root, &env, stdin)
    }

    fn cli(&mut self, root: &Path, args: &[&str], stdin: Option<&str>) -> Value {
        serde_json::from_str(&self.cli_text(root, args, stdin)).expect("GitKB JSON")
    }

    fn source(&mut self, root: &Path, slug: &str, kind: &str) -> Value {
        self.cli(
            root,
            &["show", slug, "--source", kind, "--raw-content", "--json"],
            None,
        )
    }

    fn project(&mut self, name: &str) -> PathBuf {
        let root = self.run.join(name);
        std::fs::create_dir_all(root.join(".bboxy")).unwrap();
        let env = self.env.clone();
        self.run(&["git", "init", "--initial-branch=main"], &root, &env, None);
        self.run(
            &["git", "config", "user.name", "Private GitKB Fixture"],
            &root,
            &env,
            None,
        );
        self.run(
            &["git", "config", "user.email", "fixture@example.invalid"],
            &root,
            &env,
            None,
        );
        self.cli_text(
            &root,
            &["init", "--name", "Private bboxy GitKB conformance"],
            None,
        );
        root
    }

    fn connect(
        &mut self,
        root: &Path,
        binary: Option<&Path>,
        env: Option<Vec<(String, String)>>,
        fixture: bool,
    ) -> Result<Conn, Failure> {
        let data = self.run.join(format!("data-{}", self.clients));
        self.clients += 1;
        let binary = binary.unwrap_or(&self.plugin).to_owned();
        let env = env.unwrap_or_else(|| self.env.clone());
        let client = Client::spawn(&binary, &data, &env).expect("spawn plugin");
        let mut params = json!({"root": root.display().to_string(), "unknown_future": true});
        if fixture {
            params["fixture"] = json!({"version": 1, "kind": "data_source_conformance"});
        }
        let mut conn = Conn {
            client,
            data: data.canonicalize().unwrap(),
            initialization: Value::Null,
        };
        conn.initialization = conn
            .client
            .initialize(params, if fixture { LONG } else { WAIT })?;
        Ok(conn)
    }
}

struct Conn {
    client: Client,
    data: PathBuf,
    initialization: Value,
}

fn with_unknown(args: Value) -> Value {
    let mut args = args;
    args["unknown_future"] = true.into();
    args
}

impl Conn {
    fn result(&mut self, method: &str, args: Value) -> Result<Value, Failure> {
        let wait = if method == "commit" { LONG } else { WAIT };
        self.client
            .call(&format!("data_source_{method}"), with_unknown(args), wait)
    }

    fn call(&mut self, method: &str, args: Value) -> Value {
        match self.result(method, args.clone()) {
            Ok(value) => value,
            Err(failure) => panic!("{method} {args}: {failure:?}"),
        }
    }

    fn refuse(&mut self, method: &str, args: Value, code: &str) -> Value {
        match self.result(method, args.clone()) {
            Err(Failure::Tool(value)) if value["code"] == code => value,
            other => panic!("{method} {args}: expected {code}, got {other:?}"),
        }
    }

    fn rpc(&mut self, method: &str, params: Value) -> Result<Value, Failure> {
        self.client.request(method, params, WAIT)
    }

    fn create(&mut self, slug: &str, title: &str, kind: &str, fields: Value) -> Value {
        let mut fm = json!({"type": kind, "title": title});
        for (k, v) in fields.as_object().into_iter().flatten() {
            fm[k] = v.clone();
        }
        self.call(
            "write",
            json!({"path": format!("{slug}.md"), "frontmatter": fm, "body": format!("# {title}\n"),
                   "signed_record": format!("fixture opaque host proof {slug}")}),
        )["record"]
            .clone()
    }

    fn resolve(&mut self, locator: &str) -> Value {
        self.call("resolve", json!({ "locator": locator }))
    }

    fn collect(&mut self, method: &str, args: Value) -> Vec<Value> {
        let mut items = Vec::new();
        let mut request = args;
        for _ in 0..100 {
            let page = self.call(method, request.clone());
            items.extend(page["items"].as_array().unwrap().iter().cloned());
            if page["next"].is_null() {
                return items;
            }
            request["cursor"] = page["next"].clone();
        }
        panic!("pagination failed to terminate");
    }

    fn close(self) -> Option<i32> {
        self.client.close(Duration::from_secs(5))
    }
}

fn slugs(items: &[Value], key: &str) -> Vec<String> {
    items
        .iter()
        .map(|item| {
            let item = if key.is_empty() { item } else { &item[key] };
            item["slug"].as_str().unwrap().to_owned()
        })
        .collect()
}

fn reference(record: &Value) -> Value {
    let mut out = Map::new();
    for key in ["id", "external_id", "number", "slug", "path", "digest"] {
        out.insert(key.into(), record[key].clone());
    }
    Value::Object(out)
}

fn check(fx: &mut Fixture, conns: &mut Vec<Conn>) -> Vec<&'static str> {
    // Descriptor: `--shape` and a bare run with closed stdin agree.
    let shape = testkit::shape(&fx.plugin).unwrap();
    let mut bare = Command::new(&fx.plugin)
        .current_dir(&fx.run)
        .env_clear()
        .envs(fx.env.iter().cloned())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    drop(bare.stdin.take());
    let bare = bare.wait_with_output().unwrap();
    assert!(bare.status.success());
    let bare: Value = serde_json::from_slice(&bare.stdout).expect("bare run prints the shape");
    assert_eq!(shape, bare);
    assert_eq!(shape["name"], "GitKB");
    assert_eq!(shape["signed_writer"], true);
    let skill =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("SKILL.md")).unwrap();
    let skills = shape["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "gitkb");
    assert_eq!(skills[0]["content"], skill.as_str());
    let description = skills[0]["description"].as_str().unwrap();
    assert!(skill.contains(&format!("description: {description}\n")));
    let commands: BTreeSet<&str> = shape["systems"][0]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        commands,
        BTreeSet::from([
            "resolve",
            "fetch",
            "search",
            "list",
            "links",
            "graph",
            "write",
            "changes_since",
            "commit"
        ])
    );

    // Disposable native bootstrap.
    let a = fx.project("root-a");
    let b = fx.project("root-b");
    let checker = fx.run.join("checker-knowledge/.bboxy");
    std::fs::create_dir_all(&checker).unwrap();
    std::fs::write(
        checker.join("p0-fixture.md"),
        "---\nid: 00000000000000000000000007\ntype: note\nnumber: 7\ntitle: Plugin conformance\n---\nPlugin conformance fixture.\n",
    )
    .unwrap();
    let mut bootstrap = fx.connect(&checker, None, None, true).unwrap();
    assert_eq!(bootstrap.resolve("#7")["number"], 7);
    let written = bootstrap.call(
        "write",
        json!({"path": "p0-conformance.md", "frontmatter": {"type": "note", "title": "Plugin conformance"},
               "body": "P-0 conformance\n"}),
    );
    assert_eq!(written["record"]["slug"], "p0-conformance");
    let minor = bootstrap.call(
        "write",
        json!({"path": "p0-conformance-minor.md", "frontmatter": {"type": "note", "title": "Plugin conformance"},
               "body": "P-0 conformance\n", "p0_future_minor_field": {"ignored": true}}),
    );
    assert_eq!(minor["record"]["slug"], "p0-conformance-minor");
    conns.push(bootstrap);
    let mut repeat = fx.connect(&checker, None, None, true).unwrap();
    assert_eq!(repeat.resolve("#7")["number"], 7);
    conns.push(repeat);

    // Skill resources.
    let mut client = fx.connect(&a.join(".bboxy"), None, None, false).unwrap();
    let mut other = fx.connect(&b.join(".kb"), None, None, false).unwrap();
    assert_eq!(
        client.initialization["capabilities"]["resources"],
        json!({})
    );
    assert!(
        client.initialization["capabilities"]
            .get("extensions")
            .is_none()
    );
    let uri = "gitkb://skills/gitkb/SKILL.md";
    let resources = client.rpc("resources/list", json!({})).unwrap()["resources"].clone();
    assert_eq!(
        resources,
        json!([{"uri": uri, "name": "gitkb", "description": description, "mimeType": "text/markdown"}])
    );
    for conn in [&mut client, &mut other] {
        let contents =
            conn.rpc("resources/read", json!({ "uri": uri })).unwrap()["contents"].clone();
        assert_eq!(contents.as_array().unwrap().len(), 1);
        assert_eq!(contents[0]["uri"], uri);
        assert_eq!(contents[0]["mimeType"], "text/markdown");
        assert_eq!(contents[0]["text"], skill.as_str());
    }
    let escaped = format!("{uri}?path=../../outside");
    for unknown in [
        "gitkb://skills/other/SKILL.md",
        "gitkb://skills/gitkb/../SKILL.md",
        "gitkb://skills/gitkb/%2e%2e/SKILL.md",
        "gitkb://skills/gitkb/%53KILL.md",
        escaped.as_str(),
        "file:///etc/passwd",
    ] {
        match client.rpc("resources/read", json!({ "uri": unknown })) {
            Err(Failure::Protocol(error)) => {
                assert_eq!(error["code"], -32002, "{unknown}");
                assert_eq!(error["data"]["code"], "unknown_resource");
            }
            other => panic!("{unknown}: {other:?}"),
        }
    }
    assert_eq!(
        client.rpc("tools/list", json!({})).unwrap()["tools"]
            .as_array()
            .unwrap()
            .len(),
        11,
        "nine data_source methods plus create_next and sync_status"
    );

    // Unsigned write refusal; create with native fields.
    client.refuse(
        "write",
        json!({"path": "notes/unsigned.md", "frontmatter": {}, "body": ""}),
        "signed_writer_unavailable",
    );
    let one = client.create(
        "notes/shared",
        "Needle fixture",
        "note",
        json!({"id": "01ARZ3NDEKTSV4RRFFQ69G5FAV", "number": 77, "status": "active",
               "nested": {"preserved": [1, true, "x"]}, "due_date": "2026-11-01T00:00:00Z"}),
    );
    assert_eq!(one["id"], "01ARZ3NDEKTSV4RRFFQ69G5FAV");
    assert_eq!(one["number"], 77);
    assert_eq!(one["external_id"], "01563e3a-b5d3-d676-4c61-efb99302bd5b");
    let raw = fx.source(&a, "notes/shared", "workspace");
    assert_eq!(raw["document"]["id"], one["external_id"]);
    assert_eq!(
        raw["document"]["frontmatter"]["nested"],
        json!({"preserved": [1, true, "x"]})
    );
    // The host proof stays in the plugin data dir, never in the document.
    assert!(
        raw["document"]["frontmatter"]
            .get("bboxy_signed_record")
            .is_none()
    );
    assert_eq!(
        one["signed_record"],
        "fixture opaque host proof notes/shared"
    );
    assert!(one["frontmatter"].get("bboxy_signed_record").is_none());

    // Locator forms and fetch.
    let one_id = one["id"].as_str().unwrap().to_owned();
    let one_external = one["external_id"].as_str().unwrap().to_owned();
    for locator in [
        "notes/shared",
        "notes/shared.md",
        one_id.as_str(),
        one_external.as_str(),
        "#77",
    ] {
        assert_eq!(client.resolve(locator)["id"], one["id"], "{locator}");
    }
    let shared_ref = client.resolve("notes/shared");
    let fetched = client.call("fetch", json!({ "ref": shared_ref }));
    assert_eq!(fetched["frontmatter"]["due_date"], "2026-11-01T00:00:00Z");
    assert_eq!(fetched["body"], "# Needle fixture\n");

    // Same slug in two roots.
    let sibling = other.create("notes/shared", "Other reality", "note", json!({}));
    assert_ne!(sibling["id"], one["id"]);
    other.refuse("fetch", json!({ "ref": shared_ref }), "stale_ref");
    assert_eq!(other.resolve("notes/shared")["id"], sibling["id"]);

    // Change cursors.
    let baseline = client.call("changes_since", json!({"cursor": "0"}))["next"].clone();
    let unchanged = client.call("changes_since", json!({ "cursor": baseline }));
    assert_eq!(unchanged["changed"], json!([]));
    let baseline = unchanged["next"].clone();
    let mut task = client.create(
        "tasks/harmony-7",
        "Lifecycle",
        "task",
        json!({"id": "01ARZ3NDEKTSV4RRFFQ69G5FAW", "status": "open", "swarm": "fixture-a",
               "assignee": "seat:fixture"}),
    );
    assert_eq!(task["number"], 7);
    let task_slug = task["slug"].as_str().unwrap().to_owned();
    let changes = client.call("changes_since", json!({ "cursor": baseline }));
    assert_eq!(
        slugs(changes["changed"].as_array().unwrap(), ""),
        std::slice::from_ref(&task_slug)
    );
    assert_eq!(
        client.call("changes_since", json!({"cursor": changes["next"]}))["changed"],
        json!([])
    );
    other.refuse(
        "changes_since",
        json!({"cursor": changes["next"]}),
        "stale_cursor",
    );

    // Lifecycle through GitKB's native status alone.
    for status in ["claimed", "running", "reported", "accepted"] {
        let r = client.resolve(&task_slug);
        task = client.call("fetch", json!({ "ref": r }));
        let mut fm = task["frontmatter"].clone();
        fm["status"] = status.into();
        let proof = format!("fixture admitted intent {status}");
        let update = client.call(
            "write",
            json!({"ref": r, "etag": task["etag"], "frontmatter": fm, "body": task["body"],
                   "signed_record": proof}),
        );
        task = update["record"].clone();
        assert_eq!(update["signed_record"], proof.as_str());
        assert_eq!(task["signed_record"], proof.as_str());
        assert_eq!(task["frontmatter"]["status"], status);
        assert_eq!(update["lossy"], false);
        assert_eq!(task["external_id"], "01563e3a-b5d3-d676-4c61-efb99302bd5c");
        let actual = fx.source(&a, &task_slug, "workspace");
        let native = &actual["document"]["frontmatter"];
        assert!(native.get("bboxy_status").is_none());
        assert!(native.get("bboxy_signed_record").is_none());
        assert_eq!(
            native["status"],
            if status == "running" {
                "active"
            } else {
                status
            }
        );
        if status == "running" {
            // Status filters accept the portable or the native value.
            for filter in [
                json!({"type": "task", "status": "running"}),
                json!({"type": "task", "status": "active"}),
            ] {
                let items = client.collect("list", json!({ "filter": filter }));
                assert_eq!(
                    slugs(&items, ""),
                    std::slice::from_ref(&task_slug),
                    "{filter}"
                );
            }
            for query in ["type:task status:running", "type:task status:active"] {
                let items = client.collect("search", json!({ "query": query }));
                assert_eq!(
                    slugs(&items, "ref"),
                    std::slice::from_ref(&task_slug),
                    "{query}"
                );
            }
        }
    }

    // Filters.
    let accepted = json!({"filter": {"type": "task", "status": "accepted"}});
    assert_eq!(
        client.call("list", accepted)["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let narrowed = json!({"filter": {"type": "task", "status": "accepted", "swarm": "fixture-a",
                                     "assignee": "seat:fixture"}});
    assert_eq!(
        client.call("list", narrowed)["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        client.call(
            "list",
            json!({"filter": {"type": "task", "swarm": "fixture-b"}})
        )["items"],
        json!([])
    );
    assert_eq!(
        client.call(
            "list",
            json!({"filter": {"type": "task", "assignee": "seat:other"}})
        )["items"],
        json!([])
    );
    assert!(
        client
            .collect(
                "search",
                json!({"query": "Lifecycle type:task", "filter": {"swarm": "fixture-b"}})
            )
            .is_empty()
    );
    assert!(
        client
            .collect(
                "search",
                json!({"query": "Lifecycle type:task", "filter": {"assignee": "seat:other"}})
            )
            .is_empty()
    );
    let hits = client.collect(
        "search",
        json!({"query": "Lifecycle type:task", "filter": {"swarm": "fixture-a", "assignee": "seat:fixture"}}),
    );
    assert_eq!(slugs(&hits, "ref"), std::slice::from_ref(&task_slug));

    // Swarm graph.
    let absent = client.call(
        "graph",
        json!({"scope": {"kind": "swarm", "graph": "task", "swarm": "fixture-b"}}),
    );
    assert_eq!(absent["nodes"], json!([]));
    assert_eq!(absent["edges"], json!([]));
    let scoped = client.call(
        "graph",
        json!({"scope": {"kind": "swarm", "graph": "task", "swarm": "fixture-a"}}),
    );
    let ids: Vec<&Value> = scoped["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| &n["id"])
        .collect();
    assert_eq!(ids, [&task["id"]]);

    // Explicit native commit writes only the selected task.
    let task_ref = client.resolve(&task_slug);
    assert!(task_ref["updated_at"].is_u64(), "{task_ref}");
    assert_eq!(
        scoped["nodes"][0]["ref"]["updated_at"],
        task_ref["updated_at"]
    );
    let commit = client.call(
        "commit",
        json!({"refs": [task_ref], "message": "Commit only selected fixture task"}),
    );
    assert!(truthy(&commit["commit"]));
    assert_eq!(
        slugs(commit["committed"].as_array().unwrap(), ""),
        std::slice::from_ref(&task_slug)
    );
    let committed = fx.source(&a, &task_slug, "committed");
    assert_eq!(committed["exists"], true);
    assert_eq!(committed["document"]["frontmatter"]["status"], "accepted");
    assert_eq!(fx.source(&a, "notes/shared", "committed")["exists"], false);
    assert_eq!(other.resolve("notes/shared")["id"], sibling["id"]);
    other.refuse(
        "commit",
        json!({"refs": commit["committed"], "message": "Cannot cross roots"}),
        "not_found",
    );

    // Ambiguous #7.
    client.create(
        "notes/other-7",
        "Duplicate family number",
        "note",
        json!({}),
    );
    let ambiguous = client.refuse("resolve", json!({"locator": "#7"}), "ambiguous_locator");
    assert_eq!(ambiguous["candidates"].as_array().unwrap().len(), 2);

    // Paging and stale cursors.
    let first = client.call("list", json!({"filter": {"type": "note"}, "limit": 1}));
    assert!(truthy(&first["next"]));
    let second = client.call(
        "list",
        json!({"filter": {"type": "note"}, "limit": 1, "cursor": first["next"]}),
    );
    assert_ne!(first["items"][0]["id"], second["items"][0]["id"]);
    other.refuse(
        "list",
        json!({"limit": 1, "cursor": first["next"]}),
        "stale_cursor",
    );
    client.refuse(
        "list",
        json!({"filter": {"type": "task"}, "cursor": first["next"]}),
        "stale_cursor",
    );
    let full = client.call("list", json!({"filter": {"type": "note"}, "limit": 32}));
    // A full page holds every note at once (notes/shared, notes/other-7).
    assert!(full["next"].is_null(), "{full}");
    assert_eq!(full["items"].as_array().unwrap().len(), 2, "{full}");

    // Strict search.
    let needle = client.collect("search", json!({"query": "Needle type:note", "limit": 1}));
    assert_eq!(needle[0]["ref"]["slug"], "notes/shared");
    client.create("notes/one-term", "Needle only", "note", json!({}));
    let strict = client.collect(
        "search",
        json!({"query": "Needle fixture type:note", "limit": 1}),
    );
    assert_eq!(slugs(&strict, "ref"), ["notes/shared"]);
    let refs = json!([
        client.resolve("notes/shared"),
        client.resolve("notes/one-term")
    ]);
    client.call(
        "commit",
        json!({"refs": refs, "message": "Private native strict-search fixture"}),
    );
    let strict = client.collect(
        "search",
        json!({"query": "Needle fixture type:note", "limit": 1}),
    );
    assert_eq!(slugs(&strict, "ref"), ["notes/shared"]);

    // External edit: stale_ref and stale_cursor.
    let old = client.resolve("notes/shared");
    let record = client.call("fetch", json!({ "ref": old }));
    let source = fx.source(&a, "notes/shared", "workspace");
    let edited = format!(
        "{}\nexternal edit\n",
        source["rawContent"].as_str().unwrap()
    );
    fx.cli(
        &a,
        &[
            "workspace",
            "write",
            "notes/shared",
            "--stdin",
            "--if-generation",
            source["generation"].as_str().unwrap(),
            "--origin-id",
            "external-fixture-editor",
            "--json",
        ],
        Some(&edited),
    );
    client.refuse("fetch", json!({ "ref": old }), "stale_ref");
    client.refuse(
        "write",
        json!({"ref": old, "etag": record["etag"], "frontmatter": record["frontmatter"],
               "body": "must not overwrite", "signed_record": "fixture stale"}),
        "stale_ref",
    );
    client.refuse(
        "list",
        json!({"filter": {"type": "note"}, "limit": 1, "cursor": first["next"]}),
        "stale_cursor",
    );
    assert!(
        fx.source(&a, "notes/shared", "workspace")["rawContent"]
            .as_str()
            .unwrap()
            .contains("external edit")
    );

    // Links.
    let current_ref = client.resolve("notes/shared");
    let current = client.call("fetch", json!({ "ref": current_ref }));
    // updated_at: epoch milliseconds, equal on every path, advanced by a write.
    assert!(old["updated_at"].is_u64(), "{old}");
    assert!(
        current_ref["updated_at"].as_u64() > old["updated_at"].as_u64(),
        "a write advances updated_at: {old} then {current_ref}"
    );
    assert_eq!(current["updated_at"], current_ref["updated_at"]);
    let listed = client.collect("list", json!({"filter": {"type": "note"}}));
    let listed = listed.iter().find(|r| r["slug"] == "notes/shared").unwrap();
    assert_eq!(listed["updated_at"], current_ref["updated_at"]);
    let linked = client.call(
        "write",
        json!({"ref": reference(&current), "etag": current["etag"], "frontmatter": current["frontmatter"],
               "body": format!("{}\n[[tasks/harmony-7]]\n", current["body"].as_str().unwrap()),
               "signed_record": "fixture link"}),
    )["record"]
        .clone();
    let linked_ref = client.resolve(linked["slug"].as_str().unwrap());
    let outgoing = client.call("links", json!({ "ref": linked_ref }))["outgoing"].clone();
    assert!(
        outgoing
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["to"] == task["id"] && e["rel"] == "references")
    );
    let task_ref = client.resolve(&task_slug);
    let incoming = client.call("links", json!({ "ref": task_ref }))["incoming"].clone();
    assert!(
        incoming
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["from"] == linked["id"])
    );

    // Graph paging: a one-node page continues; full pages cover every record once.
    let graph = client.call(
        "graph",
        json!({"scope": {"kind": "all", "graph": "markdown"}, "limit": 1}),
    );
    assert_eq!(graph["nodes"].as_array().unwrap().len(), 1);
    assert!(truthy(&graph["next"]));
    assert_eq!(graph["edges_truncated"], false);
    let mut nodes = Vec::new();
    let mut request = json!({"scope": {"kind": "all", "graph": "markdown"}, "limit": 2});
    let mut pages = 0;
    loop {
        let page = client.call("graph", request.clone());
        assert!(page["nodes"].as_array().unwrap().len() <= 2);
        nodes.extend(
            page["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n["id"].as_str().unwrap().to_owned()),
        );
        pages += 1;
        assert!(pages < 100, "graph continuation did not finish");
        if page["next"].is_null() {
            break;
        }
        request["cursor"] = page["next"].clone();
    }
    let expected: BTreeSet<String> = client
        .collect("list", json!({"limit": 32}))
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect();
    let unique: BTreeSet<String> = nodes.iter().cloned().collect();
    assert_eq!(nodes.len(), unique.len());
    assert_eq!(unique, expected);

    // Refusals.
    client.refuse(
        "graph",
        json!({"scope": {"kind": "all", "graph": "execution"}}),
        "unsupported_graph",
    );
    client.refuse(
        "resolve",
        json!({"locator": "../outside"}),
        "path_outside_root",
    );
    client.refuse("list", json!({"limit": 0}), "invalid_limit");
    client.refuse(
        "resolve",
        json!({"locator": "notes/shared", "root": b.display().to_string()}),
        "host_field_in_arguments",
    );
    client.refuse(
        "write",
        json!({"path": "../escape.md", "frontmatter": {"title": "x", "type": "note"}, "body": "x",
               "signed_record": "fixture"}),
        "path_outside_root",
    );
    client.refuse(
        "write",
        json!({"path": "notes/shared.md", "frontmatter": {"title": "x", "type": "note"}, "body": "x",
               "signed_record": "fixture"}),
        "path_exists",
    );

    // Image resources, including symlink refusal.
    std::fs::create_dir(a.join("images")).unwrap();
    std::fs::write(a.join("images/pixel.png"), PIXEL).unwrap();
    let image = client.create(
        "notes/image",
        "Image",
        "note",
        json!({"id": "01ARZ3NDEKTSV4RRFFQ69G5FAX"}),
    );
    let image_ref = client.resolve("notes/image");
    let image = client.call(
        "write",
        json!({"ref": image_ref, "etag": image["etag"],
               "frontmatter": image["frontmatter"], "body": "![[images/pixel.png]]\n",
               "signed_record": "fixture image"}),
    )["record"]
        .clone();
    let image_slug = image["slug"].as_str().unwrap().to_owned();
    let image_ref = client.resolve(&image_slug);
    let resource = client.call("fetch", json!({ "ref": image_ref }))["resources"][0].clone();
    let prefix = format!("{}/resources/", file_uri(&client.data));
    assert!(
        resource["uri"].as_str().unwrap().starts_with(&prefix),
        "{resource}"
    );
    assert!(resource["size"].as_u64().unwrap() > 0);
    assert_eq!(resource["mimeType"], "image/png");
    std::fs::remove_file(a.join("images/pixel.png")).unwrap();
    std::os::unix::fs::symlink(b.join("outside.png"), a.join("images/pixel.png")).unwrap();
    let image_ref = client.resolve(&image_slug);
    client.refuse(
        "fetch",
        json!({ "ref": image_ref }),
        "resource_outside_data_dir",
    );

    // Canonical committed source materializes through native generation CAS.
    fx.cli(
        &a,
        &[
            "commit",
            "--json",
            "-m",
            "Private conformance fixture",
            "notes/shared",
        ],
        None,
    );
    fx.cli_text(&a, &["clear", "notes/shared", "--force"], None);
    let committed_ref = client.resolve("notes/shared");
    let committed = client.call("fetch", json!({ "ref": committed_ref }));
    assert_eq!(committed["gitkb"]["source"]["kind"], "committed");
    // Batched (committed, no workspace file) and exact references agree.
    let listed = client.collect("list", json!({"limit": 32}));
    for item in &listed {
        assert_eq!(item, &client.resolve(item["slug"].as_str().unwrap()));
    }
    assert!(listed.iter().any(|item| item["slug"] == "notes/shared"));
    // Every graph node, batched (committed-only) or exact, carries exactly
    // what fetch returns.
    let mut batched = 0;
    let mut request = json!({"scope": {"kind": "all", "graph": "markdown"}, "limit": 128});
    loop {
        let page = client.call("graph", request.clone());
        for node in page["nodes"].as_array().unwrap() {
            if node["ref"]["slug"] == image_slug.as_str() {
                continue; // Its fetch is refused above: the image is now a symlink.
            }
            let record = client.call("fetch", json!({"ref": node["ref"]}));
            assert_eq!(
                node["frontmatter"], record["frontmatter"],
                "{}",
                node["ref"]
            );
            assert_eq!(node["ref"]["digest"], record["digest"]);
            assert_eq!(node["id"], record["id"]);
            assert_eq!(node["title"], record["frontmatter"]["title"]);
            assert_eq!(node["type"], record["frontmatter"]["type"]);
            if record["gitkb"]["source"]["kind"] == "committed" {
                batched += 1;
            }
        }
        if page["next"].is_null() {
            break;
        }
        request["cursor"] = page["next"].clone();
    }
    assert!(
        batched > 0,
        "no committed-only node exercised the batch path"
    );
    let committed_ref = client.resolve("notes/shared");
    client.call(
        "write",
        json!({"ref": committed_ref, "etag": committed["etag"],
               "frontmatter": committed["frontmatter"], "body": committed["body"],
               "signed_record": "fixture committed update"}),
    );

    // Catalog index: identical results to the backend path, and freshness.
    conns.extend(index_checks(fx));

    // Traced backend with a hostile environment proves child env and cwd.
    let trace = fx.run.join("backend-trace.tsv");
    let wrapper = fx.run.join("traced-git-kb");
    std::fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nprintf '%s\\t%s\\t%s\\t%s\\n' \"$(pwd -P)\" \"${{GITKB_ROOT-}}\" \"$(env | sed 's/=.*//' | sort | tr '\\n' ',')\" \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
            trace.display(),
            fx.backend.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let target = env_path("BBOXY_GITKB_TRACE_TARGET");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(&cargo)
        .args([
            "build",
            "--locked",
            "-p",
            "bboxy450-term-data_source-gitkb",
            "--bin",
            "bboxy450-term-data_source-gitkb",
            "--target-dir",
        ])
        .arg(&target)
        .env("BBOXY_GITKB_BIN", &wrapper)
        // A private build dir: never wait on, or feed, a shared one.
        .env("CARGO_BUILD_BUILD_DIR", target.join("build-dir"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap();
    fx.commands
        .push(json!({"command": ["cargo", "build", "traced plugin"],
        "cwd": env!("CARGO_MANIFEST_DIR"), "exit": status.code()}));
    assert!(status.success(), "traced plugin build");
    let traced_plugin = target.join("debug/bboxy450-term-data_source-gitkb");
    let mut hostile = fx.env.clone();
    hostile.extend([
        ("GITKB_ROOT".into(), b.display().to_string()),
        ("GITKB_WORKSPACE".into(), "wrong".into()),
        ("BBOXY_CAPABILITY".into(), "fixture-never-forward".into()),
        ("API_TOKEN".into(), "fixture-never-forward".into()),
    ]);
    let mut traced = fx
        .connect(&a, Some(&traced_plugin), Some(hostile), false)
        .unwrap();
    traced.resolve("notes/shared");
    // The background export also runs through the traced backend; let it
    // finish so no backend start can race the pin check below.
    wait_ready(&mut traced);
    let rows: Vec<Vec<String>> = std::fs::read_to_string(&trace)
        .unwrap()
        .lines()
        .map(|l| l.split('\t').map(str::to_owned).collect::<Vec<_>>())
        .filter(|r| r.get(3).is_some_and(|args| !args.contains("--version")))
        .collect();
    assert!(!rows.is_empty());
    let allowed = [
        "PATH",
        "HOME",
        "LANG",
        "TMPDIR",
        "TERM",
        "LC_CTYPE",
        "GITKB_ROOT",
        "GITKB_WORKSPACE",
        // Added by /bin/sh or macOS for the wrapper process itself.
        "PWD",
        "OLDPWD",
        "SHLVL",
        "_",
        "__CF_USER_TEXT_ENCODING",
    ];
    for row in &rows {
        assert_eq!(row[0], traced.data.display().to_string(), "{row:?}");
        assert_eq!(row[1], a.display().to_string(), "{row:?}");
        for name in row[2].split(',').filter(|n| !n.is_empty()) {
            assert!(allowed.contains(&name), "forwarded {name}: {row:?}");
        }
    }

    // Pin enforcement: an existing client and a new client refuse a changed backend.
    let mut script = std::fs::read_to_string(&wrapper).unwrap();
    script.push_str("\n# dependency changed\n");
    std::fs::write(&wrapper, script).unwrap();
    let before = std::fs::read(&trace).unwrap();
    traced.refuse(
        "resolve",
        json!({"locator": "notes/shared"}),
        "backend_changed",
    );
    assert_eq!(
        std::fs::read(&trace).unwrap(),
        before,
        "changed backend executed through the existing client"
    );
    conns.push(traced);
    match fx.connect(&a, Some(&traced_plugin), None, false) {
        Err(failure) => assert_eq!(failure.code(), Some("backend_changed"), "{failure:?}"),
        Ok(_) => panic!("changed backend unexpectedly admitted"),
    }

    // Installer writes only explicit local artifacts and notices.
    let config = b.join(".bboxy/settings.toml");
    std::fs::write(&config, "peer_config = 'preserve'\n").unwrap();
    // An example, not a binary: only the plugin executable is installable (decision 362).
    let installer = env_path("BBOXY_GITKB_INSTALLER").display().to_string();
    let installer = installer.as_str();
    let plugin = fx.plugin.display().to_string();
    let workspace = b.display().to_string();
    let env = fx.env.clone();
    let run = fx.run.clone();
    fx.run(
        &[installer, &plugin, "--workspace", &workspace],
        &run,
        &env,
        None,
    );
    assert_eq!(
        std::fs::read_to_string(&config).unwrap(),
        "peer_config = 'preserve'\n"
    );
    let installed: Vec<String> = std::fs::read_dir(b.join(".bboxy/plugins"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(installed, ["bboxy450-term-data_source-gitkb"]);
    assert!(b.join(".bboxy/plugin-notices/gitkb/NOTICE").is_file());
    let mode = std::fs::metadata(b.join(".bboxy/plugins/bboxy450-term-data_source-gitkb"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);

    conns.push(client);
    conns.push(other);
    vec![
        "shape",
        "canonical embedded skill and fixed MCP resource",
        "unknown and escaped skill URIs refused",
        "all nine tools",
        "real source-aware create/update",
        "UUID/ULID and numbers",
        "host intent storage and lifecycle projection (signature verification belongs to host)",
        "status filters accept portable and native values",
        "two isolated roots with same slug",
        "query/root/snapshot cursors",
        "native incoming/outgoing graph",
        "stale external edit rejection",
        "confined resource copy",
        "committed/workspace source transition",
        "batched and exact references agree",
        "real selected-only commit, unrelated pending source preserved",
        "catalog index equals the backend path; workspace and commit freshness",
        "committed references digest id and version; an external commit makes them stale",
        "disposable native bootstrap",
        "scrubbed native process environment and private cwd",
        "backend hash pin on existing and new clients",
        "local installation preserves config",
        "clean shutdown",
    ]
}

fn truthy(value: &Value) -> bool {
    !(value.is_null() || value == "" || value == false)
}

/// The plugin's index facts from `health`.
fn index_facts(conn: &mut Conn) -> Value {
    conn.client.call("health", json!({}), WAIT).unwrap()["index"].clone()
}

/// Poll `health` until the committed index matches the current store.
fn wait_ready(conn: &mut Conn) -> Value {
    for _ in 0..400 {
        let facts = index_facts(conn);
        if facts["ready"] == true {
            return facts;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("index did not become ready: {}", index_facts(conn));
}

/// A native workspace write through the real CLI (another editor).
fn native_write(fx: &mut Fixture, root: &Path, slug: &str, fields: &str, update: bool) {
    let mut args = vec![
        "workspace",
        "write",
        slug,
        "--stdin",
        "--origin-id",
        "index-fixture-editor",
        "--json",
        "--create-if-missing",
    ];
    let mut identity = String::new();
    let generation;
    if update {
        // A cleared workspace has no document: the committed one supplies the
        // identity, and the absent workspace generation guards the write.
        let source = fx.source(root, slug, "workspace");
        let id = match source["document"]["id"].as_str() {
            Some(id) => id.to_owned(),
            None => fx.source(root, slug, "committed")["document"]["id"]
                .as_str()
                .unwrap()
                .to_owned(),
        };
        identity = format!("id: {id}\n");
        generation = source["generation"].as_str().unwrap().to_owned();
        args.extend(["--if-generation", &generation]);
    }
    let raw = format!("---\n{identity}slug: {slug}\ntitle: {slug}\n{fields}---\nbody {slug}\n");
    fx.cli(root, &args, Some(&raw));
}

/// Checked-out assignments are visible before commit. The native-path parity
/// matrix uses matching committed/workspace fields: native `--where` intentionally
/// reads committed fields, while the index serves the selected workspace source.
/// Workspace writes change only the workspace signal; commits rebuild the index.
fn index_checks(fx: &mut Fixture) -> Vec<Conn> {
    let root = fx.project("root-index");
    for (slug, fields) in [
        ("tasks/a", "type: task\nstatus: active\nassignee: alice\n"),
        (
            "tasks/b",
            "type: task\nstatus: draft\nassignee: bob\nbboxy_status: claimed\n",
        ),
        ("tasks/c", "type: task\nstatus: active\nassignee: carol\n"),
        (
            "tasks/d",
            "type: task\nstatus: Active\nassignee: seat:01JABCDEFGHJKMNPQRSTVWXYZ0\n",
        ),
        (
            "tasks/e",
            "type: task\nstatus: active\nassignee: '123'\nswarm: x y\n",
        ),
        ("bugs/h", "type: bug\nstatus: active\nassignee: bob\n"),
        ("notes/n", "type: note\nstatus: draft\nassignee: carol\n"),
    ] {
        native_write(fx, &root, slug, fields, false);
    }
    fx.cli(
        &root,
        &[
            "commit",
            "--json",
            "-m",
            "Index fixture",
            "--",
            "tasks/a",
            "tasks/b",
            "tasks/c",
            "tasks/d",
            "tasks/e",
            "bugs/h",
            "notes/n",
        ],
        None,
    );
    // Committed alice, workspace carol: the index follows the workspace assignment.
    native_write(
        fx,
        &root,
        "tasks/a",
        "type: task\nstatus: completed\nassignee: carol\nswarm: s9\n",
        true,
    );
    native_write(
        fx,
        &root,
        "tasks/w",
        "type: task\nstatus: active\nassignee: carol\nswarm: s9\n",
        false,
    );
    let mut off_env = fx.env.clone();
    off_env.push(("BBOXY_GITKB_INDEX".into(), "off".into()));
    let mut on_env = fx.env.clone();
    on_env.push(("BBOXY_GITKB_INDEX".into(), "on".into()));
    let mut on = fx.connect(&root, None, Some(on_env), false).unwrap();
    // A cold process's first filtered read waits for the build in progress
    // instead of taking the per-call backend path.
    let carol_task = json!({"filter": {"type": "task", "assignee": "carol"}});
    assert_eq!(
        slugs(&on.collect("list", carol_task.clone()), ""),
        ["tasks/a", "tasks/c", "tasks/w"]
    );
    let cold = index_facts(&mut on);
    assert_eq!(cold["fallbacks"], 0, "{cold}");
    assert_eq!(cold["served"], 1, "{cold}");
    let mut off = fx.connect(&root, None, Some(off_env), false).unwrap();
    assert_eq!(index_facts(&mut off)["enabled"], false);
    assert_eq!(
        slugs(&off.collect("list", carol_task), ""),
        ["tasks/c", "tasks/w"]
    );
    // Compare the two paths with the same source fields. Keep the uncommitted
    // assignment witness above; do not require it to match native --where.
    native_write(
        fx,
        &root,
        "tasks/a",
        "type: task\nstatus: active\nassignee: alice\n",
        true,
    );
    let ready = wait_ready(&mut on);
    assert_eq!(ready["documents"], 7, "{ready}");
    let filters = || {
        let mut all = vec![
            json!({"type": "task"}),
            json!({"assignee": "carol"}),
            json!({"swarm": "s9"}),
            json!({"swarm": "x y"}),
            json!({"type": "task", "status": "running"}),
            json!({"status": "active"}),
            json!({"type": "task", "status": "open"}),
            json!({"type": "task", "status": "claimed"}),
            json!({"type": "task", "status": "completed"}),
            json!({"type": "task", "status": "claimed", "assignee": "bob"}),
        ];
        for assignee in [
            "alice",
            "carol",
            "bob",
            "123",
            "seat:01JABCDEFGHJKMNPQRSTVWXYZ0",
            "01JABCDEFGHJKMNPQRSTVWXYZ0",
            "seat:01JZZZZZZZZZZZZZZZZZZZZZZZ",
        ] {
            for kind in ["task", "bug", "epic"] {
                all.push(json!({"type": kind, "assignee": assignee}));
            }
        }
        all
    };
    let compare = |on: &mut Conn, off: &mut Conn| {
        for filter in filters() {
            let request = json!({ "filter": filter });
            let served = index_facts(on)["served"].as_u64().unwrap();
            let indexed = on.collect("list", request.clone());
            assert_eq!(indexed, off.collect("list", request.clone()), "{filter}");
            let search = json!({"query": "body type:task", "filter": filter});
            assert_eq!(
                on.collect("search", search.clone()),
                off.collect("search", search),
                "{filter}"
            );
            // Values native parsing might type or split use the backend.
            let indexed = filter["swarm"] != "x y" && filter["assignee"] != "123";
            let now = index_facts(on)["served"].as_u64().unwrap();
            assert_eq!(now > served, indexed, "{filter}");
        }
        let graph = json!({"scope": {"kind": "type", "graph": "markdown", "type": "task"}});
        assert_eq!(on.call("graph", graph.clone()), off.call("graph", graph));
    };
    compare(&mut on, &mut off);
    let carol = json!({"filter": {"type": "task", "assignee": "carol"}});
    assert_eq!(
        slugs(&on.collect("list", carol.clone()), ""),
        ["tasks/c", "tasks/w"]
    );

    // An uncommitted reassignment must remain visible alongside workspace-only tasks.
    native_write(
        fx,
        &root,
        "tasks/a",
        "type: task\nstatus: completed\nassignee: carol\nswarm: s9\n",
        true,
    );
    // A workspace write changes the workspace signal, not the store signal.
    let before = index_facts(&mut on);
    native_write(
        fx,
        &root,
        "tasks/x",
        "type: task\nstatus: active\nassignee: carol\n",
        false,
    );
    let written = index_facts(&mut on);
    assert_eq!(written["store_signal"], before["store_signal"]);
    assert_ne!(written["workspace_signal"], before["workspace_signal"]);
    assert_eq!(written["ready"], true);
    assert_eq!(
        slugs(&on.collect("list", carol.clone()), ""),
        ["tasks/a", "tasks/c", "tasks/w", "tasks/x"]
    );
    // A commit changes the store signal; filters fall back until the rebuild.
    fx.cli(
        &root,
        &["commit", "--json", "-m", "Commit workspace edit", "tasks/a"],
        None,
    );
    let committed = index_facts(&mut on);
    assert_ne!(committed["store_signal"], written["store_signal"]);
    // The next filtered call starts the rebuild and waits for it within its
    // budget instead of falling back.
    let count = |facts: &Value, key: &str| facts[key].as_u64().unwrap();
    let listed = on.collect("list", carol.clone());
    assert_eq!(listed, off.collect("list", carol.clone()));
    assert_eq!(
        slugs(&listed, ""),
        ["tasks/a", "tasks/c", "tasks/w", "tasks/x"]
    );
    let after = wait_ready(&mut on);
    assert_eq!(count(&after, "fallbacks"), count(&committed, "fallbacks"));
    assert_eq!(count(&after, "waits"), count(&committed, "waits") + 1);
    assert!(count(&after, "served") > count(&committed, "served"));
    assert_eq!(after["indexed_signal"], after["store_signal"]);
    assert!(after["builds"].as_u64().unwrap() >= 2, "{after}");
    compare(&mut on, &mut off);

    // A committed-only reference digests its native id and version: an
    // external commit bumps the version, so the old reference goes stale,
    // and index-served graph nodes carry the new committed reference.
    fx.cli_text(&root, &["clear", "tasks/c", "--force"], None);
    let old = on.resolve("tasks/c");
    assert_eq!(
        on.call("fetch", json!({ "ref": old }))["gitkb"]["source"]["kind"],
        "committed"
    );
    native_write(
        fx,
        &root,
        "tasks/c",
        "type: task\nstatus: active\nassignee: carol\nswarm: s7\n",
        true,
    );
    fx.cli(
        &root,
        &["commit", "--json", "-m", "External commit", "tasks/c"],
        None,
    );
    fx.cli_text(&root, &["clear", "tasks/c", "--force"], None);
    on.refuse("fetch", json!({ "ref": old }), "stale_ref");
    off.refuse("fetch", json!({ "ref": old }), "stale_ref");
    let new = on.resolve("tasks/c");
    assert_ne!(new["digest"], old["digest"]);
    assert_eq!(new, off.resolve("tasks/c"));
    // A filtered read starts the rebuild for the new store state.
    on.collect("list", carol.clone());
    wait_ready(&mut on);
    let graph = json!({"scope": {"kind": "type", "graph": "markdown", "type": "task"}});
    let node = on.call("graph", graph)["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["ref"]["slug"] == "tasks/c")
        .cloned()
        .unwrap();
    assert_eq!(node["ref"], new);
    let record = on.call("fetch", json!({ "ref": new }));
    assert_eq!(node["frontmatter"], record["frontmatter"]);
    assert_eq!(record["frontmatter"]["swarm"], "s7");
    vec![on, off]
}

/// A portable ULID, a native canonical UUID and a slug, as every result uses them.
fn identity_ok(item: &Value) -> bool {
    let ulid = item["id"].as_str().is_some_and(|id| {
        id.len() == 26
            && id
                .bytes()
                .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b))
    });
    let uuid = item["external_id"]
        .as_str()
        .is_some_and(|id| id.len() == 36 && id.matches('-').count() == 4);
    ulid && uuid && item["slug"].is_string()
}

/// `git-kb show <slug> --json` relationships.
fn native_relationships(fx: &mut Fixture, root: &Path, slug: &str) -> Value {
    let doc = fx.cli(root, &["show", slug, "--json"], None)["documents"][0].clone();
    json!({"references": doc["references"], "depends_on": doc["depends_on"],
           "blocks": doc["blocks"], "parent": doc["parent"]})
}

/// The git-kb CLI side-by-side review (task 5004): one check per finding,
/// each reporting RED with its evidence instead of stopping the run.
fn review_checks(fx: &mut Fixture) -> Vec<String> {
    let root = fx.project("review");
    let mut conn = fx.connect(&root, None, None, false).unwrap();
    let mut red = Vec::new();
    let mut expect = |finding: &str, ok: bool, evidence: String| {
        eprintln!(
            "finding {finding}: {} {evidence}",
            if ok { "GREEN" } else { "RED" }
        );
        if !ok {
            red.push(format!("{finding}: {evidence}"));
        }
    };

    // 1. The signed record stays out of the document.
    let created = conn.call(
        "write",
        json!({"path": "notes/signed.md", "frontmatter": {"type": "note", "title": "Signed",
               "authorship": {"kind": "seat"}}, "body": "first\n",
               "signed_record": "review proof one"}),
    );
    let record = created["record"].clone();
    let raw = fx.source(&root, "notes/signed", "workspace");
    let native = raw["document"]["frontmatter"].clone();
    expect(
        "1 no bboxy_signed_record in the document",
        native.get("bboxy_signed_record").is_none()
            && !raw["rawContent"]
                .as_str()
                .unwrap()
                .contains("review proof one"),
        format!("native frontmatter {native}"),
    );
    expect(
        "1 seat attribution kept",
        native["authorship"] == json!({"kind": "seat"}),
        format!("authorship {}", native["authorship"]),
    );
    let fetched = conn.call("fetch", json!({"ref": reference(&record)}));
    expect(
        "1 fetch returns the proof for this version",
        fetched["signed_record"] == "review proof one",
        format!("signed_record {}", fetched["signed_record"]),
    );
    // Another editor changes only the body and keeps every frontmatter line.
    let source = raw["rawContent"]
        .as_str()
        .unwrap()
        .replace("first\n", "edited\n");
    let generation = raw["generation"].as_str().unwrap().to_owned();
    fx.cli(
        &root,
        &[
            "workspace",
            "write",
            "notes/signed",
            "--stdin",
            "--origin-id",
            "review-editor",
            "--if-generation",
            &generation,
            "--json",
        ],
        Some(&source),
    );
    let edited = conn.resolve("notes/signed");
    let after = conn.call("fetch", json!({"ref": edited}));
    expect(
        "1 a CLI edit leaves no stale proof",
        after.get("signed_record").is_none(),
        format!("signed_record after CLI edit {}", after["signed_record"]),
    );

    // 2. Next numbered slug in a family, with the CLI's allocation fields.
    let next = conn
        .client
        .call(
            "tool_create_next",
            json!({"family": "tasks/demo", "title": "First"}),
            WAIT,
        )
        .map_err(|f| format!("{f:?}"));
    match &next {
        Ok(next) => expect(
            "2 create next in family",
            next["slug"] == "tasks/demo-1"
                && next["authority"] == "local"
                && next.get("sync_required").is_some()
                && identity_ok(&next["ref"]),
            format!("{next}"),
        ),
        Err(failure) => expect("2 create next in family", false, failure.clone()),
    }

    // 3. One status vocabulary: one native status, every host state round-trips.
    let task = conn.call(
        "write",
        json!({"path": "tasks/status.md", "frontmatter": {"type": "task", "title": "Status",
               "status": "open"}, "body": "s\n", "signed_record": "review status"}),
    )["record"]
        .clone();
    let mut current = task;
    for (host, gitkb) in [
        ("open", "backlog"),
        ("claimed", "claimed"),
        ("running", "active"),
        ("reported", "reported"),
        ("rejected", "rejected"),
        ("accepted", "accepted"),
        ("blocked", "blocked"),
        ("done", "completed"),
    ] {
        let mut fm = current["frontmatter"].clone();
        fm["status"] = host.into();
        let written = conn.result(
            "write",
            json!({"ref": reference(&current), "etag": current["etag"], "frontmatter": fm,
                   "body": current["body"], "signed_record": format!("review {host}")}),
        );
        let Ok(written) = written else {
            expect("3 status round trip", false, format!("{host}: {written:?}"));
            continue;
        };
        current = written["record"].clone();
        let native =
            fx.source(&root, "tasks/status", "workspace")["document"]["frontmatter"].clone();
        expect(
            "3 status round trip",
            current["frontmatter"]["status"] == host
                && native["status"] == gitkb
                && native.get("bboxy_status").is_none(),
            format!(
                "{host}: read {} native {native}",
                current["frontmatter"]["status"]
            ),
        );
    }
    let mut fm = current["frontmatter"].clone();
    fm["status"] = "investigating".into();
    let refused = conn.result(
        "write",
        json!({"ref": reference(&current), "etag": current["etag"], "frontmatter": fm,
               "body": current["body"], "signed_record": "review unknown"}),
    );
    expect(
        "3 a status the host does not know is refused",
        matches!(&refused, Err(Failure::Tool(v)) if v["code"] == "unmappable_status"),
        format!("{refused:?}"),
    );
    native_write(
        fx,
        &root,
        "tasks/legacy",
        "type: task\nstatus: active\nbboxy_status: claimed\n",
        false,
    );
    native_write(
        fx,
        &root,
        "tasks/drafted",
        "type: task\nstatus: draft\n",
        false,
    );
    let legacy = conn.resolve("tasks/legacy");
    let legacy = conn.call("fetch", json!({"ref": legacy}));
    let drafted = conn.resolve("tasks/drafted");
    let drafted = conn.call("fetch", json!({"ref": drafted}));
    expect(
        "3 legacy bboxy_status reads through native status",
        legacy["frontmatter"]["status"] == "running" && drafted["frontmatter"]["status"] == "open",
        format!(
            "legacy {} drafted {}",
            legacy["frontmatter"]["status"], drafted["frontmatter"]["status"]
        ),
    );
    // An unknown native status reads as open and stays raw in the document,
    // also through an update that keeps the task open.
    native_write(
        fx,
        &root,
        "tasks/investigating",
        "type: task\nstatus: investigating\n",
        false,
    );
    let unknown = conn.resolve("tasks/investigating");
    let unknown = conn.call("fetch", json!({"ref": unknown}));
    let mut fm = unknown["frontmatter"].clone();
    fm["title"] = "Investigating, retitled".into();
    let kept = conn.result(
        "write",
        json!({"ref": reference(&unknown), "etag": unknown["etag"], "frontmatter": fm,
               "body": unknown["body"], "signed_record": "review investigating"}),
    );
    let raw =
        fx.source(&root, "tasks/investigating", "workspace")["document"]["frontmatter"]["status"]
            .clone();
    let open_items = slugs(
        &conn.collect(
            "list",
            json!({"filter": {"type": "task", "status": "open"}}),
        ),
        "",
    );
    expect(
        "3 an unknown native status reads open and stays raw",
        unknown["frontmatter"]["status"] == "open"
            && matches!(&kept, Ok(v) if v["record"]["frontmatter"]["status"] == "open" && v["lossy"] == false)
            && raw == "investigating"
            && ["tasks/investigating", "tasks/drafted"]
                .iter()
                .all(|s| open_items.iter().any(|o| o == s)),
        format!(
            "read {} kept {kept:?} raw {raw} open {open_items:?}",
            unknown["frontmatter"]["status"]
        ),
    );

    // 4. id is the ULID, external_id the UUID, slug the path, in every result.
    let listed = conn.collect("list", json!({}));
    let searched = conn.collect("search", json!({"query": "Signed"}));
    let graph = conn.call(
        "graph",
        json!({"scope": {"kind": "all", "graph": "markdown"}}),
    );
    let changes = conn.call("changes_since", json!({"cursor": "0"}));
    let mut results: Vec<Value> = vec![
        record.clone(),
        fetched.clone(),
        conn.resolve("notes/signed"),
    ];
    results.extend(listed);
    results.extend(searched.iter().map(|hit| hit["ref"].clone()));
    results.extend(
        graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["ref"].clone()),
    );
    let bad: Vec<&Value> = results.iter().filter(|r| !identity_ok(r)).collect();
    let nodes_ok = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["id"] == n["ref"]["id"]);
    expect(
        "4 identifiers consistent in every result",
        bad.is_empty() && nodes_ok && changes["changed"] == json!([]),
        format!("{} results, inconsistent {bad:?}", results.len()),
    );
    for locator in [
        "notes/signed",
        "notes/signed.md",
        record["id"].as_str().unwrap(),
        record["external_id"].as_str().unwrap(),
    ] {
        expect(
            "4 slug and ULID both address a doc",
            conn.resolve(locator)["id"] == record["id"],
            locator.to_owned(),
        );
    }

    // 7. Fetch relationships equal `git-kb show <slug> --json`.
    native_write(
        fx,
        &root,
        "tasks/parent",
        "type: task\nstatus: active\n",
        false,
    );
    native_write(
        fx,
        &root,
        "tasks/child",
        "type: task\nstatus: active\nparent: tasks/parent\ndepends_on: [tasks/status]\n",
        false,
    );
    native_write(
        fx,
        &root,
        "tasks/orphan",
        "type: task\nstatus: active\nparent: tasks/missing-2092\n",
        false,
    );
    fx.cli(
        &root,
        &[
            "commit",
            "--json",
            "-m",
            "review relationships",
            "--",
            "tasks/parent",
            "tasks/child",
            "tasks/orphan",
            "tasks/status",
        ],
        None,
    );
    for slug in [
        "tasks/child",
        "tasks/orphan",
        "tasks/parent",
        "tasks/status",
    ] {
        let expected = native_relationships(fx, &root, slug);
        let r = conn.resolve(slug);
        let fetched = conn.call("fetch", json!({"ref": r}));
        expect(
            "7 fetch relationships equal git-kb show",
            fetched["relationships"] == expected,
            format!(
                "{slug}: plugin {} git-kb {expected}",
                fetched["relationships"]
            ),
        );
    }

    // Doctor: identical fetches of a workspace record keep its timestamps;
    // a committed document's checkout carries the committed created_at.
    for slug in ["notes/signed", "tasks/child"] {
        let r = conn.resolve(slug);
        let first = conn.call("fetch", json!({"ref": r}));
        std::thread::sleep(Duration::from_millis(1100));
        let second = conn.call("fetch", json!({"ref": r}));
        let times = |v: &Value| {
            (
                v["gitkb"]["created_at"].clone(),
                v["gitkb"]["modified_at"].clone(),
            )
        };
        let source = first["gitkb"]["source"]["kind"].clone();
        expect(
            "doctor identical workspace fetches keep created_at and modified_at",
            source == "workspace"
                && times(&first) == times(&second)
                && first["gitkb"]["created_at"].is_string(),
            format!(
                "{slug} {source}: {:?} then {:?}",
                times(&first),
                times(&second)
            ),
        );
    }
    let committed = fx.cli(
        &root,
        &[
            "show",
            "--source",
            "committed",
            "--raw-content",
            "--json",
            "--",
            "tasks/child",
        ],
        None,
    )["document"]["created_at"]
        .clone();
    let r = conn.resolve("tasks/child");
    let checkout = conn.call("fetch", json!({"ref": r}));
    expect(
        "doctor a checkout carries the committed created_at",
        checkout["gitkb"]["created_at"] == committed,
        format!(
            "plugin {} committed {committed}",
            checkout["gitkb"]["created_at"]
        ),
    );

    // 8. A read-only sync check reads the remote from the KB config.
    let none = conn.client.call(
        "tool_sync_status",
        json!({"pathspec": "tasks/demo-1"}),
        WAIT,
    );
    expect(
        "8 no configured remote is refused, never origin",
        matches!(&none, Err(Failure::Tool(v)) if v["code"] == "remote_unconfigured"),
        format!("{none:?}"),
    );
    fx.cli_text(
        &root,
        &["remote", "add", "review", "http://127.0.0.1:9/none/kb"],
        None,
    );
    let one = conn.client.call(
        "tool_sync_status",
        json!({"pathspec": "tasks/demo-1"}),
        LONG,
    );
    expect(
        "8 sync status for a pathspec",
        matches!(&one, Ok(v) if v["remote"] == "review"
            && v["command"] == json!(["sync", "status", "review", "tasks/demo-1", "--json"])
            && v["status"].is_object()),
        format!("{one:?}"),
    );
    // 2. A remote-reserved family offline falls back to a provisional slug.
    let mut config = std::fs::read_to_string(root.join(".kb/config.toml")).unwrap();
    config = config.replace("families = []\n", "");
    config.push_str(
        "\n[[slug_authority.families]]\nprefix = \"tasks/prov\"\nremote = \"review\"\n\
         reservation = \"try\"\noffline_fallback = \"provisional\"\n",
    );
    std::fs::write(root.join(".kb/config.toml"), config).unwrap();
    let provisional = conn.client.call(
        "tool_create_next",
        json!({"family": "tasks/prov", "title": "Offline"}),
        LONG,
    );
    expect(
        "2 provisional allocation keeps the CLI's authority and sync_required",
        matches!(&provisional, Ok(v) if v["slug"] == "tasks/prov-1"
            && v["authority"] == "local_provisional" && v["sync_required"] == true),
        format!("{provisional:?}"),
    );
    fx.cli_text(
        &root,
        &["remote", "add", "second", "http://127.0.0.1:9/none/kb2"],
        None,
    );
    let two = conn.client.call(
        "tool_sync_status",
        json!({"pathspec": "tasks/demo-1"}),
        WAIT,
    );
    let unknown = conn.client.call(
        "tool_sync_status",
        json!({"pathspec": "tasks/demo-1", "remote": "origin"}),
        WAIT,
    );
    expect(
        "8 several remotes need an explicit configured one",
        matches!(&two, Err(Failure::Tool(v)) if v["code"] == "remote_required")
            && matches!(&unknown, Err(Failure::Tool(v)) if v["code"] == "unknown_remote"),
        format!("{two:?} / {unknown:?}"),
    );
    assert_eq!(conn.close(), Some(0));
    red
}

#[test]
#[ignore = "real GitKB fixtures; run through make check-gitkb-plugin"]
fn review() {
    let mut fx = Fixture::new();
    eprintln!("private fixture: {}", fx.run.display());
    let red = review_checks(&mut fx);
    assert!(red.is_empty(), "review findings still red: {red:#?}");
    if std::env::var_os("BBOXY_GITKB_RETAIN").is_none() {
        std::fs::remove_dir_all(&fx.run).unwrap();
    }
}

#[test]
#[ignore = "real GitKB fixtures; run through make check-gitkb-plugin"]
fn conformance() {
    let mut fx = Fixture::new();
    let retain = std::env::var_os("BBOXY_GITKB_RETAIN").is_some();
    eprintln!("private fixture: {}", fx.run.display());
    let mut conns = Vec::new();
    let mut checks = check(&mut fx, &mut conns);
    let red = review_checks(&mut fx);
    assert!(red.is_empty(), "review findings red: {red:#?}");
    checks.push(
        "git-kb CLI review: signed record, next slug, status table, ids, relationships, sync",
    );
    for conn in conns {
        assert_eq!(conn.close(), Some(0), "plugin exits cleanly on stdin EOF");
    }
    let report = json!({
        "result": "passed",
        "checks": checks,
        "plugin_sha256": sha256(&std::fs::read(&fx.plugin).unwrap()),
        "commands": fx.commands,
    });
    let evidence = std::env::var_os("BBOXY_GITKB_EVIDENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| fx.run.join("commands.json"));
    if let Some(parent) = evidence.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&evidence, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!(
        "{}",
        json!({"result": "passed", "checks": checks.len(), "evidence": evidence.display().to_string(),
               "plugin_sha256": report["plugin_sha256"]})
    );
    if !retain {
        std::fs::remove_dir_all(&fx.run).unwrap();
    }
}

/// A capture sink: each run stores its stdin payload and its environment.
fn capture_sink(dir: &Path) -> PathBuf {
    let sink = dir.join("activity-sink");
    std::fs::create_dir_all(dir.join("captured")).unwrap();
    std::fs::write(
        &sink,
        format!(
            "#!/bin/sh\nf=$(mktemp '{0}/captured/run.XXXXXX')\ncat > \"$f.json\"\nenv > \"$f.env\"\necho \"$@\" > \"$f.args\"\n",
            dir.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&sink, std::fs::Permissions::from_mode(0o700)).unwrap();
    sink
}

/// Captured sink runs not seen before, as (payload, environment) pairs.
fn captured(dir: &Path, seen: &mut BTreeSet<PathBuf>) -> Vec<(Value, String)> {
    let mut runs = Vec::new();
    let mut paths: Vec<_> = std::fs::read_dir(dir.join("captured"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    for path in paths {
        if seen.insert(path.clone()) {
            let payload = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let env = std::fs::read_to_string(path.with_extension("env")).unwrap();
            runs.push((payload, env));
        }
    }
    runs
}

// keeps: seat attribution — a seat's write and commit through the plugin
// export GitKB activity to that seat's ATC sink; an unattributed one exports none.
#[test]
#[ignore = "real GitKB fixtures; run with the check-gitkb-plugin environment and --exact attribution_reaches_the_gitkb_activity_sink"]
fn attribution_reaches_the_gitkb_activity_sink() {
    let mut fx = Fixture::new();
    let root = fx.project("attributed");
    let sink = capture_sink(&fx.run);
    let lent = |seat: &str| -> std::collections::BTreeMap<String, String> {
        [
            ("SESSION_ACTIVITY_RESOURCE_ID", format!("xses-attr-{seat}")),
            ("SESSION_ACTIVITY_RESOURCE_KIND", "external".into()),
            ("SESSION_ACTIVITY_PROVIDER", format!("provider-{seat}")),
            ("SESSION_ACTIVITY_SINK_COMMAND", sink.display().to_string()),
            (
                "SESSION_ACTIVITY_SINK_ARGS_JSON",
                r#"["activity","record","--json"]"#.into(),
            ),
            ("ATC_SESSION_ID", format!("xses-attr-{seat}")),
            ("GITKB_AGENT_ID", format!("agent-{seat}")),
            ("ATC_TOKEN", "secret-token-must-not-pass".into()),
        ]
        .map(|(k, v)| (k.to_owned(), v))
        .into()
    };
    let mut conn = fx.connect(&root, None, None, false).expect("initialize");
    let mut seen = BTreeSet::new();
    let mut evidence = Vec::new();
    for (slug, seat) in [
        ("notes/attr-a", Some("a")),
        ("notes/attr-none", None),
        ("notes/attr-b", Some("b")),
    ] {
        let env = seat.map(lent).unwrap_or_default();
        let write = conn
            .client
            .call_attributed(
                "data_source_write",
                with_unknown(json!({"path": format!("{slug}.md"),
                    "frontmatter": {"type": "note", "title": slug},
                    "body": format!("# {slug}\n"),
                    "signed_record": format!("fixture opaque host proof {slug}")})),
                &env,
                WAIT,
            )
            .unwrap_or_else(|f| panic!("write {slug}: {f:?}"));
        let commit = conn
            .client
            .call_attributed(
                "data_source_commit",
                with_unknown(json!({"refs": [reference(&write["record"])],
                    "message": format!("attributed commit {slug}")})),
                &env,
                LONG,
            )
            .unwrap_or_else(|f| panic!("commit {slug}: {f:?}"));
        assert!(truthy(&commit["commit"]), "{commit}");
        let runs = captured(&fx.run, &mut seen);
        match seat {
            None => assert!(runs.is_empty(), "an unattributed commit exported {runs:?}"),
            Some(seat) => {
                assert!(!runs.is_empty(), "the attributed commit exported nothing");
                for (payload, env) in &runs {
                    assert_eq!(payload["source"], "gitkb", "{payload}");
                    assert_eq!(
                        payload["resource_id"],
                        format!("xses-attr-{seat}"),
                        "{payload}"
                    );
                    assert_eq!(payload["provider"], format!("provider-{seat}"), "{payload}");
                    assert!(!env.contains("secret-token"), "{env}");
                    assert!(
                        env.contains(&format!("ATC_SESSION_ID=xses-attr-{seat}\n")),
                        "{env}"
                    );
                    assert!(
                        env.contains(&format!("GITKB_AGENT_ID=agent-{seat}\n")),
                        "{env}"
                    );
                    let other = if seat == "a" { "b" } else { "a" };
                    assert!(!env.contains(&format!("attr-{other}")), "{env}");
                }
                evidence.extend(runs.into_iter().map(|(payload, _)| payload));
            }
        }
    }
    assert_eq!(conn.close(), Some(0));
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"sink_payloads": evidence})).unwrap()
    );
    if std::env::var_os("BBOXY_GITKB_RETAIN").is_none() {
        std::fs::remove_dir_all(&fx.run).unwrap();
    }
}
