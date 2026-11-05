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
    let task = Ulid::new();
    let f = Fixture::open_seeded(false, false, |repo, _| {
        git(repo, &["init", "-b", "main"]);
        std::fs::write(repo.join("source.txt"), "source\n").unwrap();
        std::fs::write(repo.join(".gitignore"), ".bboxy/\n").unwrap();
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
        assert_ne!(row["state"], "failed", "{result}");
        if row["state"] == state {
            return row.clone();
        }
        assert!(Instant::now() < deadline, "{result}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn prepare(f: &Fixture, task: Ulid) -> (Ulid, Value) {
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
