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
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fs,
    io::Read,
    os::unix::{ffi::OsStrExt, fs::MetadataExt, process::CommandExt},
    path::{Component, Path, PathBuf},
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
const OUTPUT_LIMIT: usize = 65536;

struct Capture {
    head: Vec<u8>,
    tail: VecDeque<u8>,
    digest: Sha256,
    bytes: usize,
}

impl Capture {
    fn new() -> Self {
        Self {
            head: Vec::new(),
            tail: VecDeque::new(),
            digest: Sha256::new(),
            bytes: 0,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        const HEAD: usize = OUTPUT_LIMIT / 2;
        // Reserve space for the truncation marker within the same byte bound.
        const TAIL: usize = OUTPUT_LIMIT - HEAD - 128;
        self.digest.update(chunk);
        self.bytes += chunk.len();

        if self.bytes <= OUTPUT_LIMIT {
            self.head.extend_from_slice(chunk);
            return;
        }

        if self.head.len() > HEAD {
            self.tail.extend(self.head.drain(HEAD..));
        }
        let take = chunk.len().min(HEAD - self.head.len());
        self.head.extend_from_slice(&chunk[..take]);
        let remaining = &chunk[take..];
        let remaining = &remaining[remaining.len().saturating_sub(TAIL)..];
        let drop = (self.tail.len() + remaining.len()).saturating_sub(TAIL);
        self.tail.drain(..drop);
        self.tail.extend(remaining);
    }

    fn finish(mut self) -> (Vec<u8>, bool) {
        let clipped = self.bytes > OUTPUT_LIMIT;
        if clipped {
            let marker = format!(
                "\n[{} bytes total, sha256:{:x}]\n",
                self.bytes,
                self.digest.finalize()
            );
            self.head.extend_from_slice(marker.as_bytes());
            self.head.extend(self.tail);
        }
        (self.head, clipped)
    }
}

fn output_with<T: Send + 'static>(
    mut command: Command,
    timeout: Duration,
    state: T,
    observe: impl FnMut(&mut T, &[u8]) -> std::io::Result<()> + Send + 'static,
) -> Result<(Output, bool, T)> {
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
    fn reader<T: Send + 'static>(
        mut pipe: impl Read + AsRawFd + Send + 'static,
        cancel: Arc<AtomicBool>,
        mut state: T,
        mut observe: impl FnMut(&mut T, &[u8]) -> std::io::Result<()> + Send + 'static,
    ) -> mpsc::Receiver<std::io::Result<(Capture, T)>> {
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
                let mut saved = Capture::new();
                let mut buf = [0; 8192];
                while !cancel.load(Ordering::Acquire) {
                    match pipe.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            observe(&mut state, &buf[..n])?;
                            saved.push(&buf[..n]);
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
                Ok((saved, state))
            })();
            let _ = tx.send(result);
        });
        rx
    }
    let out = reader(
        child.stdout.take().expect("piped stdout"),
        cancel.clone(),
        state,
        observe,
    );
    let err = reader(
        child.stderr.take().expect("piped stderr"),
        cancel.clone(),
        (),
        |_, _| Ok(()),
    );
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
            let (stdout, state) = stdout.take().unwrap();
            let (stdout, clipped) = stdout.finish();
            let (stderr, _) = stderr.take().unwrap().0.finish();
            return Ok((
                Output {
                    status,
                    stdout,
                    stderr,
                },
                clipped,
                state,
            ));
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

fn command(repo: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    command
}
const GIT_TIMEOUT: Duration = Duration::from_secs(120);

pub(super) fn output(command: Command, timeout: Duration) -> Result<(Output, bool)> {
    output_with(command, timeout, (), |_, _| Ok(())).map(|(out, clipped, ())| (out, clipped))
}

fn run(repo: &Path, args: &[&str]) -> Result<(Output, bool)> {
    output(command(repo, args), GIT_TIMEOUT)
}

fn success(out: &Output, args: &[&str]) -> Result<()> {
    if out.status.success() {
        return Ok(());
    }
    Err(invalid(format!(
        "workspace_git_failed: git {} failed ({}); verify the requested source, setup and repository credentials",
        args.first().copied().unwrap_or("command"),
        out.status
    )))
}

fn checked(repo: &Path, args: &[&str]) -> Result<String> {
    let (out, clipped) = run(repo, args)?;
    success(&out, args)?;
    if clipped {
        return Err(invalid(
            "workspace_preservation_unproven: Git result exceeded the complete-result bound",
        ));
    }
    String::from_utf8(out.stdout)
        .map(|s| s.trim().to_owned())
        .map_err(Into::into)
}

fn records<T: Send + 'static>(
    repo: &Path,
    args: &[&str],
    state: T,
    mut record: impl FnMut(&mut T, &[u8]) + Send + 'static,
) -> Result<T> {
    let (out, _, (pending, state)) = output_with(
        command(repo, args),
        GIT_TIMEOUT,
        (Vec::new(), state),
        move |(pending, state), mut chunk| {
            while let Some(end) = chunk.iter().position(|&b| b == 0) {
                if pending.len() + end > OUTPUT_LIMIT {
                    return Err(std::io::Error::other("oversized Git record"));
                }
                pending.extend_from_slice(&chunk[..end]);
                record(state, pending);
                pending.clear();
                chunk = &chunk[end + 1..];
            }
            pending.extend_from_slice(chunk);
            if pending.len() > OUTPUT_LIMIT {
                return Err(std::io::Error::other("oversized Git record"));
            }
            Ok(())
        },
    )?;
    success(&out, args)?;
    if !pending.is_empty() {
        return Err(invalid(
            "workspace_preservation_unproven: unterminated Git record",
        ));
    }
    Ok(state)
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
/// Every checkout Git knows for `repo`, main checkout first (porcelain order).
pub(super) fn worktrees(repo: &Path) -> Result<Vec<std::path::PathBuf>> {
    let list = checked(repo, &["worktree", "list", "--porcelain"])?;
    Ok(list
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(std::path::PathBuf::from)
        .collect())
}
fn registered(r: &Record) -> Result<bool> {
    let expected = [b"worktree ", r.cwd.as_os_str().as_encoded_bytes()].concat();
    let branch = format!("branch refs/heads/{}", r.branch);
    let (_, found) = records(
        &r.source.repository,
        &["worktree", "list", "--porcelain", "-z"],
        (false, false),
        move |(ours, found), row| {
            if row.is_empty() || row.starts_with(b"worktree ") {
                *ours = row == expected;
            }
            *found |= *ours && row == branch.as_bytes();
        },
    )?;
    Ok(found)
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
        .0
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
// Git's matching mode reports an ignored directory once, even when it
// contains thousands of files. Keep all other ignored paths private.
fn disposable(path: &Path) -> bool {
    if path == Path::new("verification/evidence") {
        return true;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        "target" | "dist" | "node_modules" | ".wrangler" | ".cache" | "__pycache__"
    ) || name.starts_with(".chrome-")
}

fn verify_clean(repo: &Path) -> Result<()> {
    let (dirty, outputs) = records(
        repo,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=normal",
            "--ignored=matching",
            "--ignore-submodules=none",
            "--no-renames",
        ],
        (false, Vec::new()),
        |(dirty, outputs), row| {
            if *dirty {
                return;
            }
            if let Some(name) = row
                .strip_prefix(b"!! ")
                .and_then(|path| path.strip_suffix(b"/"))
                && disposable(Path::new(std::ffi::OsStr::from_bytes(name)))
            {
                outputs.push(name.to_vec());
            } else {
                *dirty = true;
            }
        },
    )?;
    let private = || {
        invalid(
            "workspace_dirty: preserve tracked, untracked and ignored private data before release",
        )
    };
    if dirty {
        return Err(private());
    }
    for path in outputs {
        let path = std::str::from_utf8(&path)?;
        let rule = checked(
            repo,
            &[
                "-c",
                "core.quotePath=false",
                "check-ignore",
                "-v",
                "--",
                &format!("./{path}"),
            ],
        )?;
        let Some((rule, _)) = rule.split_once('\t') else {
            return Err(private());
        };
        let mut rule = rule.rsplitn(3, ':');
        let (Some(pattern), Some(_), Some(source)) = (rule.next(), rule.next(), rule.next()) else {
            return Err(private());
        };
        let name = Path::new(path).file_name().and_then(|name| name.to_str());
        let pattern = pattern.trim_end_matches('/').rsplit('/').next();
        let names_output = pattern == name
            || (pattern == Some(".chrome-*")
                && name.is_some_and(|name| name.starts_with(".chrome-")));
        let source = Path::new(source);
        if !names_output
            || source.file_name().is_none_or(|name| name != ".gitignore")
            || !source
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(private());
        }
        let source = source.to_str().ok_or_else(private)?;
        let (track, _) = run(
            repo,
            &[
                "--literal-pathspecs",
                "ls-files",
                "--error-unmatch",
                "--",
                source,
            ],
        )?;
        if !track.status.success() {
            return Err(private());
        }
    }
    Ok(())
}

fn verify_clean_tree(repo: &Path) -> Result<()> {
    verify_clean(repo)?;
    let modules = records(
        repo,
        &[
            "submodule",
            "foreach",
            "--quiet",
            "--recursive",
            r#"printf '%s\0' "$displaypath""#,
        ],
        Vec::new(),
        |modules, path| modules.push(PathBuf::from(std::ffi::OsStr::from_bytes(path))),
    )?;
    for path in modules {
        if path.as_os_str().is_empty()
            || !path
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(invalid(
                "workspace_preservation_unproven: unexpected submodule path",
            ));
        }
        verify_clean(&repo.join(path))?;
    }
    Ok(())
}

fn ancestor(repo: &Path, old: &str, new: &str) -> Result<bool> {
    let (out, _) = run(repo, &["merge-base", "--is-ancestor", old, new])?;
    Ok(out.status.success())
}

fn matching_tree(repo: &Path, integration: &str, commit: &str) -> Result<bool> {
    let format = "--format=%T %s";
    let (out, clipped) = run(
        repo,
        &["show", "-s", "--no-show-signature", "-z", format, commit],
    )?;
    success(&out, &["show"])?;
    let signature = out.stdout.strip_suffix(b"\0").ok_or_else(|| {
        invalid("workspace_preservation_unproven: unexpected Git commit signature record")
    })?;
    if clipped {
        return Err(invalid(
            "workspace_preservation_unproven: oversized Git commit signature record",
        ));
    }
    let signature = signature.to_vec();
    records(
        repo,
        &["log", "--no-show-signature", "-z", format, integration],
        false,
        move |found, row| *found |= row == signature,
    )
}

fn preserved_pointers(r: &Record, integration: &str, commit: &str) -> Result<bool> {
    let repo = &r.source.repository;
    let parents = checked(repo, &["show", "-s", "--format=%P", commit])?;
    if parents.split_whitespace().count() != 1 {
        return Ok(false);
    }
    let (_, pointers) = records(
        repo,
        &[
            "diff-tree",
            "-r",
            "-z",
            "--no-abbrev",
            "--no-commit-id",
            "--no-renames",
            commit,
        ],
        (None, Some(Vec::new())),
        |(header, pointers), row| {
            if pointers.is_none() {
                return;
            }
            if let Some(pin) = header.take() {
                pointers.as_mut().unwrap().push((pin, row.to_vec()));
            } else if row.starts_with(b":160000 160000 ")
                && let Some(pin) = row.split(|&b| b == b' ').nth(3)
            {
                *header = Some(pin.to_vec());
            } else {
                *pointers = None;
            }
        },
    )?;
    let Some(pointers) = pointers.filter(|pointers| !pointers.is_empty()) else {
        return Ok(false);
    };
    for (pin, path) in pointers {
        let pin = std::str::from_utf8(&pin)?;
        let path = std::str::from_utf8(&path)?;
        let (entry, clipped) = run(
            repo,
            &[
                "--literal-pathspecs",
                "ls-tree",
                "-z",
                integration,
                "--",
                path,
            ],
        )?;
        if !entry.status.success() || clipped || !entry.stdout.starts_with(b"160000 commit ") {
            return Ok(false);
        }
        let Some((pin_field, _)) = std::str::from_utf8(&entry.stdout)?.split_once('\t') else {
            return Ok(false);
        };
        let landed = pin_field.trim_start_matches("160000 commit ");
        if pin == landed {
            continue;
        }
        let mut preserved = false;
        for checkout in [&r.cwd, repo] {
            let module = checkout.join(path);
            if module.join(".git").try_exists()? && ancestor(&module, pin, landed)? {
                preserved = true;
                break;
            }
        }
        if !preserved {
            return Ok(false);
        }
    }
    Ok(true)
}

fn verify_preserved(r: &Record) -> Result<()> {
    let repo = &r.source.repository;
    let head = checked(&r.cwd, &["rev-parse", "HEAD"])?;
    let integration = checked(
        repo,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{commit}}", r.source.integration_ref),
        ],
    )?;
    if ancestor(repo, &head, &integration)? {
        return Ok(());
    }

    let range = format!("{integration}...{head}");
    const PAGE: usize = 128;
    let mut skip = 0;
    loop {
        let commits = checked(
            repo,
            &[
                "rev-list",
                "--right-only",
                "--cherry-pick",
                &format!("--max-count={PAGE}"),
                &format!("--skip={skip}"),
                &range,
            ],
        )?;
        for commit in commits.lines() {
            if !matching_tree(repo, &integration, commit)?
                && !preserved_pointers(r, &integration, commit)?
            {
                return Err(invalid(format!(
                    "workspace_unpublished_commits: {commit} is not preserved at integration_ref",
                )));
            }
        }
        if commits.lines().count() < PAGE {
            return Ok(());
        }
        skip += PAGE;
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
    verify_clean_tree(&r.cwd)?;
    verify_preserved(r)?;
    // A live editor/build/harness with this directory open is not safe to
    // remove, even if its seat record is closed. No broad process kills.
    ensure_unused(&r.cwd)?;
    // Only tracked-ignore build directories passed the private data check.
    // Git's non-force operations remove them with the rest of the checkout.
    verify_identity(r)?;
    verify_clean_tree(&r.cwd)?;
    let deinit = ["submodule", "deinit", "--all"];
    let (out, _) = run(&r.cwd, &deinit)?;
    success(&out, &deinit)?;
    // Git refuses even a clean/deinitialized worktree while its private module
    // repositories remain under worktrees/<id>/modules. Preserve that owned
    // Git data in the same common directory before non-force removal; no
    // branch/object/private module history is discarded to satisfy Git.
    next.preserved_module_repositories = preserve_modules(r)?;
    let remove = [
        "worktree",
        "remove",
        r.cwd
            .to_str()
            .ok_or_else(|| invalid("workspace_path_invalid"))?,
    ];
    let (out, _) = run(&r.source.repository, &remove)?;
    success(&out, &remove)?;
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
    // A process's cwd is a checkout handle even when it has no files open.
    let mut lsof = Command::new("lsof");
    lsof.args(["-t", "+D"]).arg(cwd);
    let (open, _) = output(lsof, Duration::from_secs(20))
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
