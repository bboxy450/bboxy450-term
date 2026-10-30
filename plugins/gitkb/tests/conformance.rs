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
        9
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
    assert_eq!(
        raw["document"]["frontmatter"]["bboxy_signed_record"],
        one["signed_record"]
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

    // Lifecycle with native bboxy_status/status.
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
        assert_eq!(native["bboxy_status"], status);
        assert_eq!(
            native["status"],
            if status == "accepted" {
                "completed"
            } else {
                "active"
            }
        );
        assert_eq!(native["bboxy_signed_record"], proof.as_str());
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
    assert_eq!(
        committed["document"]["frontmatter"]["bboxy_status"],
        "accepted"
    );
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
    let committed_ref = client.resolve("notes/shared");
    client.call(
        "write",
        json!({"ref": committed_ref, "etag": committed["etag"],
               "frontmatter": committed["frontmatter"], "body": committed["body"],
               "signed_record": "fixture committed update"}),
    );

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

#[test]
#[ignore = "real GitKB fixtures; run through make check-gitkb-plugin"]
fn conformance() {
    let mut fx = Fixture::new();
    let retain = std::env::var_os("BBOXY_GITKB_RETAIN").is_some();
    eprintln!("private fixture: {}", fx.run.display());
    let mut conns = Vec::new();
    let checks = check(&mut fx, &mut conns);
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
