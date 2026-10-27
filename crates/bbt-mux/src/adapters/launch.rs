use super::HarnessRecord;
use crate::{LaunchSpec, registry::SeatRecord, store::Result};
pub use bboxy450_harness_wake_up::launch::{CODEX_HOOK_EVENTS, exec_when_ready};
use std::path::{Path, PathBuf};

pub struct LaunchContext<'a> {
    pub seat: &'a SeatRecord,
    pub cwd: &'a Path,
    pub role_prompt: &'a str,
    pub executable: &'a Path,
    pub overlay_parent: &'a Path,
    pub user_home: &'a Path,
    pub user_zdotdir: Option<&'a Path>,
    pub socket: &'a Path,
    pub capability: [u8; 32],
    pub resume: bool,
}
pub struct LaunchOptions {
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
    pub role_prompt: String,
    pub executable: PathBuf,
    pub user_home: PathBuf,
    pub user_zdotdir: Option<PathBuf>,
    pub resume: bool,
}
pub struct LaunchPlan {
    pub spec: LaunchSpec,
    pub overlay: PathBuf,
    pub harness_session_id: Option<String>,
    pub notes: Vec<&'static str>,
}

/// Apply the operator's assignments at exec, after login startup has loaded the
/// user's tools. Passing them to the outer shell would let .zprofile overwrite
/// values which the launcher promises to run explicitly.
pub(crate) fn with_environment(spec: &mut LaunchSpec, env: &[(String, String)]) {
    if env.is_empty() {
        return;
    }
    fn wrap(program: &mut String, args: &mut Vec<String>, env: &[(String, String)]) {
        let command = std::mem::replace(program, "/usr/bin/env".into());
        let original = std::mem::take(args);
        *args = env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        args.push(command);
        args.extend(original);
    }
    wrap(&mut spec.program, &mut spec.args, env);
    if let Some(companion) = &mut spec.companion {
        wrap(&mut companion.program, &mut companion.args, env);
    }
}
impl LaunchPlan {
    /// The owner calls this only after process exit, never at close-grace start.
    pub fn cleanup(self) -> Result<()> {
        bboxy450_harness_wake_up::launch::cleanup_overlay(
            &self.overlay,
            self.spec
                .companion
                .as_ref()
                .map(|c| c.ready_socket.as_path()),
        )?;
        Ok(())
    }
}
pub fn plan(record: &HarnessRecord, context: &LaunchContext<'_>) -> Result<LaunchPlan> {
    bbt_rt::assert_not_ui_thread();
    record.validate()?;
    plan_neutral(&super::mapping::harness(record), context)
}

/// The shell loads the user's tools/settings, then exec keeps the owned PID.
fn user_login_shell() -> String {
    let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
    let mut storage = vec![0u8; 16 * 1024];
    let mut found = std::ptr::null_mut();
    // SAFETY: getpwuid_r writes only into the supplied record/buffer; a successful
    // non-null result owns a NUL-terminated pw_shell until storage is dropped.
    let result = unsafe {
        libc::getpwuid_r(
            libc::geteuid(),
            entry.as_mut_ptr(),
            storage.as_mut_ptr().cast(),
            storage.len(),
            &mut found,
        )
    };
    if result == 0 && !found.is_null() {
        let entry = unsafe { entry.assume_init() };
        if !entry.pw_shell.is_null() {
            let shell = unsafe { std::ffi::CStr::from_ptr(entry.pw_shell) }.to_string_lossy();
            if Path::new(shell.as_ref()).is_absolute() {
                return shell.into_owned();
            }
        }
    }
    std::env::var("SHELL")
        .ok()
        .filter(|value| Path::new(value).is_absolute())
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                "/bin/zsh"
            } else {
                "/bin/sh"
            }
            .into()
        })
}

pub(crate) fn in_login_shell(
    spec: &mut LaunchSpec,
    kind: &str,
    home: &Path,
    zdotdir: Option<&Path>,
) {
    if kind == "zsh" {
        return;
    } // Already an integrated interactive login shell.
    let shell = user_login_shell();
    fn wrap(program: &mut String, args: &mut Vec<String>, shell: &str) {
        let executable = std::mem::replace(program, shell.into());
        let original = std::mem::take(args);
        if Path::new(shell)
            .file_name()
            .is_some_and(|name| name == "fish")
        {
            *args = vec!["-ilc".into(), "exec $argv".into(), executable];
        } else {
            *args = vec![
                "-ilc".into(),
                "exec \"$@\"".into(),
                "bboxy450-harness".into(),
                executable,
            ];
        }
        args.extend(original);
    }
    fn environment(
        env: &mut std::collections::BTreeMap<String, String>,
        home: &Path,
        zdotdir: Option<&Path>,
        shell: &str,
    ) {
        env.entry("HOME".into())
            .or_insert_with(|| home.to_string_lossy().into_owned());
        env.entry("ZDOTDIR".into())
            .or_insert_with(|| zdotdir.unwrap_or(home).to_string_lossy().into_owned());
        env.entry("SHELL".into()).or_insert_with(|| shell.into());
    }
    wrap(&mut spec.program, &mut spec.args, &shell);
    environment(&mut spec.env, home, zdotdir, &shell);
    if let Some(companion) = &mut spec.companion {
        wrap(&mut companion.program, &mut companion.args, &shell);
        environment(&mut companion.env, home, zdotdir, &shell);
    }
}

pub(super) fn plan_neutral(
    record: &bboxy450_harness_wake_up::HarnessRecord,
    context: &LaunchContext<'_>,
) -> Result<LaunchPlan> {
    super::validate_role(&record.kind, &context.seat.role)?;
    let skill = bboxy450_harness_wake_up::launch::render_skill(
        include_str!("skills/bboxy450/SKILL.md"),
        &serde_json::to_string(&context.seat.role.name)?,
        &serde_json::to_string(&context.seat.role.class)?,
    );
    let plan = bboxy450_harness_wake_up::launch::plan(
        record,
        &bboxy450_harness_wake_up::LaunchContext {
            id: context.seat.id,
            generation: context.seat.generation,
            name: &context.seat.name,
            role_name: &context.seat.role.name,
            harness_session_id: context.seat.harness_session_id.as_deref(),
            skill: &skill,
            cwd: context.cwd,
            role_prompt: context.role_prompt,
            executable: context.executable,
            overlay_parent: context.overlay_parent,
            user_home: context.user_home,
            user_zdotdir: context.user_zdotdir,
            socket: context.socket,
            capability: context.capability,
            resume: context.resume,
        },
    )?;
    let plan = super::mapping::launch_plan(plan, context.seat.id, context.seat.generation);
    plan.spec.validate()?;
    Ok(plan)
}

#[cfg(test)]
mod login_shell_tests {
    use super::*;
    use std::{
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
    };

    #[test]
    #[ignore = "night: real login-shell exec boundary; make test-boundaries runs it"]
    #[allow(
        clippy::disallowed_methods,
        reason = "private shell/exec fixture, never UI"
    )]
    fn login_shell_exec_keeps_pid_exports_and_literal_arguments_for_harness_and_companion() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("tools");
        std::fs::create_dir(&bin).unwrap();
        let executable = bin.join("fixture-harness");
        std::fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\000' \"$$\" \"$BBT_LOGIN_TOOL\" \"$BBT_RC_TOOL\" \"$@\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(
            root.path().join(".zprofile"),
            "export BBT_LOGIN_TOOL=profile\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join(".zshrc"),
            format!(
                "export BBT_RC_TOOL=rc\nexport PATH='{}:/usr/bin:/bin'\n",
                bin.display()
            ),
        )
        .unwrap();
        let literal = "space 'quote' \"double\" ; $HOME $(echo must-not-expand)";
        let mut spec = LaunchSpec::new("fixture-harness", root.path());
        spec.args = vec![literal.into(), "".into()];
        spec.companion = Some(Box::new(bbt_supervisor::protocol::CompanionSpec {
            program: "fixture-harness".into(),
            args: vec!["app-server".into(), literal.into()],
            env: Default::default(),
            ready_socket: root.path().join("runtime.sock"),
        }));
        in_login_shell(&mut spec, "codex", root.path(), Some(root.path()));
        let companion = spec.companion.as_ref().unwrap();
        for (program, args, env, expected) in [
            (&spec.program, &spec.args, &spec.env, vec![literal, ""]),
            (
                &companion.program,
                &companion.args,
                &companion.env,
                vec!["app-server", literal],
            ),
        ] {
            let child = Command::new(program)
                .args(args)
                .env_clear()
                .envs(env)
                .current_dir(root.path())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let pid = child.id();
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let fields: Vec<_> = output
                .stdout
                .split(|byte| *byte == 0)
                .map(|field| String::from_utf8_lossy(field).into_owned())
                .collect();
            assert_eq!(
                fields[0],
                pid.to_string(),
                "shell must exec, not fork the harness"
            );
            assert_eq!(&fields[1..3], ["profile", "rc"]);
            assert_eq!(&fields[3..fields.len() - 1], expected);
        }
    }
}
