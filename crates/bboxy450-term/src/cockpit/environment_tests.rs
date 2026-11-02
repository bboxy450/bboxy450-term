// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Disposable subprocesses, no global env mutation.
use super::*;
use std::os::unix::fs::PermissionsExt;
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn detached_backend_start_scrubs_the_callers_session_before_exec() {
    const KEYS: &[&str] = &[
        "CLAUDECODE",
        "CLAUDE_PID",
        "CLAUDE_CODE_CHILD_SESSION",
        "CLAUDE_CODE_MESSAGING_TOKEN",
        "CLAUDE_CODE_SESSION_ATTENDED",
        "CLAUDE_CODE_REMOTE_SESSION_ID",
        "CLAUDE_CODE_BRIDGE_SESSION_ID",
        "CODEX_THREAD_ID",
        "PI_SESSION_ID",
        "OMP_SESSION_FILE",
    ];
    if std::env::var_os("BBOXY_LAUNCH_ENV_CHILD").is_none() {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.env_clear().env("BBOXY_LAUNCH_ENV_CHILD", "1")
            .env("PATH", "/usr/bin:/bin").env("CODEX_HOME", "/private-config")
            .args(["--exact", "cockpit::environment_tests::detached_backend_start_scrubs_the_callers_session_before_exec"]);
        for key in KEYS {
            command.env(key, "caller-marker");
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("environment-spy");
    std::fs::write(&executable, "#!/bin/sh\nexec /usr/bin/env -0\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let config = Config {
        executable,
        run_dir: root.path().join("run"),
        repository: root.path().into(),
        socket: root.path().join("mux.sock"),
        shell_config: Some(root.path().into()),
        ctl: false,
        test_account: None,
    };
    for entry in ["supervisor", "logic", "desktop"] {
        let mut process = child(&config, entry, std::ffi::OsStr::new("/usr/bin:/bin")).unwrap();
        let mut bytes = Vec::new();
        process
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut bytes)
            .await
            .unwrap();
        assert!(process.wait().await.unwrap().success());
        let values: BTreeMap<_, _> = bytes
            .split(|b| *b == 0)
            .filter_map(|entry| std::str::from_utf8(entry).unwrap().split_once('='))
            .collect();
        for key in KEYS {
            assert!(
                !values.contains_key(key),
                "{entry} retained caller key {key}"
            );
            assert_eq!(
                std::env::var(key).unwrap(),
                "caller-marker",
                "parent was mutated"
            );
        }
        assert_eq!(values["CODEX_HOME"], "/private-config");
        assert_eq!(values["PATH"], "/usr/bin:/bin");
        assert_eq!(values["BBOXY_RUN_DIR"], config.run_dir.to_str().unwrap());
        assert_eq!(values["ZDOTDIR"], root.path().to_str().unwrap());
    }
}
