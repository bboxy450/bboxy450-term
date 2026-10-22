//! Issuer changes are explicit account-journal operations, never ledger rewrites.
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{
    messaging::{
        keys::{LocalSeatKeys, SeatKeys},
        wire::{Envelope, Issuer, IssuerKind, parse_public},
    },
    registry::{Lifecycle, SeatRecord},
    store::{Result, Store, digest, invalid, sqlite::SqliteStore},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuerState {
    pub kind: IssuerKind,
    pub key: String,
    pub cutover_epoch: String,
    pub at: u64,
    pub account_head_id: Option<String>,
    pub previous: Option<String>,
    pub root_fingerprint: Option<String>,
    pub legacy: Vec<LegacyGeneration>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyGeneration {
    pub seat: Ulid,
    pub generation: u64,
    pub issuer: String,
    pub author: Option<String>,
    /// Exclusive sender sequence fence. None only for a generation still live
    /// at cutover, which keeps its original key until its next generation.
    pub before: Option<u64>,
}

pub(crate) fn get(tx: &Transaction<'_>) -> Result<Option<IssuerState>> {
    let bytes: Option<Vec<u8>> = tx
        .query_row(
            "SELECT value FROM kv WHERE key='identity/issuer'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let state: Option<IssuerState> = bytes
        .map(|b| Ok::<_, crate::store::Error>(serde_json::from_slice(&b)?))
        .transpose()?;
    let mut query = tx.prepare("SELECT key FROM kv WHERE key GLOB 'identity/issuer-history/*'")?;
    let epochs = query
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    check_epoch(state.as_ref(), epochs.iter().map(String::as_str))?;
    Ok(state)
}
fn append(tx: &Transaction<'_>, state: &IssuerState) -> Result<()> {
    let bytes = serde_json::to_vec(state)?;
    tx.execute(
        "INSERT INTO kv VALUES(?1,?2)",
        params![
            format!("identity/issuer-history/{}", state.cutover_epoch),
            &bytes
        ],
    )?;
    tx.execute("INSERT INTO kv VALUES('identity/issuer',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [&bytes])?;
    tx.execute("INSERT INTO kv VALUES('messaging/issuer',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_vec(&Issuer { kind: state.kind, key: state.key.clone() })?])?;
    Ok(())
}
fn exists(tx: &Transaction<'_>, table: &str) -> Result<bool> {
    Ok(tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table],
        |r| r.get(0),
    )?)
}

/// Called in exactly the transaction that installs the verified epoch-1 head.
pub(crate) fn bootstrap(
    tx: &Transaction<'_>,
    key: String,
    fingerprint: String,
    head: String,
    now: u64,
) -> Result<()> {
    if get(tx)?.is_some() {
        return Err(invalid(
            "identity issuer already journaled; explicit reset required",
        ));
    }
    let old: Option<Vec<u8>> = tx
        .query_row(
            "SELECT value FROM kv WHERE key='messaging/issuer'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    let old: Option<Issuer> = old.map(|b| serde_json::from_slice(&b)).transpose()?;
    if old.as_ref().is_some_and(|i| i.kind != IssuerKind::LocalDev) {
        return Err(invalid(
            "issuer cutover can only advance local-dev to gpg-root",
        ));
    }
    let mut legacy = std::collections::BTreeMap::<(Ulid, u64), LegacyGeneration>::new();
    if let Some(old) = old {
        parse_public(&old.key)?;
        if exists(tx, "msg_grants")? {
            let mut query = tx.prepare("SELECT data FROM msg_grants")?;
            for bytes in query.query_map([], |r| r.get::<_, Vec<u8>>(0))? {
                let grant: serde_json::Value = serde_json::from_slice(&bytes?)?;
                let seat: Ulid = serde_json::from_value(grant["seat"].clone())?;
                let generation = grant["generation"]
                    .as_u64()
                    .ok_or_else(|| invalid("invalid legacy generation"))?;
                let author = grant["author"]
                    .as_str()
                    .ok_or_else(|| invalid("missing legacy author"))?
                    .to_string();
                parse_public(&author)?;
                let sequence: u64 = tx.query_row(
                    "SELECT sequence FROM msg_counters WHERE seat=?1 AND generation=?2",
                    params![seat.to_string(), crate::store::sqlint(generation)?],
                    |r| crate::messaging::ledger::u64_at(r, 0),
                )?;
                legacy.insert(
                    (seat, generation),
                    LegacyGeneration {
                        seat,
                        generation,
                        issuer: old.key.clone(),
                        author: Some(author),
                        before: Some(
                            sequence
                                .checked_add(1)
                                .ok_or_else(|| invalid("legacy sequence exhausted"))?,
                        ),
                    },
                );
            }
        }
        let mut query = tx.prepare("SELECT value FROM kv WHERE substr(key,1,5)='seat/'")?;
        for bytes in query.query_map([], |r| r.get::<_, Vec<u8>>(0))? {
            let record: SeatRecord = serde_json::from_slice(&bytes?)?;
            if record.lifecycle == Lifecycle::ClosedRetained {
                continue;
            }
            legacy
                .entry((record.id, record.generation))
                .and_modify(|entry| entry.before = None)
                .or_insert(LegacyGeneration {
                    seat: record.id,
                    generation: record.generation,
                    issuer: old.key.clone(),
                    author: None,
                    before: None,
                });
        }
    }
    append(
        tx,
        &IssuerState {
            kind: IssuerKind::GpgRoot,
            key,
            cutover_epoch: "1".into(),
            at: now,
            account_head_id: Some(head),
            previous: None,
            root_fingerprint: Some(fingerprint),
            legacy: legacy.into_values().collect(),
        },
    )
}

/// Operator identity action only, deliberately absent from MCP/seat APIs.
/// Starts another local-dev epoch; retained head/certificates and ledgers stay.
pub fn reset_identity(
    account: &SqliteStore,
    local: &LocalSeatKeys,
    now: u64,
) -> Result<IssuerState> {
    bbt_rt::assert_not_ui_thread();
    let mut db = account.connection()?;
    let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut old = get(&tx)?.ok_or_else(|| invalid("identity reset requires an issuer journal"))?;
    if exists(&tx, "msg_counters")? {
        for entry in old.legacy.iter_mut().filter(|e| e.before.is_none()) {
            let last: Option<u64> = tx
                .query_row(
                    "SELECT sequence FROM msg_counters WHERE seat=?1 AND generation=?2",
                    params![
                        entry.seat.to_string(),
                        crate::store::sqlint(entry.generation)?
                    ],
                    |r| crate::messaging::ledger::u64_at(r, 0),
                )
                .optional()?;
            entry.before = Some(
                last.unwrap_or(0)
                    .checked_add(1)
                    .ok_or_else(|| invalid("legacy sequence exhausted"))?,
            );
        }
    }
    let epoch = counter(&old.cutover_epoch)?
        .checked_add(1)
        .ok_or_else(|| invalid("issuer epoch exhausted"))?;
    let new = IssuerState {
        kind: IssuerKind::LocalDev,
        key: local.issuer().key,
        cutover_epoch: epoch.to_string(),
        at: now,
        account_head_id: old.account_head_id.clone(),
        previous: Some(digest(&tx.query_row(
            "SELECT value FROM kv WHERE key=?1",
            [format!("identity/issuer-history/{}", old.cutover_epoch)],
            |r| r.get::<_, Vec<u8>>(0),
        )?)),
        root_fingerprint: old.root_fingerprint.clone(),
        legacy: old.legacy,
    };
    append(&tx, &new)?;
    tx.commit()?;
    Ok(new)
}
pub(crate) fn counter(s: &str) -> Result<u64> {
    let n: u64 = s.parse()?;
    if n == 0 || n.to_string() != s {
        return Err(invalid("noncanonical identity epoch"));
    }
    Ok(n)
}
fn check_epoch<'a>(
    state: Option<&IssuerState>,
    epochs: impl Iterator<Item = &'a str>,
) -> Result<()> {
    let latest = epochs
        .map(|key| {
            counter(
                key.strip_prefix("identity/issuer-history/")
                    .ok_or_else(|| invalid("invalid issuer journal key"))?,
            )
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max();
    if let Some(latest) = latest
        && state.map(|s| counter(&s.cutover_epoch)).transpose()? != Some(latest)
    {
        return Err(invalid("issuer journal rollback"));
    }
    Ok(())
}
pub(crate) fn read(account: &SqliteStore) -> Result<Option<IssuerState>> {
    let state: Option<IssuerState> = account
        .get("identity/issuer")?
        .map(|b| Ok::<_, crate::store::Error>(serde_json::from_slice(&b)?))
        .transpose()?;
    let epochs = account.list("identity/issuer-history/")?;
    check_epoch(state.as_ref(), epochs.iter().map(|(k, _)| k.as_str()))?;
    Ok(state)
}
pub(crate) fn legacy(state: &IssuerState, seat: Ulid, generation: u64) -> bool {
    state.kind == IssuerKind::LocalDev
        || state
            .legacy
            .iter()
            .any(|e| e.seat == seat && e.generation == generation && e.before.is_none())
}
pub(crate) fn remember_local(tx: &Transaction<'_>, env: &Envelope) -> Result<()> {
    let mut state = get(tx)?.ok_or_else(|| invalid("missing issuer journal"))?;
    if let Some(entry) = state.legacy.iter_mut().find(|e| {
        e.seat == env.seat && e.generation == env.process_generation && e.issuer == env.issuer
    }) {
        if entry.author.as_ref().is_some_and(|a| a != &env.author) {
            return Err(invalid("legacy key changed within generation"));
        }
        if entry
            .before
            .is_some_and(|limit| env.sender_sequence >= limit)
        {
            return Err(invalid("local-dev sender sequence is after cutover"));
        }
        entry.author = Some(env.author.clone());
    } else if state.kind == IssuerKind::LocalDev {
        state.legacy.push(LegacyGeneration {
            seat: env.seat,
            generation: env.process_generation,
            issuer: env.issuer.clone(),
            author: Some(env.author.clone()),
            before: None,
        });
    } else {
        return Err(invalid("local-dev generation was not live at cutover"));
    }
    tx.execute(
        "UPDATE kv SET value=?1 WHERE key='identity/issuer'",
        [serde_json::to_vec(&state)?],
    )?;
    Ok(())
}
pub(crate) fn fence(tx: &Transaction<'_>, seat: Ulid, generation: u64) -> Result<()> {
    let mut state = get(tx)?.ok_or_else(|| invalid("missing issuer journal"))?;
    for entry in state
        .legacy
        .iter_mut()
        .filter(|e| e.seat == seat && e.generation != generation && e.before.is_none())
    {
        let last: Option<u64> = tx
            .query_row(
                "SELECT sequence FROM msg_counters WHERE seat=?1 AND generation=?2",
                params![seat.to_string(), crate::store::sqlint(entry.generation)?],
                |r| crate::messaging::ledger::u64_at(r, 0),
            )
            .optional()?;
        entry.before = Some(
            last.unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| invalid("legacy sequence exhausted"))?,
        );
    }
    tx.execute(
        "UPDATE kv SET value=?1 WHERE key='identity/issuer'",
        [serde_json::to_vec(&state)?],
    )?;
    Ok(())
}
