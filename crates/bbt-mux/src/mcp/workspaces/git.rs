// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Local provider. Only short publication transactions live in the catalogue;
//! every Git/submodule/process inspection here runs on a bounded IO worker.
use super::{Record, Source, State};
use crate::store::{Result, invalid};
use std::{
    fs,
    io::Read,
    os::unix::{fs::MetadataExt, process::CommandExt},
    path::Path,
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

thread_local! {
    static PROVIDER_LEASE:std::cell::RefCell<Option<std::sync::Arc<std::fs::File>>>=const{std::cell::RefCell::new(None)};
}
pub(super) fn with_provider_lease<T>(
    file: std::sync::Arc<std::fs::File>,
    job: impl FnOnce() -> Result<T>,
) -> Result<T> {
    struct Restore(Option<std::sync::Arc<std::fs::File>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            PROVIDER_LEASE.with(|slot| {
                slot.replace(self.0.take());
            });
        }
    }
    let _restore = Restore(PROVIDER_LEASE.with(|slot| slot.replace(Some(file))));
    job()
}
pub(super) fn inherit_provider_lease(
    command: &mut Command,
) -> Option<std::sync::Arc<std::fs::File>> {
    use std::os::fd::AsRawFd;
    let lease = PROVIDER_LEASE.with(|slot| slot.borrow().clone());
    if let Some(file) = &lease {
        let fd = file.as_raw_fd();
        // Only this child clears CLOEXEC. A process-global inheritable FD would
        // accidentally lend the lease to unrelated concurrent harnesses.
        unsafe {
            command.pre_exec(move || {
                let flags = libc::fcntl(fd, libc::F_GETFD);
                if flags < 0 || libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    lease
}
pub(super) fn output(mut command: Command, timeout: Duration) -> Result<Output> {
    use std::{
        os::fd::AsRawFd,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let _inherited_lease = inherit_provider_lease(&mut command);
    let mut child = command.spawn()?;
    let pid = child.id();
    let cancel = Arc::new(AtomicBool::new(false));
    struct Cancel(Arc<AtomicBool>);
    impl Drop for Cancel {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let _cancel_on_exit = Cancel(cancel.clone());
    type Captured = (Vec<u8>, bool);
    fn reader(
        mut pipe: impl Read + AsRawFd + Send + 'static,
        cancel: Arc<AtomicBool>,
    ) -> mpsc::Receiver<std::io::Result<Captured>> {
        let (tx, rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let result = (|| {
                let fd = pipe.as_raw_fd();
                // SAFETY: pipe owns this descriptor for the reader's lifetime.
                let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
                if flags < 0
                    || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                let mut saved = Vec::new();
                let mut exceeded = false;
                let mut buf = [0; 8192];
                while !cancel.load(Ordering::Acquire) {
                    match pipe.read(&mut buf) {
                        Ok(0) => return Ok((saved, exceeded)),
                        Ok(n) => {
                            let take = n.min(65536usize.saturating_sub(saved.len()));
                            exceeded |= take < n;
                            saved.extend_from_slice(&buf[..take]);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            let mut wait = libc::pollfd {
                                fd,
                                events: libc::POLLIN | libc::POLLHUP,
                                revents: 0,
                            };
                            // Event-driven pipe wait; bounded timeout observes
                            // cancellation without an aggressive polling loop.
                            unsafe {
                                libc::poll(&mut wait, 1, 100);
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(e) => return Err(e),
                    }
                }
                Ok((saved, exceeded))
            })();
            let _ = tx.send(result);
        });
        rx
    }
    let out = reader(child.stdout.take().expect("piped stdout"), cancel.clone());
    let err = reader(child.stderr.take().expect("piped stderr"), cancel.clone());
    let deadline = Instant::now() + timeout;
    let (mut exited, mut stdout, mut stderr) = (false, None, None);
    loop {
        if !exited {
            // WNOWAIT keeps the leader PID reserved while a descendant can
            // still own our output pipes. try_wait would reap it too early and
            // make a later group kill vulnerable to PID/group reuse.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            let observed = unsafe {
                libc::waitid(
                    libc::P_PID,
                    pid as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if observed < 0 {
                return Err(invalid(
                    "workspace_provider_unknown: child wait custody unavailable; inspect original resources",
                ));
            }
            exited = info.si_signo == libc::SIGCHLD;
        }
        if stdout.is_none()
            && let Ok(value) = out.try_recv()
        {
            stdout = Some(value.map_err(|e| {
                cancel.store(true, Ordering::Release);
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                invalid(format!(
                    "workspace_provider_unknown: stdout read failed: {e}"
                ))
            })?);
        }
        if stderr.is_none()
            && let Ok(value) = err.try_recv()
        {
            stderr = Some(value.map_err(|e| {
                cancel.store(true, Ordering::Release);
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                invalid(format!(
                    "workspace_provider_unknown: stderr read failed: {e}"
                ))
            })?);
        }
        if exited && stdout.is_some() && stderr.is_some() {
            cancel.store(true, Ordering::Release);
            // Leader is observed exited but unreaped, so this group ID cannot
            // have been reused. End any owned background helper before reap.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            let status = child.wait()?;
            let (stdout, stdout_exceeded) = stdout.take().unwrap();
            let (stderr, stderr_exceeded) = stderr.take().unwrap();
            if stdout_exceeded || stderr_exceeded {
                // Bounded retention is not a complete preservation/identity
                // proof. Keep draining, then fail closed instead of allowing
                // a later omitted git-cherry/worktree row to change a verdict.
                return Err(invalid(
                    "workspace_provider_output_exceeded: process output exceeded the 65536-byte evidence bound; retained output cannot prove resource identity or source preservation",
                ));
            }
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        if Instant::now() >= deadline {
            cancel.store(true, Ordering::Release);
            // The child (including an exited zombie leader) is still unreaped.
            // Its exact process group remains ours; descendants retaining a
            // pipe are terminated before releasing the leader's PID custody.
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            return Err(invalid(
                "workspace_provider_timeout: inspect original workspace; no automatic resubmission",
            ));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn run(repo: &Path, args: &[&str]) -> Result<Output> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    output(command, Duration::from_secs(120))
}
fn checked(repo: &Path, args: &[&str]) -> Result<String> {
    let out = run(repo, args)?;
    if !out.status.success() {
        return Err(invalid(format!(
            "workspace_git_failed: git {} failed ({}); verify the requested source, setup and repository credentials",
            args.first().copied().unwrap_or("command"),
            out.status
        )));
    }
    String::from_utf8(out.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(Into::into)
}
pub(super) fn identity(path: &Path) -> Result<(u64, u64)> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || m.file_type().is_symlink() {
        return Err(invalid(
            "workspace_identity_changed: expected an owned real directory",
        ));
    }
    Ok((m.dev(), m.ino()))
}
fn reference(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || value.starts_with('-')
        || value.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(invalid(
            "source_unavailable: expected a bounded Git revision, not command options",
        ));
    }
    Ok(())
}
pub(super) fn source(repository: &Path, base: &str, integration: Option<&str>) -> Result<Source> {
    reference(base)?;
    let top = checked(repository, &["rev-parse", "--show-toplevel"])?;
    if Path::new(&top).canonicalize()? != repository.canonicalize()? {
        return Err(invalid(
            "workspace_repository_invalid: select the Git repository root, not an arbitrary subdirectory",
        ));
    }
    let common = checked(
        repository,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    let common = Path::new(&common).canonicalize()?;
    let commit = checked(
        repository,
        &["rev-parse", "--verify", &format!("{base}^{{commit}}")],
    )?;
    let integration = match integration {
        Some(value) => value.to_owned(),
        None => {
            let symbolic = checked(repository, &["rev-parse", "--symbolic-full-name", base])?;
            if symbolic.is_empty() {
                commit.clone()
            } else {
                symbolic
            }
        }
    };
    reference(&integration)?;
    checked(
        repository,
        &[
            "rev-parse",
            "--verify",
            &format!("{integration}^{{commit}}"),
        ],
    )?;
    Ok(Source {
        repository: repository.to_owned(),
        common_identity: identity(&common)?,
        common,
        base: commit,
        integration_ref: integration,
    })
}
fn same_source(r: &Record) -> Result<()> {
    if identity(&r.source.common)? != r.source.common_identity {
        return Err(invalid(
            "workspace_identity_changed: source common directory was replaced",
        ));
    }
    let common = checked(
        &r.source.repository,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    if Path::new(&common).canonicalize()? != r.source.common {
        return Err(invalid(
            "workspace_identity_changed: source repository changed",
        ));
    }
    Ok(())
}
fn registered(r: &Record) -> Result<bool> {
    let list = checked(&r.source.repository, &["worktree", "list", "--porcelain"])?;
    let expected = format!("worktree {}", r.cwd.to_string_lossy());
    Ok(list.split("\n\n").any(|block| {
        block.lines().next() == Some(expected.as_str())
            && block
                .lines()
                .any(|line| line == format!("branch refs/heads/{}", r.branch))
    }))
}
fn verify_identity(r: &Record) -> Result<()> {
    same_source(r)?;
    let current = identity(&r.cwd)?;
    if r.checkout_identity != Some(current) || !registered(r)? {
        return Err(invalid(
            "workspace_identity_changed: checkout identity/registration differs",
        ));
    }
    if checked(
        &r.cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )? != r.source.common.to_string_lossy()
    {
        return Err(invalid(
            "workspace_identity_changed: checkout common directory differs",
        ));
    }
    if checked(&r.cwd, &["symbolic-ref", "--short", "HEAD"])? != r.branch {
        return Err(invalid(
            "workspace_identity_changed: checkout branch differs",
        ));
    }
    Ok(())
}
pub(super) fn verify(r: &Record) -> Result<()> {
    verify_identity(r)?;
    let modules = checked(&r.cwd, &["submodule", "status", "--recursive"])?;
    if modules
        .lines()
        .any(|line| line.starts_with(['-', '+', 'U']))
    {
        return Err(invalid(
            "workspace_setup_incomplete: pinned submodules are not initialized at their recorded commits",
        ));
    }
    Ok(())
}
pub(super) fn prepare(r: &Record) -> Result<Record> {
    same_source(r)?;
    // No existence-based adoption. The durable identity and branch were
    // reserved before add; only restart inspect may recover their registration.
    if r.checkout_identity != Some(identity(&r.cwd)?)
        || fs::read_dir(&r.cwd)?.next().is_some()
        || run(
            &r.source.repository,
            &[
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{}", r.branch),
            ],
        )?
        .status
        .success()
    {
        return Err(invalid(
            "workspace_collision: reserved path or branch already exists",
        ));
    }
    checked(
        &r.source.repository,
        &[
            "worktree",
            "add",
            "-b",
            &r.branch,
            r.cwd
                .to_str()
                .ok_or_else(|| invalid("workspace_path_invalid"))?,
            &r.source.base,
        ],
    )?;
    let mut next = r.clone();
    next.checkout_identity = Some(identity(&r.cwd)?);
    // This is source materialization at the exact gitlink, not an arbitrary
    // shell setup recipe or a pull to a newer harness version.
    checked(
        &r.cwd,
        &["submodule", "update", "--init", "--recursive", "--checkout"],
    )?;
    verify(&next)?;
    if checked(&r.cwd, &["rev-parse", "HEAD"])? != r.source.base {
        return Err(invalid(
            "workspace_source_changed: prepared checkout does not match requested base",
        ));
    }
    next.state = State::Ready;
    next.error = None;
    Ok(next)
}
pub(super) fn inspect(r: &Record) -> Result<Record> {
    same_source(r)?;
    let mut next = r.clone();
    if r.state == State::Releasing {
        if !r.cwd.try_exists()? && !registered(r)? {
            next.state = State::Released;
            next.error = None;
            return Ok(next);
        }
        if r.cwd.try_exists()? && r.checkout_identity != Some(identity(&r.cwd)?) {
            return Err(invalid(
                "workspace_identity_changed: original release resource was replaced",
            ));
        }
        return Err(invalid(
            "workspace_release_incomplete: original provider lease is idle and resource remains; preserve its contents, then request workspace_release with the current revision and a new operation ID",
        ));
    }
    if !r.cwd.try_exists()? {
        return Err(invalid(
            "workspace_prepare_incomplete: no checkout found; original create was not repeated",
        ));
    }
    verify(r)?;
    if checked(&r.cwd, &["rev-parse", "HEAD"])? != r.source.base {
        return Err(invalid(
            "workspace_source_changed: unbound recovered checkout differs from requested base",
        ));
    }
    next.checkout_identity = Some(identity(&r.cwd)?);
    next.state = State::Ready;
    next.error = None;
    Ok(next)
}
pub(super) fn process_exists(pid: u32) -> bool {
    // PID reuse is a conservative hold, never permission to kill a process.
    unsafe {
        libc::kill(pid as i32, 0) == 0
            || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }
}
pub(super) fn release(r: &Record) -> Result<Record> {
    same_source(r)?;
    let mut next = r.clone();
    if !r.cwd.try_exists()? {
        if registered(r)? {
            return Err(invalid(
                "workspace_identity_changed: registered checkout path disappeared",
            ));
        }
        next.state = State::Released;
        next.error = None;
        return Ok(next);
    }
    if r.checkout_identity != Some(identity(&r.cwd)?) {
        return Err(invalid(
            "workspace_identity_changed: release cannot adopt an unclaimed checkout",
        ));
    }
    if !registered(r)? {
        // A known failed prepare may own only the empty directory reserved
        // before Git add. remove_dir refuses any private/partial data, and no
        // preexisting branch or administrative resource is removed.
        ensure_unused(&r.cwd)?;
        fs::remove_dir(&r.cwd).map_err(|_|invalid("workspace_preservation_unproven: unregistered partial checkout is not empty; preserve it before release"))?;
        next.state = State::Released;
        next.error = None;
        return Ok(next);
    }
    verify_identity(r)?;
    let status = checked(
        &r.cwd,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignored",
        ],
    )?;
    if !status.is_empty() {
        return Err(invalid(
            "workspace_dirty: preserve tracked, untracked and ignored private data before release",
        ));
    }
    let nested = checked(
        &r.cwd,
        &[
            "submodule",
            "foreach",
            "--quiet",
            "--recursive",
            "git status --porcelain=v1 --untracked-files=all --ignored",
        ],
    )?;
    if !nested.is_empty() {
        return Err(invalid(
            "workspace_dirty: preserve private/untracked files inside submodules before release",
        ));
    }
    let head = checked(&r.cwd, &["rev-parse", "HEAD"])?;
    let preserved = run(
        &r.source.repository,
        &[
            "merge-base",
            "--is-ancestor",
            &head,
            &r.source.integration_ref,
        ],
    )?
    .status
    .success();
    if !preserved {
        // Patch-id alone is not source preservation: git cherry omits merges
        // and ignores whitespace. Permit a rebased equivalent only when every
        // ordinary patch is represented AND the complete tracked tree matches;
        // unmatched merge commits always need explicit preservation.
        let merges = checked(
            &r.source.repository,
            &[
                "rev-list",
                "--merges",
                &format!("{}..{head}", r.source.integration_ref),
            ],
        )?;
        let trees = checked(
            &r.source.repository,
            &[
                "rev-parse",
                &format!("{head}^{{tree}}"),
                &format!("{}^{{tree}}", r.source.integration_ref),
            ],
        )?;
        let trees = trees.lines().collect::<Vec<_>>();
        if !merges.is_empty() || trees.len() != 2 || trees[0] != trees[1] {
            return Err(invalid(
                "workspace_unpublished_commits: unmatched commits/merges or source tree differ from integration_ref",
            ));
        }
        let cherry = checked(
            &r.source.repository,
            &["cherry", &r.source.integration_ref, &head],
        )?;
        if cherry.lines().any(|line| line.starts_with('+')) {
            return Err(invalid(
                "workspace_unpublished_commits: source commits are not preserved at integration_ref",
            ));
        }
    }
    // A live editor/build/harness with this directory open is not safe to
    // remove, even if its seat record is closed. No broad process kills.
    ensure_unused(&r.cwd)?;
    // Recheck identity and dirtiness at Git's own non-force removal boundary.
    verify_identity(r)?;
    // Deinitialize only after the same clean/preservation/open-file checks.
    // Git's non-force deinit refuses changed module work; no ignored/private
    // module data was omitted by the recursive status check above.
    checked(&r.cwd, &["submodule", "deinit", "--all"])?;
    // Git refuses even a clean/deinitialized worktree while its private module
    // repositories remain under worktrees/<id>/modules. Preserve that owned
    // Git data in the same common directory before non-force removal; no
    // branch/object/private module history is discarded to satisfy Git.
    next.preserved_module_repositories = preserve_modules(r)?;
    checked(
        &r.source.repository,
        &[
            "worktree",
            "remove",
            r.cwd
                .to_str()
                .ok_or_else(|| invalid("workspace_path_invalid"))?,
        ],
    )?;
    if r.cwd.try_exists()? || registered(r)? {
        return Err(invalid(
            "workspace_release_unknown: removal outcome not established",
        ));
    }
    // Keep the source branch as a preservation ref. Removing it is a separate
    // repository policy decision, never a side effect of closing a worker.
    next.state = State::Released;
    next.error = None;
    Ok(next)
}

fn preserve_modules(r: &Record) -> Result<Option<std::path::PathBuf>> {
    let admin = std::path::PathBuf::from(checked(&r.cwd, &["rev-parse", "--absolute-git-dir"])?)
        .canonicalize()?;
    let pointer =
        Path::new(std::fs::read_to_string(admin.join("gitdir"))?.trim()).canonicalize()?;
    if admin.parent() != Some(r.source.common.join("worktrees").as_path())
        || pointer != r.cwd.join(".git").canonicalize()?
    {
        return Err(invalid(
            "workspace_identity_changed: module metadata does not belong to this worktree",
        ));
    }
    let modules = admin.join("modules");
    let parent = r.source.common.join("bboxy-retained-workspace-modules");
    let destination = parent.join(&r.resource_token);
    let archive = destination.join("modules");
    if modules.try_exists()? {
        identity(&modules)?;
        match std::fs::create_dir(&parent) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
        identity(&parent)?;
        // A new per-resource directory is exclusive: no overwrite/adoption of
        // an unrelated existing archive, even if it has the requested name.
        std::fs::create_dir(&destination).map_err(|_| {
            invalid("workspace_identity_changed: retained module destination already exists")
        })?;
        std::fs::write(destination.join("workspace-id"), r.workspace_id.to_string())?;
        std::fs::rename(&modules, &archive)?;
        return Ok(Some(archive));
    }
    if archive.try_exists()? {
        identity(&parent)?;
        identity(&destination)?;
        identity(&archive)?;
        if std::fs::read_to_string(destination.join("workspace-id"))? != r.workspace_id.to_string()
        {
            return Err(invalid(
                "workspace_identity_changed: retained module archive ownership differs",
            ));
        }
        return Ok(Some(archive));
    }
    Ok(None)
}

fn ensure_unused(cwd: &Path) -> Result<()> {
    let mut lsof = Command::new("lsof");
    lsof.args(["-t", "+D"]).arg(cwd);
    let open = output(lsof, Duration::from_secs(20))
        .map_err(|_| invalid("workspace_preservation_unproven: process inspection unavailable"))?;
    if !open.stdout.is_empty() {
        return Err(invalid(
            "workspace_in_use: an external process still has the checkout open",
        ));
    }
    if open.status.code() != Some(1) || !open.stderr.is_empty() {
        return Err(invalid(
            "workspace_preservation_unproven: process inspection did not prove the directory unused",
        ));
    }
    Ok(())
}
