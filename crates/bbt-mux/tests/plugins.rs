#![allow(
    clippy::disallowed_methods,
    reason = "owned conformance fixture IO and Tokio test runtime, never UI IO"
)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn fixture(fault: Option<&str>, provider: &str) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root
        .path()
        .join(format!("bboxy450-term-data_source-{provider}"));
    let source = include_str!("fixtures/plugins/bboxy450-term-data_source-reference");
    let source = if let Some(fault) = fault {
        source.replacen("FAULT = None", &format!("FAULT = {fault:?}"), 1)
    } else {
        source.to_owned()
    };
    fs::write(&path, source).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    (root, path)
}

fn reaped(pids: &[u32]) {
    for pid in pids {
        // Only recorded fixture children; existence checks send no signal.
        let exists = unsafe { libc::kill(*pid as i32, 0) };
        assert_eq!(exists, -1, "fixture child {pid} still exists");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
}

// keeps: spec plugins
#[tokio::test]
async fn reference_uses_one_shape_and_all_eight_methods_then_reaps_its_children() {
    let (_root, path) = fixture(None, "reference");
    let report = bbt_mux::plugins::check(&path).await;
    assert!(report.ok, "{report:?}");
    assert_eq!(report.shape_runs, 1);
    assert_eq!(report.owned_pids.len(), 2);
    assert_eq!(report.sha256.as_ref().unwrap().len(), 64);
    assert!(
        report
            .checks
            .iter()
            .any(|s| s.contains("eight data_source"))
    );
    reaped(&report.owned_pids);
}

// keeps: spec plugins
#[tokio::test]
async fn six_required_refusals_are_named_and_never_leave_a_child() {
    for (fault, provider, reason) in [
        (Some("unknown_contract"), "unknown", "unknown_contract"),
        (Some("major"), "major", "major_mismatch"),
        (Some("stdout"), "stdout", "non_protocol_stdout"),
        (Some("budget"), "budget", "hook_budget_exceeded"),
        (
            None,
            "this-provider-name-is-deliberately-longer-than-the-tool-name-budget",
            "tool_name_too_long",
        ),
        (
            Some("reject_unknown"),
            "reject-unknown",
            "unknown_field_rejected",
        ),
    ] {
        let (_root, path) = fixture(fault, provider);
        let report = bbt_mux::plugins::check(&path).await;
        assert!(!report.ok, "{report:?}");
        assert_eq!(report.refusal.as_ref().unwrap().code, reason, "{report:?}");
        reaped(&report.owned_pids);
    }
}

// keeps: spec plugins
#[tokio::test]
async fn protocol_errors_command_errors_eof_bad_results_and_fixed_deadlines_stay_distinct() {
    for (fault, reason) in [
        ("wrong_id", "request_id_mismatch"),
        ("protocol_error", "jsonrpc_error"),
        ("command_error", "command_error"),
        ("eof", "plugin_eof"),
        ("bad_result", "result_schema_mismatch"),
        ("timeout", "deadline_exceeded"),
        ("huge_stdout", "frame_too_large"),
    ] {
        let (_root, path) = fixture(Some(fault), fault.replace('_', "-").as_str());
        let report = bbt_mux::plugins::check(&path).await;
        assert_eq!(report.refusal.as_ref().unwrap().code, reason, "{report:?}");
        reaped(&report.owned_pids);
    }
}

// keeps: spec plugins
#[tokio::test]
async fn stderr_is_drained_with_a_bound_and_eof_escalation_reaps_a_term_ignoring_child() {
    for fault in ["stderr_flood", "ignore_shutdown"] {
        let (_root, path) = fixture(Some(fault), fault.replace('_', "-").as_str());
        let report = bbt_mux::plugins::check(&path).await;
        assert!(report.ok, "{report:?}");
        if fault == "stderr_flood" {
            assert!(report.stderr_truncated);
        }
        reaped(&report.owned_pids);
    }
}

// keeps: spec plugins
#[tokio::test]
async fn directories_symlinks_and_production_copies_do_not_bootstrap_trust() {
    let (root, path) = fixture(None, "reference");
    let report = bbt_mux::plugins::check(root.path()).await;
    assert_eq!(report.refusal.unwrap().code, "directory_plugin_refused");
    let symlink = root.path().join("bboxy450-term-data_source-link");
    std::os::unix::fs::symlink(&path, &symlink).unwrap();
    assert_eq!(
        bbt_mux::plugins::check(&symlink)
            .await
            .refusal
            .unwrap()
            .code,
        "plugin_symlink_refused"
    );
    let production = root.path().join("plugins/bin");
    fs::create_dir_all(&production).unwrap();
    let copy = production.join(path.file_name().unwrap());
    fs::copy(&path, &copy).unwrap();
    assert_eq!(
        bbt_mux::plugins::check(&copy).await.refusal.unwrap().code,
        "untrusted_production_file"
    );
    assert!(!root.path().join("account").exists());
}

// keeps: spec plugins
#[tokio::test]
async fn task_provider_must_return_every_app_signed_write_byte_for_byte() {
    for (fault, expected) in [
        (None, None),
        (Some("omit_signed_record"), Some("signed_record_missing")),
        (Some("alter_signed_record"), Some("signed_record_changed")),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("bboxy450-term-task_system-reference");
        let source = include_str!("fixtures/plugins/bboxy450-term-data_source-reference").replacen(
            "SYSTEM = \"data_source\"",
            "SYSTEM = \"task_system\"",
            1,
        );
        let source = fault.map_or_else(
            || source.clone(),
            |fault| source.replacen("FAULT = None", &format!("FAULT = {fault:?}"), 1),
        );
        fs::write(&path, source).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let report = bbt_mux::plugins::check(&path).await;
        assert_eq!(
            report.refusal.as_ref().map(|e| e.code.as_str()),
            expected,
            "{report:?}"
        );
        if expected.is_none() {
            assert!(
                report
                    .checks
                    .iter()
                    .any(|s| s.contains("DSSE fixture bytes"))
            );
        }
        reaped(&report.owned_pids);
    }
}

// keeps: spec plugins
#[tokio::test]
async fn the_other_five_contracts_are_probed_and_cross_field_results_are_checked() {
    for (system, fault, expected) in [
        ("session", None, None),
        ("provider", None, None),
        ("repo_graph", None, None),
        ("decision_model", None, None),
        ("event_sink", None, None),
        (
            "decision_model",
            Some("unknown_choice"),
            Some("decision_choice_unknown"),
        ),
        (
            "decision_model",
            Some("unknown_probability"),
            Some("decision_probability_unknown"),
        ),
        (
            "event_sink",
            Some("alter_envelope_id"),
            Some("envelope_id_changed"),
        ),
        (
            "provider",
            Some("bad_result"),
            Some("result_schema_mismatch"),
        ),
        (
            "session",
            Some("reject_unknown"),
            Some("unknown_field_rejected"),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .join(format!("bboxy450-term-{system}-reference"));
        let source = include_str!("fixtures/plugins/bboxy450-term-data_source-reference").replacen(
            "SYSTEM = \"data_source\"",
            &format!("SYSTEM = {system:?}"),
            1,
        );
        let source = fault.map_or_else(
            || source.clone(),
            |fault| source.replacen("FAULT = None", &format!("FAULT = {fault:?}"), 1),
        );
        fs::write(&path, source).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let report = bbt_mux::plugins::check(&path).await;
        assert_eq!(
            report.refusal.as_ref().map(|r| r.code.as_str()),
            expected,
            "{system}: {report:?}"
        );
        reaped(&report.owned_pids);
    }
}
