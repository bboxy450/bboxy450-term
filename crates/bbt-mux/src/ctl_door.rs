// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The operator's `ctl` door on `gui.sock` (decision #260, invariant #261). Off by default:
//! with `ctl` closed nothing is written and the window is the only client. Opening it
//! writes `run_dir/gui.endpoint` (path, capability, generation; mode 0600; removed when the
//! door closes) and installs a peer policy on the listener: the window keeps its door, a
//! process whose ancestry holds this app's supervisor (a seat, or anything a seat started)
//! is refused with the reason, and every other admitted peer comes through as `ctl`.
//! **This is not an OS security boundary**: a seat could double-fork and reparent to
//! launchd. It stops the accidental case, an agent finding the binary in its PATH.
use crate::{logic::LogicClient, mcp::Door};
use std::{
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

/// The refusal a seat reads at the door.
pub const REFUSAL: &str = "ctl is the operator's door; your tools are the MCP catalogue";
/// The endpoint file's name inside the run directory.
pub const ENDPOINT: &str = "gui.endpoint";
/// Parent links walked before giving up (a real chain is a handful deep).
const MAX_DEPTH: usize = 64;

/// Who is who at the door: the app's supervisor (every seat descends from it) and the
/// window process (logic's own parent), each by pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Peers {
    pub supervisor: u32,
    pub gui: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Admit {
    Door(Door),
    /// Refused, with the reason the peer reads.
    Refused(&'static str),
}

/// The refusal for a peer whose ancestry cannot be placed: no pid, or a parent chain that
/// does not reach the root within the walk.
pub const UNPLACED: &str = "ctl refused: this process's ancestry could not be placed";

/// Classifies a connecting peer by its pid. A peer we cannot place (no pid, or a chain that
/// does not reach the root within [`MAX_DEPTH`]) is refused: with the door open, an
/// unplaced peer does not get the operator's hand.
pub(crate) fn classify(peer: Option<i32>, peers: Peers) -> Admit {
    classify_with(peer, peers, parent)
}

fn classify_with(peer: Option<i32>, peers: Peers, parent: impl Fn(u32) -> Option<u32>) -> Admit {
    let Some(pid) = peer.and_then(|pid| u32::try_from(pid).ok()) else {
        return Admit::Refused(UNPLACED);
    };
    if pid == peers.gui {
        return Admit::Door(Door::Gui);
    }
    let mut at = pid;
    for _ in 0..MAX_DEPTH {
        if at == peers.supervisor {
            return Admit::Refused(REFUSAL);
        }
        match parent(at) {
            Some(up) if up != at && up != 0 => at = up,
            _ => return Admit::Door(Door::Ctl),
        }
    }
    Admit::Refused(UNPLACED)
}

/// The parent of `pid`, if the system will say.
#[cfg(target_os = "macos")]
pub fn parent(pid: u32) -> Option<u32> {
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
    // SAFETY: proc_pidinfo writes at most `size` bytes into `info`, a plain C struct.
    let read = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    (read == size).then_some(info.pbi_ppid)
}
/// The parent of `pid`, if the system will say.
#[cfg(target_os = "linux")]
pub fn parent(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // `pid (comm) state ppid ...`; comm may hold spaces and parentheses.
    stat.rsplit_once(')')?
        .1
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
/// The parent of `pid`, if the system will say.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn parent(_pid: u32) -> Option<u32> {
    None
}

/// The open door: removes the endpoint file and the peer policy when dropped.
pub struct CtlDoor {
    file: PathBuf,
    client: LogicClient,
}
impl CtlDoor {
    pub fn file(&self) -> &Path {
        &self.file
    }
}
impl Drop for CtlDoor {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
        self.client.set_gui_peers(None);
    }
}

/// Opens the door: installs `peers` and writes the endpoint file (mode 0600, replaced
/// atomically). The capability changes with every logic start, so a stale file left by a
/// crash never admits.
pub async fn open(
    client: &LogicClient,
    run_dir: &Path,
    peers: Peers,
) -> crate::store::Result<CtlDoor> {
    let info = client.gui_endpoint().await?;
    client.set_gui_peers(Some(peers));
    let capability: String = info.capability.iter().map(|b| format!("{b:02x}")).collect();
    let body = serde_json::json!({
        "path": info.path,
        "capability": capability,
        "generation": info.generation,
    })
    .to_string();
    std::fs::create_dir_all(run_dir)?;
    let file = run_dir.join(ENDPOINT);
    let staged = run_dir.join(format!(".{ENDPOINT}.{}", std::process::id()));
    let _ = std::fs::remove_file(&staged);
    let written = (|| {
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&staged)?;
        out.write_all(body.as_bytes())?;
        out.sync_all()?;
        std::fs::rename(&staged, &file)
    })();
    if let Err(error) = written {
        let _ = std::fs::remove_file(&staged);
        client.set_gui_peers(None);
        return Err(error.into());
    }
    Ok(CtlDoor {
        file,
        client: client.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: invariant ctl-opt-in
    #[test]
    fn the_window_keeps_its_door_and_a_supervisor_descendant_is_refused() {
        // A synthetic ancestry makes classification independent of the runner's
        // process tree. The real endpoint/ancestry boundary remains in gui_endpoint.
        let parent = |pid| match pid {
            42 => Some(30),
            30 => Some(2),
            2 => Some(1),
            _ => Some(0),
        };
        assert_eq!(
            classify_with(
                Some(42),
                Peers {
                    supervisor: 2,
                    gui: 42
                },
                parent
            ),
            Admit::Door(Door::Gui)
        );
        assert_eq!(
            classify_with(
                Some(42),
                Peers {
                    supervisor: 2,
                    gui: 0
                },
                parent
            ),
            Admit::Refused(REFUSAL)
        );
        let unrelated = Peers {
            supervisor: 99,
            gui: 0,
        };
        assert_eq!(
            classify_with(Some(42), unrelated, parent),
            Admit::Door(Door::Ctl)
        );
        for pid in [None, Some(-1)] {
            assert_eq!(
                classify_with(pid, unrelated, parent),
                Admit::Refused(UNPLACED)
            );
        }
    }

    // keeps: invariant ctl-opt-in
    #[test]
    fn a_chain_that_never_reaches_the_root_is_refused() {
        let peers = Peers {
            supervisor: u32::MAX,
            gui: 0,
        };
        // An endless chain (each pid's parent is the next) exhausts the walk.
        assert_eq!(
            classify_with(Some(2), peers, |pid| Some(pid + 1)),
            Admit::Refused(UNPLACED)
        );
        // The same chain, ending at the root just inside the walk, comes through as ctl.
        let last = 2 + MAX_DEPTH as u32 - 1;
        let to_root = |pid: u32| Some(if pid == last { 0 } else { pid + 1 });
        assert_eq!(
            classify_with(Some(2), peers, to_root),
            Admit::Door(Door::Ctl)
        );
        // And one link longer is refused.
        let to_root = |pid: u32| Some(if pid == last + 1 { 0 } else { pid + 1 });
        assert_eq!(
            classify_with(Some(2), peers, to_root),
            Admit::Refused(UNPLACED)
        );
    }
}
