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
        let output = Command::new("sh")
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

#[test]
fn native_bundle_recipe_installs_the_same_requirement_without_real_signing() {
    let root = tempfile::tempdir().unwrap();
    let p = root.path();
    for dir in ["scripts", "packaging/macos", "target/debug", "tools"] {
        fs::create_dir_all(p.join(dir)).unwrap();
    }
    fs::write(
        p.join("scripts/bundle-macos.sh"),
        include_str!("../../../scripts/bundle-macos.sh"),
    )
    .unwrap();
    for name in ["Info.plist", "bboxy.icns"] {
        fs::write(p.join("packaging/macos").join(name), "fixture").unwrap();
    }
    for name in ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE"] {
        fs::write(p.join(name), "fixture").unwrap();
    }
    fs::write(p.join("target/debug/bboxy450-term"), "fixture binary").unwrap();
    for (name, body) in [
        ("uname", "printf 'Darwin\\n'".to_string()),
        ("security", format!("printf '%s\\n' {IDENTITY}")),
        ("cargo", "exit 0".into()),
        ("plutil", "exit 0".into()),
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
    let output = Command::new("sh")
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
}
