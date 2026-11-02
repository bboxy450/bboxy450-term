// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Copied from bboxy450-log-converter: https://github.com/bboxy450/bboxy450-log-converter/blob/bb87743bea428d95832ca0bf5a28bd8d3c08df8f/tests/cli.rs
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

#![allow(
    clippy::disallowed_methods,
    reason = "unattended CLI fixtures, never a UI turn"
)]
use std::process::Command;
#[test]
fn help_version_and_skeleton_status_are_honest() {
    let cli = env!("CARGO_BIN_EXE_bboxy450-log-converter");
    let help = Command::new(cli).arg("--help").output().unwrap();
    assert!(help.status.success());
    let text = String::from_utf8(help.stdout).unwrap();
    assert!(text.contains("bboxy450-harness"));
    assert!(!text.contains("Bboxy"));
    for command in [
        "parse",
        "lower",
        "convert",
        "sanitize",
        "cat",
        "watch",
        "switch-prepare",
        "export",
    ] {
        assert!(text.contains(command));
        assert_eq!(
            Command::new(cli)
                .arg(command)
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
    }
    assert!(
        Command::new(cli)
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(
        Command::new(cli)
            .arg("bogus")
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn provider_cli_aliases_keep_the_internal_tag() {
    use bboxy450_log_converter::ir::Provider;
    use std::str::FromStr;
    assert_eq!(Provider::from_str("bboxy").unwrap(), Provider::Bboxy);
    assert_eq!(
        Provider::from_str("bboxy450-harness").unwrap(),
        Provider::Bboxy
    );
    assert_eq!(Provider::Bboxy.as_str(), "bboxy");
    assert!(Provider::from_str("bogus").is_err());
}
