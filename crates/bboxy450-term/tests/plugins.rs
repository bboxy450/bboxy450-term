#![allow(
    clippy::disallowed_methods,
    reason = "owned headless conformance fixture IO, never the UI thread"
)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn real_binary_runs_headless_against_a_fixture_without_opening_the_production_root() {
    let root = tempfile::tempdir().unwrap();
    let binary = root.path().join("bboxy450-term-data_source-reference");
    fs::write(
        &binary,
        include_str!("../../bbt-mux/tests/fixtures/plugins/bboxy450-term-data_source-reference"),
    )
    .unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    let protected = root.path().join("protected-run-root");
    fs::create_dir(&protected).unwrap();
    fs::write(
        protected.join("sentinel"),
        b"production root must stay unchanged",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["plugin", "check"])
        .arg(&binary)
        .env("BBOXY_RUN_DIR", &protected)
        .env("BBOXY_CAPABILITY", "fixture-secret-not-forwarded")
        .env("ICQ_PLUGIN_FIXTURE", "fixture-secret-not-forwarded")
        .env("LC_API_KEY", "fixture-secret-not-forwarded")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["scope"], "isolated-conformance-fixture");
    assert_eq!(report["shape_runs"], 1);
    assert_eq!(fs::read_dir(&protected).unwrap().count(), 1);
    assert_eq!(
        fs::read(protected.join("sentinel")).unwrap(),
        b"production root must stay unchanged"
    );
    let help = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["plugin", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("plugin check <binary>"));
    let production = protected.join("plugins");
    fs::create_dir(&production).unwrap();
    let candidate = production.join(binary.file_name().unwrap());
    fs::copy(&binary, &candidate).unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_bboxy450-term"))
        .args(["plugin", "check"])
        .arg(&candidate)
        .env("BBOXY_RUN_DIR", &protected)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    let refusal: serde_json::Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(refusal["refusal"]["code"], "untrusted_production_file");
    assert!(!protected.join("account").exists());
}
