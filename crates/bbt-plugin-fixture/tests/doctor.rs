// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Red/green controls for the SDK's plugin doctor. The clean doctor fixture
//! and the good fixtures pass; each fault variant breaks one host rule and
//! the doctor must flag exactly that check id as an error.
#![allow(
    clippy::disallowed_methods,
    reason = "private fixture builds and plugin processes"
)]

use std::path::{Path, PathBuf};

use bbt_plugin_fixture::{Kind, Knobs, doctor};
use bbt_plugin_sdk::doctor::{CHECKS, Options, Report, Severity, run};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// The doctor fixture installed under the name that selects `fault`.
fn installed(dir: &Path, fault: Option<&str>) -> PathBuf {
    Knobs::of(Kind::Doctor).install(&dir.join(doctor::installed(fault)))
}

fn options() -> Options {
    Options {
        children: vec![doctor::CHILD.into()],
        ..Options::default()
    }
}

fn shown(report: &Report) -> String {
    format!("{}\n{}", report.human(), report.json())
}

// keeps: plugin doctor green control — the clean fixture passes every check
// that can run on a bare binary, including writes and observed children.
#[test]
fn the_clean_doctor_fixture_has_no_gaps() {
    let dir = scratch();
    let report = run(&installed(dir.path(), None), &options());
    assert!(report.gaps.is_empty(), "{}", shown(&report));
    for (id, _) in CHECKS {
        if id.starts_with("models.") {
            continue;
        }
        assert!(
            report.passed.contains(id),
            "{id} did not run:\n{}",
            shown(&report)
        );
    }
}

// keeps: plugin doctor red controls — one fault per check, each flagged by
// its own id as an error.
#[test]
fn every_fault_is_flagged_by_its_own_check() {
    let dir = scratch();
    let mut missed = Vec::new();
    for fault in doctor::FAULTS {
        let report = run(&installed(dir.path(), Some(fault)), &options());
        let flagged = report
            .of(fault)
            .iter()
            .any(|gap| gap.severity == Severity::Error);
        eprintln!(
            "{fault}: {:?}",
            report.of(fault).first().map(|g| &g.evidence)
        );
        if !flagged {
            missed.push(format!("{fault}:\n{}", report.human()));
        }
    }
    assert!(missed.is_empty(), "not flagged:\n{}", missed.join("\n"));
}

#[test]
fn a_misnamed_binary_is_flagged() {
    let dir = scratch();
    let path = Knobs::of(Kind::Doctor).install(&dir.path().join("bboxy450-term-nosuch-doctor"));
    let report = run(&path, &Options::default());
    assert!(!report.of("shape.name").is_empty(), "{}", shown(&report));
}

fn models(dir: &Path) -> Options {
    Options {
        models: Some(dir.to_owned()),
        ..Options::default()
    }
}

#[test]
fn missing_and_failing_models_are_flagged() {
    let dir = scratch();
    let binary = installed(dir.path(), None);
    let empty = dir.path().join("empty-models");
    std::fs::create_dir_all(&empty).unwrap();
    let report = run(&binary, &models(&empty));
    assert!(
        !report.of("models.present").is_empty(),
        "{}",
        shown(&report)
    );

    // The SDK's own shipped models pass natively (and in Java TLC when set).
    let shipped = workspace().join("crates/bbt-plugin-sdk/models");
    let report = run(&binary, &models(&shipped));
    assert!(
        report.passed.contains(&"models.native"),
        "{}",
        shown(&report)
    );

    // A model whose invariant fails, and a hand-edited one.
    let broken = dir.path().join("broken-models");
    std::fs::create_dir_all(&broken).unwrap();
    for entry in std::fs::read_dir(&shipped).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), broken.join(entry.file_name())).unwrap();
    }
    let tla = broken.join("PluginRuntime.tla");
    let text = std::fs::read_to_string(&tla).unwrap();
    std::fs::write(
        &tla,
        text.replace("Inv0 == (v7 <= v0)", "Inv0 == (v7 < v0)"),
    )
    .unwrap();
    let report = run(&binary, &models(&broken));
    assert!(
        report
            .of("models.native")
            .iter()
            .any(|g| g.evidence.contains("expected pass")),
        "{}",
        shown(&report)
    );
    let java = Options::default();
    if java.java.is_some() && java.tlc_jar.is_some() {
        assert!(
            report
                .of("models.java")
                .iter()
                .any(|g| g.evidence.contains("safety")),
            "{}",
            shown(&report)
        );
    } else {
        eprintln!(
            "models.java: BBOXY450_JAVA/BBOXY450_TLC_JAR unset; the Java red control did not run"
        );
    }
    std::fs::write(&tla, text.replace("(v7 <= v0)", "v7 <= v0")).unwrap();
    let report = run(&binary, &models(&broken));
    assert!(
        report
            .of("models.native")
            .iter()
            .any(|g| g.evidence.contains("native checker's subset")),
        "{}",
        shown(&report)
    );
}

// keeps: plugin doctor — the good host fixture (a real Markdown data_source,
// plain and with the Harmony signed-writer knobs) passes with no error. The
// reference fixture is a canned P-0 case server (fixed records, repeating
// opaque cursors), so it is not a green control here.
#[test]
fn the_good_root_fixtures_pass() {
    let dir = scratch();
    for (name, knobs) in [
        ("bboxy450-term-data_source-root", Knobs::root()),
        (
            "bboxy450-term-data_source-harmony",
            Knobs {
                harmony: true,
                ..Knobs::root()
            },
        ),
    ] {
        let report = run(&knobs.install(&dir.path().join(name)), &Options::default());
        eprintln!("{name}:\n{}", report.human());
        assert!(report.is_clean(), "{name}:\n{}", shown(&report));
    }
}
