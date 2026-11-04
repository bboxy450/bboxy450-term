// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "bounded GUI startup/lifecycle IO, before the UI loop exists"
)]
//! Replace only the GUI. No operator ctl door, capability file, or backend stop.
use super::*;
use std::{io::Read, process::Command as Process, time::Instant};
const TIMEOUT: Duration = Duration::from_secs(35);
fn lock_owner(run: &Path) -> Result<u32> {
    let mut child = Process::new("/usr/sbin/lsof")
        .args(["-t", "--"])
        .arg(run.join("app.owner.lock"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let started = Instant::now();
    while child.try_wait()?.is_none() {
        if started.elapsed() >= Duration::from_secs(3) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(invalid("GUI lock observation timed out"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut bytes = Vec::new();
    child
        .stdout
        .take()
        .ok_or_else(|| invalid("GUI lock observation unavailable"))?
        .take(4096)
        .read_to_end(&mut bytes)?;
    let mut pids: Vec<u32> = String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|s| s.parse().ok())
        .filter(|pid| *pid != std::process::id())
        .collect();
    pids.sort_unstable();
    pids.dedup();
    if pids.len() != 1 {
        return Err(invalid("GUI replacement requires one exact lock owner"));
    }
    Ok(pids[0])
}
fn current(run: &Path) -> Result<Option<owners::ProcessIdentity>> {
    if super::frontend_running(run)? != Some(true) {
        return Ok(None);
    }
    // Identify the holder of the GUI-only lease, not an agent/backend PID from
    // an arbitrary file. This also supports legacy GUIs without the receipt.
    let pid = lock_owner(run)?;
    let path = run.join("gui.owner.json");
    match owners::public_gui_file(&path) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            file.by_ref().take(4096).read_to_end(&mut bytes)?;
            let value: serde_json::Value = serde_json::from_slice(&bytes)?;
            let identity: owners::ProcessIdentity =
                serde_json::from_value(value["process"].clone())?;
            // A retained receipt can precede a legacy GUI which does not write
            // one. Only the actual lease holder and its kernel birth authorize
            // replacement; the receipt is evidence, never signal authority.
            Ok(Some(if identity.pid() == pid && identity.alive() {
                identity
            } else {
                owners::ProcessIdentity::read(pid)?
            }))
        }
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
        {
            Ok(Some(owners::ProcessIdentity::read(pid)?))
        }
        Err(error) => Err(error),
    }
}
fn run_dir(args: &[String]) -> Result<PathBuf> {
    args.windows(2)
        .find(|p| p[0] == "--run-dir")
        .map(|p| Ok(PathBuf::from(&p[1])))
        .unwrap_or_else(|| {
            Ok(bbt_mux::transport::default_socket_path()?
                .parent()
                .ok_or_else(|| invalid("run directory unavailable"))?
                .to_owned())
        })
}
/// Explicit mode plus an optional captured public owner fence. Unknown variants
/// refuse rather than silently entering ordinary backend recovery.
pub(super) fn preservation(args: &[String]) -> Result<(bool, Option<BackendIdentity>)> {
    let mut preserve = false;
    let mut expected = None;
    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--preserve-backend" if !preserve => preserve = true,
            "--preserve-backend-owners" if expected.is_none() => {
                let value = args.get(i + 1).filter(|v| v.len() <= 4096).ok_or_else(|| {
                    invalid("preserve_backend: missing bounded public owner fence")
                })?;
                expected = Some(serde_json::from_str(value)?);
            }
            "--preserve-backend-check" => (),
            value if value.starts_with("--preserve-backend") => {
                return Err(invalid("invalid preserve-backend option"));
            }
            _ => (),
        }
    }
    if expected.is_some() && !preserve {
        return Err(invalid("backend owner fence requires --preserve-backend"));
    }
    Ok((preserve, expected))
}
fn backend_check(args: &[String]) -> Result<BackendIdentity> {
    let config = Config::discover(Some(run_dir(args)?))?;
    bbt_rt::handle().block_on(preserve_backend_preflight(&config))
}

/// Allowlist public compatibility facts. Never serialize LogicReady: it contains
/// the endpoint capability, identity notes and private paths.
pub(super) fn incompatibility(ready: &LogicReady, path: &std::ffi::OsStr) -> String {
    let mut mismatches = serde_json::Map::new();
    for (name, expected, observed) in [
        (
            "mcp_control_version",
            MCP_CONTROL_VERSION,
            ready.mcp_control_version,
        ),
        (
            "gui_state_version",
            bbt_proto::gui::WINDOW_STATE_VERSION,
            ready.gui_state_version,
        ),
        (
            "swarm_metadata_version",
            bbt_proto::gui::SWARM_METADATA_VERSION,
            ready.swarm_metadata_version,
        ),
        (
            "ui_control_version",
            bbt_proto::gui::UI_CONTROL_VERSION,
            ready.ui_control_version,
        ),
        (
            "swarm_workspace_version",
            bbt_proto::gui::SWARM_WORKSPACE_VERSION,
            ready.swarm_workspace_version,
        ),
    ] {
        if expected != observed {
            mismatches.insert(
                name.into(),
                serde_json::json!({"expected":expected,"observed":observed}),
            );
        }
    }
    let expected = shell_path::fingerprint(path);
    if ready.launch_path_fingerprint.as_deref() != Some(&expected) {
        let observed = match ready.launch_path_fingerprint.as_deref() {
            None => "missing",
            Some(value) if value.len() == 16 && value.bytes().all(|b| b.is_ascii_hexdigit()) => {
                value
            }
            Some(_) => "invalid_fingerprint",
        };
        mismatches.insert(
            "launch_path_fingerprint".into(),
            serde_json::json!({"expected":expected,"observed":observed}),
        );
    }
    format!(
        "preserve_backend: incompatible Logic schema or launch PATH; refresh forbidden; mismatches={}",
        serde_json::Value::Object(mismatches)
    )
}

/// Credential-free process/lock observation for the canonical installer.
pub fn probe(args: &[String]) -> Result<Option<serde_json::Value>> {
    if !args
        .iter()
        .any(|s| s == "--gui-handoff-probe" || s == "--preserve-backend-check")
    {
        return Ok(None);
    }
    let (_, expected) = preservation(args)?;
    let backend = if args.iter().any(|s| s == "--preserve-backend-check") {
        let identity = backend_check(args)?;
        if expected
            .as_ref()
            .is_some_and(|expected| *expected != identity)
        {
            return Err(invalid("preserve_backend: owner changed after preflight"));
        }
        Some(identity)
    } else {
        None
    };
    let run = run_dir(args)?;
    let owner = current(&run)?;
    let gui = if owner.is_some() {
        owners::public_gui_file(&run.join("gui.owner.json"))
            .ok()
            .and_then(|mut f| {
                let mut b = Vec::new();
                f.by_ref().take(4096).read_to_end(&mut b).ok()?;
                serde_json::from_slice::<serde_json::Value>(&b)
                    .ok()
                    .filter(|v| {
                        serde_json::from_value::<owners::ProcessIdentity>(v["process"].clone())
                            .ok()
                            .as_ref()
                            == owner.as_ref()
                    })
                    .map(|v| serde_json::json!({"gui":v["gui"],"repository":v["repository"]}))
            })
    } else {
        None
    };
    Ok(Some(
        serde_json::json!({"preserve_backend_supported":1,"backend":backend,"running":owner.is_some(),"process":owner,"gui":gui.as_ref().map(|v|&v["gui"]),"repository":gui.as_ref().map(|v|&v["repository"])}),
    ))
}
/// Explicit normal GUI startup action, before claiming its lock. No ctl/capability door.
/// The installer passes its captured public owner identity to fence concurrent replacement.
pub fn prepare(args: &[String]) -> Result<()> {
    let (preserve, expected) = preservation(args)?;
    let injected_failure = args.iter().any(|s| s == "--verify-ui-only-after-detach");
    if injected_failure
        && (!args.iter().any(|s| s == "--unattended")
            || Config::test_account_from_args(args)?.is_none())
    {
        return Err(invalid(
            "UI-only failure fixture requires a private unattended account",
        ));
    }
    if preserve {
        let identity = backend_check(args)?;
        if expected
            .as_ref()
            .is_some_and(|expected| *expected != identity)
        {
            return Err(invalid(
                "preserve_backend: owners changed before GUI detach",
            ));
        }
    }
    if !args.iter().any(|s| s == "--replace-gui") {
        return Ok(());
    }
    let root = run_dir(args)?;
    let owner = current(&root)?;
    if let Some(expected) = args.windows(2).find(|p| p[0] == "--replace-gui-owner") {
        let expected: Option<owners::ProcessIdentity> = serde_json::from_str(&expected[1])?;
        if owner != expected {
            return Err(invalid(
                "GUI owner changed after install preflight; replacement refused",
            ));
        }
    }
    let Some(owner) = owner else {
        return Ok(());
    };
    // SIGHUP closes ONLY the exact GUI lock holder. Current GUIs use CloseWindow
    // and save/detach; legacy default HUP exits only this PID. Never a process group.
    if !owner.alive() {
        return Ok(());
    }
    owner.signal(libc::SIGHUP)?;
    let started = Instant::now();
    while owner.alive() || super::frontend_running(&root)? == Some(true) {
        if started.elapsed() >= TIMEOUT {
            return Err(invalid("GUI detach did not complete; no force termination"));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if injected_failure {
        return Err(invalid("private UI-only failure after GUI detach"));
    }
    Ok(())
}

#[cfg(test)]
mod ui_status_tests {
    use super::*;
    use std::{
        ffi::OsStr,
        os::unix::{
            fs::{OpenOptionsExt, PermissionsExt},
            io::AsRawFd,
        },
    };

    fn ready() -> LogicReady {
        serde_json::from_value(serde_json::json!({
            "root":"/private/secret-endpoint", "capability":vec![197;32],
            "identity":{"state":"held","reason":"private-token-value"},
            "custody_backend":"private-custody", "account_root":"secret-account",
            "mcp_control_version":MCP_CONTROL_VERSION,
            "gui_state_version":bbt_proto::gui::WINDOW_STATE_VERSION,
            "swarm_metadata_version":bbt_proto::gui::SWARM_METADATA_VERSION,
            "ui_control_version":bbt_proto::gui::UI_CONTROL_VERSION,
            "swarm_workspace_version":bbt_proto::gui::SWARM_WORKSPACE_VERSION,
            "launch_path_fingerprint":shell_path::fingerprint(OsStr::new("/fixture/bin")),
        }))
        .unwrap()
    }

    #[test]
    fn ui_status_diagnostic_separates_every_version_and_path_without_private_fields() {
        let good = ready();
        let path = OsStr::new("/fixture/bin");
        assert!(logic_compatible(&good, path));
        assert!(incompatibility(&good, path).ends_with("mismatches={}"));
        for field in [
            "mcp_control_version",
            "gui_state_version",
            "swarm_metadata_version",
            "ui_control_version",
            "swarm_workspace_version",
        ] {
            let mut value = serde_json::to_value(&good).unwrap();
            value[field] = serde_json::json!(65535);
            let changed: LogicReady = serde_json::from_value(value).unwrap();
            assert!(!logic_compatible(&changed, path));
            let text = incompatibility(&changed, path);
            let facts: serde_json::Value =
                serde_json::from_str(text.split_once("mismatches=").unwrap().1).unwrap();
            assert_eq!(facts.as_object().unwrap().len(), 1);
            assert_eq!(facts[field]["observed"], 65535);
            assert!(facts[field]["expected"].is_u64());
            assert!(!text.contains("launch_path_fingerprint"));
        }
        for (observed, expected) in [
            (None, "missing"),
            (Some("token=private-token-value"), "invalid_fingerprint"),
            (Some("0123456789abcdef"), "0123456789abcdef"),
        ] {
            let mut changed = good.clone();
            changed.launch_path_fingerprint = observed.map(str::to_owned);
            assert!(!logic_compatible(&changed, path));
            let text = incompatibility(&changed, path);
            let facts: serde_json::Value =
                serde_json::from_str(text.split_once("mismatches=").unwrap().1).unwrap();
            assert_eq!(facts.as_object().unwrap().len(), 1);
            assert_eq!(facts["launch_path_fingerprint"]["observed"], expected);
            assert_eq!(
                facts["launch_path_fingerprint"]["expected"],
                shell_path::fingerprint(path)
            );
            for secret in [
                "private-token-value",
                "secret-endpoint",
                "private-custody",
                "secret-account",
                "capability",
                "[197",
            ] {
                assert!(!text.contains(secret), "private field in diagnostic");
            }
        }
    }

    #[tokio::test]
    async fn ui_status_readonly_preflight_refuses_cold_and_incompatible_private_owners() {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = Config {
            executable: root.path().join("must-not-execute"),
            socket: root.path().join("mux.sock"),
            run_dir: root.path().into(),
            repository: root.path().into(),
            shell_config: Some(root.path().into()),
            ctl: false,
            test_account: Some(root.path().into()),
        };
        let cold = preserve_backend_preflight(&config)
            .await
            .unwrap_err()
            .to_string();
        assert!(cold.contains("cold startup refused"));
        assert_eq!(
            std::fs::read_dir(root.path()).unwrap().count(),
            0,
            "cold probe wrote files"
        );
        let lock = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(root.path().join("mux.sock.lock"))
            .unwrap();
        // SAFETY: private fixture owns this descriptor and releases it on drop.
        assert_eq!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        owners::write(
            &config,
            "supervisor",
            SupervisorReady {
                socket: config.socket.clone(),
                capability: [197; 32],
                pid: std::process::id(),
            },
        )
        .unwrap();
        let mut incompatible = ready();
        incompatible.ui_control_version = 65535;
        owners::write(&config, "logic", incompatible).unwrap();
        let before = ["logic.owner.json", "supervisor.owner.json"]
            .map(|name| std::fs::read(root.path().join(name)).unwrap());
        let refusal = preserve_backend_preflight(&config)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            refusal.contains("ui_control_version")
                && refusal.contains("65535")
                && refusal.contains("refresh forbidden")
        );
        for secret in [
            "private-token-value",
            "secret-endpoint",
            "private-custody",
            "secret-account",
            "capability",
        ] {
            assert!(!refusal.contains(secret));
        }
        assert_eq!(
            before,
            ["logic.owner.json", "supervisor.owner.json"]
                .map(|name| std::fs::read(root.path().join(name)).unwrap())
        );
        assert!(!config.socket.exists(), "probe started a backend");
        assert!(
            owners::supervisor_held(&config).unwrap(),
            "probe disturbed owner lock"
        );
    }
}
