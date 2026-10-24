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
}
impl LaunchPlan {
    /// The owner calls this only after process exit, never at close-grace start.
    pub fn cleanup(self) -> Result<()> {
        std::fs::remove_dir_all(self.overlay)?;
        Ok(())
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
