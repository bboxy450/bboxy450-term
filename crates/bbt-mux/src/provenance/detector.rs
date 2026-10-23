//! Bounded filesystem hints and full-file hashing. No content enters an event.
use super::artifacts::Attribution;
use crate::{
    Ulid,
    store::{Result, invalid, now_ms},
};
use notify::{
    RecursiveMode, Watcher,
    event::{ModifyKind, RenameMode},
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::CString,
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

pub(crate) const PENDING: usize = 4096;
pub(crate) const HASH_JOBS: usize = 4;
pub(crate) const MAX_FILE: u64 = 64 * 1024 * 1024;

#[derive(Clone)]
struct Root {
    path: PathBuf,
    prefix: String,
    dir: Arc<File>,
}
#[derive(Clone)]
struct Binding {
    generation: u64,
    root: usize,
    cwd: PathBuf,
    alias: PathBuf,
    root_alias: Option<PathBuf>,
}
#[derive(Clone)]
struct Signal {
    root: usize,
    path: PathBuf,
    previous: Option<PathBuf>,
    kind: String,
    source: String,
    reported: Option<(Ulid, u64)>,
    at: u64,
}
#[derive(Default)]
struct Inbox {
    queue: VecDeque<Signal>,
    lost: u64,
}
impl Inbox {
    fn push(&mut self, signal: Signal) {
        if signal.kind == "modified"
            && let Some(old) = self.queue.iter_mut().find(|old| {
                old.root == signal.root && old.path == signal.path && old.kind == "modified"
            })
        {
            // Preserve a reported source when a later generic FS hint coalesces.
            if signal.reported.is_some() || old.reported.is_none() {
                *old = signal;
            }
            return;
        }
        if self.queue.len() >= PENDING {
            self.lost = self.lost.saturating_add(1);
        } else {
            self.queue.push_back(signal);
        }
    }
}
#[derive(Debug)]
pub(crate) struct Hashed {
    pub commit: Option<String>,
    pub reference: String,
    pub previous_ref: Option<String>,
    pub kind: String,
    pub source: String,
    pub attribution: Attribution,
    pub digest: Option<String>,
    pub bytes: Option<u64>,
    pub reason: Option<String>,
    pub at: u64,
}

enum Finished {
    File(Hashed),
    Git(
        usize,
        std::result::Result<Option<super::git::Snapshot>, String>,
        u64,
    ),
}

pub(crate) struct Detector {
    primary: PathBuf,
    roots: Vec<Root>,
    bindings: BTreeMap<Ulid, Binding>,
    watchers: Vec<notify::RecommendedWatcher>,
    inbox: Arc<Mutex<Inbox>>,
    jobs: tokio::task::JoinSet<Finished>,
    git_state: BTreeMap<usize, super::git::Snapshot>,
    git_at: std::time::Instant,
    git_cursor: usize,
    git_failed: std::collections::BTreeSet<usize>,
    git_job: Option<tokio::task::Id>,
    in_flight: BTreeMap<String, tokio::task::Id>,
    handle: tokio::runtime::Handle,
    gaps: VecDeque<String>,
}
fn ignored(relative: &Path) -> bool {
    if relative.file_name().is_some_and(|name| {
        name.to_str()
            .is_some_and(|name| name.starts_with('.') && name.ends_with(".tmp"))
    }) {
        return true;
    }
    let parts: Vec<_> = relative
        .components()
        .filter_map(|c| match c {
            Component::Normal(p) => p.to_str(),
            _ => None,
        })
        .collect();
    parts.iter().any(|p| {
        matches!(
            *p,
            ".git" | "target" | "node_modules" | ".gnupg" | ".ssh" | ".icq" | ".claude" | ".codex"
        )
    }) || parts
        .windows(2)
        .any(|w| w[0] == ".bboxy" && matches!(w[1], "log" | "artifacts"))
}
fn relative(root: &Path, path: &Path) -> Option<PathBuf> {
    let relative = path.strip_prefix(root).ok()?;
    if relative.as_os_str().is_empty()
        || relative.as_os_str().len() > 4096
        || relative.to_str().is_none()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || ignored(relative)
    {
        return None;
    }
    Some(relative.to_owned())
}
fn locate(root: &Root, path: &Path) -> Option<String> {
    let path = path.to_str()?;
    Some(format!("{}{path}", root.prefix))
}
fn open_regular(root: &Root, path: &Path) -> Result<File> {
    let parts: Vec<_> = path
        .components()
        .map(|p| match p {
            Component::Normal(p) => Ok(p),
            _ => Err(invalid("non-normal artifact path")),
        })
        .collect::<Result<_>>()?;
    let (name, parents) = parts
        .split_last()
        .ok_or_else(|| invalid("empty artifact path"))?;
    let mut directory = root.dir.try_clone()?;
    for component in parents {
        let component = CString::new(component.as_bytes())?;
        // SAFETY: owned directory FD and NUL-terminated single path component.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                component.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: openat returned a new owned FD.
        directory = unsafe { File::from_raw_fd(fd) };
    }
    let name = CString::new(name.as_bytes())?;
    // SAFETY: final component cannot traverse a symlink; nonblocking avoids FIFOs.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: openat returned a new owned FD.
    let file = unsafe { File::from_raw_fd(fd) };
    if !file.metadata()?.is_file() {
        return Err(invalid("artifact is not a regular file"));
    }
    Ok(file)
}
fn fingerprint(meta: &std::fs::Metadata) -> (u64, u64, u64, i64, i64) {
    (
        meta.dev(),
        meta.ino(),
        meta.len(),
        meta.mtime(),
        meta.mtime_nsec(),
    )
}
fn hash(root: Root, signal: Signal, attribution: Attribution) -> Hashed {
    let mut result = Hashed {
        commit: None,
        reference: locate(&root, &signal.path).unwrap(),
        previous_ref: signal.previous.as_ref().and_then(|p| locate(&root, p)),
        kind: signal.kind,
        source: signal.source,
        attribution,
        digest: None,
        bytes: None,
        reason: None,
        at: signal.at,
    };
    if result.kind == "deleted" {
        return result;
    }
    let read = (|| -> Result<Option<(String, u64)>> {
        let file = open_regular(&root, &signal.path)?;
        let before = file.metadata()?;
        if before.len() > MAX_FILE {
            result.bytes = Some(before.len());
            return Ok(None);
        }
        let mut hash = Sha256::new();
        let mut bytes = 0;
        let mut reader = &file;
        let mut buffer = [0; 65536];
        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            bytes += n as u64;
            if bytes > MAX_FILE {
                return Ok(None);
            }
            hash.update(&buffer[..n]);
        }
        if fingerprint(&before) != fingerprint(&file.metadata()?) {
            return Err(invalid("artifact changed while hashing"));
        }
        Ok(Some((format!("sha256:{:x}", hash.finalize()), bytes)))
    })();
    match read {
        Ok(Some((digest, bytes))) => {
            result.digest = Some(digest);
            result.bytes = Some(bytes);
        }
        Ok(None) => result.reason = Some("artifact-digest-size-bound".into()),
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
        {
            result.kind = "deleted".into();
        }
        Err(_) => result.reason = Some("artifact-path-unavailable-or-changed".into()),
    }
    result
}

fn bounded_gap(queue: &mut VecDeque<String>, reason: String) {
    if queue.len() < 16 && !queue.contains(&reason) {
        queue.push_back(reason);
    }
}

impl Detector {
    pub fn new(repository: &Path, handle: tokio::runtime::Handle) -> Result<Self> {
        let primary = repository.canonicalize()?;
        let mut detector = Self {
            primary: primary.clone(),
            roots: vec![],
            bindings: BTreeMap::new(),
            watchers: vec![],
            inbox: Arc::new(Mutex::new(Inbox::default())),
            jobs: tokio::task::JoinSet::new(),
            git_state: BTreeMap::new(),
            git_at: std::time::Instant::now(),
            git_cursor: 0,
            git_failed: std::collections::BTreeSet::new(),
            git_job: None,
            in_flight: BTreeMap::new(),
            handle,
            gaps: VecDeque::new(),
        };
        if detector.add_root(&primary).is_err() {
            bounded_gap(&mut detector.gaps, "artifact-watch-root-unavailable".into());
        }
        Ok(detector)
    }
    fn add_root(&mut self, path: &Path) -> Result<usize> {
        if let Some(id) = self.roots.iter().position(|root| root.path == path) {
            return Ok(id);
        }
        if self.roots.len() >= 128 {
            return Err(invalid("artifact watch root budget exceeded"));
        }
        if path == Path::new("/")
            || std::env::var_os("HOME")
                .is_some_and(|home| Path::new(&home).canonicalize().ok().as_deref() == Some(path))
        {
            return Err(invalid("home-wide artifact watch is unavailable"));
        }
        use std::os::unix::fs::OpenOptionsExt;
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let id = self.roots.len();
        let prefix = if path == self.primary {
            String::new()
        } else {
            format!(
                "worktree:{:x}/",
                Sha256::digest(path.as_os_str().as_bytes())
            )
        };
        let root = Root {
            path: path.to_owned(),
            prefix,
            dir: Arc::new(directory),
        };
        let inbox = self.inbox.clone();
        let observed = root.path.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                let mut queue = inbox.lock().unwrap_or_else(|e| e.into_inner());
                let Ok(event) = result else {
                    queue.lost = queue.lost.saturating_add(1);
                    return;
                };
                if event.need_rescan() {
                    queue.lost = queue.lost.saturating_add(1);
                }
                let kind = match event.kind {
                    notify::EventKind::Create(notify::event::CreateKind::Folder)
                    | notify::EventKind::Remove(notify::event::RemoveKind::Folder) => return,
                    notify::EventKind::Create(_) => "created",
                    notify::EventKind::Remove(_) => "deleted",
                    notify::EventKind::Modify(ModifyKind::Name(RenameMode::From)) => "deleted",
                    notify::EventKind::Modify(ModifyKind::Name(RenameMode::To)) => "created",
                    notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both))
                        if event.paths.len() == 2 =>
                    {
                        "renamed"
                    }
                    notify::EventKind::Modify(_) => "modified",
                    _ => return,
                };
                if kind == "renamed" {
                    let from = relative(&observed, &event.paths[0]);
                    let to = relative(&observed, &event.paths[1]);
                    let change = match (from, to) {
                        (Some(previous), Some(path)) => Some((path, Some(previous), "renamed")),
                        (Some(path), None) => Some((path, None, "deleted")),
                        (None, Some(path)) => Some((path, None, "created")),
                        (None, None) => None,
                    };
                    if let Some((path, previous, kind)) = change {
                        queue.push(Signal {
                            root: id,
                            path,
                            previous,
                            kind: kind.into(),
                            source: "filesystem".into(),
                            reported: None,
                            at: now_ms(),
                        });
                    }
                } else {
                    if event.paths.len() > 128 {
                        queue.lost = queue.lost.saturating_add((event.paths.len() - 128) as u64);
                    }
                    for path in event
                        .paths
                        .into_iter()
                        .take(128)
                        .filter_map(|p| relative(&observed, &p))
                    {
                        queue.push(Signal {
                            root: id,
                            path,
                            previous: None,
                            kind: kind.into(),
                            source: "filesystem".into(),
                            reported: None,
                            at: now_ms(),
                        });
                    }
                }
            })?;
        watcher.watch(path, RecursiveMode::Recursive)?;
        self.watchers.push(watcher);
        self.roots.push(root);
        Ok(id)
    }
    pub fn register(&mut self, seat: Ulid, generation: u64, cwd: &Path, local: bool) -> Result<()> {
        if !local {
            return Ok(());
        }
        if ignored(cwd) {
            return Err(invalid("artifact cwd is excluded"));
        }
        let alias = if cwd.is_absolute() {
            cwd.to_owned()
        } else {
            std::env::current_dir()?.join(cwd)
        };
        let cwd = cwd.canonicalize()?;
        let path = if cwd.starts_with(&self.primary) {
            self.primary.clone()
        } else {
            cwd.ancestors()
                .take(32)
                .find(|path| {
                    path.join(".git")
                        .symlink_metadata()
                        .is_ok_and(|m| !m.file_type().is_symlink() && (m.is_file() || m.is_dir()))
                })
                .ok_or_else(|| invalid("artifact cwd is not a registered repository/worktree"))?
                .to_owned()
        };
        let root = self.add_root(&path)?;
        let depth = cwd.strip_prefix(&path)?.components().count();
        let root_alias = alias
            .ancestors()
            .nth(depth)
            .filter(|candidate| candidate.canonicalize().ok().as_deref() == Some(path.as_path()))
            .map(Path::to_owned);
        self.bindings.insert(
            seat,
            Binding {
                generation,
                root,
                cwd,
                alias,
                root_alias,
            },
        );
        Ok(())
    }
    pub fn normalize(&self, seat: Ulid, generation: u64, reference: &str) -> Option<String> {
        if reference.contains("://") || reference.starts_with("git:") {
            return None;
        }
        let binding = self
            .bindings
            .get(&seat)
            .filter(|b| b.generation == generation)?;
        let root = &self.roots[binding.root];
        if !root.prefix.is_empty()
            && let Some(path) = reference.strip_prefix(&root.prefix)
        {
            return relative(&root.path, &root.path.join(path)).and_then(|p| locate(root, &p));
        }
        if reference.split('/').next().is_some_and(|p| p.contains(':')) {
            return None;
        }
        let path = Path::new(reference);
        let absolute = if path.is_absolute() {
            if let Some(relative) = binding
                .root_alias
                .as_ref()
                .and_then(|alias| path.strip_prefix(alias).ok())
            {
                root.path.join(relative)
            } else if let Ok(relative) = path.strip_prefix(&binding.alias) {
                binding.cwd.join(relative)
            } else {
                path.to_owned()
            }
        } else {
            binding.cwd.join(path)
        };
        relative(&root.path, &absolute).and_then(|p| locate(root, &p))
    }
    pub fn owns(&self, seat: Ulid, generation: u64, reference: &str) -> bool {
        let Some(binding) = self
            .bindings
            .get(&seat)
            .filter(|b| b.generation == generation)
        else {
            return false;
        };
        let root = &self.roots[binding.root];
        if root.prefix.is_empty() && reference.starts_with("worktree:") {
            return false;
        }
        let Some(path) = reference.strip_prefix(&root.prefix) else {
            return false;
        };
        if path.split('/').next().is_some_and(|p| p.contains(':')) {
            return false;
        }
        relative(&root.path, &root.path.join(path)).is_some()
    }
    pub fn hook(&mut self, seat: Ulid, generation: u64, event: &crate::adapters::hook::ShimEvent) {
        if !matches!(event.event.as_str(), "PostToolUse" | "post_tool_use")
            || !matches!(
                event.raw["tool_name"].as_str(),
                Some("Write" | "Edit" | "MultiEdit" | "write_file" | "edit_file")
            )
        {
            return;
        }
        let Some(path) = event.raw["tool_input"]["file_path"]
            .as_str()
            .or_else(|| event.raw["tool_input"]["path"].as_str())
        else {
            return;
        };
        let Some(binding) = self
            .bindings
            .get(&seat)
            .filter(|b| b.generation == generation)
        else {
            return;
        };
        let root = &self.roots[binding.root];
        let path = Path::new(path);
        let absolute = if path.is_absolute() {
            if let Some(relative) = binding
                .root_alias
                .as_ref()
                .and_then(|alias| path.strip_prefix(alias).ok())
            {
                root.path.join(relative)
            } else if let Ok(relative) = path.strip_prefix(&binding.alias) {
                binding.cwd.join(relative)
            } else {
                path.to_owned()
            }
        } else {
            binding.cwd.join(path)
        };
        let Some(path) = relative(&root.path, &absolute) else {
            bounded_gap(
                &mut self.gaps,
                "artifact-hook-path-outside-root-or-excluded".into(),
            );
            return;
        };
        self.inbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Signal {
                root: binding.root,
                path,
                previous: None,
                kind: "modified".into(),
                source: "harness/tool".into(),
                reported: Some((seat, generation)),
                at: now_ms(),
            });
    }
    pub fn osc(&mut self, seat: Ulid, generation: u64, params: &[Vec<u8>]) {
        use base64::Engine;
        if params.first().map(Vec::as_slice) != Some(b"7502") {
            return;
        }
        let Some(text) = params.get(1).and_then(|p| std::str::from_utf8(p).ok()) else {
            return;
        };
        let pairs: BTreeMap<_, _> = text.split(':').filter_map(|p| p.split_once('=')).collect();
        if pairs.get("v") != Some(&"1") {
            return;
        }
        let Some(data) = pairs
            .get("d")
            .filter(|d| d.len() <= crate::adapters::hook::MAX_RAW * 2)
            .and_then(|d| base64::engine::general_purpose::STANDARD.decode(d).ok())
        else {
            return;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&data) else {
            return;
        };
        let event = crate::adapters::hook::ShimEvent {
            event: pairs.get("ev").unwrap_or(&"").to_string(),
            raw: value["raw"].clone(),
        };
        self.hook(seat, generation, &event);
    }
    pub fn reconcile_known(&mut self, references: impl IntoIterator<Item = String>) {
        let mut inbox = self.inbox.lock().unwrap_or_else(|e| e.into_inner());
        let mut clipped = false;
        for reference in references {
            if inbox.queue.len() >= PENDING {
                clipped = true;
                break;
            }
            if reference.contains("://") || reference.starts_with("git:") {
                continue;
            }
            let root = self
                .roots
                .iter()
                .enumerate()
                .filter(|(_, root)| {
                    (!root.prefix.is_empty() && reference.starts_with(&root.prefix))
                        || (root.prefix.is_empty() && !reference.starts_with("worktree:"))
                })
                .max_by_key(|(_, root)| root.prefix.len());
            if let Some((id, root)) = root {
                let path = PathBuf::from(reference.strip_prefix(&root.prefix).unwrap());
                if path
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                    || ignored(&path)
                {
                    continue;
                }
                if inbox
                    .queue
                    .iter()
                    .any(|signal| signal.root == id && signal.path == path)
                {
                    continue;
                }
                let kind = if root.path.join(&path).symlink_metadata().is_ok() {
                    "modified"
                } else {
                    "deleted"
                };
                inbox.push(Signal {
                    root: id,
                    path,
                    previous: None,
                    kind: kind.into(),
                    source: "filesystem/reconcile".into(),
                    reported: None,
                    at: now_ms(),
                });
            }
        }
        drop(inbox);
        if clipped {
            bounded_gap(
                &mut self.gaps,
                "artifact-reconcile-capacity: additional known paths remain unverified".into(),
            );
        }
    }
    /// Four IO jobs total, one Git snapshot at a time. A completed Git snapshot
    /// yields at most 128 changes plus an explicit gap; files add at most four.
    pub fn poll(&mut self, active: &[(Ulid, u64, u64)]) -> (Vec<Hashed>, Vec<String>) {
        let mut completed = Vec::new();
        while let Some(job) = self.jobs.try_join_next_with_id() {
            let id = match &job {
                Ok((id, _)) => *id,
                Err(error) => error.id(),
            };
            self.in_flight.retain(|_, task| *task != id);
            if self.git_job == Some(id) {
                self.git_job = None;
            }
            match job {
                Ok((_, Finished::File(row))) => completed.push(row),
                Ok((_, Finished::Git(root, result, at))) => match result {
                    Ok(Some(snapshot)) => {
                        self.git_failed.remove(&root);
                        if snapshot.clipped {
                            bounded_gap(
                                &mut self.gaps,
                                "git-ref-scan-bound: retained first128 refs".into(),
                            );
                        }
                        if let Some(old) = self.git_state.insert(root, snapshot.clone()) {
                            let prefix = self.roots[root].prefix.clone();
                            let mut attribution = Attribution::inferred(vec![]);
                            if let Some(seat) = snapshot.trailer {
                                attribution.candidates.push(seat);
                                attribution.reason = Some("git-trailer-unverified".into());
                            }
                            if snapshot.head != old.head
                                && let Some(commit) = snapshot.head.clone()
                            {
                                completed.push(Hashed {
                                    commit: Some(commit.clone()),
                                    reference: format!("git:{prefix}commit/{commit}"),
                                    previous_ref: None,
                                    kind: "committed".into(),
                                    source: "git/commit".into(),
                                    attribution: attribution.clone(),
                                    digest: snapshot.digest.clone(),
                                    bytes: None,
                                    reason: Some(
                                        "observed HEAD change; commit authorship unverified".into(),
                                    ),
                                    at,
                                });
                            }
                            for name in snapshot
                                .refs
                                .keys()
                                .chain(old.refs.keys())
                                .collect::<std::collections::BTreeSet<_>>()
                            {
                                if snapshot.refs.get(name) == old.refs.get(name) {
                                    continue;
                                }
                                let target = snapshot.refs.get(name);
                                completed.push(Hashed {
                                    commit: target.cloned(),
                                    reference: format!("git:{prefix}{name}"),
                                    previous_ref: None,
                                    kind: if target.is_none() {
                                        "deleted"
                                    } else if old.refs.contains_key(name) {
                                        "modified"
                                    } else {
                                        "created"
                                    }
                                    .into(),
                                    source: "git/ref".into(),
                                    attribution: attribution.clone(),
                                    digest: target.map(|value| {
                                        format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
                                    }),
                                    bytes: None,
                                    reason: Some("git-ref-value; authorship unverified".into()),
                                    at,
                                });
                                if completed.len() >= 128 {
                                    bounded_gap(&mut self.gaps, "git-ref-change-bound".into());
                                    break;
                                }
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(_) => {
                        if self.git_failed.insert(root) {
                            bounded_gap(&mut self.gaps, "git-observation-unavailable".into());
                        }
                    }
                },
                Err(_) => bounded_gap(&mut self.gaps, "artifact-hash-job-lost".into()),
            }
        }
        if self.jobs.len() < HASH_JOBS
            && self.git_job.is_none()
            && !self.roots.is_empty()
            && self.git_at.elapsed() >= std::time::Duration::from_secs(1)
        {
            let root = self.git_cursor % self.roots.len();
            self.git_cursor = self.git_cursor.wrapping_add(1);
            self.git_at = std::time::Instant::now();
            if self.roots[root]
                .path
                .join(".git")
                .symlink_metadata()
                .is_ok()
            {
                let path = self.roots[root].path.clone();
                let task = self.jobs.spawn_blocking_on(
                    move || Finished::Git(root, super::git::Snapshot::load(&path), now_ms()),
                    &self.handle,
                );
                self.git_job = Some(task.id());
            }
        }
        let mut inbox = self.inbox.lock().unwrap_or_else(|e| e.into_inner());
        if inbox.lost != 0 {
            bounded_gap(
                &mut self.gaps,
                format!(
                    "artifact-watch-overflow: {} hints; reconciliation required",
                    inbox.lost
                ),
            );
            inbox.lost = 0;
        }
        let mut budget = inbox.queue.len().min(64);
        while self.jobs.len() < HASH_JOBS && budget > 0 {
            budget -= 1;
            let Some(signal) = inbox.queue.pop_front() else {
                break;
            };
            let root = self.roots[signal.root].clone();
            let reference = locate(&root, &signal.path).unwrap();
            let previous = signal
                .previous
                .as_ref()
                .and_then(|path| locate(&root, path));
            if self.in_flight.contains_key(&reference)
                || previous
                    .as_ref()
                    .is_some_and(|reference| self.in_flight.contains_key(reference))
            {
                inbox.queue.push_back(signal);
                continue;
            }

            let attribution = if let Some((seat, generation)) = signal.reported {
                Attribution::reported(seat, generation)
            } else {
                Attribution::inferred(
                    active
                        .iter()
                        .filter(|(seat, generation, at)| {
                            signal.at.abs_diff(*at) <= 2000
                                && self.bindings.get(seat).is_some_and(|binding| {
                                    binding.generation == *generation && binding.root == signal.root
                                })
                        })
                        .map(|(seat, generation, _)| (*seat, *generation))
                        .collect(),
                )
            };
            let job = self.jobs.spawn_blocking_on(
                move || Finished::File(hash(root, signal, attribution)),
                &self.handle,
            );
            self.in_flight.insert(reference, job.id());
            if let Some(previous) = previous {
                self.in_flight.insert(previous, job.id());
            }
        }
        let gaps = self.gaps.drain(..self.gaps.len().min(16)).collect();
        (completed, gaps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn signal(path: String) -> Signal {
        Signal {
            root: 0,
            path: path.into(),
            previous: None,
            kind: "modified".into(),
            source: "filesystem".into(),
            reported: None,
            at: 1,
        }
    }
    #[test]
    fn hint_overflow_is_bounded_and_reported_hint_survives_coalescing() {
        let mut inbox = Inbox::default();
        let seat = Ulid::new();
        let mut reported = signal("reported".into());
        reported.reported = Some((seat, 7));
        reported.source = "harness/tool".into();
        inbox.push(reported);
        inbox.push(signal("reported".into()));
        assert_eq!(inbox.queue[0].reported, Some((seat, 7)));
        for n in 0..PENDING + 10 {
            inbox.push(signal(format!("file-{n}")));
        }
        assert_eq!(inbox.queue.len(), PENDING);
        assert_eq!(inbox.lost, 11);
        assert_eq!(
            Attribution::inferred(vec![(seat, 7), (Ulid::new(), 1)]).seat,
            None
        );
    }
    #[test]
    fn root_fd_refuses_symlink_parents_and_preserves_full_digest_semantics() {
        let temp = tempfile::tempdir().unwrap();
        let root = Root {
            path: temp.path().to_owned(),
            prefix: String::new(),
            dir: Arc::new(File::open(temp.path()).unwrap()),
        };
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), b"private").unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("link")).unwrap();
        assert!(open_regular(&root, Path::new("link/secret")).is_err());
        assert!(relative(&root.path, &root.path.join("../escape")).is_none());
        assert!(relative(&root.path, &root.path.join(".gnupg/key")).is_none());
        std::fs::write(temp.path().join("output"), b"complete").unwrap();
        let row = hash(root, signal("output".into()), Attribution::inferred(vec![]));
        assert_eq!(
            row.digest,
            Some(format!("sha256:{:x}", Sha256::digest(b"complete")))
        );
        assert_eq!(row.bytes, Some(8));
        assert!(row.reason.is_none());
    }
}

#[cfg(test)]
mod rescan_tests {
    use super::*;
    #[test]
    fn saturated_rescan_reports_incomplete_without_refilling_overflow() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let mut detector = Detector::new(temp.path(), runtime.handle().clone()).unwrap();
        detector.reconcile_known((0..PENDING + 1).map(|n| format!("missing-{n}")));
        let inbox = detector.inbox.lock().unwrap();
        assert_eq!(inbox.queue.len(), PENDING);
        assert_eq!(inbox.lost, 0);
        drop(inbox);
        assert!(
            detector
                .gaps
                .iter()
                .any(|reason| reason.starts_with("artifact-reconcile-capacity"))
        );
        detector.reconcile_known((0..PENDING).map(|n| format!("missing-{n}")));
        assert_eq!(detector.inbox.lock().unwrap().lost, 0);
    }
}
