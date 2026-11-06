// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Exact native admission, independently of a Python-side preflight poll.
use super::*;

fn context(f: &Fixture, seat: Ulid) -> Value {
    let value = f.call(seat, Ulid::new(), "python_execution_context", json!({}));
    assert_ne!(value["isError"], true, "{value}");
    value["structuredContent"]["context"].clone()
}
fn guarded(f: &Fixture, seat: Ulid, expected: &Value, path: &str) -> Value {
    let mut bridge = f.bridge(seat);
    initialize(&mut bridge);
    rpc(&mut bridge,2,"tools/call",json!({"name":"knowledge_write","arguments":{
        "path":path,"frontmatter":{"type":"note","title":"Python guarded write"},"body":"Bounded private fixture."},
        "_meta":{"bboxy450/python_context":expected}}))["result"].clone()
}

#[test]
fn python_native_guard_refuses_between_preflight_and_dispatch_root_changes_and_aba() {
    let f = Fixture::real();
    let seat = f.oracle.id;
    let captured = context(&f, seat);
    assert_eq!(captured["guard_version"], 1);
    assert!(captured["workspace"].is_null());
    let other = f._temp.path().join("other-source");
    std::fs::create_dir_all(other.join(".bboxy/notes")).unwrap();
    f.registry()
        .update_swarm_workdir(f.oracle.swarm, Some(other.to_str().unwrap()))
        .unwrap();
    let changed = context(&f, seat);
    assert_ne!(changed["knowledge_binding"], captured["knowledge_binding"]);
    let denied = guarded(&f, seat, &captured, "notes/must-not-retarget.md");
    assert_eq!(
        denied["structuredContent"]["error"]["code"], "execution_context_changed",
        "{denied}"
    );
    assert!(!other.join(".bboxy/notes/must-not-retarget.md").exists());
    assert!(
        !f.config
            .repository
            .join(".bboxy/notes/must-not-retarget.md")
            .exists()
    );
    let accepted = guarded(&f, seat, &changed, "notes/current-root.md");
    assert_ne!(accepted["isError"], true, "{accepted}");
    assert!(other.join(".bboxy/notes/current-root.md").is_file());
    f.registry()
        .update_swarm_workdir(f.oracle.swarm, Some(f.config.repository.to_str().unwrap()))
        .unwrap();
    let returned = context(&f, seat);
    assert_eq!(
        returned["knowledge_binding"]["root_id"],
        captured["knowledge_binding"]["root_id"]
    );
    assert_ne!(
        returned["knowledge_binding"]["revision"],
        captured["knowledge_binding"]["revision"]
    );
    let denied = guarded(&f, seat, &captured, "notes/must-not-reuse-old-epoch.md");
    assert_eq!(
        denied["structuredContent"]["error"]["code"],
        "execution_context_changed"
    );
    assert!(
        !f.config
            .repository
            .join(".bboxy/notes/must-not-reuse-old-epoch.md")
            .exists()
    );
}

#[test]
fn python_native_context_guard_rejects_foreign_generation_and_keeps_unmanaged_cwd_local() {
    let f = Fixture::real();
    let seat = f.oracle.id;
    let captured = context(&f, seat);
    let mut cwd = captured.clone();
    cwd["cwd"] = json!(f._temp.path().join("captured-harness-source"));
    let accepted = guarded(&f, seat, &cwd, "notes/unmanaged-source-is-separate.md");
    assert_ne!(accepted["isError"], true, "{accepted}");
    let mut wrong = captured;
    wrong["process_generation"] = json!(2);
    let denied = guarded(&f, seat, &wrong, "notes/foreign-generation.md");
    assert_eq!(
        denied["structuredContent"]["error"]["code"],
        "execution_context_changed"
    );
    assert!(
        !f.config
            .repository
            .join(".bboxy/notes/foreign-generation.md")
            .exists()
    );
}

struct HarnessFixture {
    child: std::process::Child,
    run: PathBuf,
    socket: PathBuf,
}
impl Drop for HarnessFixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.run.join("release-native-write"), b"release");
        let _ = std::fs::write(self.run.join("stop"), b"stop");
        let status = self
            .child
            .wait()
            .expect("owned private Host must retire its exact children");
        if !std::thread::panicking() {
            assert!(status.success(), "private Host failed: {status}");
        }
    }
}
impl HarnessFixture {
    fn start(f: &Fixture, seat: Ulid, cwd: &std::path::Path) -> Self {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let binary = std::env::var_os("BBOXY_PYTHON_EXECUTE_FIXTURE")
            .expect("run make test-python-execute-boundaries");
        let endpoint = f.runtime.block_on(f.client().endpoint(seat)).unwrap();
        let run = f._temp.path().join(format!("python-host-{seat}"));
        std::fs::create_dir(&run).unwrap();
        std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = run.join("input.json");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&config)
            .unwrap();
        serde_json::to_writer(&mut file,&json!({"socket":endpoint.path,"seat":seat,"generation":endpoint.generation,"capability":endpoint.capability,"cwd":cwd,"run_dir":run})).unwrap();
        file.sync_all().unwrap();
        drop(file);
        let mut command = std::process::Command::new(binary);
        command.arg("host").arg(config);
        for (key, _) in std::env::vars_os() {
            if key.as_encoded_bytes().starts_with(b"BBOXY_") {
                command.env_remove(key);
            }
        }
        command.env("BBOXY_SEAT_ID", seat.to_string());
        let child = command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap();
        let marker = run.join("runtime-socket");
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut fixture = Self {
            child,
            run,
            socket: PathBuf::new(),
        };
        while !marker.is_file() {
            assert!(
                fixture.child.try_wait().unwrap().is_none(),
                "private Host exited before MCP readiness"
            );
            assert!(
                Instant::now() < deadline,
                "private Host did not start its native MCP client"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        fixture.socket = PathBuf::from(std::fs::read_to_string(marker).unwrap());
        loop {
            let value = fixture.call("python_execute_orient", json!({}));
            if value["structuredContent"]["ready"] == true {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "native MCP readiness did not settle: {value}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        fixture
    }
    fn call(&self, tool: &str, args: Value) -> Value {
        let mut stream = UnixStream::connect(&self.socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut send = |value: Value| {
            let mut bytes = serde_json::to_vec(&value).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).unwrap();
        };
        send(
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"private-native-python","version":"1"}}}),
        );
        send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        send(
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":tool,"arguments":args}}),
        );
        let mut reader = std::io::BufReader::new(stream);
        loop {
            let bytes = read_frame(&mut reader).unwrap().unwrap();
            let reply: Value = serde_json::from_slice(&bytes).unwrap();
            if reply["id"] == 2 {
                return reply["result"].clone();
            }
        }
    }
    fn execute(&self, source: &str, inputs: Value) -> Value {
        let start = self.call(
            "python_execute",
            json!({"source":source,"inputs":inputs,"op_id":Ulid::new()}),
        );
        assert_ne!(start["isError"], true, "{start}");
        let id = start["structuredContent"]["execution_id"].clone();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let value = self.call("python_execution_get", json!({"execution_id":id}));
            let status = value["structuredContent"]["status"]
                .as_str()
                .unwrap_or("refused");
            if !matches!(status, "starting" | "running" | "cancelling") {
                return value;
            }
            assert!(Instant::now() < deadline, "{value}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[test]
#[ignore = "Make builds the private native bridge/Host example first"]
fn python_managed_worktree_executes_python_and_mcp_against_canonical_kb() {
    let (f, task) = workspace_tests::fixture_real();
    let (id, ready) = workspace_tests::prepare(&f, task);
    let source = PathBuf::from(ready["cwd"].as_str().unwrap());
    let spawned=data(f.mutate(f.oracle.id,Ulid::new(),"seat_spawn",json!({"swarm_id":f.oracle.swarm,"approved_profile":"zsh","role":"executor","workspace":{"id":id,"expected_revision":ready["revision"]}})));
    let worker: Ulid = serde_json::from_value(spawned["seat_id"].clone()).unwrap();
    let delegated = f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({"task_path":task.to_string(),"seat_id":worker}),
    );
    assert_ne!(delegated["isError"], true, "{delegated}");
    let harness = HarnessFixture::start(&f, worker, &source);
    let orientation = harness.call("python_execute_orient", json!({}));
    assert_eq!(
        orientation["structuredContent"]["ready"], true,
        "{orientation}"
    );
    assert_eq!(
        orientation["structuredContent"]["context"]["workspace"]["workspace_id"],
        json!(id)
    );
    let result=harness.execute("import pathlib\npathlib.Path('python-source.txt').write_text('private source')\nresponse=await mcp.call('native','knowledge_write',inputs.report)\nemit({'cwd':str(pathlib.Path.cwd()),'record':response.structured.record})",json!({"report":{"path":"reports/python-runtime.md","frontmatter":{"type":"report","title":"Python runtime fixture","task":"#3","author":format!("seat:{worker}"),"created":"2026-11-05T00:00:00Z"},"body":"Source work and canonical knowledge stay separate."}}));
    assert_eq!(
        result["structuredContent"]["status"], "completed",
        "{result}"
    );
    assert!(source.join("python-source.txt").is_file());
    assert!(
        f.config
            .repository
            .join(".bboxy/reports/python-runtime.md")
            .is_file()
    );
    assert!(!source.join(".bboxy/reports/python-runtime.md").exists());
    let oracle_harness = HarnessFixture::start(&f, f.oracle.id, &f.config.repository);
    let oriented=oracle_harness.execute("import json\noriented=await mcp.call('native','orient',{})\nassert json.loads(oriented.content[0].text)==oriented.structured\nemit({'hints':oriented.hints,'tools':[tool.name for tool in oriented.structured.tools]})",json!({}));
    assert_eq!(
        oriented["structuredContent"]["status"], "completed",
        "{oriented}"
    );
    let hints = oriented["structuredContent"]["emitted"][0]["hints"]
        .as_array()
        .unwrap();
    for expected in [
        "knowledge_orient",
        "workspace_orient",
        "python_execute_orient",
    ] {
        assert!(
            hints.iter().any(|hint| hint["params"]["name"] == expected),
            "missing {expected}: {oriented}"
        );
    }
    drop(oracle_harness);

    // The real Bridge::run has only one stdio request loop. Hold a successful
    // knowledge_write's reply there, after its actual native effect. Python
    // status/context must use the independent trusted native read lane rather
    // than queueing behind that acknowledgement on the effect peer.
    std::fs::write(harness.run.join("hold-native-write"), b"hold").unwrap();
    let held=harness.call("python_execute",json!({"source":"await mcp.call('native','knowledge_write',inputs.report)","inputs":{"report":{"path":"reports/python-held.md","frontmatter":{"type":"report","title":"Held native runtime fixture","task":"#3","author":format!("seat:{worker}"),"created":"2026-11-05T00:00:00Z"},"body":"The native write completed before its serial reply was released."}}}));
    assert_ne!(held["isError"], true, "{held}");
    let held_id = held["structuredContent"]["execution_id"].clone();
    let until = Instant::now() + Duration::from_secs(10);
    while !harness.run.join("held-native-write").is_file() {
        assert!(
            Instant::now() < until,
            "native write never reached the held serial acknowledgement"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        f.config
            .repository
            .join(".bboxy/reports/python-held.md")
            .is_file()
    );
    let before = Instant::now();
    let inspected = harness.call("python_execution_get", json!({"execution_id":held_id}));
    assert!(
        before.elapsed() < Duration::from_millis(1500),
        "get waited behind the held native effect: {inspected}"
    );
    assert_eq!(
        inspected["structuredContent"]["status"], "running",
        "{inspected}"
    );
    let independent = harness.execute("emit('independent native context')", json!({}));
    assert_eq!(
        independent["structuredContent"]["status"], "completed",
        "{independent}"
    );
    let before = Instant::now();
    let cancelled = harness.call("python_execution_cancel", json!({"execution_id":held_id}));
    assert!(
        before.elapsed() < Duration::from_millis(1500),
        "cancel waited behind the held native effect"
    );
    assert_eq!(
        cancelled["structuredContent"]["cancel_requested"], true,
        "{cancelled}"
    );
    std::fs::write(harness.run.join("release-native-write"), b"release").unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let value = harness.call("python_execution_get", json!({"execution_id":held_id}));
        if value["structuredContent"]["status"] == "cancelled" {
            assert_eq!(
                value["structuredContent"]["children"][0]["status"], "unknown",
                "{value}"
            );
            break;
        }
        assert!(Instant::now() < until, "{value}");
        std::thread::sleep(Duration::from_millis(5));
    }
    let closed = f.mutate(
        f.oracle.id,
        Ulid::new(),
        "seat_close",
        json!({"seat_id":worker}),
    );
    assert_ne!(closed["isError"], true, "{closed}");
    let denied = harness.call(
        "python_execute",
        json!({"source":"open('must-not-start.txt','w').write('bad')"}),
    );
    assert_eq!(denied["isError"], true, "{denied}");
    assert!(!source.join("must-not-start.txt").exists());
}
