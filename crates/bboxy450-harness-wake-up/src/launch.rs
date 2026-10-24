use super::HarnessRecord;
use crate::{Result, invalid};
use std::collections::BTreeMap;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub struct LaunchContext<'a> {
    pub id: crate::Ulid,
    pub generation: u64,
    pub name: &'a str,
    pub role_name: &'a str,
    pub harness_session_id: Option<&'a str>,
    pub skill: &'a str,
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
pub struct LaunchPlan {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub overlay: PathBuf,
    pub harness_session_id: Option<String>,
    pub notes: Vec<&'static str>,
    /// A seat-owned service the caller starts before the PTY command.
    pub runtime: Option<RuntimePlan>,
}

/// Neutral process/socket plan; the caller owns the service for the seat lifetime.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RuntimePlan {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub endpoint: PathBuf,
}

/// Release a launch overlay and its optional owned short socket alias.
pub fn cleanup_overlay(overlay: &Path, endpoint: Option<&Path>) -> Result<()> {
    if let Some(parent) = endpoint.and_then(Path::parent)
        && parent != overlay
    {
        let name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if parent.parent() != Some(Path::new("/tmp"))
            || !name
                .strip_prefix("bboxy-cx-")
                .is_some_and(|id| id.parse::<crate::Ulid>().is_ok())
            || std::fs::read_link(parent)? != overlay
        {
            return Err(invalid("unowned runtime socket alias"));
        }
        std::fs::remove_file(parent)?;
    }
    std::fs::remove_dir_all(overlay)?;
    Ok(())
}

fn runtime_endpoint(overlay: &Path) -> Result<PathBuf> {
    let endpoint = overlay.join("codex.sock");
    if endpoint.as_os_str().as_encoded_bytes().len() < 104 {
        return Ok(endpoint);
    }
    // The requested locator stays in the 0700 overlay; Codex can resolve it
    // to its own socket. A short alias avoids Darwin's sun_path bound.
    let alias = Path::new("/tmp").join(format!("bboxy-cx-{}", crate::Ulid::new()));
    std::os::unix::fs::symlink(overlay, &alias)?;
    Ok(alias.join("codex.sock"))
}
impl LaunchPlan {
    /// The owner calls this only after process exit, never at close-grace start.
    pub fn cleanup(self) -> Result<()> {
        cleanup_overlay(
            &self.overlay,
            self.runtime.as_ref().map(|r| r.endpoint.as_path()),
        )
    }
}
/// The lifecycle events a Codex seat's inline hook overlay installs, one hook each, all
/// running `<this binary> hook --agent codex`. The Codex hook-review recipe (#270) matches
/// Codex's count against this list and never a typed number.
pub const CODEX_HOOK_EVENTS: [&str; 10] = [
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "PreCompact",
    "PostCompact",
];

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
pub(crate) fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}
/// Expand caller-owned skill content with already serialized placeholder values.
/// The template mechanism is shared; its policy and prose belong to the caller.
pub fn render_skill(template: &str, role_name: &str, role_class: &str) -> String {
    template
        .replace("{{ROLE_NAME}}", role_name)
        .replace("{{ROLE_CLASS}}", role_class)
}
fn claude_wiring(overlay: &Path, context: &LaunchContext<'_>) -> Result<PathBuf> {
    write(
        &overlay.join("mcp.json"),
        &serde_json::to_vec(&serde_json::json!({"mcpServers":{"bboxy":{
            "command":context.executable,"args":["mcp"]
        }}}))?,
    )?;
    let plugin = overlay.join("bboxy450");
    for dir in [
        plugin.join(".claude-plugin"),
        plugin.join("skills/bboxy450"),
    ] {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }
    write(
        &plugin.join(".claude-plugin/plugin.json"),
        &serde_json::to_vec(&serde_json::json!({
            "name":"bboxy450","version":env!("CARGO_PKG_VERSION"),
            "description":"Seat control and coordination through the local bboxy MCP server"
        }))?,
    )?;
    write(
        &plugin.join("skills/bboxy450/SKILL.md"),
        context.skill.as_bytes(),
    )?;
    Ok(plugin)
}
pub fn plan(record: &HarnessRecord, context: &LaunchContext<'_>) -> Result<LaunchPlan> {
    crate::assert_not_ui_thread();
    record.validate()?;
    if !context.executable.is_absolute()
        || !context.cwd.is_absolute()
        || !context.overlay_parent.is_absolute()
    {
        return Err(invalid("launch paths must be absolute"));
    }
    let overlay = context
        .overlay_parent
        .join(format!("launch-{}", crate::Ulid::new()));
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&overlay)?;
    let mut spec = LaunchPlan {
        program: record.command[0].clone(),
        args: record.command[1..].to_vec(),
        cwd: context.cwd.to_owned(),
        env: BTreeMap::new(),
        overlay: overlay.clone(),
        harness_session_id: None,
        notes: Vec::new(),
        runtime: None,
    };
    spec.env
        .insert("BBOXY_SEAT_ID".into(), context.id.to_string());
    spec.env
        .insert("BBOXY_GENERATION".into(), context.generation.to_string());
    spec.env
        .insert("BBOXY_ROLE".into(), context.role_name.into());
    spec.env
        .insert("BBOXY_SOCK".into(), context.socket.to_string_lossy().into());
    spec.env.insert(
        "BBOXY_CAPABILITY".into(),
        context
            .capability
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    );
    spec.env
        .insert("TERM_PROGRAM".into(), "bboxy450-term".into());
    spec.env.insert(
        "TERM_PROGRAM_VERSION".into(),
        env!("CARGO_PKG_VERSION").into(),
    );
    let mut session = context.harness_session_id.map(str::to_owned);
    let mut notes = Vec::new();
    match record.kind.as_str() {
        "claude" => {
            if spec.args.iter().any(|a| a == "--bare") {
                return Err(invalid("Claude --bare disables hook/socket integration"));
            }
            let command = format!(
                "{} hook --agent claude",
                quote(&context.executable.to_string_lossy())
            );
            let mut hooks = serde_json::Map::new();
            if record.hooks {
                for ev in [
                    "SessionStart",
                    "SessionEnd",
                    "UserPromptSubmit",
                    "PreToolUse",
                    "PostToolUse",
                    "PermissionRequest",
                    "Notification",
                    "Stop",
                    "StopFailure",
                    "SubagentStart",
                    "SubagentStop",
                ] {
                    hooks.insert(
                        ev.into(),
                        serde_json::json!([{"hooks":[{"type":"command","command":command}]}]),
                    );
                }
            }
            let settings = overlay.join("claude-settings.json");
            write(
                &settings,
                &serde_json::to_vec(
                    &serde_json::json!({"crossSessionInbound":"accept","hooks":hooks}),
                )?,
            )?;
            let plugin = claude_wiring(&overlay, context)?;
            spec.args.extend([
                "--append-system-prompt".into(),
                context.role_prompt.into(),
                "--settings".into(),
                settings.to_string_lossy().into(),
                "--mcp-config".into(),
                overlay.join("mcp.json").to_string_lossy().into(),
                "--plugin-dir".into(),
                plugin.to_string_lossy().into(),
                "--name".into(),
                context.name.into(),
            ]);
            if let Some(id) = &session {
                spec.args.extend([
                    if context.resume {
                        "--resume"
                    } else {
                        "--session-id"
                    }
                    .into(),
                    id.clone(),
                ]);
            } else {
                // Native Claude requires UUID spelling. It is not a durable mux ID.
                let mut bytes = crate::Ulid::new().to_bytes();
                bytes[6] = (bytes[6] & 15) | 64;
                bytes[8] = (bytes[8] & 63) | 128;
                let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                let id = format!(
                    "{}-{}-{}-{}-{}",
                    &h[..8],
                    &h[8..12],
                    &h[12..16],
                    &h[16..20],
                    &h[20..]
                );
                spec.args.extend(["--session-id".into(), id.clone()]);
                session = Some(id);
            }
        }
        "codex" => {
            spec.args.extend([
                "-c".into(),
                format!(
                    "developer_instructions={}",
                    serde_json::to_string(&format!("{}\n\n{}", context.role_prompt, context.skill))?
                ),
                "-c".into(),
                format!("mcp_servers.bboxy.command={}", serde_json::to_string(context.executable)?),
                "-c".into(),
                "mcp_servers.bboxy.args=[\"mcp\"]".into(),
                "-c".into(),
                "mcp_servers.bboxy.default_tools_approval_mode=\"approve\"".into(),
                "-c".into(),
                "check_for_update_on_startup=false".into(),
                "-c".into(),
                "mcp_servers.bboxy.env_vars=[\"BBOXY_SOCK\",\"BBOXY_CAPABILITY\",\"BBOXY_SEAT_ID\",\"BBOXY_GENERATION\"]".into(),
            ]);
            notes.push("Codex skill text is in per-launch developer_instructions; a per-session skill path remains unverified");
            if record.hooks {
                let notify = serde_json::json!([
                    context.executable.to_string_lossy(),
                    "hook",
                    "--agent",
                    "codex-notify"
                ]);
                spec.args.extend(["-c".into(), format!("notify={notify}")]);
                // Inline overlay, never ~/.codex/hooks.json or trusted_hash. If
                // Codex asks to trust it, the operator handles its native chooser.
                let cmd = format!(
                    "{} hook --agent codex",
                    quote(&context.executable.to_string_lossy())
                );
                let mut entries = Vec::new();
                for ev in CODEX_HOOK_EVENTS {
                    entries.push(format!(
                        "{ev}=[{{hooks=[{{type=\"command\",command={}}}]}}]",
                        serde_json::to_string(&cmd)?
                    ));
                }
                spec.args
                    .extend(["-c".into(), format!("hooks={{{}}}", entries.join(","))]);
                notes.push("Codex inline hook schema/trust needs native approval; no bypass or trust hashes written");
            }
            if context.resume {
                let id = session
                    .as_ref()
                    .ok_or_else(|| invalid("Codex resume requires thread ID"))?;
                spec.args.extend(["resume".into(), id.clone()]);
            }
            let endpoint = runtime_endpoint(&overlay)?;
            let url = format!("unix://{}", endpoint.display());
            let inherited = record.command.len() - 1;
            let mut args = spec.args[..inherited].to_vec();
            args.extend(["app-server".into(), "--listen".into(), url.clone()]);
            for pair in spec.args[inherited..].windows(2) {
                if pair[0] == "-c" || pair[0] == "--config" {
                    args.extend([pair[0].clone(), pair[1].clone()]);
                }
            }
            spec.runtime = Some(RuntimePlan {
                program: spec.program.clone(),
                args,
                env: spec.env.clone(),
                endpoint,
            });
            spec.args
                .splice(inherited..inherited, ["--remote".into(), url]);
            // Only this seat's PTY waits for its runtime. exec preserves its PID;
            // neither the supervisor actor nor the caller's serial RPC waits.
            let mut args = vec![
                "harness-exec".into(),
                spec.runtime
                    .as_ref()
                    .unwrap()
                    .endpoint
                    .to_string_lossy()
                    .into(),
                spec.program.clone(),
            ];
            args.append(&mut spec.args);
            spec.program = context.executable.to_string_lossy().into();
            spec.args = args;
        }
        "zsh" => {
            let user = context.user_zdotdir.unwrap_or(context.user_home);
            spec.env
                .insert("BBOXY_USER_ZDOTDIR".into(), user.to_string_lossy().into());
            spec.env
                .insert("ZDOTDIR".into(), overlay.to_string_lossy().into());
            write(&overlay.join(".zshenv"),b"typeset -g _bbt_overlay=$ZDOTDIR\ntypeset -g _bbt_user_zdotdir=$BBOXY_USER_ZDOTDIR\nZDOTDIR=$_bbt_user_zdotdir\n[[ -r $ZDOTDIR/.zshenv ]] && source $ZDOTDIR/.zshenv\nZDOTDIR=$_bbt_overlay\n")?;
            for file in [".zprofile", ".zlogin", ".zlogout"] {
                write(&overlay.join(file),format!("ZDOTDIR=$_bbt_user_zdotdir\n[[ -r $ZDOTDIR/{file} ]] && source $ZDOTDIR/{file}\nZDOTDIR=$_bbt_overlay\n").as_bytes())?;
            }
            write(&overlay.join(".zshrc"), ZSH_RC.as_bytes())?;
            spec.args.push("-il".into());
        }
        _ => {
            notes.push("generic harness: cadence is display-only; autonomous PTY submit requires operator handoff");
        }
    }
    spec.harness_session_id = session;
    spec.notes = notes;
    Ok(spec)
}

fn runtime_ready(endpoint: &Path) -> bool {
    // mio connect never blocks on a full Unix accept queue.
    mio::net::UnixStream::connect(endpoint).is_ok_and(|socket| {
        socket.take_error().is_ok_and(|error| error.is_none()) && socket.peer_addr().is_ok()
    })
}

/// Private CLI entry for an already supervisor-owned PTY. Wait only on the
/// explicitly selected seat socket, then replace this process with its TUI.
/// Normal CODEX_HOME and authentication stay with Codex; no credential is copied.
pub fn exec_when_ready(args: &[String]) -> Result<()> {
    crate::assert_not_ui_thread();
    use std::os::unix::process::CommandExt;
    let [endpoint, program, rest @ ..] = args else {
        return Err(invalid("harness-exec needs socket and command"));
    };
    let endpoint = Path::new(endpoint);
    if !endpoint.is_absolute() || program.is_empty() {
        return Err(invalid("harness-exec needs an absolute socket and command"));
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if runtime_ready(endpoint) {
            break;
        }
        if std::time::Instant::now() >= deadline {
            return Err(invalid("seat runtime readiness deadline elapsed"));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Err(std::process::Command::new(program).args(rest).exec().into())
}

const ZSH_RC: &str = r#"ZDOTDIR=$_bbt_user_zdotdir
[[ -r $ZDOTDIR/.zshrc ]] && source $ZDOTDIR/.zshrc
ZDOTDIR=$_bbt_overlay
autoload -Uz add-zsh-hook
_bbt_preexec() { printf '\033]133;C\007' }
_bbt_precmd() {
  local code=$? path=$PWD
  path=${path//\%/\%25}; path=${path// /\%20}
  path=${path//$'\n'/\%0A}; path=${path//$'\r'/\%0D}
  path=${path//$'\a'/\%07}; path=${path//$'\e'/\%1B}
  printf '\033]133;D;%d\007\033]133;A\007\033]7;file://%s%s\007' $code "$HOST" "$path"
  if [[ $PROMPT != *$'\e]133;B\a'* ]]; then PROMPT+=$'%{\e]133;B\a%}'; fi
}
add-zsh-hook preexec _bbt_preexec
add-zsh-hook precmd _bbt_precmd
"#;

#[cfg(test)]
mod runtime_tests {
    use super::*;

    #[test]
    fn long_runtime_paths_keep_the_locator_in_the_overlay_and_release_the_alias() {
        let root = tempfile::tempdir().unwrap();
        let overlay = root.path().join("x".repeat(120));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&overlay)
            .unwrap();
        let endpoint = runtime_endpoint(&overlay).unwrap();
        assert!(endpoint.as_os_str().as_encoded_bytes().len() < 104);
        let alias = endpoint.parent().unwrap().to_owned();
        assert_eq!(std::fs::read_link(&alias).unwrap(), overlay);
        let listener = std::os::unix::net::UnixListener::bind(&endpoint).unwrap();
        assert!(overlay.join("codex.sock").exists());
        drop(listener);
        cleanup_overlay(&overlay, Some(&endpoint)).unwrap();
        assert!(!overlay.exists());
        assert!(alias.symlink_metadata().is_err());
        assert!(root.path().exists());
    }
}
