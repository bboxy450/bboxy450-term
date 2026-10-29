// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Explicit document commits, with exact path staging and the repository's signer.
use super::data_source as d;
use std::{
    io::Read,
    os::unix::process::CommandExt,
    path::{Component, Path},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

pub(crate) fn validate(request: &d::CommitRequest) -> d::Result<()> {
    if request.refs.is_empty()
        || request.refs.len() > 32
        || request.message.trim().is_empty()
        || request.message.len() > 4096
        || request
            .message
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(d::CommandError::new(
            "invalid_commit",
            "commit requires 1..32 records and a bounded message",
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for reference in &request.refs {
        let path = Path::new(&reference.path);
        if !ids.insert(&reference.id)
            || path.is_absolute()
            || path.extension().is_none_or(|e| e != "md")
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(d::CommandError::new(
                "invalid_commit",
                "records must be distinct confined Markdown paths",
            ));
        }
    }
    Ok(())
}
fn git(root: &Path, args: &[&str], deadline: Instant) -> d::Result<Output> {
    bbt_rt::assert_not_ui_thread();
    if Instant::now() >= deadline {
        return Err(d::CommandError::new(
            "commit_not_confirmed",
            "Document commit exceeded its 15 s operation deadline; inspect the original operation before retry.",
        ));
    }
    let failure = |e: std::io::Error| d::CommandError::new("git_unavailable", e.to_string());
    let mut stdout = tempfile::tempfile().map_err(failure)?;
    let mut stderr = tempfile::tempfile().map_err(failure)?;
    let mut command = Command::new("git");
    // Literal pathspecs include filenames containing []/*/: without expansion.
    command
        .current_dir(root)
        .arg("--literal-pathspecs")
        .args(args)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(stdout.try_clone().map_err(failure)?)
        .stderr(stderr.try_clone().map_err(failure)?);
    for (key, _) in std::env::vars_os() {
        if key.to_str().is_some_and(|k| k.starts_with("GIT_")) {
            command.env_remove(key);
        }
    }
    command.env("GIT_TERMINAL_PROMPT", "0");
    struct OwnedGit(std::process::Child);
    impl Drop for OwnedGit {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                // SAFETY: this recorded process group belongs to our Git child.
                unsafe {
                    libc::kill(-(self.0.id() as i32), libc::SIGKILL);
                }
                let _ = self.0.wait();
            }
        }
    }
    let mut child = OwnedGit(command.spawn().map_err(failure)?);
    let status = loop {
        if Instant::now() >= deadline
            || stdout.metadata().map_err(failure)?.len() > 64 * 1024
            || stderr.metadata().map_err(failure)?.len() > 64 * 1024
        {
            // This process group was created above for this one Git operation.
            // SAFETY: child.id is the recorded leader of our own child group.
            unsafe {
                libc::kill(-(child.0.id() as i32), libc::SIGKILL);
            }
            let _ = child.0.wait();
            return Err(d::CommandError::new(
                "commit_not_confirmed",
                "Git exceeded the shared 15 s commit deadline or 64 KiB output bound; inspect the original operation before retry.",
            ));
        }
        if let Some(status) = child.0.try_wait().map_err(failure)? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if stdout.metadata().map_err(failure)?.len() > 64 * 1024
        || stderr.metadata().map_err(failure)?.len() > 64 * 1024
    {
        return Err(d::CommandError::new(
            "commit_not_confirmed",
            "Git output exceeded 64 KiB; inspect the original operation before retry.",
        ));
    }
    use std::io::{Seek, SeekFrom};
    stdout.seek(SeekFrom::Start(0)).map_err(failure)?;
    stderr.seek(SeekFrom::Start(0)).map_err(failure)?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    stdout
        .take(64 * 1024)
        .read_to_end(&mut out)
        .map_err(failure)?;
    stderr
        .take(64 * 1024)
        .read_to_end(&mut err)
        .map_err(failure)?;
    Ok(Output {
        status,
        stdout: out,
        stderr: err,
    })
}
fn checked(root: &Path, args: &[&str], deadline: Instant) -> d::Result<String> {
    let result = git(root, args, deadline)?;
    if !result.status.success() {
        return Err(d::CommandError::new(
            "commit_failed",
            format!(
                "git exited {:?}: {}",
                result.status.code(),
                String::from_utf8_lossy(&result.stderr)
                    .chars()
                    .take(2048)
                    .collect::<String>()
            ),
        ));
    }
    String::from_utf8(result.stdout)
        .map_err(|e| d::CommandError::new("invalid_git_result", e.to_string()))
}
pub(crate) fn local(root: &Path, request: d::CommitRequest) -> d::Result<d::CommitResult> {
    let deadline = Instant::now() + Duration::from_secs(15);
    validate(&request)?;
    let repository = checked(root, &["rev-parse", "--show-toplevel"], deadline)?;
    let repository = Path::new(repository.trim())
        .canonicalize()
        .map_err(|e| d::CommandError::new("git_unavailable", e.to_string()))?;
    let root = root
        .canonicalize()
        .map_err(|e| d::CommandError::new("invalid_path", e.to_string()))?;
    let mut paths = Vec::new();
    for reference in &request.refs {
        let mut path = root.clone();
        for component in Path::new(&reference.path).components() {
            path.push(component);
            let metadata = path
                .symlink_metadata()
                .map_err(|e| d::CommandError::new("invalid_path", e.to_string()))?;
            if metadata.file_type().is_symlink() {
                return Err(d::CommandError::new("invalid_path", "symlinks are refused"));
            }
        }
        let path = path.strip_prefix(&repository).map_err(|_| {
            d::CommandError::new("invalid_path", "document is outside the Git repository")
        })?;
        paths.push(
            path.to_str()
                .ok_or_else(|| d::CommandError::new("invalid_path", "Git path must be UTF-8"))?
                .to_owned(),
        );
    }
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    checked(&repository, &args, deadline)?;
    let mut diff = vec!["diff", "--cached", "--quiet", "--"];
    diff.extend(paths.iter().map(String::as_str));
    let result = git(&repository, &diff, deadline)?;
    match result.status.code() {
        Some(0) => {
            return Err(d::CommandError::new(
                "nothing_to_commit",
                "selected records have no changes",
            ));
        }
        Some(1) => (),
        _ => {
            return Err(d::CommandError::new(
                "commit_failed",
                "could not inspect staged document changes",
            ));
        }
    }
    let mut args = vec![
        "commit",
        "-S",
        "--only",
        "-m",
        request.message.as_str(),
        "--",
    ];
    args.extend(paths.iter().map(String::as_str));
    // Once Git is dispatched, an exit/readback failure cannot prove that no
    // commit was created (a hook or result channel may fail after the effect).
    let unconfirmed = |error: d::CommandError| {
        d::CommandError::new(
            "commit_not_confirmed",
            format!("{error}; inspect the original operation before retrying"),
        )
    };
    checked(&repository, &args, deadline).map_err(unconfirmed)?;
    let commit = checked(&repository, &["rev-parse", "HEAD"], deadline).map_err(unconfirmed)?;
    Ok(d::CommitResult {
        commit: commit.trim().into(),
        committed: request.refs,
    })
}
