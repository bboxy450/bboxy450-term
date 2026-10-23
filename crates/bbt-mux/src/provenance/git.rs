//! Read-only, bounded Git metadata. No hooks, credentials, network or signing.
use crate::Ulid;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(crate) struct Snapshot {
    pub head: Option<String>,
    pub digest: Option<String>,
    pub trailer: Option<Ulid>,
    pub refs: BTreeMap<String, String>,
    pub clipped: bool,
}
fn oid(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn git_directory(root: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::OpenOptionsExt;
    let path = root.join(".git");
    let metadata = path.symlink_metadata().ok()?;
    if metadata.file_type().is_symlink() {
        return None;
    }
    let directory = if metadata.is_dir() {
        path.canonicalize().ok()?
    } else {
        let mut bytes = String::new();
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .ok()?
            .take(4097)
            .read_to_string(&mut bytes)
            .ok()?;
        if bytes.len() > 4096 {
            return None;
        }
        let raw = Path::new(bytes.trim().strip_prefix("gitdir: ")?);
        (if raw.is_absolute() {
            raw.to_owned()
        } else {
            root.join(raw)
        })
        .canonicalize()
        .ok()?
    };
    // Worktrees may refer to the parent's .git/worktrees directory, but a
    // forged gitdir must never turn this observer into a private-home reader.
    if !directory.components().any(|p| p.as_os_str() == ".git")
        || directory.components().any(|p| {
            matches!(
                p.as_os_str().to_str(),
                Some(".gnupg" | ".ssh" | ".icq" | ".claude" | ".codex")
            )
        })
    {
        return None;
    }
    Some(directory)
}
fn run(root: &Path, directory: &Path, args: &[&str], limit: u64) -> Result<Vec<u8>, String> {
    let mut child = Command::new("git")
        .env_clear()
        .env(
            "PATH",
            std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into()),
        )
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "--git-dir",
        ])
        .arg(directory)
        .arg("--work-tree")
        .arg(root)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "git observer unavailable")?;
    let pid = child.id();
    let stdout = child.stdout.take().unwrap();
    let read = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| "git observer status unavailable")?
        {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = read.join();
            return Err(format!("git observer timed out (owned child {pid})"));
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let bytes = read
        .join()
        .map_err(|_| "git observer reader lost")?
        .map_err(|_| "git observer read failed")?;
    if !status.success() {
        return Err("git observer command rejected".into());
    }
    if bytes.len() as u64 > limit {
        return Err("git observer output bound".into());
    }
    Ok(bytes)
}
impl Snapshot {
    pub fn load(root: &Path) -> Result<Option<Self>, String> {
        let Some(directory) = git_directory(root) else {
            return Ok(None);
        };
        let head = run(root, &directory, &["rev-parse", "--verify", "HEAD"], 128)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .map(|s| s.trim().to_owned())
            .filter(|s| oid(s));
        let refs = run(
            root,
            &directory,
            &[
                "for-each-ref",
                "--count=129",
                "--format=%(refname)%09%(objectname)",
            ],
            65536,
        )?;
        let refs = String::from_utf8(refs).map_err(|_| "git ref encoding unavailable")?;
        let mut entries = BTreeMap::new();
        let mut clipped = false;
        for line in refs.lines() {
            if entries.len() >= 128 {
                clipped = true;
                break;
            }
            let Some((name, target)) = line.split_once('\t') else {
                return Err("git ref response malformed".into());
            };
            if !name.starts_with("refs/")
                || name.len() > 1024
                || name.chars().any(char::is_control)
                || !oid(target)
            {
                return Err("git ref outside metadata bounds".into());
            }
            entries.insert(name.into(), target.into());
        }
        let body = head
            .as_deref()
            .and_then(|id| run(root, &directory, &["cat-file", "commit", id], 65536).ok());
        let digest = body
            .as_ref()
            .map(|body| format!("sha256:{:x}", Sha256::digest(body)));
        let trailer = body
            .as_ref()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|body| {
                let entries: Vec<_> = body
                    .lines()
                    .filter_map(|line| {
                        line.strip_prefix("Bboxy-Seat:")
                            .and_then(|v| v.trim().parse::<Ulid>().ok())
                    })
                    .collect();
                (entries.len() == 1).then(|| entries[0])
            });
        Ok(Some(Self {
            head,
            digest,
            trailer,
            refs: entries,
            clipped,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn bounded_git_snapshot_observes_refs_without_signing_or_running_hooks() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let source_dir = git_directory(&source).unwrap();
        // Copy the existing signed commit object; never sign with the operator key.
        let signed = run(&source, &source_dir, &["cat-file", "commit", "HEAD"], 65536).unwrap();
        assert!(signed.windows(7).any(|w| w == b"gpgsig "));
        let temp = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "--quiet", "--initial-branch=main"])
                .arg(temp.path())
                .status()
                .unwrap()
                .success()
        );
        let mut child = Command::new("git")
            .arg("-C")
            .arg(temp.path())
            .args(["hash-object", "-t", "commit", "-w", "--stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&signed).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let oid = String::from_utf8(output.stdout).unwrap();
        let oid = oid.trim();
        let directory = git_directory(temp.path()).unwrap();
        run(
            temp.path(),
            &directory,
            &["update-ref", "refs/heads/main", oid],
            128,
        )
        .unwrap();
        let first = Snapshot::load(temp.path()).unwrap().unwrap();
        assert_eq!(first.head.as_deref(), Some(oid));
        assert_eq!(
            first.digest,
            Some(format!("sha256:{:x}", Sha256::digest(&signed)))
        );
        run(
            temp.path(),
            &directory,
            &["update-ref", "refs/heads/feature", oid],
            128,
        )
        .unwrap();
        let second = Snapshot::load(temp.path()).unwrap().unwrap();
        assert_eq!(second.refs.len(), 2);
        assert_eq!(second.refs["refs/heads/feature"], oid);
        assert!(run(temp.path(), &directory, &["cat-file", "commit", oid], 16).is_err());
        // A forged private-home gitdir is ignored rather than read.
        let fake = tempfile::tempdir().unwrap();
        std::fs::create_dir(fake.path().join(".ssh")).unwrap();
        std::fs::write(fake.path().join(".git"), "gitdir: .ssh\n").unwrap();
        assert!(Snapshot::load(fake.path()).unwrap().is_none());
    }
}
