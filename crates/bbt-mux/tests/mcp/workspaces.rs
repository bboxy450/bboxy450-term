// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Real private Git checkouts through the admitted native MCP/control path.
use super::*;
fn git(repo: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Workspace Fixture",
            "-c",
            "user.email=workspace@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn fixture() -> (Fixture, Ulid) {
    fixture_kind(false)
}
pub(super) fn fixture_real() -> (Fixture, Ulid) {
    fixture_kind(true)
}
fn fixture_kind(real: bool) -> (Fixture, Ulid) {
    let task = Ulid::new();
    let f = Fixture::open_seeded(real, false, |repo, _| {
        git(repo, &["init", "-b", "main"]);
        std::fs::write(repo.join("source.txt"), "source\n").unwrap();
        std::fs::write(repo.join(".gitignore"), ".bboxy/\ntarget/\n").unwrap();
        git(repo, &["add", "source.txt", ".gitignore"]);
        git(repo, &["commit", "-m", "base"]);
        std::fs::write(repo.join(".bboxy/tasks/managed.md"),format!("---\nid: {task}\nnumber: 3\ntype: task\ntitle: Managed fixture\nstatus: open\nassignee: none\nverify: 'true'\nallowed_paths: [source.txt]\n---\nImplement the fixture.\n")).unwrap();
        std::fs::write(repo.join(".bboxy/tasks/managed-other.md"),format!("---\nid: {}\nnumber: 4\ntype: task\ntitle: Other managed fixture\nstatus: open\nassignee: none\nverify: 'true'\nallowed_paths: [source.txt]\n---\nIndependent fixture.\n",Ulid::from(777u128))).unwrap();
    });
    (f, task)
}
fn wait(f: &Fixture, id: Ulid, state: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let result = data(f.call(
            f.oracle.id,
            Ulid::new(),
            "workspace_get",
            json!({"workspace_id":id}),
        ));
        let row = &result["workspace"];
        assert!(state == "failed" || row["state"] != "failed", "{result}");
        if row["state"] == state {
            return row.clone();
        }
        assert!(Instant::now() < deadline, "{result}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub(super) fn prepare(f: &Fixture, task: Ulid) -> (Ulid, Value) {
    let op = Ulid::new();
    let args =
        json!({"task_path":task.to_string(),"repository":"current","provider":"git","base":"HEAD"});
    let started = Instant::now();
    let response = data(f.mutate(f.oracle.id, op, "workspace_prepare", args.clone()));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(response["workspace"]["state"], "preparing");
    let id = serde_json::from_value(response["workspace"]["workspace_id"].clone()).unwrap();
    let ready = wait(f, id, "ready");
    let replay = data(f.mutate(f.oracle.id, op, "workspace_prepare", args));
    assert_eq!(replay["workspace"]["workspace_id"], json!(id));
    (id, ready)
}
#[test]
fn managed_workspace_raw_cwd_spawn_is_rejected_without_opening_a_seat() {
    let (f, task) = fixture();
    let (id, ready) = prepare(&f, task);
    let args = json!({
        "swarm_id": f.oracle.swarm, "approved_profile": "zsh",
        "role": "executor", "name": "raw checkout", "cwd": ready["cwd"],
    });
    let op = Ulid::new();
    let response = f.mutate(f.oracle.id, op, "seat_spawn", args.clone());
    let seats = data(f.call(f.oracle.id, Ulid::new(), "seat_list", json!({})));
    let error = &response["structuredContent"]["error"];
    assert_eq!(error["code"], "rejected", "{response}; seats: {seats}");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("workspace_use_handle"),
        "{response}"
    );
    let reserved: Ulid =
        serde_json::from_value(response["structuredContent"]["seat_id"].clone()).unwrap();
    assert!(f.registry().get(reserved).is_err(), "{seats}");
    assert_eq!(wait(&f, id, "ready")["revision"], ready["revision"]);
    assert_eq!(f.mutate(f.oracle.id, op, "seat_spawn", args), response);
}

#[test]
fn managed_workspace_delegating_oracle_respawns_after_task_amendment() {
    let (f, task) = fixture_real();
    let (id, ready) = prepare(&f, task);
    let mut args = json!({
        "swarm_id": f.oracle.swarm, "approved_profile": "zsh", "role": "executor",
        "workspace": {"id": id, "expected_revision": ready["revision"]},
    });
    let first = data(f.mutate(f.oracle.id, Ulid::new(), "seat_spawn", args.clone()));
    let seat: Ulid = serde_json::from_value(first["seat_id"].clone()).unwrap();
    data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({"task_path": task, "seat_id": seat}),
    ));
    let before = data(knowledge_fetch(&f, f.oracle.id, &task.to_string()));
    let mut amendment = knowledge_update_args(&before);
    amendment["body"] = json!("Implement the amended fixture.");
    let after = data(f.mutate(f.oracle.id, Ulid::new(), "knowledge_write", amendment));
    assert_ne!(after["record"]["etag"], before["etag"]);
    f.runtime
        .block_on(f.client().close(
            seat,
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
    let ready = wait(&f, id, "ready");
    args["workspace"]["expected_revision"] = ready["revision"].clone();
    let replacement = data(f.mutate(f.oracle.id, Ulid::new(), "seat_spawn", args));
    assert_eq!(replacement["status"], "applied");
    assert_ne!(replacement["seat_id"], first["seat_id"]);
    let workspace = wait(&f, id, "in_use");
    assert_eq!(workspace["cwd"], ready["cwd"]);
    assert_eq!(workspace["holder"]["seat_id"], replacement["seat_id"]);
    assert_eq!(workspace["holder"]["pid"], replacement["pid"]);
}

#[test]
fn managed_workspace_mcp_prepares_launches_and_delegates_with_canonical_kb() {
    let (mut f, task) = fixture();
    let (id, ready) = prepare(&f, task);
    let cwd = PathBuf::from(ready["target"]["cwd"].as_str().unwrap());
    assert!(cwd.join("source.txt").is_file());
    assert!(!cwd.join(".bboxy").exists());
    let launch = Ulid::new();
    let args = json!({"swarm_id":f.oracle.swarm,"approved_profile":"zsh","role":"executor","workspace":{"id":id,"expected_revision":ready["revision"]}});
    let response = data(f.mutate(f.oracle.id, launch, "seat_spawn", args.clone()));
    let seat: Ulid = serde_json::from_value(response["seat_id"].clone()).unwrap();
    assert_eq!(
        data(f.mutate(f.oracle.id, launch, "seat_spawn", args))["seat_id"],
        json!(seat)
    );
    let own = data(f.call(seat, Ulid::new(), "workspace_get", json!({})));
    assert_eq!(own["workspace"]["workspace_id"], json!(id));
    assert_eq!(own["workspace"]["state"], "in_use");
    let forbidden = f.call(
        seat,
        Ulid::new(),
        "workspace_prepare",
        json!({"task_path":task.to_string(),"repository":"current","provider":"git","base":"HEAD"}),
    );
    assert_eq!(forbidden["isError"], true);
    let assigned = f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({"task_path":task.to_string(),"seat_id":seat}),
    );
    assert_eq!(assigned["isError"], false, "{assigned}");
    // Supervisor ownership survives logic restart; reconnection confirms the
    // original writing PID and canonical KB rather than launching a replacement.
    f.restart();
    let fetched = data(f.call(
        seat,
        Ulid::new(),
        "knowledge_fetch",
        json!({"locator":task.to_string()}),
    ));
    assert_eq!(fetched["id"], json!(task));
    let oriented = data(f.call(seat, Ulid::new(), "orient", json!({})));
    assert_eq!(
        oriented["implementation_workspace"]["workspace_id"],
        json!(id)
    );
    let current = wait(&f, id, "in_use");
    let refused = f.call(
        f.oracle.id,
        Ulid::new(),
        "workspace_release",
        json!({"workspace_id":id,"expected_revision":current["revision"]}),
    );
    assert_eq!(refused["isError"], true);
    assert!(cwd.exists());
}
#[test]
fn managed_workspace_mcp_releases_a_built_worktree_after_rebase() {
    let (f, task) = fixture_real();
    let (id, ready) = prepare(&f, task);
    let cwd = PathBuf::from(ready["cwd"].as_str().unwrap());
    let seat: Ulid = serde_json::from_value(
        data(f.mutate(
            f.oracle.id,
            Ulid::new(),
            "seat_spawn",
            json!({
                "swarm_id": f.oracle.swarm, "approved_profile": "zsh", "role": "executor",
                "workspace": {"id": id, "expected_revision": ready["revision"]},
            }),
        ))["seat_id"]
            .clone(),
    )
    .unwrap();
    wait(&f, id, "in_use");

    std::fs::write(cwd.join("source.txt"), "int main(void) { return 0; }\n").unwrap();
    git(&cwd, &["commit", "-am", "worker source"]);
    let worker = git(&cwd, &["rev-parse", "HEAD"]);
    let built = std::process::Command::new("/bin/sh")
        .args([
            "-ec",
            "mkdir target; cc -x c source.txt -o target/build; ./target/build",
        ])
        .current_dir(&cwd)
        .status()
        .unwrap();
    assert!(built.success());
    for i in 0..1200 {
        std::fs::write(
            cwd.join("target")
                .join(format!("{i:04}-{}", "x".repeat(65))),
            b"build",
        )
        .unwrap();
    }
    assert!(git(&cwd, &["status", "--porcelain", "--ignored", "-uall"]).len() > 65536);

    f.runtime
        .block_on(f.client().close(
            seat,
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

    let release = |revision: &Value| {
        data(f.mutate(
            f.oracle.id,
            Ulid::new(),
            "workspace_release",
            json!({"workspace_id":id,"expected_revision":revision}),
        ))
    };
    let ready = wait(&f, id, "ready");
    assert_eq!(
        release(&ready["revision"])["workspace"]["state"],
        "releasing"
    );
    let failed = wait(&f, id, "failed");
    assert!(
        failed["error"]
            .as_str()
            .unwrap()
            .contains("workspace_unpublished_commits")
    );
    assert!(cwd.join("target/build").is_file());

    let repo = &f.config.repository;
    std::fs::write(repo.join("upstream.txt"), "upstream\n").unwrap();
    git(repo, &["add", "upstream.txt"]);
    git(repo, &["commit", "-m", "upstream"]);
    git(repo, &["checkout", "-b", "landing-fixture", &worker]);
    git(repo, &["-c", "core.hooksPath=/dev/null", "rebase", "main"]);
    let rebased = git(repo, &["rev-parse", "HEAD"]);
    assert_ne!(rebased, worker);
    git(repo, &["checkout", "main"]);
    git(repo, &["merge", "--ff-only", &rebased]);

    assert_eq!(
        release(&failed["revision"])["workspace"]["state"],
        "releasing"
    );
    let released = wait(&f, id, "released");
    assert!(!cwd.exists());
    assert_eq!(
        git(repo, &["rev-parse", released["branch"].as_str().unwrap()]),
        worker
    );
}
#[test]
fn managed_workspace_mcp_restart_replay_and_guarded_release() {
    let (mut f, task) = fixture();
    let (id, ready) = prepare(&f, task);
    let cwd = PathBuf::from(ready["cwd"].as_str().unwrap());
    f.restart();
    let ready = wait(&f, id, "ready");
    assert_eq!(
        git(&f.config.repository, &["worktree", "list", "--porcelain"])
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        2
    );
    let before = data(f.call(f.oracle.id, Ulid::new(), "self_status", json!({})));
    assert_eq!(before["actor"]["seat_id"], json!(f.oracle.id));
    let release = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "workspace_release",
        json!({"workspace_id":id,"expected_revision":ready["revision"]}),
    ));
    assert_eq!(release["workspace"]["state"], "releasing");
    wait(&f, id, "released");
    assert!(!cwd.exists());
    let list = data(f.call(
        f.oracle.id,
        Ulid::new(),
        "workspace_list",
        json!({"limit":1}),
    ));
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
}
#[test]
fn managed_workspace_mcp_rejects_wrong_source_and_unsupported_provider() {
    let (f, task) = fixture();
    for args in [
        json!({"task_path":task.to_string(),"repository":"/tmp","provider":"git","base":"HEAD"}),
        json!({"task_path":task.to_string(),"repository":"current","provider":"cloud","base":"HEAD"}),
    ] {
        let value = f.call(f.oracle.id, Ulid::new(), "workspace_prepare", args);
        assert_eq!(value["isError"], true, "{value}");
    }
    assert_eq!(
        git(&f.config.repository, &["worktree", "list", "--porcelain"])
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        1
    );
}

#[test]
fn managed_workspace_mcp_slow_git_does_not_block_owner_or_messages() {
    use std::os::unix::fs::PermissionsExt;
    let (f, task) = fixture();
    let entered = f._temp.path().join("hook-entered");
    let release = f._temp.path().join("hook-release");
    let hook = f.config.repository.join(".git/hooks/post-checkout");
    std::fs::write(
        &hook,
        format!(
            "#!/bin/sh\nprintf ready > '{}'\nwhile [ ! -f '{}' ]; do sleep 0.02; done\n",
            entered.display(),
            release.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o700)).unwrap();
    // Cleanup releases only our own private hook, even if an assertion fails.
    struct Unblock(PathBuf);
    impl Drop for Unblock {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, "release");
        }
    }
    let _unblock = Unblock(release.clone());
    let response = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "workspace_prepare",
        json!({"task_path":task.to_string(),"repository":"current","provider":"git","base":"HEAD"}),
    ));
    let id: Ulid = serde_json::from_value(response["workspace"]["workspace_id"].clone()).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !entered.exists() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let before = Instant::now();
    let status = data(f.call(f.oracle.id, Ulid::new(), "self_status", json!({})));
    assert_eq!(status["actor"]["seat_id"], json!(f.oracle.id));
    assert!(before.elapsed() < Duration::from_secs(2));
    let orientation = data(f.call(f.oracle.id, Ulid::new(), "workspace_orient", json!({})));
    assert_eq!(orientation["can_provision"], true);
    let list = data(f.call(f.oracle.id, Ulid::new(), "workspace_list", json!({})));
    assert_eq!(list["items"][0]["state"], "preparing");
    let message = f.mutate(
        f.oracle.id,
        Ulid::new(),
        "message_send",
        json!({"destination":"operator/inbox","text":"private responsiveness fixture"}),
    );
    assert_eq!(message["isError"], false, "{message}");
    let other = f.config.repository.join("other-project");
    std::fs::create_dir(&other).unwrap();
    git(&other, &["init", "-b", "main"]);
    std::fs::write(other.join("source.txt"), "independent").unwrap();
    git(&other, &["add", "source.txt"]);
    git(&other, &["commit", "-m", "base"]);
    let response=data(f.mutate(f.oracle.id,Ulid::new(),"workspace_prepare",json!({"task_path":Ulid::from(777u128).to_string(),"repository":other,"provider":"git","base":"HEAD"})));
    let other_id = serde_json::from_value(response["workspace"]["workspace_id"].clone()).unwrap();
    wait(&f, other_id, "ready");
    assert!(
        !release.exists(),
        "independent preparation completed before the held job"
    );
    std::fs::write(&release, "release").unwrap();
    wait(&f, id, "ready");
}

#[test]
fn managed_workspace_mcp_child_project_keeps_parent_knowledge_binding() {
    let (f, task) = fixture();
    let child = f.config.repository.join("project");
    std::fs::create_dir(&child).unwrap();
    git(&child, &["init", "-b", "main"]);
    std::fs::write(
        child.join("source.txt"),
        "child project
",
    )
    .unwrap();
    git(&child, &["add", "source.txt"]);
    git(&child, &["commit", "-m", "child"]);
    let response = data(f.mutate(
        f.oracle.id,
        Ulid::new(),
        "workspace_prepare",
        json!({"task_path":task.to_string(),"repository":child,"provider":"git","base":"HEAD"}),
    ));
    let id: Ulid = serde_json::from_value(response["workspace"]["workspace_id"].clone()).unwrap();
    let ready = wait(&f, id, "ready");
    let cwd = PathBuf::from(ready["cwd"].as_str().unwrap());
    assert_eq!(
        std::fs::read_to_string(cwd.join("source.txt")).unwrap(),
        "child project
"
    );
    assert!(!cwd.join(".bboxy").exists());
    let launched=data(f.mutate(f.oracle.id,Ulid::new(),"seat_spawn",json!({"swarm_id":f.oracle.swarm,"approved_profile":"zsh","role":"executor","workspace":{"id":id,"expected_revision":ready["revision"]}})));
    let seat: Ulid = serde_json::from_value(launched["seat_id"].clone()).unwrap();
    let assigned = f.mutate(
        f.oracle.id,
        Ulid::new(),
        "task_delegate",
        json!({"task_path":task.to_string(),"seat_id":seat}),
    );
    assert_eq!(assigned["isError"], false, "{assigned}");
    let document = data(f.call(
        seat,
        Ulid::new(),
        "knowledge_fetch",
        json!({"locator":task.to_string()}),
    ));
    assert_eq!(document["id"], json!(task));
}

#[test]
fn managed_workspace_pre_send_link_down_releases_the_unrequested_writer() {
    use bbt_mux::logic::SupervisorConnection;
    use bbt_mux::provider::{HostId, LinkState};
    use bbt_mux::{Action, Reply};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    struct Probe {
        inner: LostInputReceipt,
        armed: Arc<AtomicBool>,
        trace: Arc<Mutex<Vec<Action>>>,
        store: SqliteStore,
        workspace: Ulid,
    }
    impl SupervisorConnection for Probe {
        fn request(&mut self, action: Action) -> bbt_mux::store::Result<Reply> {
            self.trace.lock().unwrap().push(action.clone());
            if matches!(&action, Action::ListOwnedSeats) && self.armed.load(Ordering::SeqCst) {
                let bytes = self
                    .store
                    .get(&format!("workspace/{}", self.workspace))?
                    .unwrap();
                let reserved: Value = serde_json::from_slice(&bytes)?;
                // Caller admission also reads owned seats before reservation.
                if reserved["state"] == "in_use" && self.armed.swap(false, Ordering::SeqCst) {
                    assert_eq!(reserved["holder"]["launch_requested"], false);
                    assert!(reserved["holder"]["pid"].is_null());
                    return Err(Box::new(bbt_mux::provider::fake::LinkDown {
                        host: HostId::Local,
                        state: LinkState::Connecting,
                    }));
                }
            }
            self.inner.request(action)
        }
        fn receive(&mut self, wait: Duration) -> bbt_mux::store::Result<Option<bbt_mux::Event>> {
            self.inner.receive(wait)
        }
    }
    let (mut f, task) = fixture_real();
    let (id, ready) = prepare(&f, task);
    f.logic.take().unwrap().shutdown().unwrap();
    let armed = Arc::new(AtomicBool::new(false));
    let trace = Arc::new(Mutex::new(Vec::new()));
    // A new supervisor connection replaces the current logic lease.
    let Reply::Seats { seats: before } = f.connection().request(Action::ListOwnedSeats).unwrap()
    else {
        panic!("native seat observation");
    };
    f.logic = Some(
        Logic::start(
            Probe {
                inner: LostInputReceipt {
                    client: f.connection(),
                    lose: false,
                    attempts: f.input_attempts.clone(),
                    pty_bytes: f.pty_bytes.clone(),
                    history_gap: f.history_gap.clone(),
                    native_runtime: f.native_runtime.clone(),
                },
                armed: armed.clone(),
                trace: trace.clone(),
                store: SqliteStore::open(&f.config.run_dir.join("implementation-workspaces"))
                    .unwrap(),
                workspace: id,
            },
            f.config.clone(),
            key(),
            f.runtime.handle(),
        )
        .unwrap(),
    );
    trace.lock().unwrap().clear();
    armed.store(true, Ordering::SeqCst);
    let response = f.mutate(
        f.oracle.id,
        Ulid::new(),
        "seat_spawn",
        json!({
            "swarm_id": f.oracle.swarm, "approved_profile": "zsh", "role": "executor",
            "workspace": {"id": id, "expected_revision": ready["revision"]},
        }),
    );
    let result = &response["structuredContent"];
    assert_eq!(result["error"]["code"], "rejected", "{response}");
    assert!(
        result["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Connecting"),
        "{response}"
    );
    let seat: Ulid = serde_json::from_value(result["seat_id"].clone()).unwrap();
    assert!(f.registry().get(seat).is_err());
    assert!(
        !armed.load(Ordering::SeqCst),
        "the fresh pre-read did not run"
    );
    let actions = trace.lock().unwrap().clone();
    assert!(
        actions.iter().any(|a| matches!(a, Action::ListOwnedSeats)),
        "{actions:?}"
    );
    assert!(
        !actions.iter().any(|a| matches!(a, Action::Spawn { .. })),
        "{actions:?}"
    );
    let released = wait(&f, id, "ready");
    assert!(released["holder"].is_null(), "{released}");
    f.logic.take().unwrap().shutdown().unwrap();
    let Reply::Seats { seats: after } = f.connection().request(Action::ListOwnedSeats).unwrap()
    else {
        panic!("native seat observation");
    };
    assert_eq!(after.len(), before.len());
    assert!(!after.iter().any(|info| info.seat == seat));
    eprintln!(
        "managed pre-send Connecting: probe saw launch_requested=false/PID=null, trace={actions:?}, registry absent, native seats={} unchanged, refused seat/PID absent, workspace ready/no holder",
        after.len()
    );
}
