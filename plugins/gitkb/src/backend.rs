// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The pinned `git-kb` process boundary: scrubbed environment, private cwd,
//! file-backed stdio with output caps, deadline/cancel kill and refusal
//! mapping. Translated from bboxy450-term `plugins/gitkb/adapter.py`
//! (`Adapter.cli`, `verify_backend`), with one change: the binary is hashed
//! once and later calls compare cheap file identity before re-hashing.
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use bbt_plugin_sdk::{Ctx, Refusal};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::native::{hex, random_hex};

/// Largest accepted stdout or stderr of one backend call.
pub const MAX_BACKEND: u64 = 32 * 1024 * 1024;

/// The pinned digest and file identity last verified, kept in the plugin's
/// private data directory so a new process need not hash the binary again.
const VERIFIED_RECORD: &str = "backend-verified";

/// Cheap file identity: a change means the binary must be hashed again.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Identity([i64; 7]);

fn identity(path: &Path) -> std::io::Result<Identity> {
    let m = std::fs::metadata(path)?;
    Ok(Identity([
        m.dev() as i64,
        m.ino() as i64,
        m.size() as i64,
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    ]))
}

#[derive(Clone)]
pub struct Backend {
    pub path: PathBuf,
    sha256: String,
    verified: Option<Identity>,
    pub env: Vec<(String, String)>,
    pub cwd: PathBuf,
}

/// The host's environment allowlist minus anything credential-like.
pub fn scrubbed_env() -> Vec<(String, String)> {
    std::env::vars()
        .filter(|(k, _)| {
            (matches!(k.as_str(), "PATH" | "HOME" | "LANG" | "TMPDIR" | "TERM")
                || k.starts_with("LC_"))
                && !["TOKEN", "KEY", "SECRET", "BBOXY_", "ICQ_"]
                    .iter()
                    .any(|w| k.to_uppercase().contains(w))
        })
        .collect()
}

impl Backend {
    pub fn new(path: &str, sha256: &str, cwd: PathBuf) -> Result<Self, Refusal> {
        if path.is_empty() || sha256.is_empty() {
            return Err(Refusal::new(
                "backend_unavailable",
                "this executable was built without a GitKB pin; rebuild with GITKB_BIN",
            ));
        }
        Ok(Self {
            path: path.into(),
            sha256: sha256.into(),
            verified: None,
            env: scrubbed_env(),
            cwd,
        })
    }

    pub fn set_env(&mut self, key: &str, value: &str) {
        self.env.retain(|(k, _)| k != key);
        self.env.push((key.into(), value.into()));
    }

    /// Hash the binary only when its identity differs from the last verified
    /// one (always on the first call). A different digest is `backend_changed`.
    pub fn verify(&mut self, ctx: &Ctx) -> Result<(), Refusal> {
        let changed = || {
            Refusal::new(
                "backend_changed",
                "the installed GitKB changed; rebuild and retrust this executable",
            )
        };
        let before = identity(&self.path).map_err(|_| changed())?;
        if self.verified.as_ref() == Some(&before) {
            return Ok(());
        }
        self.verified = None;
        // An earlier process of this plugin hashed this exact file: a 400 MB
        // binary takes seconds under load, beyond the host's per-call deadline.
        let record = self.cwd.join(VERIFIED_RECORD);
        let line = format!("{} {:?}\n", self.sha256, before.0);
        if std::fs::read_to_string(&record).is_ok_and(|saved| saved == line) {
            self.verified = Some(before);
            return Ok(());
        }
        ctx.check()?;
        let mut file = File::open(&self.path).map_err(|_| changed())?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 1 << 20];
        loop {
            ctx.check()?;
            let n = file.read(&mut buffer).map_err(|_| changed())?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        ctx.check()?;
        let after = identity(&self.path).map_err(|_| changed())?;
        if hex(&hasher.finalize()) != self.sha256 || after != before {
            return Err(changed());
        }
        // Best effort: without the record the next process hashes again.
        let partial = self
            .cwd
            .join(format!("{VERIFIED_RECORD}.{}", random_hex(8)));
        let saved = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&partial)
            .and_then(|mut file| file.write_all(line.as_bytes()))
            .and_then(|()| std::fs::rename(&partial, &record));
        if saved.is_err() {
            let _ = std::fs::remove_file(&partial);
        }
        self.verified = Some(after);
        Ok(())
    }

    /// Run `git-kb --no-pager --color never <args>` and parse its JSON.
    pub fn cli(
        &mut self,
        ctx: &Ctx,
        args: &[&str],
        stdin: Option<&str>,
        allow_text: bool,
    ) -> Result<Value, Refusal> {
        let cwd = self.cwd.clone();
        self.cli_in(ctx, &cwd, args, stdin, allow_text)
    }

    pub fn cli_in(
        &mut self,
        ctx: &Ctx,
        cwd: &Path,
        args: &[&str],
        stdin: Option<&str>,
        allow_text: bool,
    ) -> Result<Value, Refusal> {
        let io = |e: std::io::Error| Refusal::new("backend_failed", format!("local IO: {e}"));
        let (status, mut out, mut err) = self.spawn(ctx, cwd, args, stdin, MAX_BACKEND)?;
        let stdout = read_all(&mut out).map_err(io)?;
        let parsed = serde_json::from_slice::<Value>(&stdout).ok();
        let exit = status.code().unwrap_or(-1);
        if exit != 0 {
            let value = match parsed {
                Some(v @ Value::Object(_)) => Some(v),
                _ => serde_json::from_slice::<Value>(&read_all(&mut err).map_err(io)?)
                    .ok()
                    .filter(Value::is_object),
            };
            // Native GitKB checks this capacity before moving/replacing the
            // source file. Preserve that definite refusal without echoing paths
            // or document-bearing stderr. Other failures remain uncertain.
            if let Some(refusal) = workspace_capacity_refusal(args, value.as_ref()) {
                return Err(refusal.with("backend_exit", exit));
            }
            let mut code = value
                .as_ref()
                .and_then(|v| v["type"].as_str())
                .unwrap_or("backend_failed")
                .to_owned();
            code = match code.as_str() {
                "source_generation_mismatch" => "etag_mismatch".into(),
                "document_not_found" => "not_found".into(),
                _ => code,
            };
            if value
                .as_ref()
                .and_then(|v| v["message"].as_str())
                .is_some_and(|m| m.contains("Author is required"))
            {
                code = "author_unconfigured".into();
            }
            // Do not echo CLI stderr, which may include private document content.
            let hint = if code == "author_unconfigured" {
                "; configure native GitKB user.name and user.email"
            } else {
                ""
            };
            return Err(Refusal::new(
                code,
                format!("GitKB refused {} (exit {exit}){hint}", args[0]),
            )
            .with("backend_exit", exit));
        }
        match parsed {
            Some(value) => Ok(value),
            None if allow_text => Ok(Value::Object(Default::default())),
            None => Err(Refusal::new(
                "backend_protocol_error",
                "GitKB did not return JSON",
            )),
        }
    }

    /// Run `git-kb --no-pager --color never <args>` with stdout and stderr in
    /// private unlinked files, each capped at `cap` bytes, killing it on
    /// cancellation, deadline or overflow.
    fn spawn(
        &mut self,
        ctx: &Ctx,
        cwd: &Path,
        args: &[&str],
        stdin: Option<&str>,
        cap: u64,
    ) -> Result<(ExitStatus, File, File), Refusal> {
        self.verify(ctx)?;
        let io = |e: std::io::Error| Refusal::new("backend_failed", format!("local IO: {e}"));
        // No shell, inherited GitKB root/workspace, repository cwd or credentials.
        // The calling seat's attribution (ATC session, activity export) applies
        // to this child only: every git-kb run is its own process, and index
        // builds use detached contexts, so one seat's never reaches another's.
        let mut input = anonymous().map_err(io)?;
        if let Some(text) = stdin {
            input.write_all(text.as_bytes()).map_err(io)?;
        }
        input.seek(SeekFrom::Start(0)).map_err(io)?;
        let out = anonymous().map_err(io)?;
        let err = anonymous().map_err(io)?;
        ctx.check()?;
        let mut child = Command::new(&self.path)
            .args(["--no-pager", "--color", "never"])
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .envs(self.env.iter().cloned())
            .envs(ctx.attribution())
            .stdin(Stdio::from(input))
            .stdout(Stdio::from(out.try_clone().map_err(io)?))
            .stderr(Stdio::from(err.try_clone().map_err(io)?))
            .spawn()
            .map_err(io)?;
        let size = |f: &File| f.metadata().map(|m| m.len()).unwrap_or(0);
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(io)? {
                break status;
            }
            let failure = if ctx.is_cancelled() {
                Some("cancelled")
            } else if ctx.remaining().is_zero() {
                Some("deadline_exceeded")
            } else if size(&out).max(size(&err)) > cap {
                Some("backend_response_too_large")
            } else {
                None
            };
            if let Some(code) = failure {
                let _ = child.kill();
                let _ = child.wait();
                let operation = if args[0] == "workspace" && args.len() > 1 {
                    format!("{} {}", args[0], args[1])
                } else {
                    args[0].to_owned()
                };
                let hint = if ["workspace write", "commit", "checkout", "init"]
                    .contains(&operation.as_str())
                {
                    "; a write may have applied, inspect its source before retrying"
                } else {
                    ""
                };
                return Err(Refusal::new(
                    code,
                    format!("{operation} backend stopped{hint}"),
                ));
            }
            // Short polls first: most GitKB calls finish in tens of ms.
            let pause = if started.elapsed() < Duration::from_millis(50) {
                1
            } else {
                5
            };
            std::thread::sleep(Duration::from_millis(pause).min(ctx.remaining()));
        };
        if size(&out).max(size(&err)) > cap {
            return Err(Refusal::new(
                "backend_response_too_large",
                format!("GitKB output exceeded {} MiB", cap >> 20),
            ));
        }
        Ok((status, out, err))
    }

    /// Run a read-only command whose stdout may exceed [`MAX_BACKEND`], up to
    /// `cap` bytes, and return that stdout rewound for streaming parses.
    pub fn cli_output(&mut self, ctx: &Ctx, args: &[&str], cap: u64) -> Result<File, Refusal> {
        let io = |e: std::io::Error| Refusal::new("backend_failed", format!("local IO: {e}"));
        let cwd = self.cwd.clone();
        let (status, mut out, _) = self.spawn(ctx, &cwd, args, None, cap)?;
        if !status.success() {
            let exit = status.code().unwrap_or(-1);
            return Err(Refusal::new(
                "backend_failed",
                format!("GitKB refused {} (exit {exit})", args[0]),
            )
            .with("backend_exit", exit));
        }
        out.seek(SeekFrom::Start(0)).map_err(io)?;
        Ok(out)
    }
}

impl Backend {
    /// Read only the public naming policy, through GitKB's own config parser.
    /// Never return the complete config: other sections can contain credentials.
    /// Keep the native TOML value intact instead of implementing another parser.
    pub fn task_slug_authority(&mut self, ctx: &Ctx) -> Result<Option<String>, Refusal> {
        let cwd = self.cwd.clone();
        let (status, mut out, mut err) = self.spawn(
            ctx,
            &cwd,
            &["config", "get", "slug_authority"],
            None,
            64 * 1024,
        )?;
        let read = |file: &mut File| {
            read_all(file).map_err(|e| Refusal::new("backend_failed", format!("local IO: {e}")))
        };
        if !status.success() {
            // This exact pinned-CLI refusal means there is no configured family.
            // Malformed config, a changed backend or other failures stay errors.
            let error = read(&mut err)?;
            if status.code() == Some(1)
                && String::from_utf8_lossy(&error)
                    .trim()
                    .starts_with("error: key 'slug_authority' not found in config.toml")
            {
                return Ok(None);
            }
            return Err(Refusal::new(
                "backend_failed",
                "GitKB could not read the repository task naming policy",
            )
            .with("backend_exit", status.code().unwrap_or(-1)));
        }
        let text = String::from_utf8(read(&mut out)?)
            .map_err(|_| Refusal::new("backend_protocol_error", "GitKB config was not UTF-8"))?;
        Ok(Some(text.trim().to_owned()))
    }

    /// Run a read-only status command and return its exit code with the JSON
    /// object it printed: stdout, else the object after any log lines on
    /// stderr (GitKB reports failures there). Nothing else from stderr is kept.
    pub fn cli_report(&mut self, ctx: &Ctx, args: &[&str]) -> Result<(i32, Value), Refusal> {
        let io = |e: std::io::Error| Refusal::new("backend_failed", format!("local IO: {e}"));
        let cwd = self.cwd.clone();
        let (status, mut out, mut err) = self.spawn(ctx, &cwd, args, None, MAX_BACKEND)?;
        let object = |bytes: Vec<u8>| -> Option<Value> {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let at = if text.starts_with('{') {
                0
            } else {
                text.find("\n{")? + 1
            };
            serde_json::from_str::<Value>(&text[at..])
                .ok()
                .filter(Value::is_object)
        };
        let value = object(read_all(&mut out).map_err(io)?)
            .or_else(|| read_all(&mut err).ok().and_then(object))
            .ok_or_else(|| Refusal::new("backend_protocol_error", "GitKB did not return JSON"))?;
        Ok((status.code().unwrap_or(-1), value))
    }
}

fn read_all(file: &mut File) -> std::io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = Vec::new();
    file.take(MAX_BACKEND + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// An unlinked private temporary file (Python `tempfile.TemporaryFile`).
fn anonymous() -> std::io::Result<File> {
    let dir = std::env::temp_dir();
    loop {
        let path = dir.join(format!(".bboxy-gitkb-{}", random_hex(12)));
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&path)
        {
            Ok(file) => {
                std::fs::remove_file(&path)?;
                return Ok(file);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

// Uses the native GitKB guard in crates/gitkb-core/src/kb/workspace_utils.rs:
// https://github.com/gitkb/gitkb-core (protocol error recognition, no storage code copied).
fn workspace_capacity_refusal(args: &[&str], value: Option<&Value>) -> Option<Refusal> {
    if !args.starts_with(&["workspace", "write"]) {
        return None;
    }
    let value = value?;
    let message = value["message"].as_str()?;
    (value["type"] == "general_error"
        && message.starts_with("workspace error: refusing to ")
        && message.contains("durable displaced-inode inventory reached its workspace-wide limit"))
        .then(|| Refusal::new("workspace_capacity_exhausted",
            "GitKB refused this write before replacing the document because its retained-version capacity is full. Update GitKB or ask the operator to inspect retained versions, then fetch the current record before a new attempt."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn capacity_refusal_is_definite_only_for_the_native_pre_write_error() {
        let value = serde_json::json!({"type":"general_error","message":
            "workspace error: refusing to save workspace file '/private/document' because the durable displaced-inode inventory reached its workspace-wide limit (at least 1024 files)"});
        let refusal =
            workspace_capacity_refusal(&["workspace", "write", "tasks/example"], Some(&value))
                .unwrap();
        assert_eq!(refusal.code, "workspace_capacity_exhausted");
        assert!(!refusal.message.contains("/private/document"));
        assert!(workspace_capacity_refusal(&["workspace", "resolve"], Some(&value)).is_none());
        let other = serde_json::json!({"type":"general_error","message":"commit result unavailable after replacement"});
        assert!(workspace_capacity_refusal(&["workspace", "write"], Some(&other)).is_none());
    }

    fn env_of(file: &Path) -> BTreeMap<String, String> {
        std::fs::read_to_string(file)
            .unwrap()
            .lines()
            .filter_map(|line| line.split_once('='))
            // The shell's own bookkeeping, not the spawned environment.
            .filter(|(k, _)| !matches!(*k, "PWD" | "OLDPWD" | "SHLVL" | "_"))
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    fn lent(seat: &str) -> BTreeMap<String, String> {
        [
            ("SESSION_ACTIVITY_RESOURCE_ID", format!("xses-{seat}")),
            ("SESSION_ACTIVITY_RESOURCE_KIND", "external".into()),
            ("SESSION_ACTIVITY_PROVIDER", "claude".into()),
            ("SESSION_ACTIVITY_CONTEXT_FILE", format!("/s/{seat}.json")),
            ("SESSION_ACTIVITY_SINK_COMMAND", "/opt/atc dir/atc".into()),
            (
                "SESSION_ACTIVITY_SINK_ARGS_JSON",
                r#"["activity","record","--json"]"#.into(),
            ),
            ("ATC_SESSION_ID", format!("xses-{seat}")),
            ("GITKB_AGENT_ID", format!("agent-{seat}")),
            ("ATC_TOKEN", "must-not-pass".into()),
            ("ATC_REGISTRY_PATH", "/must/not/pass".into()),
        ]
        .map(|(k, v)| (k.to_owned(), v))
        .into()
    }

    // keeps: one hash per binary, not per process — a second backend in the
    // same data directory trusts the record; a changed binary is hashed again.
    #[test]
    fn a_verified_backend_is_not_hashed_again_by_the_next_process() {
        let dir = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("bboxy-gitkb-verified-{}", random_hex(8)));
        std::fs::create_dir(&dir).unwrap();
        let fake = dir.join("git-kb");
        std::fs::write(&fake, "#!/bin/sh\necho '{}'\n").unwrap();
        let sha = hex(&Sha256::digest(b"#!/bin/sh\necho '{}'\n"));
        let ctx = Ctx::detached(Duration::from_secs(10));
        Backend::new(fake.to_str().unwrap(), &sha, dir.clone())
            .unwrap()
            .verify(&ctx)
            .unwrap();
        assert!(dir.join(VERIFIED_RECORD).is_file());
        // A cancelled context refuses any hashing, so only the record passes.
        let cancelled = Ctx::detached(Duration::ZERO);
        Backend::new(fake.to_str().unwrap(), &sha, dir.clone())
            .unwrap()
            .verify(&cancelled)
            .expect("the record verifies without hashing");
        // A rewritten binary has a new identity and digest: backend_changed.
        std::fs::write(&fake, "#!/bin/sh\necho changed\n").unwrap();
        let error = Backend::new(fake.to_str().unwrap(), &sha, dir.clone())
            .unwrap()
            .verify(&ctx)
            .unwrap_err();
        assert_eq!(error.code, "backend_changed");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    // keeps: seat attribution — a write-shaped git-kb run carries exactly the
    // calling seat's allowlisted attribution; an unattributed run carries none;
    // two seats' runs on one reused backend never cross.
    #[test]
    fn git_kb_children_carry_exactly_their_own_calls_attribution() {
        let dir = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("bboxy-gitkb-attribution-{}", random_hex(8)));
        std::fs::create_dir(&dir).unwrap();
        // A fake git-kb: record the environment under the last argument.
        let fake = dir.join("git-kb");
        let script = format!(
            "#!/bin/sh\nfor a; do last=$a; done\nenv > '{}'/\"$last\".env\necho '{{}}'\n",
            dir.display()
        );
        std::fs::write(&fake, &script).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
        let sha = hex(&Sha256::digest(script.as_bytes()));
        let mut backend = Backend::new(fake.to_str().unwrap(), &sha, dir.clone()).unwrap();
        backend.set_env("GITKB_ROOT", "/root");
        backend.set_env("GITKB_WORKSPACE", "main");
        let base: BTreeMap<String, String> = backend.env.iter().cloned().collect();
        let ctx = |env: Option<BTreeMap<String, String>>| {
            let ctx = Ctx::detached(Duration::from_secs(10));
            env.map_or(ctx.clone(), |env| ctx.with_attribution(env))
        };
        for (name, attribution) in [
            ("a", Some(lent("a"))),
            ("none", None),
            ("b", Some(lent("b"))),
            ("again", None),
        ] {
            backend
                .cli(
                    &ctx(attribution.clone()),
                    &["commit", "--json", "-m", "attributed", "--", name],
                    None,
                    false,
                )
                .unwrap();
            let mut expected = base.clone();
            if let Some(mut lent) = attribution {
                lent.remove("ATC_TOKEN");
                lent.remove("ATC_REGISTRY_PATH");
                assert_eq!(lent.len(), 8, "every allowlisted name");
                expected.extend(lent);
            }
            assert_eq!(env_of(&dir.join(format!("{name}.env"))), expected, "{name}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
