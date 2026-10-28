// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Advisory wake accounting. No body, endpoint, capability or native note is saved.
use std::{
    fs::{DirBuilder, OpenOptions},
    io::{self, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use bbt_proto::messaging::{MessageClass, WakeMode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ulid::Ulid;

use crate::adapters::{Binding, Evidence, WakeOutcome};

/// Admission-time policy, rather than the receiver's possibly changed policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub id: Ulid,
    pub destination: String,
    pub class: MessageClass,
    pub mode: Option<WakeMode>,
}

fn uniform(members: &[Member], field: &str) -> Value {
    let values: Vec<Value> = members
        .iter()
        .map(|member| {
            serde_json::to_value(member).expect("wake metadata serializes")[field].clone()
        })
        .collect();
    match values.first() {
        Some(first) if values.iter().all(|value| value == first) => first.clone(),
        Some(_) => json!("mixed"),
        None => Value::Null,
    }
}

fn timestamp(ms: u64) -> io::Result<String> {
    let seconds: libc::time_t = (ms / 1000)
        .try_into()
        .map_err(|_| io::Error::other("wake timestamp out of range"))?;
    // SAFETY: gmtime_r initializes this local tm from a valid time_t pointer.
    let time = unsafe {
        let mut value: libc::tm = std::mem::zeroed();
        if libc::gmtime_r(&seconds, &mut value).is_null() {
            return Err(io::Error::other("wake timestamp out of range"));
        }
        value
    };
    let year = time.tm_year + 1900;
    if !(1970..=9999).contains(&year) {
        return Err(io::Error::other("wake timestamp out of range"));
    }
    Ok(format!(
        "{year:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        time.tm_mon + 1,
        time.tm_mday,
        time.tm_hour,
        time.tm_min,
        time.tm_sec,
        ms % 1000
    ))
}

pub(crate) fn record(
    work: &super::ledger::Work,
    binding: &Binding,
    harness: &str,
    outcome: &WakeOutcome,
    now: u64,
) -> io::Result<Option<Value>> {
    if !matches!(
        outcome.evidence,
        Evidence::NativeQueued | Evidence::NativeWritten | Evidence::NativeUnknown
    ) {
        return Ok(None); // Definitively pre-write failures did not deliver a wake.
    }
    Ok(Some(json!({
        "kind": "stage", "stage": "wake", "id": format!("wake:{}",work.id),
        "at": timestamp(now)?, "agent": format!("seat:{}",work.recipient),
        "cut": {"branch": null, "sha": null}, "attribution": "admitted recipient; cut unavailable",
        "seat": work.recipient, "generation": binding.generation,
        "harness": harness, "session": binding.session, "attempt": work.id,
        "subagent": work.agent, "native_id": outcome.native_id, "evidence": outcome.evidence,
        "destination": uniform(&work.wake_members,"destination"),
        "class": uniform(&work.wake_members,"class"), "mode": uniform(&work.wake_members,"mode"),
        "members": work.wake_members,
        "message_ids": work.messages.iter().map(|message|message.view().id).collect::<Vec<_>>(),
        "input_tokens": null, "token_attribution": "unknown; native admission is not turn acceptance"
    })))
}

fn state_dir() -> io::Result<PathBuf> {
    let path = std::env::var_os("BBOXY_LANDING_STATE_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("XDG_STATE_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
                })
                .map(|root| root.join("bboxy450-term/landing"))
        })
        .ok_or_else(|| io::Error::other("wake ledger directory unavailable"))?;
    if !path.is_absolute() {
        return Err(io::Error::other("wake ledger directory must be absolute"));
    }
    Ok(path)
}

fn private_directory(path: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(path)?;
    let metadata = path.symlink_metadata()?;
    // SAFETY: geteuid has no arguments and returns the current process identity.
    if !metadata.is_dir() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err(io::Error::other("wake ledger directory is not private"));
    }
    if metadata.mode() & 0o077 != 0 {
        // Older stage writers created intermediate directories with the umask.
        // Tighten only our owned ledger directories, never the state root.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub(crate) fn append(root: &Path, row: &Value) -> io::Result<()> {
    let at = row["at"]
        .as_str()
        .ok_or_else(|| io::Error::other("missing wake timestamp"))?;
    let day = at
        .get(..10)
        .filter(|day| {
            day.bytes().enumerate().all(|(i, b)| {
                if matches!(i, 4 | 7) {
                    b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            })
        })
        .ok_or_else(|| io::Error::other("invalid wake day"))?;
    let ledger = root.join("ledger");
    private_directory(&ledger)?;
    let directory = ledger.join(day);
    private_directory(&directory)?;
    let mut bytes = serde_json::to_vec(row)?;
    bytes.push(b'\n');
    if bytes.len() > 64 * 1024 {
        return Err(io::Error::other("wake metadata bound"));
    }
    // One append, no lock, fsync, retry, collector, or wait on another agent.
    // NONBLOCK also prevents a replaced FIFO from holding the delivery owner.
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(directory.join("ledger.jsonl"))?;
    let metadata = file.metadata()?;
    // SAFETY: geteuid has no arguments and returns the current process identity.
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(io::Error::other(
            "wake ledger is not a private regular file",
        ));
    }
    if file.write(&bytes)? != bytes.len() {
        return Err(io::Error::other("short wake ledger append"));
    }
    Ok(())
}

pub(crate) fn note(
    work: &super::ledger::Work,
    binding: &Binding,
    harness: &str,
    outcome: &WakeOutcome,
    now: u64,
) {
    let result = record(work, binding, harness, outcome, now)
        .and_then(|row| row.map_or(Ok(()), |row| append(&state_dir()?, &row)));
    if let Err(error) = result {
        eprintln!(
            "ledger: wake accounting unavailable ({:?}); delivery unchanged",
            error.kind()
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)] // Private database/file fixtures.
    use super::*;
    use crate::{
        messaging::{Messaging, keys::LocalSeatKeys, wire::SendRequest},
        registry::{Registry, SeatRecord},
        store::{object_dir, sqlite::SqliteStore},
    };
    use ed25519_dalek::SigningKey;
    use std::{os::unix::fs::PermissionsExt, sync::Arc};

    fn work(mode: WakeMode) -> (tempfile::TempDir, super::super::ledger::Work, Binding) {
        let root = tempfile::tempdir().unwrap();
        let registry =
            Registry::new(SqliteStore::open(&object_dir(root.path(), "account").unwrap()).unwrap());
        let swarm = Ulid::new();
        let sender = SeatRecord::new(Ulid::new(), swarm, "sender", "zsh");
        let receiver = SeatRecord::new(Ulid::new(), swarm, "receiver", "zsh");
        registry.insert(&sender).unwrap();
        registry.insert(&receiver).unwrap();
        let destination = format!("seats/{}/inbox", receiver.id);
        registry
            .set_attention(
                receiver.id,
                1,
                bbt_proto::registry::AttentionRule {
                    destination: destination.clone(),
                    class: MessageClass::Conversation,
                    mode,
                },
            )
            .unwrap();
        let mut messaging = Messaging::open(
            root.path(),
            Arc::new(LocalSeatKeys::new(SigningKey::from_bytes(&[67; 32]))),
        )
        .unwrap();
        for at in 1..=2 {
            messaging
                .send(
                    sender.id,
                    1,
                    SendRequest::text(&destination, "PRIVATE BODY AND TOOL OUTPUT"),
                    at,
                )
                .unwrap();
        }
        // A later edit must not change the accounting mode of admitted work.
        registry
            .set_attention(
                receiver.id,
                1,
                bbt_proto::registry::AttentionRule {
                    destination,
                    class: MessageClass::Conversation,
                    mode: WakeMode::Inbox,
                },
            )
            .unwrap();
        let work = messaging
            .prepare(swarm, receiver.id, 600_001)
            .unwrap()
            .unwrap();
        let binding = Binding {
            seat: receiver.id,
            session: "fixture-session".into(),
            pid: 1,
            process_start: "fixture".into(),
            generation: 1,
            attachment_generation: 1,
            runtime: None,
        };
        (root, work, binding)
    }

    #[test]
    fn wake_batch_is_one_private_metadata_line_with_pinned_modes() {
        let (root, work, binding) = work(WakeMode::Wake);
        let outcome = WakeOutcome {
            evidence: Evidence::NativeQueued,
            native_id: Some("queue-id-not-a-turn".into()),
            note: "PRIVATE CAPABILITY AND NOTE".into(),
        };
        let row = record(&work, &binding, "codex", &outcome, 1_792_886_400_123)
            .unwrap()
            .unwrap();
        assert_eq!(row["at"], "2026-10-25T00:00:00.123Z");
        assert_eq!(row["mode"], "wake");
        assert_eq!(row["class"], "conversation");
        assert_eq!(row["members"].as_array().unwrap().len(), 2);
        assert_eq!(row["message_ids"].as_array().unwrap().len(), 2);
        assert!(row["input_tokens"].is_null());
        append(root.path(), &row).unwrap();
        let path = root.path().join("ledger/2026-10-25/ledger.jsonl");
        let bytes = std::fs::read_to_string(&path).unwrap();
        assert_eq!(bytes.lines().count(), 1);
        assert!(!bytes.contains("PRIVATE"));
        assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(
            path.parent()
                .unwrap()
                .metadata()
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        let before = record(
            &work,
            &binding,
            "codex",
            &WakeOutcome {
                evidence: Evidence::RecipientStored,
                native_id: None,
                note: "pre-write refusal".into(),
            },
            0,
        )
        .unwrap();
        assert!(before.is_none());
        let unknown = record(
            &work,
            &binding,
            "codex",
            &WakeOutcome::unknown("ambiguous write"),
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(unknown["evidence"], "native-unknown");
        assert!(unknown["input_tokens"].is_null());
    }

    #[test]
    fn digest_snapshot_and_legacy_attempts_are_accounted_honestly() {
        let (_, work, binding) = work(WakeMode::Digest);
        assert!(work.digest);
        let row = record(
            &work,
            &binding,
            "claude",
            &WakeOutcome::unknown("opaque"),
            0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(row["mode"], "digest");
        assert_eq!(row["members"].as_array().unwrap().len(), 2);
        let mut old = serde_json::to_value(work).unwrap();
        old.as_object_mut().unwrap().remove("wake_members");
        let old: super::super::ledger::Work = serde_json::from_value(old).unwrap();
        let row = record(&old, &binding, "claude", &WakeOutcome::unknown("opaque"), 0)
            .unwrap()
            .unwrap();
        assert!(row["mode"].is_null());
        assert_eq!(row["message_ids"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn mixed_batch_metadata_never_invents_a_per_destination_cost() {
        let (_, work, _) = work(WakeMode::Wake);
        let mut members = work.wake_members;
        assert_eq!(uniform(&members, "mode"), "wake");
        members[1].destination = "channels/status".into();
        members[1].class = MessageClass::Event;
        members[1].mode = Some(WakeMode::Digest);
        for field in ["destination", "class", "mode"] {
            assert_eq!(uniform(&members, field), "mixed");
        }
    }

    #[test]
    fn unavailable_or_replaced_ledger_refuses_without_waiting_or_following() {
        let (root, work, binding) = work(WakeMode::Wake);
        let row = record(
            &work,
            &binding,
            "claude",
            &WakeOutcome::unknown("opaque"),
            0,
        )
        .unwrap()
        .unwrap();
        std::fs::write(root.path().join("unavailable"), b"ordinary file").unwrap();
        assert!(append(&root.path().join("unavailable"), &row).is_err());
        let day = root.path().join("ledger/1970-01-01");
        private_directory(&day).unwrap();
        let other = root.path().join("other");
        std::fs::write(&other, b"unchanged").unwrap();
        let target = day.join("ledger.jsonl");
        std::os::unix::fs::symlink(&other, &target).unwrap();
        assert!(append(root.path(), &row).is_err());
        assert_eq!(std::fs::read(&other).unwrap(), b"unchanged");
        std::fs::remove_file(&target).unwrap();
        let name = std::ffi::CString::new(target.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: name is a valid NUL-terminated path in our disposable directory.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(append(root.path(), &row).is_err());
    }
}
