// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Import only PATH from the user's interactive login zsh, off the UI thread.
#![allow(
    clippy::disallowed_methods,
    reason = "login-shell startup and private fixtures run on cockpit IO, never UI"
)]
use super::Config;
use std::{
    ffi::{OsStr, OsString},
    hash::{Hash, Hasher},
    os::unix::ffi::OsStringExt,
    path::Path,
    process::Stdio,
    time::Duration,
};
use tokio::io::AsyncReadExt;

const MARKER: &[u8] = b"\0BBOXY_LOGIN_PATH\0";
const LIMIT: u64 = 64 * 1024;

pub(super) fn fingerprint(path: &OsStr) -> String {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hash);
    format!("{:016x}", hash.finish())
}

fn parse(output: &[u8]) -> Option<OsString> {
    let at = output
        .windows(MARKER.len())
        .rposition(|part| part == MARKER)?;
    let tail = output.get(at + MARKER.len()..)?;
    let value = &tail[..tail.iter().position(|byte| *byte == 0)?];
    (!value.is_empty()).then(|| OsString::from_vec(value.to_vec()))
}

pub(super) async fn load(config: &Config) -> OsString {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let zdotdir = config
        .shell_config
        .as_deref()
        .or(config.test_account.as_deref());
    match probe(zdotdir, &inherited, Duration::from_secs(3)).await {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("login zsh PATH unavailable ({reason}); retaining inherited PATH");
            inherited
        }
    }
}

async fn probe(
    zdotdir: Option<&Path>,
    inherited: &OsStr,
    bound: Duration,
) -> Result<OsString, String> {
    let mut command = tokio::process::Command::new("/bin/zsh");
    command
        .args(["-ilc", "printf '\\0BBOXY_LOGIN_PATH\\0%s\\0' \"$PATH\""])
        .env("PATH", inherited)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .process_group(0);
    if let Some(root) = zdotdir {
        command.env("ZDOTDIR", root);
    }
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let pid = child.id().expect("new zsh has a pid");
    let mut stdout = child
        .stdout
        .take()
        .expect("piped zsh stdout")
        .take(LIMIT + 1);
    let mut bytes = Vec::new();
    let result = tokio::time::timeout(bound, async {
        tokio::try_join!(child.wait(), stdout.read_to_end(&mut bytes))
    })
    .await;
    match result {
        Ok(Ok((status, count))) if status.success() && count <= LIMIT as usize => {
            parse(&bytes).ok_or_else(|| "missing PATH result".into())
        }
        other => {
            // This is the private process group created above, including a hung rc child.
            if !matches!(&other, Ok(Ok(_))) {
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
            let _ = child.wait().await;
            Err(match other {
                Err(_) => format!("startup exceeded {} ms", bound.as_millis()),
                Ok(Err(error)) => error.to_string(),
                Ok(Ok(_)) => "shell failed or output exceeded 64 KiB".into(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framed_path_ignores_startup_banners_and_preserves_spaces() {
        assert_eq!(
            parse(b"banner\n\0BBOXY_LOGIN_PATH\0/a space/bin:/usr/bin\0"),
            Some(OsString::from("/a space/bin:/usr/bin"))
        );
        assert_eq!(parse(b"ordinary startup output"), None);
        assert_eq!(parse(b"\0BBOXY_LOGIN_PATH\0\0"), None);
        assert_eq!(parse(b"\0BBOXY_LOGIN_PATH\0truncated"), None);
    }

    #[tokio::test]
    #[ignore = "night: real zsh startup boundary; run explicitly for PATH changes"]
    async fn login_and_interactive_rc_are_loaded_without_importing_other_variables() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join(".zprofile"),
            "path=(/login-only/bin $path)\n",
        )
        .unwrap();
        std::fs::write(root.path().join(".zshrc"),
            "print startup-banner\nexport PRIVATE_FIXTURE=must-not-import\npath=('/interactive space/bin' $path)\n").unwrap();
        let path = probe(
            Some(root.path()),
            OsStr::new("/usr/bin:/bin"),
            Duration::from_secs(3),
        )
        .await
        .unwrap();
        let entries: Vec<_> = std::env::split_paths(&path).collect();
        assert!(entries.contains(&"/login-only/bin".into()));
        assert!(entries.contains(&"/interactive space/bin".into()));
        assert!(!path.to_string_lossy().contains("must-not-import"));
    }

    #[tokio::test]
    #[ignore = "night: real zsh startup timeout; run explicitly for PATH changes"]
    async fn a_hung_rc_is_bounded_and_the_private_shell_is_reaped() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(".zshrc"), "/bin/sleep 60\n").unwrap();
        assert!(
            probe(
                Some(root.path()),
                OsStr::new("/usr/bin:/bin"),
                Duration::from_millis(100)
            )
            .await
            .is_err()
        );
    }
}
