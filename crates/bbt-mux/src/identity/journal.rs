use std::{path::PathBuf, time::Duration};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{
    Custody, DeviceKeyStore, PendingReason as E, Result,
    agent::{Agent, Input},
    certificate::{self, Root},
    hex, proof,
};
use crate::store::{digest, sqlite::SqliteStore};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signed<T> {
    pub payload: T,
    pub digest: String,
    pub signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceGrant {
    pub version: u8,
    pub root_pubkey: String,
    pub account_epoch: String,
    pub device_id: Ulid,
    pub device_pubkey: String,
    pub device_epoch: String,
    pub serial: String,
    pub scopes: Vec<String>,
    pub role_ceiling: Vec<String>,
    pub not_before: u64,
    pub expires_at: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveDevice {
    pub device_id: Ulid,
    pub device_pubkey: String,
    pub device_epoch: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountHead {
    pub version: u8,
    pub root_pubkey: String,
    pub account_epoch: String,
    pub previous_head_id: Option<String>,
    pub active_devices: Vec<ActiveDevice>,
    pub revoked_devices: Vec<Ulid>,
    pub current_binding_ids: Vec<String>,
    pub revoked_binding_ids: Vec<String>,
    pub issued_at: u64,
}
/// Public material only. Safe to persist, export or inspect.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceIdentity {
    pub custody: Custody,
    pub root_fingerprint: String,
    pub root_certificate: String,
    pub device_certificate: String,
    pub grant: Signed<DeviceGrant>,
}
impl DeviceIdentity {
    pub fn public_device_certificate(&self) -> Result<Vec<u8>> {
        proof(B64.decode(&self.device_certificate))
    }
    /// Verify against a separately pinned fingerprint, never the one supplied by
    /// an untrusted proof bundle. ID1a accepts only the initial epoch-1 head.
    pub fn verify(&self, expected: &str, head: &Signed<AccountHead>, now: u64) -> Result<()> {
        if self.root_fingerprint != expected {
            return Err(E::FingerprintMismatch);
        }
        let root = Root::parse(&proof(B64.decode(&self.root_certificate))?, expected, now)?;
        self.grant.verify("DeviceGrant", &root.public)?;
        head.verify("AccountHead", &root.public)?;
        let grant = &self.grant.payload;
        let h = &head.payload;
        let device = decode_public(&grant.device_pubkey)?;
        certificate::verify_device(
            &root,
            &self.public_device_certificate()?,
            &device.to_bytes(),
        )?;
        if grant.version != 1
            || h.version != 1
            || grant.root_pubkey != hex(&root.public.to_bytes())
            || h.root_pubkey != grant.root_pubkey
            || grant.account_epoch != "1"
            || h.account_epoch != "1"
            || grant.device_epoch != "1"
            || grant.serial != "1"
            || grant.device_id == Ulid::nil()
            || grant.not_before > now
            || grant.expires_at <= now
            || grant.expires_at <= grant.not_before
            || grant.expires_at - grant.not_before > 30 * 24 * 3600
            || root.valid_until.is_some_and(|end| grant.expires_at > end)
            || grant.role_ceiling != ["oracle", "worker"]
            || grant.scopes != ["device_transport", "provenance_publish", "seat_issuance"]
            || h.previous_head_id.is_some()
            || h.issued_at != grant.not_before
            || !h.revoked_devices.is_empty()
            || !h.current_binding_ids.is_empty()
            || !h.revoked_binding_ids.is_empty()
            || h.active_devices.len() != 1
        {
            return Err(E::InvalidProof);
        }
        let entry = &h.active_devices[0];
        if entry.device_id != grant.device_id
            || entry.device_pubkey != grant.device_pubkey
            || entry.device_epoch != grant.device_epoch
        {
            return Err(E::InvalidProof);
        }
        Ok(())
    }
}
impl<T: Serialize> Signed<T> {
    fn issue(kind: &str, payload: T, agent: &Agent) -> Result<Self> {
        let bytes = signed_bytes(kind, &payload)?;
        let sig = agent.sign(Input::Raw(&bytes))?;
        Ok(Self {
            payload,
            digest: digest(&bytes),
            signature: B64.encode(sig),
        })
    }
    pub(crate) fn verify(&self, kind: &str, key: &VerifyingKey) -> Result<()> {
        let bytes = signed_bytes(kind, &self.payload)?;
        let signature: [u8; 64] = proof(proof(B64.decode(&self.signature))?.try_into())?;
        if self.digest != digest(&bytes) {
            return Err(E::InvalidProof);
        }
        proof(key.verify_strict(&bytes, &Signature::from_bytes(&signature)))
    }
    pub(crate) fn issue_device(kind: &str, payload: T, key: &SigningKey) -> Result<Self> {
        let bytes = signed_bytes(kind, &payload)?;
        let signature = key.sign(&bytes).to_bytes();
        proof(
            key.verifying_key()
                .verify_strict(&bytes, &Signature::from_bytes(&signature)),
        )?;
        Ok(Self {
            payload,
            digest: digest(&bytes),
            signature: B64.encode(signature),
        })
    }
}

/// No generic JSON/signing API is exported. These fixed schemas contain only
/// ASCII property names, strings, booleans/null and safe integers (no floats),
/// so sorted compact JSON is their RFC 8785/JCS representation.
fn signed_bytes(kind: &str, payload: &impl Serialize) -> Result<Vec<u8>> {
    fn sorted(value: serde_json::Value) -> Result<serde_json::Value> {
        use serde_json::Value;
        Ok(match value {
            Value::Object(map) => {
                let mut fields = std::collections::BTreeMap::new();
                for (key, value) in map {
                    if !key.is_ascii() {
                        return Err(E::InvalidProof);
                    }
                    fields.insert(key, sorted(value)?);
                }
                proof(serde_json::to_value(fields))?
            }
            Value::Array(values) => {
                Value::Array(values.into_iter().map(sorted).collect::<Result<_>>()?)
            }
            Value::Number(ref n) if n.as_u64().is_none_or(|n| n > 9_007_199_254_740_991) => {
                return Err(E::InvalidProof);
            }
            value => value,
        })
    }
    let mut bytes = format!("bboxy450-term/{kind}/v1\0").into_bytes();
    bytes.extend(proof(serde_json::to_vec(&sorted(proof(
        serde_json::to_value(payload),
    )?)?))?);
    Ok(bytes)
}
fn decode_public(encoded: &str) -> Result<VerifyingKey> {
    if encoded.len() != 64
        || !encoded
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(E::InvalidProof);
    }
    let mut key = [0; 32];
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = proof(u8::from_str_radix(&encoded[i * 2..i * 2 + 2], 16))?;
    }
    proof(VerifyingKey::from_bytes(&key))
}

/// Public primary selection and an explicit agent endpoint. Construct only in
/// the operator identity path; this is intentionally not exposed by logic/MCP.
pub struct Bootstrap {
    pub fingerprint: String,
    pub public_certificate: Vec<u8>,
    pub keygrip: String,
    pub agent_socket: PathBuf,
    pub timeout: Duration,
}
#[derive(Debug)]
pub enum BootstrapOutcome {
    Certified(Box<DeviceIdentity>),
    Pending(E),
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingDevice {
    id: Ulid,
    created: u32,
    root: String,
}

impl Bootstrap {
    pub fn run(
        &self,
        account: &SqliteStore,
        keys: &dyn DeviceKeyStore,
        now: u64,
    ) -> BootstrapOutcome {
        bbt_rt::assert_not_ui_thread();
        if let Err(reason) =
            super::status::attempted(account, &self.fingerprint, E::BootstrapInProgress, now)
        {
            return BootstrapOutcome::Pending(reason);
        }
        match self.install(account, keys, now) {
            Ok(identity) => BootstrapOutcome::Certified(Box::new(identity)),
            Err(reason) => {
                let reason = super::status::attempted(account, &self.fingerprint, reason, now)
                    .err()
                    .unwrap_or(reason);
                BootstrapOutcome::Pending(reason)
            }
        }
    }

    fn install(
        &self,
        account: &SqliteStore,
        keys: &dyn DeviceKeyStore,
        now: u64,
    ) -> Result<DeviceIdentity> {
        let root = Root::parse(&self.public_certificate, &self.fingerprint, now)?;
        let device = super::local::first_device(account, keys)?;
        let created: u32 = proof(now.try_into())?;
        let mut db = account.connection().map_err(|_| E::StorageUnavailable)?;
        // Persist only a public retry identity first. A cancelled root operation
        // retains the same Keychain item and never installs partial authority.
        let stage = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| E::StorageUnavailable)?;
        let head: Option<Vec<u8>> = stage
            .query_row("SELECT value FROM kv WHERE key='identity/head'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(|_| E::StorageUnavailable)?;
        if let Some(head) = head {
            let head: Signed<AccountHead> = proof(serde_json::from_slice(&head))?;
            let id = head
                .payload
                .active_devices
                .first()
                .ok_or(E::JournalConflict)?
                .device_id;
            let identity: Vec<u8> = stage
                .query_row(
                    "SELECT value FROM kv WHERE key=?1",
                    [format!("identity/device/{id}")],
                    |r| r.get(0),
                )
                .map_err(|_| E::StorageUnavailable)?;
            let identity: DeviceIdentity = proof(serde_json::from_slice(&identity))?;
            identity.verify(&self.fingerprint, &head, now)?;
            let key = keys.load(id)?.ok_or(E::KeychainUnavailable(-25300))?;
            if hex(&key.verifying_key().to_bytes()) != identity.grant.payload.device_pubkey {
                return Err(E::InvalidProof);
            }
            if super::transition::get(&stage)
                .map_err(|_| E::JournalConflict)?
                .is_none()
            {
                super::transition::bootstrap(
                    &stage,
                    head.payload.root_pubkey.clone(),
                    self.fingerprint.clone(),
                    head.digest.clone(),
                    now,
                )
                .map_err(|_| E::JournalConflict)?;
                stage
                    .execute(
                        "UPDATE kv SET value=?1 WHERE key='identity/head'",
                        [proof(serde_json::to_vec(&head))?],
                    )
                    .map_err(|_| E::StorageUnavailable)?;
            }
            stage
                .execute("DELETE FROM kv WHERE key='identity/attempt'", [])
                .map_err(|_| E::StorageUnavailable)?;
            stage.commit().map_err(|_| E::StorageUnavailable)?;
            return Ok(identity);
        }
        let pending: Option<Vec<u8>> = stage
            .query_row(
                "SELECT value FROM kv WHERE key='identity/pending'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| E::StorageUnavailable)?;
        if pending.is_none() {
            let pending = PendingDevice {
                id: device,
                created,
                root: root.fingerprint.clone(),
            };
            stage
                .execute(
                    "INSERT INTO kv VALUES('identity/pending',?1)",
                    [proof(serde_json::to_vec(&pending))?],
                )
                .map_err(|_| E::StorageUnavailable)?;
        }
        stage.commit().map_err(|_| E::StorageUnavailable)?;
        // One authority writer, including Keychain creation and the three agent
        // operations. A failed transaction rolls back all active authority.
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| E::StorageUnavailable)?;
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM kv WHERE key='identity/head')",
                [],
                |r| r.get(0),
            )
            .map_err(|_| E::StorageUnavailable)?;
        if active {
            return Err(E::JournalConflict);
        }
        let pending: Vec<u8> = tx
            .query_row(
                "SELECT value FROM kv WHERE key='identity/pending'",
                [],
                |r| r.get(0),
            )
            .map_err(|_| E::StorageUnavailable)?;
        let pending: PendingDevice = proof(serde_json::from_slice(&pending))?;
        if pending.root != root.fingerprint || pending.created > created {
            return Err(E::FingerprintMismatch);
        }
        let key = match keys.load(pending.id)? {
            Some(key) => key,
            None => {
                let mut seed = [0; 32];
                getrandom::fill(&mut seed).map_err(|_| E::InvalidProof)?;
                let key = SigningKey::from_bytes(&seed);
                seed.fill(0);
                keys.create(pending.id, &key)?;
                let saved = keys
                    .load(pending.id)?
                    .ok_or(E::KeychainUnavailable(-25300))?;
                if saved.verifying_key() != key.verifying_key() {
                    return Err(E::InvalidProof);
                }
                key
            }
        };
        let agent = Agent::new(
            self.agent_socket.clone(),
            self.keygrip.clone(),
            root.public,
            self.timeout,
        )?;
        let certificate = root.certify(&agent, &key, pending.id, pending.created)?;
        let root_pubkey = hex(&root.public.to_bytes());
        let device_pubkey = hex(&key.verifying_key().to_bytes());
        let grant = Signed::issue(
            "DeviceGrant",
            DeviceGrant {
                version: 1,
                root_pubkey: root_pubkey.clone(),
                account_epoch: "1".into(),
                device_id: pending.id,
                device_pubkey: device_pubkey.clone(),
                device_epoch: "1".into(),
                serial: "1".into(),
                scopes: ["device_transport", "provenance_publish", "seat_issuance"]
                    .map(str::to_string)
                    .to_vec(),
                role_ceiling: ["oracle", "worker"].map(str::to_string).to_vec(),
                not_before: now,
                expires_at: root
                    .valid_until
                    .unwrap_or(u64::MAX)
                    .min(now + 30 * 24 * 3600),
            },
            &agent,
        )?;
        let head = Signed::issue(
            "AccountHead",
            AccountHead {
                version: 1,
                root_pubkey,
                account_epoch: "1".into(),
                previous_head_id: None,
                active_devices: vec![ActiveDevice {
                    device_id: pending.id,
                    device_pubkey,
                    device_epoch: "1".into(),
                }],
                revoked_devices: vec![],
                current_binding_ids: vec![],
                revoked_binding_ids: vec![],
                issued_at: now,
            },
            &agent,
        )?;
        let identity = DeviceIdentity {
            custody: keys.custody(),
            root_fingerprint: root.fingerprint,
            root_certificate: B64.encode(&self.public_certificate),
            device_certificate: B64.encode(certificate),
            grant,
        };
        identity.verify(&self.fingerprint, &head, now)?;
        super::transition::bootstrap(
            &tx,
            head.payload.root_pubkey.clone(),
            self.fingerprint.clone(),
            head.digest.clone(),
            now,
        )
        .map_err(|_| E::JournalConflict)?;
        for (key, value) in [
            (
                "identity/head".to_string(),
                proof(serde_json::to_vec(&head))?,
            ),
            (
                format!("identity/device/{}", pending.id),
                proof(serde_json::to_vec(&identity))?,
            ),
            ("identity/roster".to_string(), b"[]".to_vec()),
        ] {
            tx.execute("INSERT INTO kv VALUES(?1,?2)", params![key, value])
                .map_err(|_| E::StorageUnavailable)?;
        }
        tx.execute("DELETE FROM kv WHERE key='identity/pending'", [])
            .map_err(|_| E::StorageUnavailable)?;
        tx.execute("DELETE FROM kv WHERE key='identity/attempt'", [])
            .map_err(|_| E::StorageUnavailable)?;
        tx.commit().map_err(|_| E::StorageUnavailable)?;
        Ok(identity)
    }
}
