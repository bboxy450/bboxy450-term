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
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use bbt_plugin_sdk::{Ctx, Refusal};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::native::{hex, random_hex};

/// Largest accepted stdout or stderr of one backend call.
pub const MAX_BACKEND: u64 = 32 * 1024 * 1024;

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
        self.verify(ctx)?;
        let io = |e: std::io::Error| Refusal::new("backend_failed", format!("local IO: {e}"));
        // No shell, inherited GitKB root/workspace, repository cwd or credentials.
        let mut input = anonymous().map_err(io)?;
        if let Some(text) = stdin {
            input.write_all(text.as_bytes()).map_err(io)?;
        }
        input.seek(SeekFrom::Start(0)).map_err(io)?;
        let mut out = anonymous().map_err(io)?;
        let mut err = anonymous().map_err(io)?;
        ctx.check()?;
        let mut child = Command::new(&self.path)
            .args(["--no-pager", "--color", "never"])
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .envs(self.env.iter().cloned())
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
            } else if size(&out).max(size(&err)) > MAX_BACKEND {
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
        if size(&out).max(size(&err)) > MAX_BACKEND {
            return Err(Refusal::new(
                "backend_response_too_large",
                "GitKB output exceeded 32 MiB",
            ));
        }
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
