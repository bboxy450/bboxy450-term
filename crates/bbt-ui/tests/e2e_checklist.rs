#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]

use bbt_ui::modules::e2e_checklist::{Checklist, Link, Promise, Record, Status, Task, Tone};

// keeps: spec e2e-acceptance §1
#[test]
fn the_real_spec_is_the_only_canonical_promise_list() {
    let promises = bbt_ui::modules::e2e_checklist::parse_promises(include_str!(
        "../../../.bboxy/specs/e2e-acceptance.md"
    ))
    .unwrap();
    assert_eq!(promises.len(), 56);
    assert_eq!(
        (promises[0].id.as_str(), promises[0].group.as_str()),
        ("T1", "terminal")
    );
    assert_eq!(
        (promises[54].id.as_str(), promises[54].group.as_str()),
        ("P2", "performance")
    );
    assert!(promises[0].text.contains("real PTY"));
    let generated = include_str!("../../../lab/web/e2e.js")
        .split_once("window.E2E_PROMISES = ")
        .unwrap()
        .1
        .trim()
        .strip_suffix(';')
        .unwrap();
    assert_eq!(
        promises,
        serde_json::from_str::<Vec<Promise>>(generated).unwrap()
    );
    let view = Checklist {
        promises,
        ..Default::default()
    };
    let out = view.project();
    assert_eq!(out.summary.untested, 56);
    assert_eq!(out.summary.tone, Tone::Red);
    assert!(out.rows.iter().all(|r| r.record.is_none()));
}

// keeps: spec e2e-acceptance §1
#[test]
fn missing_malformed_or_inconsistent_specs_fail_instead_of_empty_success() {
    use bbt_ui::modules::e2e_checklist::parse_promises;
    for source in [
        "no checklist here",
        "## 1. The checklist\n0 promises.\n## 2. Record",
        "## 1. The checklist\n1 promises.\n- **T1** no group",
        "## 1. The checklist\n2 promises.\n### terminal\n- **T1** one row",
        "## 1. The checklist\n2 promises.\n### terminal\n- **T1** one\n- **T1** duplicate",
        "## 1. The checklist\n### terminal\n- **T1** missing count",
    ] {
        assert!(parse_promises(source).is_err(), "{source}");
    }
}

fn checklist(status: &str) -> Checklist {
    Checklist {
        promises: vec![Promise {
            id: "T1".into(),
            group: "terminal".into(),
            text: "opens a real PTY".into(),
        }],
        records: vec![Record {
            function: "T1".into(),
            status: status.into(),
            last_run: Some("2026-10-24T03:12:00Z".into()),
            run_by: Some("seat:worker".into()),
            coverage: Some("full".into()),
            ..Default::default()
        }],
        tasks: vec![],
        faults: vec![],
    }
}

fn tracking(view: &mut Checklist, status: &str) {
    view.records[0].links.push(Link {
        to: "[[#312]]".into(),
        rel: "tracked-by".into(),
    });
    view.tasks.push(Task {
        reference: "#312".into(),
        status: status.into(),
    });
}

// keeps: spec e2e-acceptance §1
#[test]
fn green_keeps_manual_rows_and_run_metadata_visible() {
    let mut view = checklist("pass");
    view.promises.push(Promise {
        id: "I1".into(),
        group: "identity".into(),
        text: "operator judges the prompt".into(),
    });
    view.records.push(Record {
        function: "I1".into(),
        status: "manual".into(),
        run_by: Some("operator".into()),
        reason: Some("the Oracle's eye".into()),
        ..Default::default()
    });
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Green);
    assert_eq!((out.summary.passed, out.summary.manual), (1, 1));
    assert_eq!(out.rows[1].status, Status::Manual);
    assert_eq!(out.rows[1].record.unwrap().last_run, None);
    assert_eq!(
        out.rows[0].record.unwrap().last_run.as_deref(),
        Some("2026-10-24T03:12:00Z")
    );
    assert_eq!(
        out.rows[0].record.unwrap().run_by.as_deref(),
        Some("seat:worker")
    );
}

// keeps: spec e2e-acceptance §1
#[test]
fn missing_and_duplicate_records_name_the_gap_without_picking_metadata() {
    let mut view = checklist("pass");
    view.records.clear();
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Red);
    assert_eq!(out.rows[0].status, Status::Untested);
    assert_eq!(out.rows[0].fault.as_deref(), Some("no record"));
    assert!(out.rows[0].record.is_none());

    view.records = vec![
        checklist("pass").records.remove(0),
        checklist("fail").records.remove(0),
    ];
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Red);
    assert_eq!(out.rows[0].status, Status::Untested);
    assert_eq!(out.rows[0].fault.as_deref(), Some("doubled · 2 records"));
    assert!(out.rows[0].record.is_none());
}

// keeps: spec e2e-acceptance §1
#[test]
fn tracked_failures_are_amber_only_for_the_specified_open_task_states() {
    for state in ["open", "claimed", "running", "reported", "blocked"] {
        let mut view = checklist("fail");
        tracking(&mut view, state);
        let out = view.project();
        assert_eq!(out.summary.tone, Tone::Amber, "{state}");
        assert_eq!(out.summary.failures, 1);
        assert_eq!(out.summary.untracked, 0);
        assert_eq!(out.rows[0].status, Status::Fail);
        assert_eq!(out.rows[0].tracked_by, ["#312"]);
    }
    for state in ["accepted", "rejected", "done", "cancelled", "unknown", ""] {
        let mut view = checklist("fail");
        tracking(&mut view, state);
        let out = view.project();
        assert_eq!(out.summary.tone, Tone::Red, "{state}");
        assert_eq!(out.summary.untracked, 1);
    }
}

// keeps: spec e2e-acceptance §1
#[test]
fn blocked_is_a_failure_and_preserves_its_reason() {
    let mut view = checklist("blocked");
    view.records[0].reason = Some("waiting for a supported tool".into());
    assert_eq!(view.project().summary.tone, Tone::Red);
    tracking(&mut view, "running");
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Amber);
    assert_eq!(out.summary.failures, 1);
    assert_eq!(out.rows[0].status, Status::Blocked);
    assert_eq!(
        out.rows[0].record.unwrap().reason.as_deref(),
        Some("waiting for a supported tool")
    );
}

// keeps: spec e2e-acceptance §1
#[test]
fn only_a_resolved_tracked_by_task_counts_as_tracking() {
    let mut view = checklist("fail");
    tracking(&mut view, "open");
    view.records[0].links[0].rel = "evidence".into();
    assert_eq!(view.project().summary.tone, Tone::Red);
    view.records[0].links[0].rel = "tracked-by".into();
    view.tasks.clear();
    assert_eq!(view.project().summary.tone, Tone::Red);
    view.tasks = vec![
        Task {
            reference: "#312".into(),
            status: "open".into()
        };
        2
    ];
    assert_eq!(view.project().summary.tone, Tone::Red);
}

// keeps: spec e2e-acceptance §1
#[test]
fn tracking_lists_each_resolved_open_task_once() {
    let mut view = checklist("fail");
    tracking(&mut view, "open");
    let duplicate = view.records[0].links[0].clone();
    view.records[0].links.push(duplicate);
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Amber);
    assert_eq!(out.rows[0].tracked_by, ["#312"]);
}

// keeps: spec e2e-acceptance §1
#[test]
fn tracked_by_wikilink_aliases_preserve_task_identity() {
    for target in ["[[#312|fix]]", r"[[#312\|fix]]"] {
        let mut view = checklist("fail");
        tracking(&mut view, "open");
        view.records[0].links[0].to = target.into();
        assert_eq!(view.project().summary.tone, Tone::Amber);
    }
}

// keeps: spec e2e-acceptance §1
#[test]
fn untested_invalid_or_unknown_input_never_makes_a_green_summary() {
    let view = checklist("untested");
    assert_eq!(view.project().summary.tone, Tone::Red);
    let view = checklist("PASSED");
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Red);
    assert_eq!(
        out.rows[0].fault.as_deref(),
        Some("invalid status · PASSED")
    );
    let mut view = checklist("pass");
    view.records.push(Record {
        function: "unknown".into(),
        status: "pass".into(),
        ..Default::default()
    });
    assert_eq!(view.project().summary.faults, ["unknown function: unknown"]);
    assert_eq!(view.project().summary.tone, Tone::Red);
    let view = Checklist::default();
    assert_eq!(view.project().summary.tone, Tone::Red);
}

// keeps: spec e2e-acceptance §1
#[test]
fn canonical_order_is_preserved_and_duplicate_promises_are_visible() {
    let mut view = checklist("pass");
    view.promises.push(Promise {
        id: "D1".into(),
        group: "documents".into(),
        text: "a document".into(),
    });
    view.records.insert(
        0,
        Record {
            function: "D1".into(),
            status: "pass".into(),
            ..Default::default()
        },
    );
    let out = view.project();
    assert_eq!(
        out.rows
            .iter()
            .map(|r| r.promise.id.as_str())
            .collect::<Vec<_>>(),
        ["T1", "D1"]
    );
    view.promises.push(view.promises[0].clone());
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Red);
    assert_eq!(out.rows[0].fault.as_deref(), Some("duplicate promise T1"));
}

/// Decision #316: a pass is green only when it covers its whole promise. A pass without
/// `coverage: full` is partial (`unstated` when it names no gap): amber while an open task
/// tracks it, red when none does.
// keeps: spec e2e-acceptance §1
#[test]
fn a_partial_pass_is_amber_when_tracked_and_red_when_not() {
    let mut view = checklist("pass");
    view.records[0].coverage = Some("partial".into());
    view.records[0].missing = vec!["the one-frame bound".into()];
    let out = view.project();
    assert_eq!(out.rows[0].partial, vec!["the one-frame bound".to_owned()]);
    assert_eq!((out.summary.partial, out.summary.partial_untracked), (1, 1));
    assert_eq!(out.summary.tone, Tone::Red);

    tracking(&mut view, "open");
    let out = view.project();
    assert_eq!(out.summary.tone, Tone::Amber);
    assert_eq!(out.rows[0].tracked_by, vec!["#312".to_owned()]);

    let mut unstated = checklist("pass");
    unstated.records[0].coverage = None;
    let out = unstated.project();
    assert_eq!(out.rows[0].partial, vec!["unstated".to_owned()]);
    assert_eq!(
        out.summary.tone,
        Tone::Red,
        "a pass that does not say full is partial"
    );
}
