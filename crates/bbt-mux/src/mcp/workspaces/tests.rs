// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Private real Git/process fixtures.
use super::*;
use crate::{index::Index, knowledge::BboxySource, messaging::wire::IssuerKind, registry::Role};
use std::process::Command;

struct Fixture {
    temp: tempfile::TempDir,
    service: Service,
    index: Index,
    registry: Registry,
    actor: SeatActor,
    binding: RootBinding,
    repo: PathBuf,
    task: Ulid,
}
fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Workspace Test",
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
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("source");
        std::fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-b", "main"]);
        std::fs::write(repo.join("source.txt"), "source\n").unwrap();
        std::fs::write(repo.join(".gitignore"), ".bboxy/\nprivate/\n").unwrap();
        git(&repo, &["add", "source.txt", ".gitignore"]);
        git(&repo, &["commit", "-m", "base"]);
        let task = Ulid::new();
        let kb = repo.join(".bboxy");
        std::fs::create_dir_all(kb.join("tasks")).unwrap();
        std::fs::write(kb.join("tasks/work.md"),format!("---\nid: {task}\nnumber: 1\ntype: task\ntitle: Workspace test\nstatus: open\nassignee: none\nverify: 'true'\n---\nFixture\n")).unwrap();
        let mut index =
            Index::open(&kb, SqliteStore::open(&temp.path().join("index")).unwrap()).unwrap();
        index.scan().unwrap();
        let registry = Registry::new(SqliteStore::open(&temp.path().join("account")).unwrap());
        let actor = SeatActor {
            seat_id: Ulid::new(),
            process_generation: 1,
            swarm_id: Ulid::new(),
            role: Role {
                name: "oracle".into(),
                class: Class::Oracle,
            },
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        };
        let binding = RootBinding {
            swarm: actor.swarm_id,
            revision: 1,
            root_id: "fixture-root".into(),
        };
        let service = Service::open(&temp.path().join("runtime")).unwrap();
        Self {
            temp,
            service,
            index,
            registry,
            actor,
            binding,
            repo,
            task,
        }
    }
    fn call(&self, name: &str, args: Value) -> Call {
        Call {
            op_id: Ulid::new(),
            tool: name.into(),
            args,
        }
    }
    fn prepare(&self) -> Record {
        let c=self.call("workspace_prepare",json!({"task_path":self.task.to_string(),"repository":"current","provider":"git","base":"HEAD"}));
        let b = self
            .service
            .begin(
                &self.actor,
                &c,
                &self.repo,
                &self.binding,
                0,
                &BboxySource::new(&self.index),
                &self.registry,
            )
            .unwrap();
        let work = b.work.unwrap();
        let result = self.service.run(&work);
        self.service.finish(work.record(), result, true).unwrap()
    }
    fn begin_release(&self, r: &Record) -> Work {
        let c = self.call(
            "workspace_release",
            json!({"workspace_id":r.workspace_id,"expected_revision":r.revision}),
        );
        self.service
            .begin(
                &self.actor,
                &c,
                &self.repo,
                &self.binding,
                0,
                &BboxySource::new(&self.index),
                &self.registry,
            )
            .unwrap()
            .work
            .unwrap()
    }
}

#[test]
fn managed_workspace_real_git_preserves_source_and_separates_kb() {
    let f = Fixture::new();
    let base = git(&f.repo, &["rev-parse", "HEAD"]);
    let r = f.prepare();
    assert_eq!(r.state, State::Ready);
    assert_eq!(git(&r.cwd, &["rev-parse", "HEAD"]), base);
    assert!(!r.cwd.join(".bboxy").exists());
    assert!(f.repo.join(".bboxy/tasks/work.md").is_file());
    assert_eq!(git(&f.repo, &["symbolic-ref", "--short", "HEAD"]), "main");
    let w = f.begin_release(&r);
    let next = f
        .service
        .finish(w.record(), f.service.run(&w), true)
        .unwrap();
    assert_eq!(next.state, State::Released);
    assert!(!r.cwd.exists());
    assert_eq!(git(&f.repo, &["rev-parse", &r.branch]), base);
}
#[test]
fn managed_workspace_two_task_checkouts_isolate_writes_and_share_canonical_kb() {
    let mut f = Fixture::new();
    let first = f.prepare();
    f.task = Ulid::new();
    std::fs::write(f.repo.join(".bboxy/tasks/other.md"),format!("---\nid: {}\nnumber: 2\ntype: task\ntitle: Second workspace\nstatus: open\nassignee: none\nverify: 'true'\n---\nSeparate changes.\n",f.task)).unwrap();
    f.index.scan().unwrap();
    let second = f.prepare();
    assert_ne!(first.cwd, second.cwd);
    assert_ne!(first.branch, second.branch);
    assert_eq!(first.knowledge_binding, second.knowledge_binding);
    std::fs::write(first.cwd.join("source.txt"), "first worker\n").unwrap();
    std::fs::create_dir(first.cwd.join("target")).unwrap();
    std::fs::write(first.cwd.join("target/generated"), "build").unwrap();
    assert_eq!(
        std::fs::read_to_string(second.cwd.join("source.txt")).unwrap(),
        "source\n"
    );
    assert!(!second.cwd.join("target").exists());
    std::fs::write(second.cwd.join("source.txt"), "second worker\n").unwrap();
    assert_eq!(
        std::fs::read_to_string(first.cwd.join("source.txt")).unwrap(),
        "first worker\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("source.txt")).unwrap(),
        "source\n"
    );
}
#[test]
fn managed_workspace_refuses_untracked_ignored_and_unpublished_work() {
    let f = Fixture::new();
    let r = f.prepare();
    std::fs::write(r.cwd.join("untracked"), "keep").unwrap();
    let w = f.begin_release(&r);
    let r = f
        .service
        .finish(w.record(), f.service.run(&w), true)
        .unwrap();
    assert_eq!(r.state, State::Failed);
    assert!(r.error.as_ref().unwrap().contains("workspace_dirty"));
    assert!(r.cwd.join("untracked").exists());
    std::fs::remove_file(r.cwd.join("untracked")).unwrap();
    std::fs::create_dir(r.cwd.join("private")).unwrap();
    std::fs::write(r.cwd.join("private/report"), "private").unwrap();
    let w = f.begin_release(&r);
    let r = f
        .service
        .finish(w.record(), f.service.run(&w), true)
        .unwrap();
    assert!(r.error.as_ref().unwrap().contains("workspace_dirty"));
    std::fs::remove_dir_all(r.cwd.join("private")).unwrap();
    std::fs::write(r.cwd.join("source.txt"), "changed\n").unwrap();
    git(&r.cwd, &["add", "source.txt"]);
    git(&r.cwd, &["commit", "-m", "private change"]);
    let w = f.begin_release(&r);
    let r = f
        .service
        .finish(w.record(), f.service.run(&w), true)
        .unwrap();
    assert!(
        r.error
            .as_ref()
            .unwrap()
            .contains("workspace_unpublished_commits")
    );
    assert!(r.cwd.exists());
}
#[test]
fn managed_workspace_reservation_and_a_b_a_fences() {
    let f = Fixture::new();
    let r = f.prepare();
    let reference = Reference {
        id: r.workspace_id,
        expected_revision: r.revision,
    };
    assert!(
        f.service
            .prepare_launch(Ulid::new(), 1, f.actor.swarm_id, &r.cwd, &f.registry)
            .unwrap_err()
            .to_string()
            .contains("workspace_use_handle")
    );
    let mut stale = f.binding.clone();
    stale.revision += 2;
    assert!(f.service.resolve(&reference, &f.actor, &stale, 0).is_err());
    f.service
        .resolve(&reference, &f.actor, &f.binding, 0)
        .unwrap();
    let seat = Ulid::new();
    f.service.reserve(&r, seat, Ulid::new()).unwrap();
    assert!(f.service.reserve(&r, Ulid::new(), Ulid::new()).is_err());
    assert!(f.service.finish(&r, Ok(r.clone()), true).is_err());
    f.service.launched(r.workspace_id, seat, 1, 999999).unwrap();
    let mut worker = f.actor.clone();
    worker.seat_id = seat;
    worker.role.class = Class::Worker;
    assert_eq!(
        f.service.own(&worker).unwrap().unwrap().workspace_id,
        r.workspace_id
    );
    let live = f
        .service
        .execution_context(&worker, &f.binding, 0, None)
        .unwrap()
        .unwrap();
    assert_eq!(live.cwd, r.cwd);
    assert!(
        f.service
            .execution_context(&worker, &stale, 0, None)
            .is_err()
    );
    assert!(
        f.service
            .execution_context(&worker, &f.binding, 0, Some(&reference))
            .is_err()
    );
    assert!(
        f.service
            .delegation(seat, 1, &Ulid::new().to_string(), &f.binding)
            .is_err()
    );
    f.service
        .delegation(seat, 1, &f.task.to_string(), &f.binding)
        .unwrap();
}
#[test]
fn managed_workspace_launch_planning_rolls_back_but_uncertain_request_retains_writer() {
    let f = Fixture::new();
    let ready = f.prepare();
    let seat = Ulid::new();
    let op = Ulid::new();
    f.service.reserve(&ready, seat, op).unwrap();
    assert!(
        !f.service
            .cancel_unstarted_launch(ready.workspace_id, seat, Ulid::new())
            .unwrap()
    );
    assert!(
        f.service
            .cancel_unstarted_launch(ready.workspace_id, seat, op)
            .unwrap()
    );
    let ready = f.service.get(ready.workspace_id).unwrap();
    assert_eq!(ready.state, State::Ready);
    assert!(ready.holder.is_none());
    let seat = Ulid::new();
    let op = Ulid::new();
    f.service.reserve(&ready, seat, op).unwrap();
    f.service
        .request_launch(ready.workspace_id, seat, 1)
        .unwrap();
    assert!(
        f.service
            .request_launch(ready.workspace_id, seat, 1)
            .is_err()
    );
    assert!(
        !f.service
            .cancel_unstarted_launch(ready.workspace_id, seat, op)
            .unwrap()
    );
    let service = Service::open(&f.temp.path().join("runtime")).unwrap();
    service.observe_launch(seat, 1, 999999).unwrap();
    let observed = service.get(ready.workspace_id).unwrap();
    assert_eq!(observed.holder.unwrap().pid, Some(999999));
    service.observe_launch(seat, 1, 999999).unwrap();
    assert_eq!(
        service.get(ready.workspace_id).unwrap().revision,
        observed.revision
    );
    assert!(service.observe_launch(seat, 1, 999998).is_err());
    assert!(service.observe_launch(seat, 2, 999999).is_err());
}
#[test]
fn managed_workspace_restart_inspects_original_resource_without_duplicate_creation() {
    let f = Fixture::new();
    let c=f.call("workspace_prepare",json!({"task_path":f.task.to_string(),"repository":"current","provider":"git","base":"HEAD"}));
    let begun = f
        .service
        .begin(
            &f.actor,
            &c,
            &f.repo,
            &f.binding,
            0,
            &BboxySource::new(&f.index),
            &f.registry,
        )
        .unwrap();
    let work = begun.work.unwrap();
    let created = f.service.run(&work).unwrap();
    // Simulate loss after actual Git creation and before ready publication.
    let service = Service::open(&f.temp.path().join("runtime")).unwrap();
    assert_eq!(service.get(c.op_id).unwrap().state, State::Unknown);
    let call = f.call("workspace_get", json!({"workspace_id":c.op_id}));
    let b = service
        .begin(
            &f.actor,
            &call,
            &f.repo,
            &f.binding,
            0,
            &BboxySource::new(&f.index),
            &f.registry,
        )
        .unwrap();
    let inspect = b.work.unwrap();
    let r = service
        .finish(inspect.record(), service.run(&inspect), true)
        .unwrap();
    assert_eq!(r.state, State::Ready);
    assert_eq!(r.cwd, created.cwd);
    assert_eq!(
        git(&f.repo, &["worktree", "list", "--porcelain"])
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        2
    );
}
#[test]
fn managed_workspace_lost_release_is_inspected_before_a_new_guarded_release() {
    let f = Fixture::new();
    let ready = f.prepare();
    let old = f.begin_release(&ready);
    let service = Service::open(&f.temp.path().join("runtime")).unwrap();
    let call = f.call("workspace_get", json!({"workspace_id":ready.workspace_id}));
    let begun = service
        .begin(
            &f.actor,
            &call,
            &f.repo,
            &f.binding,
            0,
            &BboxySource::new(&f.index),
            &f.registry,
        )
        .unwrap();
    let inspect = begun.work.unwrap();
    let observed = service
        .finish(inspect.record(), service.run(&inspect), true)
        .unwrap();
    assert_eq!(observed.state, State::Failed);
    assert!(
        observed
            .error
            .as_ref()
            .unwrap()
            .contains("workspace_release_incomplete")
    );
    assert!(
        observed.cwd.exists(),
        "inspection must never repeat deletion"
    );
    assert!(
        f.service.run(&old).is_err(),
        "old queued deletion is fenced"
    );
    assert!(observed.cwd.exists());
    let release = f.begin_release(&observed);
    let done = f
        .service
        .finish(release.record(), f.service.run(&release), true)
        .unwrap();
    assert_eq!(done.state, State::Released);
    assert!(!done.cwd.exists());
}
#[test]
fn managed_workspace_collisions_and_permissions_do_not_launch_or_adopt() {
    let f = Fixture::new();
    let mut worker = f.actor.clone();
    worker.role.class = Class::Worker;
    let c=f.call("workspace_prepare",json!({"task_path":f.task.to_string(),"repository":"current","provider":"git","base":"HEAD"}));
    assert!(
        f.service
            .begin(
                &worker,
                &c,
                &f.repo,
                &f.binding,
                0,
                &BboxySource::new(&f.index),
                &f.registry
            )
            .is_err()
    );
    let b = f
        .service
        .begin(
            &f.actor,
            &c,
            &f.repo,
            &f.binding,
            0,
            &BboxySource::new(&f.index),
            &f.registry,
        )
        .unwrap();
    let w = b.work.unwrap();
    std::fs::create_dir(&w.record().cwd).unwrap();
    std::fs::write(w.record().cwd.join("foreign"), "keep").unwrap();
    let failed = f
        .service
        .finish(w.record(), f.service.run(&w), true)
        .unwrap();
    assert!(
        failed
            .error
            .as_ref()
            .unwrap()
            .contains("workspace_collision")
    );
    let release = f.begin_release(&failed);
    assert!(
        f.service
            .run(&release)
            .unwrap_err()
            .to_string()
            .contains("workspace_identity_changed")
    );
    assert!(w.record().cwd.join("foreign").exists());
}

#[test]
fn managed_workspace_failed_prepare_releases_only_its_durably_claimed_empty_directory() {
    let f = Fixture::new();
    let c=f.call("workspace_prepare",json!({"task_path":f.task.to_string(),"repository":"current","provider":"git","base":"HEAD"}));
    let begun = f
        .service
        .begin(
            &f.actor,
            &c,
            &f.repo,
            &f.binding,
            0,
            &BboxySource::new(&f.index),
            &f.registry,
        )
        .unwrap();
    let work = begun.work.unwrap();
    git(&f.repo, &["branch", &work.record().branch]);
    let failed = f
        .service
        .finish(work.record(), f.service.run(&work), true)
        .unwrap();
    assert_eq!(failed.state, State::Failed);
    assert!(failed.checkout_identity.is_some());
    assert!(std::fs::read_dir(&failed.cwd).unwrap().next().is_none());
    let release = f.begin_release(&failed);
    let done = f
        .service
        .finish(release.record(), f.service.run(&release), true)
        .unwrap();
    assert_eq!(done.state, State::Released);
    assert!(!done.cwd.exists());
    assert!(
        !git(
            &f.repo,
            &[
                "show-ref",
                "--verify",
                &format!("refs/heads/{}", done.branch)
            ]
        )
        .is_empty()
    );
}

#[test]
fn managed_workspace_truncated_provider_evidence_fails_closed_after_draining() {
    for script in [
        "dd if=/dev/zero bs=65536 count=2 2>/dev/null",
        "(dd if=/dev/zero bs=65536 count=2 2>/dev/null) >&2",
    ] {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        let error = super::git::output(command, std::time::Duration::from_secs(3)).unwrap_err();
        assert!(
            error
                .to_string()
                .starts_with("workspace_provider_output_exceeded:"),
            "{error}"
        );
    }
}
#[test]
fn managed_workspace_pipe_timeout_reaps_descendant_after_leader_exit() {
    let temp = tempfile::tempdir().unwrap();
    let pidfile = temp.path().join("descendant.pid");
    let mut command = Command::new("/bin/sh");
    command
        .args([
            "-c",
            r#"sleep 60 & printf '%s' "$!" > "$WORKSPACE_CHILD_PID"; exit 0"#,
        ])
        .env("WORKSPACE_CHILD_PID", &pidfile);
    let error = super::git::output(command, std::time::Duration::from_millis(150)).unwrap_err();
    assert!(error.to_string().contains("workspace_provider_timeout"));
    let pid = std::fs::read_to_string(pidfile).unwrap().parse().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while super::git::process_exists(pid) {
        assert!(
            std::time::Instant::now() < deadline,
            "owned descendant {pid} leaked after timeout"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[test]
fn managed_workspace_submodule_private_data_is_preserved_before_deinit_and_release() {
    let f = Fixture::new();
    let module = f.temp.path().join("module");
    std::fs::create_dir(&module).unwrap();
    git(&module, &["init", "-b", "main"]);
    std::fs::write(
        module.join("tracked"),
        "module
",
    )
    .unwrap();
    std::fs::write(
        module.join(".gitignore"),
        "private/
",
    )
    .unwrap();
    git(&module, &["add", "tracked", ".gitignore"]);
    git(&module, &["commit", "-m", "module"]);
    git(
        &f.repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            module.to_str().unwrap(),
            "module",
        ],
    );
    git(&f.repo, &["commit", "-am", "pin module"]);
    let call=f.call("workspace_prepare",json!({"task_path":f.task.to_string(),"repository":"current","provider":"git","base":"HEAD"}));
    let begun = f
        .service
        .begin(
            &f.actor,
            &call,
            &f.repo,
            &f.binding,
            0,
            &BboxySource::new(&f.index),
            &f.registry,
        )
        .unwrap();
    let work = begun.work.unwrap();
    let r = f
        .service
        .finish(work.record(), f.service.run(&work), true)
        .unwrap();
    assert!(
        r.checkout_identity.is_some(),
        "the provider must durably claim its checkout before setup"
    );
    // The real provider claims/materializes the checkout. Complete this private
    // fixture's setup with a local-file protocol override, then inspect that
    // exact owned resource. Production does not globally enable file URLs.
    git(
        &r.cwd,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "update",
            "--init",
            "--recursive",
        ],
    );
    let work = Work::Inspect(r);
    let r = f
        .service
        .finish(work.record(), f.service.run(&work), true)
        .unwrap();
    assert_eq!(r.state, State::Ready);
    std::fs::create_dir(r.cwd.join("module/private")).unwrap();
    std::fs::write(r.cwd.join("module/private/evidence"), "retain").unwrap();
    let work = f.begin_release(&r);
    let r = f
        .service
        .finish(work.record(), f.service.run(&work), true)
        .unwrap();
    assert!(r.error.as_ref().unwrap().contains("workspace_dirty"));
    assert!(r.cwd.join("module/private/evidence").exists());
    std::fs::remove_dir_all(r.cwd.join("module/private")).unwrap();
    let work = f.begin_release(&r);
    let r = f
        .service
        .finish(work.record(), f.service.run(&work), true)
        .unwrap();
    assert_eq!(r.state, State::Released, "{:?}", r.error);
    assert!(!r.cwd.exists());
    let archive = r.preserved_module_repositories.unwrap();
    assert!(archive.join("module/objects").is_dir());
    assert!(archive.join("module/HEAD").is_file());
}

#[test]
fn managed_workspace_restart_transfers_only_a_proven_dead_writer_generation() {
    let f = Fixture::new();
    f.registry
        .update_swarm_workdir(f.actor.swarm_id, Some(f.repo.to_str().unwrap()))
        .unwrap();
    let r = f.prepare();
    let seat = Ulid::new();
    f.service.reserve(&r, seat, Ulid::new()).unwrap();
    f.service
        .prepare_launch(seat, 1, f.actor.swarm_id, &r.cwd, &f.registry)
        .unwrap();
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Owned(Command::new("sleep").arg("60").spawn().unwrap());
    f.service
        .launched(r.workspace_id, seat, 1, child.0.id())
        .unwrap();
    assert!(
        f.service
            .prepare_launch(seat, 2, f.actor.swarm_id, &r.cwd, &f.registry)
            .is_err()
    );
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    f.service
        .prepare_launch(seat, 2, f.actor.swarm_id, &r.cwd, &f.registry)
        .unwrap();
    let mut worker = f.actor.clone();
    worker.seat_id = seat;
    worker.role.class = Class::Worker;
    assert!(f.service.own(&worker).is_err());
    worker.process_generation = 2;
    assert!(
        f.service
            .execution_context(&worker, &f.binding, 0, None)
            .is_err()
    );
    f.service
        .launched(r.workspace_id, seat, 2, std::process::id())
        .unwrap();
    assert_eq!(
        f.service
            .execution_context(&worker, &f.binding, 0, None)
            .unwrap()
            .unwrap()
            .workspace_id,
        r.workspace_id
    );
}
#[test]
fn managed_workspace_task_lifecycle_is_allowed_but_scope_amendment_is_fenced() {
    let f = Fixture::new();
    let r = f.prepare();
    let path = f.repo.join(".bboxy/tasks/work.md");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("status: open", "status: assigned");
    std::fs::write(&path, &text).unwrap();
    Service::verify_ready(&r, &BboxySource::new(&f.index)).unwrap();
    std::fs::write(&path, text.replace("verify: 'true'", "verify: 'false'")).unwrap();
    let error = Service::verify_ready(&r, &BboxySource::new(&f.index)).unwrap_err();
    assert!(error.to_string().contains("workspace_task_changed"));
}

#[test]
fn managed_workspace_release_refuses_merge_only_tree_change_omitted_by_cherry() {
    let f = Fixture::new();
    let r = f.prepare();
    git(&f.repo, &["checkout", "-b", "side"]);
    std::fs::write(
        f.repo.join("side.txt"),
        "side
",
    )
    .unwrap();
    git(&f.repo, &["add", "side.txt"]);
    git(&f.repo, &["commit", "-m", "side"]);
    git(&f.repo, &["checkout", "main"]);
    std::fs::write(
        f.repo.join("main.txt"),
        "main
",
    )
    .unwrap();
    git(&f.repo, &["add", "main.txt"]);
    git(&f.repo, &["commit", "-m", "main"]);
    let parent = git(&f.repo, &["rev-parse", "HEAD"]);
    git(
        &f.repo,
        &["merge", "--no-ff", "side", "-m", "accepted merge"],
    );
    git(&r.cwd, &["merge", "--ff-only", &parent]);
    git(&r.cwd, &["merge", "--no-ff", "--no-commit", "side"]);
    std::fs::write(
        r.cwd.join("source.txt"),
        "merge-only source change
",
    )
    .unwrap();
    git(&r.cwd, &["add", "source.txt"]);
    git(&r.cwd, &["commit", "-m", "unpublished evil merge"]);
    assert!(
        git(
            &f.repo,
            &["cherry", "main", &git(&r.cwd, &["rev-parse", "HEAD"])]
        )
        .is_empty(),
        "baseline cherry must omit the unmatched merge-only change"
    );
    let work = f.begin_release(&r);
    let r = f
        .service
        .finish(work.record(), f.service.run(&work), true)
        .unwrap();
    assert!(
        r.error
            .as_ref()
            .unwrap()
            .contains("workspace_unpublished_commits")
    );
    assert_eq!(
        std::fs::read_to_string(r.cwd.join("source.txt")).unwrap(),
        "merge-only source change
"
    );
}

#[test]
fn managed_workspace_orientation_follows_capabilities_not_role_label() {
    let mut actor = SeatActor {
        seat_id: Ulid::new(),
        process_generation: 1,
        swarm_id: Ulid::new(),
        role: Role {
            name: "custom-authorized-role".into(),
            class: Class::Oracle,
        },
        grant_epoch: 1,
        issuer_kind: IssuerKind::LocalDev,
    };
    let call = Call {
        op_id: Ulid::new(),
        tool: "orient".into(),
        args: json!({}),
    };
    for expected in [true, false] {
        let catalogue = super::super::tools::catalogue(&actor, vec![]);
        let result = super::super::hints::attach(
            &call,
            super::super::success(
                call.op_id,
                json!({"actor":actor,"tools":catalogue["tools"]}),
            ),
        );
        assert_eq!(
            result["structuredContent"]["hints"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["params"]["name"] == "workspace_orient"),
            expected
        );
        let text: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text["hints"], result["structuredContent"]["hints"]);
        actor.role.name = "oracle".into();
        actor.role.class = Class::Worker;
    }
}

#[test]
fn managed_workspace_provider_lease_survives_parent_descriptor_drop() {
    let f = Fixture::new();
    let r = f.prepare();
    let file = Service::provider_lease(&r).unwrap();
    file.try_lock().unwrap();
    let mut child = super::git::with_provider_lease(std::sync::Arc::new(file), || {
        let mut command = Command::new("/bin/sleep");
        command.arg("60");
        let _lease = super::git::inherit_provider_lease(&mut command);
        Ok(command.spawn()?)
    })
    .unwrap();
    // Only the child now owns that open file description. A fresh process
    // cannot call absence proof/release while this old provider can still act.
    assert!(!Service::provider_idle(&r).unwrap());
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(Service::provider_idle(&r).unwrap());
}
