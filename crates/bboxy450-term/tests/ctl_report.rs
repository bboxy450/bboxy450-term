// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods, reason = "private CLI fixture IO")]

#[test]
fn report_hints_work_offline_without_reading_an_endpoint_or_environment() {
    let private = tempfile::tempdir().unwrap();
    let endpoint = private.path().join("endpoint.json");
    std::fs::write(&endpoint, "not an endpoint").unwrap();
    for human in [false, true] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_bboxy450-term"));
        command
            .env_clear()
            .env("HOME", private.path())
            .env("BBOXY_RUN_DIR", private.path())
            .env("BBOXY_CAPABILITY", "private-report-test-sentinel")
            .args([
                "ctl",
                "debug",
                "report",
                "--op",
                "00000000000000000000000001",
            ])
            .arg("--endpoint")
            .arg(&endpoint);
        if human {
            command.arg("--human");
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("private-report-test-sentinel"));
        let reply: serde_json::Value = serde_json::from_str(&text).unwrap();
        let data = if human {
            &reply
        } else {
            assert_eq!(reply["ok"], true);
            assert_eq!(reply["op"], "00000000000000000000000001");
            assert_eq!(reply["noun"], "debug");
            assert_eq!(reply["verb"], "report");
            &reply["data"]
        };
        assert_eq!(data["repository"], "bboxy450/bboxy450-term");
        assert!(data["hint"].as_str().is_some_and(|hint| !hint.is_empty()));
    }
    assert_eq!(
        std::fs::read_to_string(endpoint).unwrap(),
        "not an endpoint"
    );
    assert_eq!(std::fs::read_dir(private.path()).unwrap().count(), 1);
}
