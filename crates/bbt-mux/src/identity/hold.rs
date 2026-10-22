//! Public attribution of an ephemeral bus run, sealed by a device after custody recovers.
use std::path::Path;

use ed25519_dalek::SigningKey;
use rusqlite::{OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{ChainSeatKeys, DeviceIdentity, Signed};
use crate::{
    bus::HoldRange,
    store::{
        Events, Result, Store, events::EventLog, invalid, now_ms, object_dir, sqlite::SqliteStore,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldWindow {
    pub kind: String,
    pub bus_only: bool,
    pub run: Ulid,
    pub process_generation: String,
    pub boot_id: String,
    pub epoch: String,
    pub public_key: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub ranges: Option<Vec<HoldRange>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldClosure {
    pub version: u8,
    pub device_id: Ulid,
    pub device_pubkey: String,
    pub device_epoch: String,
    pub closed_at: u64,
    pub window: HoldWindow,
}
impl Signed<HoldClosure> {
    /// Verify with an independently authenticated DeviceIdentity. Its root
    /// certificate/grant verification remains the caller's trust boundary.
    pub fn verify_device(&self, device: &DeviceIdentity) -> super::Result<()> {
        let grant = &device.grant.payload;
        if self.payload.version != 1
            || self.payload.device_id != grant.device_id
            || self.payload.device_pubkey != grant.device_pubkey
            || self.payload.device_epoch != grant.device_epoch
            || self.payload.window.kind != "ephemeral-hold"
            || !self.payload.window.bus_only
            || self.payload.window.ended_at.is_none()
            || self.payload.window.ranges.is_none()
        {
            return Err(super::PendingReason::InvalidProof);
        }
        self.verify(
            "HoldClosure",
            &ed25519_dalek::VerifyingKey::from_bytes(
                &crate::messaging::wire::parse_public(&grant.device_pubkey)
                    .map_err(|_| super::PendingReason::InvalidProof)?,
            )
            .map_err(|_| super::PendingReason::InvalidProof)?,
        )
    }
}
pub(crate) fn begin_bus_window(root: &Path, key: &SigningKey) -> Result<Ulid> {
    let account = SqliteStore::open(&object_dir(root, "account")?)?;
    let epoch = super::transition::read(&account)?
        .map(|s| s.cutover_epoch)
        .unwrap_or_else(|| "1".into());
    let mut db = account.connection()?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let previous: Option<Vec<u8>> = tx
        .query_row(
            "SELECT value FROM kv WHERE key='identity/bus-generation'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let generation = previous
        .map(|bytes| serde_json::from_slice::<u64>(&bytes))
        .transpose()?
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| invalid("bus generation exhausted"))?;
    tx.execute("INSERT INTO kv VALUES('identity/bus-generation',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_vec(&generation)?])?;
    let run = Ulid::new();
    let window = HoldWindow {
        kind: "ephemeral-hold".into(),
        bus_only: true,
        run,
        process_generation: generation.to_string(),
        boot_id: boot_id()?,
        epoch,
        public_key: super::hex(&key.verifying_key().to_bytes()),
        started_at: now_ms(),
        ended_at: None,
        ranges: None,
    };
    tx.execute(
        "INSERT INTO kv VALUES(?1,?2)",
        rusqlite::params![
            format!("identity/bus-signers/{run}"),
            serde_json::to_vec(&window)?
        ],
    )?;
    tx.commit()?;
    Ok(run)
}
fn boot_id() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        let mut bytes = [0u8; 128];
        let mut length = bytes.len();
        // SAFETY: the sysctl name is NUL-terminated; the output buffer and
        // length are valid, with no input buffer or mutation of system state.
        let status = unsafe {
            libc::sysctlbyname(
                c"kern.bootsessionuuid".as_ptr(),
                bytes.as_mut_ptr().cast(),
                &mut length,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != 0 || length > bytes.len() {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(std::str::from_utf8(&bytes[..length])?
            .trim_end_matches('\0')
            .into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?
            .trim()
            .into())
    }
}
pub(crate) fn seal_bus_window(root: &Path, run: Ulid, ranges: Vec<HoldRange>) -> Result<()> {
    let account = SqliteStore::open(&object_dir(root, "account")?)?;
    let path = format!("identity/bus-signers/{run}");
    let mut window: HoldWindow = serde_json::from_slice(
        &account
            .get(&path)?
            .ok_or_else(|| invalid("missing bus signer window"))?,
    )?;
    if window.ranges.is_some() {
        return Err(invalid("hold window already sealed"));
    }
    window.ended_at = Some(now_ms());
    window.ranges = Some(ranges);
    account.put(&path, &serde_json::to_vec(&window)?)
}
pub(super) fn decorate(account: &SqliteStore, status: &mut super::IdentityStatus) -> Result<()> {
    for (_, bytes) in account.list("identity/bus-signers/")? {
        let window: HoldWindow = serde_json::from_slice(&bytes)?;
        if account
            .get(&format!("identity/hold-closures/{}", window.run))?
            .is_none()
        {
            status.open_ephemeral_holds += 1;
        }
        if matches!(
            status.state,
            super::IdentityState::Pending | super::IdentityState::Held
        ) && window.ended_at.is_none()
        {
            status.custody_held_since = Some(
                status
                    .custody_held_since
                    .unwrap_or(0)
                    .max(window.started_at),
            );
        }
    }
    Ok(())
}
impl ChainSeatKeys {
    /// Close sealed or abandoned older runs only after a certified device is
    /// available. This never enrolls a primary or rewrites historical events.
    pub fn close_open_holds(&self, root: &Path) -> Result<usize> {
        let key = self.event_signer()?;
        let mut count = 0;
        for (path, bytes) in self.account.list("identity/bus-signers/")? {
            let mut window: HoldWindow = serde_json::from_slice(&bytes)?;
            let closure_path = format!("identity/hold-closures/{}", window.run);
            if self.account.get(&closure_path)?.is_some() {
                continue;
            }
            if window.ranges.is_none() {
                // A crashed process cannot seal its in-memory range. Rebuild
                // it from retained, self-verifying signed events instead.
                let mut ranges = Vec::new();
                for entry in std::fs::read_dir(root)? {
                    let entry = entry?;
                    if !entry.path().join("events.sqlite").is_file() {
                        continue;
                    }
                    let stream = entry
                        .file_name()
                        .into_string()
                        .map_err(|_| invalid("invalid stream name"))?;
                    let log = EventLog::open(&object_dir(root, &stream)?)?;
                    let mut cursor = 0;
                    let mut range: Option<HoldRange> = None;
                    loop {
                        let page = log.after(cursor, 1024)?;
                        if page.is_empty() {
                            break;
                        }
                        for envelope in page {
                            cursor = envelope.seq;
                            if envelope
                                .event
                                .source
                                .starts_with(&format!("ephemeral-hold/{}/", window.run))
                            {
                                let hash = envelope.hash()?;
                                let range = range.get_or_insert_with(|| HoldRange {
                                    stream: stream.clone(),
                                    first: cursor,
                                    first_hash: hash.clone(),
                                    last: cursor,
                                    last_hash: hash.clone(),
                                });
                                range.last = cursor;
                                range.last_hash = hash;
                            }
                        }
                    }
                    if let Some(range) = range {
                        ranges.push(range);
                    }
                }
                window.ranges = Some(ranges);
                window.ended_at = Some(now_ms());
            }
            validate_ranges(root, &window)?;
            let closure = self.sign_hold_closure(window.clone())?;
            let mut db = self.account.connection()?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT OR IGNORE INTO kv VALUES(?1,?2)",
                rusqlite::params![closure_path, serde_json::to_vec(&closure)?],
            )?;
            tx.execute(
                "UPDATE kv SET value=?2 WHERE key=?1",
                rusqlite::params![path, serde_json::to_vec(&window)?],
            )?;
            tx.commit()?;
            count += 1;
        }
        drop(key);
        Ok(count)
    }
}
fn validate_ranges(root: &Path, window: &HoldWindow) -> Result<()> {
    if window.kind != "ephemeral-hold" || !window.bus_only || window.ended_at.is_none() {
        return Err(invalid("invalid hold window"));
    }
    let public = crate::messaging::wire::parse_public(&window.public_key)?;
    for range in window
        .ranges
        .as_deref()
        .ok_or_else(|| invalid("unsealed hold range"))?
    {
        if range.first == 0 || range.first > range.last {
            return Err(invalid("invalid hold hash range"));
        }
        let log = EventLog::open(&object_dir(root, &range.stream)?)?;
        let first = log
            .at(range.first)?
            .ok_or_else(|| invalid("missing hold range start"))?;
        let last = log
            .at(range.last)?
            .ok_or_else(|| invalid("missing hold range end"))?;
        if first.hash()? != range.first_hash || last.hash()? != range.last_hash {
            return Err(invalid("hold range hash mismatch"));
        }
        let mut cursor = range.first - 1;
        while cursor < range.last {
            let page = log.after(cursor, (range.last - cursor).min(1024) as usize)?;
            if page.is_empty() {
                return Err(invalid("hold range gap"));
            }
            for event in page {
                event.verify()?;
                if event.provenance.signer != public
                    || !event
                        .event
                        .source
                        .starts_with(&format!("ephemeral-hold/{}/", window.run))
                {
                    return Err(invalid("foreign hold signer or marker"));
                }
                cursor = event.seq;
            }
        }
    }
    Ok(())
}
