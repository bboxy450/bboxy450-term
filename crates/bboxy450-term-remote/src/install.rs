// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `bboxy450-term-remote install`: put this build of bboxy450-term-remote and
//! a bboxy450-harness binary in the remote ~/.local/bin when the remote OS and
//! CPU (`uname -sm`) match this machine. Copied this way, both ends run the
//! same build. Approach adapted from herdr-core's `hc machine add --install`
//! (Apache-2.0, https://github.com/herdrdev/herdr-core, machines.rs): probe
//! uname, stream into a temp file, check the size, then mv into place.
use crate::client::Transport;
use std::io::{self, Write};
use std::path::Path;
use std::process::Stdio;

fn output(mut c: std::process::Command) -> io::Result<String> {
    let out = c.stdin(Stdio::null()).stderr(Stdio::inherit()).output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "remote command failed: {}",
            out.status
        )));
    }
    // A login banner may precede the answer; the answer is the last line.
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_owned())
}

/// Copy each (local path, remote name) pair; returns the remote uname.
pub fn install(transport: &Transport, files: &[(&Path, &str)]) -> io::Result<String> {
    let mut local = std::process::Command::new("uname");
    local.arg("-sm");
    let here = output(local)?;
    let there = output(transport.shell("uname -sm"))?;
    if here != there {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "remote is {there}, this build is {here}: install a matching bboxy450-term-remote and bboxy450-harness there"
            ),
        ));
    }
    for (path, name) in files {
        let bytes = std::fs::read(path)?;
        let line = format!(
            "set -e; d=\"$HOME/.local/bin\"; mkdir -p \"$d\"; t=\"$d/.{name}.tmp\"; head -c {n} > \"$t\";              test \"$(wc -c < \"$t\" | tr -d ' ')\" -eq {n}; chmod 755 \"$t\"; mv -f \"$t\" \"$d/{name}\"",
            n = bytes.len()
        );
        let mut c = transport.shell(&line);
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()?;
        child.stdin.take().expect("piped").write_all(&bytes)?;
        let status = child.wait()?;
        if !status.success() {
            return Err(io::Error::other(format!("copying {name} failed: {status}")));
        }
    }
    Ok(there)
}
