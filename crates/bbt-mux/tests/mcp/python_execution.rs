// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Exact native admission, independently of a Python-side preflight poll.
use super::*;
use bbt_mux::registry::Lifecycle;

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

fn sudo_close_fixture() -> (Fixture, SeatRecord, SeatRecord, Value) {
    let f = Fixture::real();
    let mut sudo = SeatRecord::new(Ulid::new(), f.oracle.swarm, "Sudo", "zsh");
    sudo.role.name = bbt_proto::gui::SUDO_ROLE.into();
    sudo.role.class = Class::Oracle;
    let mut target = SeatRecord::new(Ulid::new(), f.outside.swarm, "Target Oracle", "zsh");
    target.role = f.oracle.role.clone();
    for record in [&sudo, &target] {
        let mut spec = LaunchSpec::new("/bin/cat", f._temp.path());
        spec.seat = record.id;
        spec.generation = record.generation;
        f.runtime
            .block_on(f.client().spawn(record.clone(), spec))
            .unwrap();
    }
    let target_source = f._temp.path().join("target-source");
    let tasks = target_source.join(".bboxy/tasks");
    std::fs::create_dir_all(&tasks).unwrap();
    std::fs::write(
        tasks.join("sudo-close.md"),
        format!(
            "---\nid: {}\ntype: task\nnumber: 1\ntitle: Sudo close fixture\nstatus: accepted\nassignee: seat:{}\ndelegated_by: seat:{}\nverify: 'true'\n---\nPrivate fixture.\n",
            Ulid::new(),
            f.outside.id,
            target.id,
        ),
    )
    .unwrap();
    f.registry()
        .update_swarm_workdir(target.swarm, Some(target_source.to_str().unwrap()))
        .unwrap();
    let arguments = json!({"task_path":"tasks/sudo-close.md"});
    assert_eq!(
        data(f.call(target.id, Ulid::new(), "task_status", arguments.clone()))["metadata"]["status"],
        "accepted"
    );
    assert_ne!(
        context(&f, sudo.id)["knowledge_binding"]["root_id"],
        context(&f, target.id)["knowledge_binding"]["root_id"]
    );
    let call = json!({
        "seat_id":target.id,
        "expected_process_generation":target.generation.to_string(),
        "tool":"task_close",
        "arguments":arguments,
    });
    (f, sudo, target, call)
}

fn assert_sudo_closed_in_target(f: &Fixture, target: &SeatRecord, call: &Value) {
    let task = data(f.call(
        target.id,
        Ulid::new(),
        "task_status",
        call["arguments"].clone(),
    ));
    assert_eq!(task["metadata"]["status"], "done", "{task}");
    assert_eq!(
        task["metadata"]["closed"]["by"],
        format!("seat:{}", target.id)
    );
}

#[test]
fn python_guarded_sudo_as_closes_task_in_target_workspace() {
    let (f, sudo, target, arguments) = sudo_close_fixture();
    let call = |seat: Ulid, expected: &Value| {
        let mut bridge = f.bridge(seat);
        initialize(&mut bridge);
        rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({"name":"sudo_as","arguments":arguments,
                "_meta":{"bboxy450/python_context":expected}}),
        )
    };
    let wrong_caller = call(sudo.id, &context(&f, target.id));
    assert_eq!(
        wrong_caller["result"]["structuredContent"]["error"]["code"],
        "execution_context_changed",
        "{wrong_caller}"
    );
    let non_sudo = call(f.oracle.id, &context(&f, f.oracle.id));
    assert_eq!(
        non_sudo["result"]["structuredContent"]["error"]["code"],
        "rejected",
        "{non_sudo}"
    );
    let closed = call(sudo.id, &context(&f, sudo.id));
    assert_eq!(
        closed["result"]["structuredContent"]["task_status"],
        "done",
        "{closed}"
    );
    assert_sudo_closed_in_target(&f, &target, &arguments);
}

struct HarnessFixture {
    child: Option<std::process::Child>,
    run: PathBuf,
    socket: PathBuf,
}
impl Drop for HarnessFixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.run.join("release-native-write"), b"release");
        let _ = std::fs::write(self.run.join("stop"), b"stop");
        if let Some(child) = &mut self.child {
            let status = child
                .wait()
                .expect("private Host must retire its exact children");
            if !std::thread::panicking() {
                assert!(status.success(), "private Host failed: {status}");
            }
        }
    }
}
impl HarnessFixture {
    fn write_input(
        f: &Fixture,
        seat: Ulid,
        cwd: &std::path::Path,
        run: &std::path::Path,
    ) -> PathBuf {
        use std::os::unix::fs::OpenOptionsExt;
        let endpoint = f.runtime.block_on(f.client().endpoint(seat)).unwrap();
        let config = run.join("input.json");
        let pending = run.join("input.next");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&pending)
            .unwrap();
        serde_json::to_writer(&mut file,&json!({"socket":endpoint.path,"seat":seat,"generation":endpoint.generation,"capability":endpoint.capability,"cwd":cwd,"run_dir":run})).unwrap();
        file.sync_all().unwrap();
        drop(file);
        std::fs::rename(pending, &config).unwrap();
        config
    }

    fn run_directory(f: &Fixture, seat: Ulid) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let run = f._temp.path().join(format!("python-host-{seat}"));
        std::fs::create_dir(&run).unwrap();
        std::fs::set_permissions(&run, std::fs::Permissions::from_mode(0o700)).unwrap();
        run
    }

    fn start(f: &Fixture, seat: Ulid, cwd: &std::path::Path) -> Self {
        let binary = std::env::var_os("BBOXY_PYTHON_EXECUTE_FIXTURE")
            .expect("run make test-python-execute-boundaries");
        let run = Self::run_directory(f, seat);
        let config = Self::write_input(f, seat, cwd, &run);
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
        Self::ready(Some(child), run)
    }

    fn supervised(f: &Fixture, cwd: &std::path::Path) -> (SeatRecord, Self) {
        let seat = SeatRecord::new(Ulid::new(), f.own.swarm, "Chart Python", "bboxy450-harness");
        let run = Self::run_directory(f, seat.id);
        let binary = std::env::var("BBOXY_PYTHON_EXECUTE_FIXTURE").expect("private Python fixture");
        let install = PathBuf::from(
            std::env::var_os("BBOXY_CHART_TEST_INSTALL").expect("private chart install"),
        );
        let mut spec = LaunchSpec::new("/bin/sh", cwd);
        spec.seat = seat.id;
        spec.generation = seat.generation;
        spec.env.insert("PATH".into(), "/usr/bin:/bin".into());
        spec.env.insert(
            "HOME".into(),
            install.join("home").to_string_lossy().into_owned(),
        );
        spec.env.insert(
            "MPLCONFIGDIR".into(),
            run.join("mpl-cache").to_string_lossy().into_owned(),
        );
        spec.args = vec![
            "-c".into(),
            "for i in $(seq 1 300); do if test -f \"$2\"; then exec \"$1\" host \"$2\" --bboxy-alias; fi; sleep 0.05; done; exit 72".into(),
            "private-chart-python".into(),
            binary,
            run.join("input.json").to_string_lossy().into_owned(),
        ];
        f.runtime
            .block_on(f.client().spawn(seat.clone(), spec))
            .unwrap();
        Self::write_input(f, seat.id, cwd, &run);
        (seat, Self::ready(None, run))
    }

    fn ready(child: Option<std::process::Child>, run: PathBuf) -> Self {
        let marker = run.join("runtime-socket");
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut fixture = Self {
            child,
            run,
            socket: PathBuf::new(),
        };
        while !marker.is_file() {
            assert!(
                fixture
                    .child
                    .as_mut()
                    .is_none_or(|child| child.try_wait().unwrap().is_none()),
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
    fn bridge_call(&self, tool: &str, args: Value) -> Value {
        let home=tempfile::tempdir().unwrap();
        let input=home.path().join("mcp-input.jsonl");
        let requests=[
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"python-hint-fixture","version":"1"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":tool,"arguments":args}}),
        ];
        std::fs::write(&input,requests.into_iter().map(|r|r.to_string()+"\n").collect::<String>()).unwrap();
        let mut command=std::process::Command::new(std::env::var_os("BBOXY_PYTHON_EXECUTE_FIXTURE").unwrap());
        command.arg("bridge").arg(self.run.join("input.json"))
            .env_clear().env("HOME",home.path()).env("BBOXY_PYTHON_SOCKET",&self.socket)
            .stdin(std::fs::File::open(input).unwrap());
        let (status,stdout,stderr)=bounded_client(command,Duration::from_secs(15),&home);
        assert!(status.success(),"private Python bridge failed: {}",String::from_utf8_lossy(&stderr));
        String::from_utf8(stdout).unwrap().lines().map(|line|serde_json::from_str::<Value>(line).unwrap())
            .find(|reply|reply["id"]==2).expect("Python bridge response")["result"].clone()
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
fn python_runtime_sudo_as_closes_task_in_target_workspace() {
    let (f, sudo, target, arguments) = sudo_close_fixture();
    let harness = HarnessFixture::start(&f, sudo.id, &f.config.repository);
    let result = harness.execute(
        "response = await mcp.call('native', 'sudo_as', inputs)\nresponse.structured",
        arguments.clone(),
    );
    let execution = &result["structuredContent"];
    assert_eq!(execution["status"], "completed", "{result}");
    assert_eq!(execution["result"]["task_status"], "done", "{result}");
    assert_sudo_closed_in_target(&f, &target, &arguments);
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
    let ruling = Ulid::new();
    data(f.call(f.oracle.id,ruling,"message_send",json!({"destination":format!("seats/{worker}/inbox"),"text":"Oracle ruling during native Python execution"})));
    let orientation = harness.bridge_call("python_execute_orient", json!({}));
    assert!(
        orientation["structuredContent"]["hints"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|hint| hint["kind"] == "unread_messages")
            .any(|hint| hint["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["message_id"] == ruling.to_string())),
        "native Python tool lost the Oracle ruling: {orientation}"
    );
    assert_eq!(
        orientation["structuredContent"]["ready"], true,
        "{orientation}"
    );
    assert_eq!(
        orientation["structuredContent"]["context"]["workspace"]["workspace_id"],
        json!(id)
    );
    let result=harness.execute("import pathlib\npathlib.Path('python-source.txt').write_text('private source')\nawait commands.run(['/bin/test','-f','python-source.txt'],check=True)\nresponse=await mcp.call('native','knowledge_write',inputs.report)\nemit({'cwd':str(pathlib.Path.cwd()),'record':response.structured.record})",json!({"report":{"path":"reports/2026-11-05-python-runtime.md","frontmatter":{"type":"report","title":"Python runtime fixture","task":"#3","author":format!("seat:{worker}"),"created":"2026-11-05T00:00:00Z","commits":[],"verify":{"command":"test -f python-source.txt","exit_code":0}},"body":"Source work and canonical knowledge stay separate."}}));
    assert_eq!(
        result["structuredContent"]["status"], "completed",
        "{result}"
    );
    assert!(source.join("python-source.txt").is_file());
    assert!(
        f.config
            .repository
            .join(".bboxy/reports/2026-11-05-python-runtime.md")
            .is_file()
    );
    assert!(
        !source
            .join(".bboxy/reports/2026-11-05-python-runtime.md")
            .exists()
    );
    let oracle_harness = HarnessFixture::start(&f, f.oracle.id, &f.config.repository);
    let oriented = oracle_harness.execute(
        r#"
import json
oriented = await mcp.call('native', 'orient', {})
assert json.loads(oriented.content[0].text) == oriented.structured
oriented.hints
"#,
        json!({}),
    );
    assert_eq!(
        oriented["structuredContent"]["status"], "completed",
        "{oriented}"
    );
    let hints = oriented["structuredContent"]["result"]
        .as_array()
        .unwrap_or_else(|| panic!("native orient did not return hints: {oriented}"));
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
    let held = harness.call(
        "python_execute",
        json!({
            "wait_ms": 0,
            "source": "await commands.run(['/bin/test','-f','python-source.txt'],check=True)\nawait mcp.call('native','knowledge_write',inputs.report)",
            "inputs": {"report": {
                "path": "reports/2026-11-05-python-held.md",
                "frontmatter": {
                    "type": "report",
                    "title": "Held native runtime fixture",
                    "task": "#3",
                    "author": format!("seat:{worker}"),
                    "created": "2026-11-05T00:00:00Z",
                    "commits": [],
                    "verify": {"command": "test -f python-source.txt", "exit_code": 0}
                },
                "body": "The native write completed before its serial reply was released."
            }}
        }),
    );
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
            .join(".bboxy/reports/2026-11-05-python-held.md")
            .is_file()
    );
    let before = Instant::now();
    let inspected = harness.call(
        "python_execution_get",
        json!({"execution_id":held_id, "wait_ms":0}),
    );
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
        let value = harness.call(
            "python_execution_get",
            json!({"execution_id":held_id, "wait_ms":0, "detail":true}),
        );
        if value["structuredContent"]["status"] == "cancelled" {
            assert_eq!(
                value["structuredContent"]["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|child| child["tool"] == "knowledge_write")
                    .unwrap()["status"],
                "unknown",
                "{value}"
            );
            break;
        }
        assert!(Instant::now() < until, "{value}");
        std::thread::sleep(Duration::from_millis(5));
    }
    // This private fixture has no window. Close through the normal logic owner
    // rather than the window/undo MCP action, then exercise its real revocation.
    f.runtime
        .block_on(f.client().close(
            worker,
            1,
            Ulid::new(),
            bbt_mux::registry::Position {
                swarm: f.oracle.swarm,
                tab: Ulid::new(),
                split_path: vec![],
                ratios: vec![],
                layout_revision: 0,
            },
            0,
        ))
        .unwrap();
    let denied = harness.call(
        "python_execute",
        json!({"source":"open('must-not-start.txt','w').write('bad')"}),
    );
    assert_eq!(denied["isError"], true, "{denied}");
    assert!(!source.join("must-not-start.txt").exists());
}

#[test]
#[ignore = "make test-chart-artifacts supplies the supervised Host and pinned exporter"]
fn python_chart_artifact_observed_bytes_survive_harness_cleanup_and_native_restart() {
    let mut f = Fixture::real();
    let cwd = f.config.repository.canonicalize().unwrap();
    let (seat, host) = HarnessFixture::supervised(&f, &cwd);
    let orientation = data(host.call("python_execute_orient", json!({})));
    assert_eq!(orientation["ready"], true, "{orientation}");
    assert!(orientation["chart_export_unavailable"].is_null(), "{orientation}");
    assert!(
        orientation["interpreter"].as_str().unwrap().contains("/bboxy450-term/chart-runtime-"),
        "{orientation}"
    );
    let completed = |run: &Value| {
        if run["status"] == "completed" {
            return;
        }
        assert_eq!(run["status"], "completed_with_errors", "{run}");
        let id = &run["execution_id"];
        let original = data(host.call(
            "python_execution_get",
            json!({"execution_id":id,"detail":true}),
        ));
        assert_eq!(original["exit_code"], 0, "{original}");
        let failed: Vec<_> = original["children"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|child| child["status"] != "succeeded")
            .collect();
        assert!(!failed.is_empty());
        assert_eq!(run["failed_calls"], failed.len());
        for child in failed {
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            assert_eq!(child["status"], "failed", "{child}");
            let page = data(host.call(
                "python_execution_get",
                json!({"execution_id":id,"cursor":child["result_cursor"]}),
            ));
            assert!(page["next_cursor"].is_null());
            let bytes = STANDARD
                .decode(page["data_base64"].as_str().unwrap())
                .unwrap();
            let receipt: Value = serde_json::from_slice(&bytes).unwrap();
            let result = &receipt["structuredContent"];
            assert_eq!(result["status"], "rejected", "{receipt}");
            assert_eq!(result["error"]["code"], "provider_busy", "{receipt}");
            assert_ne!(result["error"]["unknown"], true, "{receipt}");
        }
    };
    let source = r#"import altair as alt
from pathlib import Path
async def after_nonadmission(call):
    delays = iter((0.02, 0.04, 0.08, 0.16, 0.32, 0.64))
    while True:
        try:
            return await call()
        except ToolError as error:
            delay = next(delays, None)
            if (delay is None or error.code != 'provider_busy' or error.unknown
                    or error.result.get('structuredContent', {}).get('status') != 'rejected'):
                raise
            await asyncio.sleep(delay)
Path('fixture.csv').write_text('time,value\n1,4\n2,7\n')
plain = await after_nonadmission(lambda: bboxy.artifact_store(media_type='text/plain', content='observed proxy bytes'))
direct = await after_nonadmission(lambda: mcp.call('native', 'artifact_store', {'media_type':'application/json','content':{'snapshot':42}}))
named = await after_nonadmission(lambda: tools.native.artifact_store(media_type='text/csv', source_path='fixture.csv'))
rows = alt.Data(values=[{'time': 1, 'value': 4}, {'time': 2, 'value': 7}])
plot = alt.Chart(rows).mark_bar(color='#11f183').encode(x='time:Q', y='value:Q').properties(title='Private chart', width=160, height=100)
saved = display(plot, inputs=[named])
emit({'plain':plain,'direct':direct.structured,'saved':saved,'csv':named})"#;
    let first = data(host.execute(source, json!({})));
    completed(&first);
    let result = &first["result"];
    let mut observed = Vec::new();
    for key in ["plain", "direct", "csv", "saved"] {
        let capture = &result[key]["capture"];
        let pin = &capture["observed"][0];
        assert_eq!(capture["observed"].as_array().unwrap().len(), 1, "{result}");
        assert_eq!(pin["revision"], 1);
        assert!(capture["missing"].as_array().unwrap().is_empty());
        observed.push(pin.clone());
    }
    assert!(observed.windows(2).all(|pair| pair[0] == pair[1]));
    let saved = &result["saved"];
    let read = |f: &Fixture, id: &Value, revision: u64| {
        data(f.call(
            seat.id,
            Ulid::new(),
            "artifact_read",
            json!({"id":id,"revision":revision}),
        ))
    };
    let initial = read(&f, &saved["id"], 1);
    assert_eq!(first["displayed"], json!([saved["reference"]]));
    let second = data(host.execute(r#"import altair as alt
plot = alt.Chart(alt.Data(values=[{'time': 1, 'value': 9}])).mark_bar(color='#bb2484').encode(x='time:Q', y='value:Q').properties(title='Updated chart', width=160, height=100)
saved = display(plot, update=inputs)
emit(saved)"#, saved.clone()));
    completed(&second);
    assert_eq!(second["result"]["id"], saved["id"]);
    assert_eq!(second["result"]["revision"], 2);
    assert_ne!(second["result"]["digest"], saved["digest"]);
    let pin = json!({"id":saved["id"],"revision":1});
    let pinned = data(host.execute("saved = display(inputs)\nemit(saved)", pin));
    completed(&pinned);
    assert_eq!(pinned["displayed"], json!([saved["snapshot_reference"]]));
    assert_eq!(pinned["result"]["digest"], saved["digest"]);
    let text_pin = json!({"id":result["plain"]["id"],"revision":1});
    let rejected = data(host.execute(
        "try:\n    display(inputs)\nexcept ValueError as error:\n    emit(str(error))",
        text_pin,
    ));
    completed(&rejected);
    assert_eq!(rejected["result"], "display requires a stored SVG or PNG image");
    assert!(rejected["displayed"].is_null());

    let run = host.run.clone();
    drop(host);
    let until = Instant::now() + Duration::from_secs(20);
    while !run.join("stopped").exists() {
        assert!(
            Instant::now() < until,
            "private supervised harness did not close"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let sessions = run.join("sessions");
    assert!(sessions.exists());
    std::fs::remove_dir_all(&sessions).unwrap();
    std::fs::write(cwd.join("fixture.csv"), b"live bytes changed").unwrap();
    f.restart();
    let old_source = read(&f, &observed[0]["id"], 1);
    assert_eq!(old_source["media_type"], "text/x-python");
    assert_eq!(old_source["content"], source);
    assert_eq!(old_source["digest"], observed[0]["digest"]);
    assert_eq!(
        read(&f, &result["csv"]["id"], 1)["content"],
        "time,value\n1,4\n2,7\n"
    );
    let history = read(&f, &saved["id"], 1);
    assert_eq!(history["content"], initial["content"]);
    assert_eq!(history["capture"], initial["capture"]);
    assert!(
        history["content"]
            .as_str()
            .unwrap()
            .contains("data-bboxy-font-set")
    );
    let lineage = data(f.call(
        seat.id,
        Ulid::new(),
        "provenance_lineage",
        json!({"locator":format!("artifact:{}:1",saved["id"].as_str().unwrap())}),
    ));
    assert!(
        lineage["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["basis"] == "observed" && edge["digest"] == observed[0]["digest"])
    );
}

#[test]
fn sudo_ctl_round_trip_requires_sudo_and_uses_host_dispatch() {
    fn run(
        args: Vec<String>,
        _endpoint: bbt_mux::logic::EndpointInfo,
        _root: PathBuf,
        op: Ulid,
        issuer: String,
    ) -> Result<Value, String> {
        Ok(json!({"ok":true,"op":op,"args":args,"issuer":issuer}))
    }
    let (f, sudo, _, _) = sudo_close_fixture();
    f.runtime.block_on(f.client().set_ctl_runner(run)).unwrap();
    let arguments =
        json!({"args":["message","send","operator/inbox","literal $(text) with spaces"]});
    let denied = f.call(f.oracle.id, Ulid::new(), "sudo_ctl", arguments.clone());
    assert_eq!(denied["isError"], true, "{denied}");
    let mut bridge = f.bridge(sudo.id);
    initialize(&mut bridge);
    let response = rpc(
        &mut bridge,
        2,
        "tools/call",
        json!({"name":"sudo_ctl","arguments":arguments}),
    );
    let result = &response["result"];
    assert_ne!(result["isError"], true, "{response}");
    assert_eq!(result["structuredContent"]["args"], arguments["args"]);
    assert_eq!(
        result["structuredContent"]["op"],
        result["structuredContent"]["op_id"]
    );
    assert_eq!(
        result["structuredContent"]["issuer"],
        format!("sudo:{}:1", sudo.id)
    );
}

#[test]
fn sudo_as_round_trip_returns_target_reads_and_errors() {
    let (f, sudo, target, _) = sudo_close_fixture();
    let mut bridge = f.bridge(sudo.id);
    initialize(&mut bridge);
    for tool in ["self_status", "seat_list", "task_list"] {
        let response = rpc(
            &mut bridge,
            2,
            "tools/call",
            json!({
                "name":"sudo_as", "arguments":{
                    "seat_id":target.id,
                    "expected_process_generation":target.generation.to_string(),
                    "tool":tool, "arguments":{}
                }
            }),
        );
        assert!(response.get("error").is_none(), "{response}");
        assert_ne!(response["result"]["isError"], true, "{response}");
    }
    let ghost = SeatRecord::new(Ulid::new(), target.swarm, "No live viewport", "zsh");
    f.registry().insert(&ghost).unwrap();
    let response = rpc(
        &mut bridge,
        3,
        "tools/call",
        json!({
            "name":"sudo_as", "arguments":{
                "seat_id":ghost.id,
                "expected_process_generation":ghost.generation.to_string(),
                "tool":"self_status", "arguments":{}
            }
        }),
    );
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["isError"], true, "{response}");
    let listing = data(f.call(sudo.id, Ulid::new(), "sudo_swarms", json!({})));
    let seats: Vec<_> = listing["swarms"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["seats"].as_array().unwrap())
        .collect();
    let live = seats
        .iter()
        .find(|s| s["seat_id"] == json!(target.id))
        .unwrap();
    assert_eq!(live["process_state"], "running", "{listing}");
    assert!(live["adapter"].is_object(), "{listing}");
    let gone = seats
        .iter()
        .find(|s| s["seat_id"] == json!(ghost.id))
        .unwrap();
    assert_eq!(gone["process_state"], "gone", "{listing}");
    let assigned = seats.iter().find(|s| s["seat_id"] == json!(f.own.id)).unwrap();
    assert!(assigned["task_paths"].as_array().unwrap().iter().any(|p| p.as_str().unwrap().ends_with(&format!("{}.md", f.task))), "{listing}");
    assert_eq!(
        response["result"]["structuredContent"]["error"]["code"], "process_gone",
        "{response}"
    );
}

#[test]
fn sudo_dispatch_errors_are_tool_replies_for_plain_and_python_guarded_calls() {
    let f = Fixture::real();
    let seat = f.oracle.id;
    let expected = context(&f, seat);
    assert_ne!(
        f.call(seat, Ulid::new(), "self_status", json!({}))["isError"],
        true
    );
    let db =
        rusqlite::Connection::open(f.config.run_dir.join(format!("seat-{seat}/object.sqlite")))
            .unwrap();
    db.execute_batch("CREATE TRIGGER refuse_sudo_fixture BEFORE INSERT ON kv WHEN NEW.key LIKE 'control/op/%' BEGIN SELECT RAISE(ABORT,'fixture control journal failure'); END;").unwrap();
    let mut bridge = f.bridge(seat);
    initialize(&mut bridge);
    for guarded in [false, true] {
        let mut params = json!({"name":"self_status","arguments":{}});
        if guarded {
            params["_meta"] = json!({"bboxy450/python_context":expected});
        }
        let response = rpc(&mut bridge, 2, "tools/call", params);
        assert!(response.get("error").is_none(), "{response}");
        assert_eq!(response["result"]["isError"], true, "{response}");
        assert_eq!(
            response["result"]["structuredContent"]["error"]["code"], "unknown",
            "{response}"
        );
        assert!(
            response["result"]["structuredContent"]["error"]["message"]
                .as_str()
                .unwrap()
                .contains("fixture control journal failure"),
            "{response}"
        );
    }
}

#[test]
fn sudo_swarm_archive_preserves_history_and_rejects_open_or_undoable_members() {
    let (mut f, sudo, _, _) = sudo_close_fixture();
    f.prepare_operator();
    let swarm = Ulid::new();
    let mut ghost = SeatRecord::new(Ulid::new(), swarm, "Retained member", "zsh");
    ghost.tasks.push(f.task);
    f.registry().insert(&ghost).unwrap();
    f.registry().rename_swarm(swarm, "Archive fixture").unwrap();
    f.registry()
        .update_swarm_workdir(swarm, Some(f.config.repository.to_str().unwrap()))
        .unwrap();
    let before = std::fs::read(
        f.config
            .repository
            .join(format!(".bboxy/tasks/{}.md", f.task)),
    )
    .unwrap();
    let archive = || json!({"swarm_id":swarm,"archive":true});
    refused(
        f.call(sudo.id, Ulid::new(), "sudo_swarm", archive()),
        "open or undoable",
    );
    f.registry()
        .update(ghost.id, |r| {
            r.lifecycle = Lifecycle::ClosedGrace;
            Ok(())
        })
        .unwrap();
    refused(
        f.call(sudo.id, Ulid::new(), "sudo_swarm", archive()),
        "open or undoable",
    );
    f.registry()
        .update(ghost.id, |r| {
            r.lifecycle = Lifecycle::ClosedRetained;
            Ok(())
        })
        .unwrap();
    let archived = data(f.call(sudo.id, Ulid::new(), "sudo_swarm", archive()));
    assert_eq!(archived["swarm"]["archived"], true, "{archived}");
    assert!(f.registry().swarms().unwrap().iter().all(|r| r.id != swarm));
    assert_eq!(f.registry().get(ghost.id).unwrap().tasks, ghost.tasks);
    assert_eq!(
        std::fs::read(
            f.config
                .repository
                .join(format!(".bboxy/tasks/{}.md", f.task))
        )
        .unwrap(),
        before
    );
    f.restart();
    let record = f.registry().swarm(swarm).unwrap().unwrap();
    assert!(record.archived);
    assert_eq!(record.name.as_deref(), Some("Archive fixture"));
    assert_eq!(
        record.workdir.as_deref(),
        f.config.repository.canonicalize().unwrap().to_str()
    );
    let listing = data(f.call(sudo.id, Ulid::new(), "sudo_swarms", json!({})));
    assert!(
        listing["swarms"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["swarm_id"] != json!(swarm)),
        "{listing}"
    );
    let active = SeatRecord::new(Ulid::new(), swarm, "Reactivated member", "zsh");
    f.registry().insert(&active).unwrap();
    assert!(!f.registry().swarm(swarm).unwrap().unwrap().archived);
    f.registry()
        .update(active.id, |r| {
            r.lifecycle = Lifecycle::ClosedRetained;
            Ok(())
        })
        .unwrap();
    assert_ne!(
        f.call(sudo.id, Ulid::new(), "sudo_swarm", archive())["isError"],
        true
    );
    f.registry()
        .update(active.id, |r| {
            r.lifecycle = Lifecycle::Open;
            Ok(())
        })
        .unwrap();
    assert!(!f.registry().swarm(swarm).unwrap().unwrap().archived);
}
