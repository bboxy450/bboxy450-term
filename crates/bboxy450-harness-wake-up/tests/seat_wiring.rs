#![allow(clippy::disallowed_methods)] // Fixture IO only.
use bboxy450_harness_wake_up::{
    HarnessRecord, Ulid,
    launch::{LaunchContext, plan},
};
use std::path::Path;
const PROMPT: &str =
    "ROLE: EXECUTOR\nFollow the assigned settings prompt: \"literal\" \\ path · next line.\n";
#[test]
fn resumed_generation_gets_a_fresh_overlay_and_keeps_the_native_resume_path() {
    let temp = tempfile::tempdir().unwrap();
    for kind in ["claude", "codex"] {
        let id = Ulid::new();
        let session = "00000000-0000-4000-8000-000000000001";
        let record = HarnessRecord::builtin(kind).unwrap();
        let first = plan(
            &record,
            &LaunchContext {
                id,
                generation: 1,
                name: "resumed fixture",
                role_name: "executor",
                harness_session_id: Some(session),
                skill: "caller-owned skill content",
                cwd: temp.path(),
                role_prompt: PROMPT,
                executable: Path::new("/tmp/our app"),
                overlay_parent: temp.path(),
                user_home: temp.path(),
                user_zdotdir: None,
                socket: Path::new("/tmp/first.sock"),
                capability: [31; 32],
                resume: false,
            },
        )
        .unwrap();
        let resumed = plan(
            &record,
            &LaunchContext {
                id,
                generation: 2,
                name: "resumed fixture",
                role_name: "executor",
                harness_session_id: Some(session),
                skill: "caller-owned skill content",
                cwd: temp.path(),
                role_prompt: PROMPT,
                executable: Path::new("/tmp/our app"),
                overlay_parent: temp.path(),
                user_home: temp.path(),
                user_zdotdir: None,
                socket: Path::new("/tmp/second.sock"),
                capability: [32; 32],
                resume: true,
            },
        )
        .unwrap();
        assert_ne!(first.overlay, resumed.overlay);
        assert_eq!(resumed.env["BBOXY_GENERATION"], "2");
        assert_eq!(resumed.env["BBOXY_SOCK"], "/tmp/second.sock");
        assert_ne!(
            first.env["BBOXY_CAPABILITY"],
            resumed.env["BBOXY_CAPABILITY"]
        );
        assert!(!resumed.env.contains_key("HOME"));
        assert!(!resumed.env.contains_key("CODEX_HOME"));
        let args = &resumed.args;
        assert_eq!(
            &args[args.len() - 2..],
            [
                if kind == "claude" {
                    "--resume"
                } else {
                    "resume"
                },
                session
            ]
        );
        assert!(!args.iter().any(|arg| arg == "--strict-mcp-config"));
        first.cleanup().unwrap();
        assert!(resumed.overlay.is_dir());
        resumed.cleanup().unwrap();
    }
}

/// Independent fixture client inspects the actual argv, JSON/TOML, quoted hook
/// command and inherited MCP env. The mux keeps the same client's real-PTY test.
#[test]
fn first_launch_fixture_checks_wiring_and_preserves_user_config() {
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("operator home");
    let files = [
        (".claude/settings.json", "operator settings"),
        (".claude/settings.local.json", "operator local settings"),
        (".claude/plugins/operator/plugin.json", "operator plugin"),
        (".claude/skills/operator/SKILL.md", "operator skill"),
        (".codex/config.toml", "operator config"),
        (".codex/hooks.json", "operator hooks"),
    ];
    for (name, content) in files {
        let path = home.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    let shim = temp.path().join("our 'quoted' app");
    fs::write(&shim, include_bytes!("fixtures/seat_wiring.py")).unwrap();
    fs::set_permissions(&shim, fs::Permissions::from_mode(0o700)).unwrap();
    let skill = bboxy450_harness_wake_up::launch::render_skill(
        "Role {{ROLE_NAME}} ({{ROLE_CLASS}}).\n1. **Orient.\n2. **Tasks.\n3. **Messages.\n4. **Artifacts.\nStorage does not prove model receipt",
        "\"executor\"",
        "\"worker\"",
    );
    for kind in ["claude", "codex"] {
        let id = Ulid::new();
        let output = temp.path().join(format!("{kind}.json"));
        fs::write(output.with_extension("release"), "exit").unwrap();
        let mut record = HarnessRecord::builtin(kind).unwrap();
        record.command = vec![
            shim.to_string_lossy().into(),
            kind.into(),
            output.to_string_lossy().into(),
            shim.to_string_lossy().into(),
            PROMPT.into(),
        ];
        let plan = plan(
            &record,
            &LaunchContext {
                id,
                generation: 1,
                name: "wiring fixture",
                role_name: "executor",
                harness_session_id: None,
                skill: &skill,
                cwd: temp.path(),
                role_prompt: PROMPT,
                executable: &shim,
                overlay_parent: temp.path(),
                user_home: &home,
                user_zdotdir: None,
                socket: Path::new("/tmp/fixture.sock"),
                capability: [74; 32],
                resume: false,
            },
        )
        .unwrap();
        struct Owned(std::process::Child);
        impl Drop for Owned {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let _runtime = plan.runtime.as_ref().map(|runtime| {
            Owned(
                Command::new(&runtime.program)
                    .args(&runtime.args)
                    .envs(&plan.env)
                    .envs(&runtime.env)
                    .current_dir(&plan.cwd)
                    .spawn()
                    .unwrap(),
            )
        });
        let child = Command::new(&plan.program)
            .args(&plan.args)
            .envs(&plan.env)
            .current_dir(&plan.cwd)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{kind}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        assert_eq!(report["identity"]["seat"], id.to_string());
        assert_eq!(report["identity"]["generation"], "1");
        assert_eq!(report["identity"]["role"], "executor");
        assert_eq!(report["identity"]["capability_present"], true);
        assert_eq!(report["identity"]["socket"], "/tmp/fixture.sock");
        assert_eq!(report["mcp"]["argv"], serde_json::json!(["mcp"]));
        assert_eq!(report["skill_present"], true);
        assert_eq!(report["role_prompt_preserved"], true);
        assert_eq!(
            fs::metadata(&plan.overlay).unwrap().permissions().mode() & 0o777,
            0o700
        );
        plan.cleanup().unwrap();
    }
    for (name, content) in files {
        assert_eq!(
            fs::read(home.join(name)).unwrap(),
            content.as_bytes(),
            "{name}"
        );
    }
}
