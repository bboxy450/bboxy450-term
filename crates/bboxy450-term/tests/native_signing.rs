// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "signing recipes run against throwaway fake tools, never a Keychain"
)]
use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

const IDENTITY: &str = "3943AF130A015B36824B7FBC86AA672E7B25F3C4";
const REQUIREMENT: &str = "identifier \"com.bboxy450.term\" and anchor apple generic and certificate leaf[subject.OU] = \"UQB3368A84\"";
fn executable(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

// Fixture Make commands own their build context; outer test/gate overrides
// cannot choose their target, profile, makefiles or bundle destination.
fn fixture_command(program: &str) -> Command {
    let mut command = Command::new(program);
    for var in [
        "MAKEFLAGS",
        "MFLAGS",
        "MAKELEVEL",
        "MAKEOVERRIDES",
        "MAKEFILES",
        "GNUMAKEFLAGS",
        "TARGET_DIR",
        "PROFILE",
        "BUNDLE",
        "INSTALL_DIR",
        "UNSIGNED_INSTALL_DIR",
        "DIST_DIR",
        "CARGO_TARGET_DIR",
        "BBOXY_CARGO_TARGET_DIR",
    ] {
        command.env_remove(var);
    }
    command
}

// keeps: decision #315
#[test]
fn native_runner_installs_the_team_requirement_and_reuses_it_on_the_next_run() {
    let root = tempfile::tempdir().unwrap();
    let p = root.path();
    let signer = p.join("codesign");
    let security = p.join("security");
    let binary = p.join("bboxy450-term");
    executable(
        &signer,
        r#"#!/usr/bin/env python3
import os,sys,json,pathlib
args=sys.argv[1:];root=pathlib.Path(os.environ['FIXTURE_ROOT'])
with (root/'calls').open('a') as f:f.write(json.dumps(args)+'\n')
if '-d' in args:
 print('designated => '+(os.environ['FIXTURE_REQUIREMENT'] if (root/'current').exists() else 'cdhash H"fixture-old-hash"'))
elif '--force' in args:
 assert args[args.index('--requirements')+1]=='=designated => '+os.environ['FIXTURE_REQUIREMENT']
 assert args[args.index('--identifier')+1]=='com.bboxy450.term'
 (root/'current').write_text('team')
"#,
    );
    executable(
        &security,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' {IDENTITY}\nprintf 'lookup\\n' >> \"$FIXTURE_ROOT/lookups\"\n"
        ),
    );
    executable(
        &binary,
        "#!/bin/sh\nprintf 'ran\\n' >> \"$FIXTURE_ROOT/runs\"\n",
    );
    let script = include_str!("../../../scripts/sign-run.sh")
        .replace("/usr/bin/codesign", signer.to_str().unwrap())
        .replace("/usr/bin/security", security.to_str().unwrap());
    let runner = p.join("runner.sh");
    fs::write(&runner, script).unwrap();
    for _ in 0..2 {
        let output = fixture_command("sh")
            .arg(&runner)
            .arg(&binary)
            .env("FIXTURE_ROOT", p)
            .env("FIXTURE_REQUIREMENT", REQUIREMENT)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let calls: Vec<Vec<String>> = fs::read_to_string(p.join("calls"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        calls
            .iter()
            .filter(|args| args.iter().any(|arg| arg == "--force"))
            .count(),
        1
    );
    assert_eq!(
        fs::read_to_string(p.join("lookups"))
            .unwrap()
            .lines()
            .count(),
        1,
        "matching DR avoids private signing-key lookup"
    );
    assert_eq!(
        fs::read_to_string(p.join("runs")).unwrap().lines().count(),
        2
    );
}

// keeps: task 213 — hostile outer Make context cannot escape private signing fixtures.
#[test]
fn native_bundle_recipe_installs_the_same_requirement_without_real_signing() {
    // Configure only the re-executed test process; never change global env in
    // this parallel test binary. All original recipe assertions run in it.
    let outer = tempfile::tempdir().unwrap();
    let foreign = outer.path().join("outer-target");
    let makefile = outer.path().join("outer.mk");
    fs::write(
        &makefile,
        "$(error inherited MAKEFILES escaped private fixture)\n",
    )
    .unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "native_bundle_fixture_child",
            "--ignored",
            "--nocapture",
        ])
        .env("BBOXY_NATIVE_SIGNING_FIXTURE", "1")
        .env(
            "MAKEFLAGS",
            "--eval=$(error inherited MAKEFLAGS escaped private fixture)",
        )
        .env("MFLAGS", "--silent")
        .env("MAKELEVEL", "99")
        .env("MAKEOVERRIDES", "TARGET_DIR=outer-target")
        .env("MAKEFILES", &makefile)
        .env(
            "GNUMAKEFLAGS",
            "--eval=$(error inherited GNUMAKEFLAGS escaped private fixture)",
        )
        .env("TARGET_DIR", &foreign)
        .env("CARGO_TARGET_DIR", &foreign)
        .env("BBOXY_CARGO_TARGET_DIR", &foreign)
        .env("PROFILE", "outside-profile")
        .env("BUNDLE", foreign.join("outside.app"))
        .env("INSTALL_DIR", &foreign)
        .env("UNSIGNED_INSTALL_DIR", &foreign)
        .env("DIST_DIR", &foreign)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "hostile Make-context fixture failed: status={}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !foreign.exists(),
        "fixture wrote into the outer caller's target"
    );
}

#[test]
#[ignore = "owned subprocess; parent supplies the hostile Make environment"]
fn native_bundle_fixture_child() {
    assert_eq!(
        std::env::var("BBOXY_NATIVE_SIGNING_FIXTURE").as_deref(),
        Ok("1")
    );
    let root = tempfile::tempdir().unwrap();
    let p = root.path();
    for dir in [
        "scripts",
        "packaging/macos",
        "crates/bboxy450-term",
        "target/debug",
        "tools",
    ] {
        fs::create_dir_all(p.join(dir)).unwrap();
    }
    fs::write(p.join("Makefile"), include_str!("../../../Makefile")).unwrap();
    fs::write(
        p.join("scripts/cache-env.sh"),
        "# isolated fake build tools\n",
    )
    .unwrap();
    fs::write(
        p.join("scripts/licenses.sh"),
        "#!/bin/sh\nset -eu\nfor name in LICENSE NOTICE ATTRIBUTION.md LICENSE-MIT LICENSE-APACHE; do test -s \"$name\"; done\nprintf 'checked\\n' >> \"$FIXTURE_ROOT/license-checks\"\n",
    )
    .unwrap();
    fs::write(
        p.join("crates/bboxy450-term/Cargo.toml"),
        "[package]\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(
        p.join("scripts/bundle-macos.sh"),
        include_str!("../../../scripts/bundle-macos.sh"),
    )
    .unwrap();
    for name in ["Info.plist", "bboxy.icns"] {
        fs::write(p.join("packaging/macos").join(name), "fixture").unwrap();
    }
    for name in [
        "LICENSE",
        "NOTICE",
        "ATTRIBUTION.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        fs::write(p.join(name), format!("fixture {name}")).unwrap();
    }
    fs::write(p.join("target/debug/bboxy450-term"), "fixture binary").unwrap();
    for (name, body) in [
        ("uname", "printf 'Darwin\\n'".to_string()),
        ("security", format!("printf 'lookup\\n' >> \"$FIXTURE_ROOT/lookups\"; test -z \"${{FIXTURE_NO_IDENTITY:-}}\" || exit 0; printf '%s\\n' {IDENTITY}")),
        ("cargo", "exit 0".into()),
        ("plutil", "exit 0".into()),
        ("git", "case \"$1\" in rev-list) printf '17\\n' ;; rev-parse) printf 'fixture-revision\\n' ;; diff) exit 0 ;; *) exit 2 ;; esac".into()),
    ] {
        executable(&p.join("tools").join(name), &format!("#!/bin/sh\n{body}\n"));
    }
    executable(
        &p.join("tools/codesign"),
        r#"#!/usr/bin/env python3
import os,sys,json,pathlib
with (pathlib.Path(os.environ['FIXTURE_ROOT'])/'calls').open('a') as f:f.write(json.dumps(sys.argv[1:])+'\n')
"#,
    );
    let output = fixture_command("sh")
        .arg(p.join("scripts/bundle-macos.sh"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                p.join("tools").display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .env("CARGO_TARGET_DIR", p.join("target"))
        .env("FIXTURE_ROOT", p)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls: Vec<Vec<String>> = fs::read_to_string(p.join("calls"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(calls.len(), 2);
    let signed = &calls[0];
    let at = signed
        .iter()
        .position(|arg| arg == "--requirements")
        .unwrap();
    assert_eq!(signed[at + 1], format!("=designated => {REQUIREMENT}"));
    assert!(calls[1].iter().any(|arg| arg == "--verify"));
    assert!(
        p.join("target/debug/bboxy450-term.app/Contents/MacOS/bboxy450-term")
            .is_file()
    );
    for name in [
        "LICENSE",
        "NOTICE",
        "ATTRIBUTION.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        assert_eq!(
            fs::read(
                p.join("target/debug/bboxy450-term.app/Contents/Resources")
                    .join(name)
            )
            .unwrap(),
            fs::read(p.join(name)).unwrap(),
            "bundle preserves {name} bytes"
        );
    }
    assert_eq!(
        fs::read_to_string(p.join("license-checks")).unwrap(),
        "checked\n"
    );
    let path = format!(
        "{}:{}",
        p.join("tools").display(),
        std::env::var("PATH").unwrap()
    );
    let refused = fixture_command("make")
        .arg("bundle")
        .arg("PROFILE=debug")
        .current_dir(p)
        .env("PATH", &path)
        .env("FIXTURE_ROOT", p)
        .env("FIXTURE_NO_IDENTITY", "1")
        .output()
        .unwrap();
    assert!(
        !refused.status.success(),
        "signed bundle must fail without the approved identity"
    );
    assert_eq!(
        fs::read_to_string(p.join("calls")).unwrap().lines().count(),
        2,
        "failed signed build must not fall back to ad-hoc signing"
    );
    let lookups = fs::read_to_string(p.join("lookups")).unwrap();
    let unsigned = fixture_command("make")
        .arg("bundle-unsigned")
        .current_dir(p)
        .env("PATH", &path)
        .env("FIXTURE_ROOT", p)
        .env("FIXTURE_NO_IDENTITY", "1")
        .output()
        .unwrap();
    assert!(
        unsigned.status.success(),
        "{}",
        String::from_utf8_lossy(&unsigned.stderr)
    );
    assert_eq!(
        fs::read_to_string(p.join("lookups")).unwrap(),
        lookups,
        "unsigned tests never look up the private signing identity"
    );
    let calls: Vec<Vec<String>> = fs::read_to_string(p.join("calls"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let at = calls[2].iter().position(|arg| arg == "--sign").unwrap();
    assert_eq!(calls[2][at + 1], "-");
    for args in &calls {
        assert_eq!(
            p.join(args.last().unwrap()),
            p.join("target/debug/bboxy450-term.app"),
            "every signer/verification call stays in the private fixture bundle"
        );
    }
    fs::remove_file(p.join("ATTRIBUTION.md")).unwrap();
    let calls_before = fs::read(p.join("calls")).unwrap();
    let missing = fixture_command("make")
        .arg("bundle-unsigned")
        .current_dir(p)
        .env("PATH", &path)
        .env("FIXTURE_ROOT", p)
        .output()
        .unwrap();
    assert!(
        !missing.status.success(),
        "missing attribution stops bundling before signing"
    );
    assert_eq!(fs::read(p.join("calls")).unwrap(), calls_before);
}
