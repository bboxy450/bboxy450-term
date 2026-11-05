// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;

#[test]
fn working_tree_edits_hooks_and_builds_do_not_schedule_artifact_hashes() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let mut detector = Detector::committed(temp.path(), runtime.handle().clone()).unwrap();
    detector.register(Ulid(1), 1, temp.path(), true).unwrap();
    std::fs::create_dir_all(temp.path().join("src")).unwrap();
    std::fs::create_dir_all(temp.path().join("target/debug")).unwrap();
    for n in 0..200 {
        std::fs::write(temp.path().join("src/work.rs"), format!("draft {n}")).unwrap();
        std::fs::write(
            temp.path().join("target/debug/generated"),
            format!("build {n}"),
        )
        .unwrap();
        detector.hook(Ulid(1),1,&crate::adapters::hook::ShimEvent{event:"PostToolUse".into(),raw:serde_json::json!({"tool_name":"Write","tool_input":{"file_path":"src/work.rs"}})});
    }
    detector.reconcile_known(["src/work.rs".into()]);
    let _entered = runtime.enter();
    let (rows, _) = detector.poll(&[(Ulid(1), 1, 0)]);
    assert!(rows.is_empty());
    assert!(detector.jobs.is_empty(), "drafts scheduled IO hashing jobs");
    assert!(detector.inbox.lock().unwrap().queue.is_empty());
    assert!(
        detector.watchers.is_empty(),
        "production still recursively watches a worktree"
    );
}

#[test]
fn committed_git_head_is_observed_once_without_file_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("/usr/bin/git")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .current_dir(temp.path())
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--quiet", "--initial-branch=main", "--template="]);
    std::fs::write(temp.path().join("work.txt"), "checkpoint").unwrap();
    git(&["add", "work.txt"]);
    git(&["commit", "--quiet", "-m", "explicit checkpoint"]);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let _entered = runtime.enter();
    let mut detector = Detector::committed(temp.path(), runtime.handle().clone()).unwrap();
    let mut completed = Vec::new();
    for expected in [1, 0] {
        detector.git_at = std::time::Instant::now() - std::time::Duration::from_secs(60);
        detector.poll(&[]);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while detector.git_job.is_some() {
            completed.extend(detector.poll(&[]).0);
            assert!(
                std::time::Instant::now() < deadline,
                "commit metadata did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(completed.len(), expected);
        assert!(
            completed
                .iter()
                .all(|r| r.source == "git/commit" && r.commit.is_some())
        );
        completed.clear();
    }
}
