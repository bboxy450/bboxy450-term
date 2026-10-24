//! Seat secrets live only in this logic instance. All durable rows are proofs.
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, Mutex},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use ed25519_dalek::SigningKey;
use pgp::types::KeyDetails;
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use ulid::Ulid;

use super::{
    AccountHead, DeviceIdentity, DeviceKeyStore, KeyStatus, KeychainDeviceKeys, RosterEntry,
    SeatGrant, SeatProof, SeatRoster, Signed,
    certificate::{self, Root},
    transition,
};
use crate::{
    messaging::{
        keys::{LocalSeatKeys, SeatKeys},
        wire::{Envelope, EnvelopeExt, Issuer, IssuerKind, parse_public, public},
    },
    registry::SeatRecord,
    store::{Result, Store, digest, invalid, object_dir, sqlite::SqliteStore},
};

type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;
struct Leaf {
    key: SigningKey,
}
#[derive(Default)]
struct Memory {
    leaves: BTreeMap<(Ulid, u64), Leaf>,
    verified: BTreeSet<(Ulid, u64, u64, String)>,
    renewal: BTreeMap<(Ulid, u64, String), (Envelope, SeatRecord)>,
}
pub struct ChainSeatKeys {
    pub(super) account: SqliteStore,
    identity: DeviceIdentity,
    initial_head: Signed<AccountHead>,
    pinned: String,
    custody: Arc<dyn DeviceKeyStore + Send + Sync>,
    local: LocalSeatKeys,
    issuer: Issuer,
    clock: Clock,
    memory: Mutex<Memory>,
    renewal_hold: Mutex<Option<String>>,
}
impl ChainSeatKeys {
    pub fn open(
        account: SqliteStore,
        custody: Arc<dyn DeviceKeyStore + Send + Sync>,
        pinned: &str,
        local: SigningKey,
    ) -> Result<Self> {
        Self::with_clock(
            account,
            custody,
            pinned,
            local,
            Arc::new(|| crate::store::now_ms() / 1000),
        )
    }
    /// IO clock seam for deterministic expiry/renewal tests, never a seat API.
    pub fn with_clock(
        account: SqliteStore,
        custody: Arc<dyn DeviceKeyStore + Send + Sync>,
        pinned: &str,
        local: SigningKey,
        clock: Clock,
    ) -> Result<Self> {
        let state = transition::read(&account)?
            .ok_or_else(|| invalid("identity issuer not bootstrapped"))?;
        if state.root_fingerprint.as_deref() != Some(pinned) {
            return Err(invalid("foreign root fingerprint"));
        }
        let head: Signed<AccountHead> = read(&account, "identity/head")?;
        let id = head
            .payload
            .active_devices
            .first()
            .ok_or_else(|| invalid("no active device"))?
            .device_id;
        let identity: DeviceIdentity = read(&account, &format!("identity/device/{id}"))?;
        // Authenticate installed public authority at its signed time. Current
        // expiry/custody are signing holds, not a reason to stop logic or seats;
        // every new authenticated effect still checks device_current().
        identity.verify(pinned, &head, head.payload.issued_at)?;
        if state.kind == IssuerKind::GpgRoot && state.key != identity.grant.payload.root_pubkey {
            return Err(invalid("foreign root issuer journal"));
        }
        let provider = Self {
            account,
            identity,
            initial_head: head,
            pinned: pinned.into(),
            custody,
            local: LocalSeatKeys::new(local),
            issuer: Issuer {
                kind: state.kind,
                key: state.key,
            },
            clock,
            memory: Mutex::new(Memory::default()),
            renewal_hold: Mutex::new(None),
        };
        Ok(provider)
    }
    fn device_key(&self) -> Result<SigningKey> {
        let grant = &self.identity.grant.payload;
        let key = self
            .custody
            .load(grant.device_id)?
            .ok_or_else(|| invalid("device Keychain item unavailable"))?;
        if public(&key.verifying_key().to_bytes()) != grant.device_pubkey {
            return Err(invalid("wrong device signing key"));
        }
        Ok(key)
    }
    fn device_current(&self, now: u64) -> Result<()> {
        if now >= self.identity.grant.payload.expires_at {
            return Err(invalid("expired DeviceGrant"));
        }
        let head: Signed<AccountHead> = read(&self.account, "identity/head")?;
        let root = Root::parse(
            &B64.decode(&self.identity.root_certificate)?,
            &self.pinned,
            now,
        )?;
        head.verify("AccountHead", &root.public)?;
        if head
            .payload
            .revoked_devices
            .contains(&self.identity.grant.payload.device_id)
        {
            return Err(invalid("revoked device epoch"));
        }
        if head.payload.account_epoch != self.identity.grant.payload.account_epoch {
            return Err(invalid("stale device account epoch"));
        }
        self.identity.verify(&self.pinned, &head, now)?;
        Ok(())
    }
    pub fn proof(&self, id: &str) -> Result<SeatProof> {
        read(&self.account, &format!("identity/seat-grant/{id}"))
    }
    pub fn roster(&self) -> Result<Option<Signed<SeatRoster>>> {
        let bytes = self
            .account
            .get("identity/roster")?
            .ok_or_else(|| invalid("missing roster"))?;
        if bytes == b"[]" {
            return Ok(None);
        }
        let roster: Signed<SeatRoster> = serde_json::from_slice(&bytes)?;
        roster.verify(
            "SeatRoster",
            &ed25519_dalek::VerifyingKey::from_bytes(&parse_public(
                &self.identity.grant.payload.device_pubkey,
            )?)?,
        )?;
        Ok(Some(roster))
    }
    pub fn still_local(&self) -> Result<usize> {
        let state =
            transition::read(&self.account)?.ok_or_else(|| invalid("missing issuer state"))?;
        super::status::local_count(&self.account, Some(&state))
    }
    /// Safe identity-card snapshot on the logic/IO owner; never root signing.
    pub fn status_at(&self, now: u64) -> Result<super::IdentityStatus> {
        bbt_rt::assert_not_ui_thread();
        let mut status = super::status::read(&self.account, now)?;
        if status.custody == Some(self.custody.custody()) {
            status.custody_label = Some(self.custody.custody_label().into());
        }
        if status.bootstrapped
            && let Some(reason) = self.signing_hold(now)
        {
            status.hold(reason);
        }
        Ok(status)
    }
    pub(super) fn signing_hold(&self, now: u64) -> Option<super::PendingReason> {
        use super::PendingReason as E;
        if now >= self.identity.grant.payload.expires_at {
            return Some(E::DeviceGrantExpired);
        }
        if self.device_current(now).is_err() {
            return Some(E::InvalidProof);
        }
        match self.custody.load(self.identity.grant.payload.device_id) {
            Err(reason) => return Some(reason),
            Ok(None) => return Some(E::KeychainUnavailable(-25300)),
            Ok(Some(key))
                if public(&key.verifying_key().to_bytes())
                    != self.identity.grant.payload.device_pubkey =>
            {
                return Some(E::InvalidProof);
            }
            Ok(Some(_)) => {}
        }
        self.renewal_hold().map(|_| E::SeatRenewalUnavailable)
    }
    pub(super) fn event_signer(&self) -> Result<SigningKey> {
        self.device_current((self.clock)())?;
        let key = self
            .custody
            .load(self.identity.grant.payload.device_id)?
            .ok_or_else(|| invalid("device key unavailable"))?;
        if public(&key.verifying_key().to_bytes()) != self.identity.grant.payload.device_pubkey {
            return Err(invalid("device key mismatch"));
        }
        Ok(key)
    }
    pub(crate) fn operator_signing(&self) -> Result<crate::messaging::keys::OperatorSigning> {
        let state = transition::read(&self.account)?
            .ok_or_else(|| invalid("held: missing issuer journal"))?;
        if state.kind != IssuerKind::GpgRoot {
            return Err(invalid("held: certified device authority is not active"));
        }
        let key = self.event_signer()?;
        let grant = &self.identity.grant.payload;
        Ok(crate::messaging::keys::OperatorSigning {
            device_id: grant.device_id,
            issuer: Issuer {
                kind: IssuerKind::GpgRoot,
                key: grant.root_pubkey.clone(),
            },
            device: grant.device_pubkey.clone(),
            grant: self.identity.grant.digest.clone(),
            origin: Default::default(),
            epoch: transition::counter(&grant.device_epoch)?,
            key,
        })
    }
    pub(super) fn sign_hold_closure(
        &self,
        window: super::HoldWindow,
    ) -> Result<Signed<super::HoldClosure>> {
        let key = self.event_signer()?;
        let grant = &self.identity.grant.payload;
        let closure = Signed::issue_device(
            "HoldClosure",
            super::HoldClosure {
                version: 1,
                device_id: grant.device_id,
                device_pubkey: grant.device_pubkey.clone(),
                device_epoch: grant.device_epoch.clone(),
                closed_at: crate::store::now_ms(),
                window,
            },
            &key,
        )?;
        closure.verify_device(&self.identity)?;
        Ok(closure)
    }
    pub fn renewal_hold(&self) -> Option<String> {
        self.renewal_hold.lock().ok().and_then(|h| h.clone())
    }
    /// The single IO-runtime maintenance job calls this even for idle seats.
    /// The grant alias is updated atomically; original envelopes stay untouched.
    pub fn renew_due(&self) -> Result<usize> {
        let state =
            transition::read(&self.account)?.ok_or_else(|| invalid("missing issuer journal"))?;
        if state.kind != IssuerKind::GpgRoot {
            return Ok(0);
        }
        self.device_current((self.clock)())?;
        let templates = self
            .memory
            .lock()
            .map_err(|_| invalid("seat-key memory poisoned"))?
            .renewal
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let registry = crate::registry::Registry::new(self.account.clone());
        let root = self
            .account
            .path
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| invalid("account has no run directory"))?;
        let mut renewed = 0;
        for (mut env, old_record) in templates {
            let record = registry.get(env.seat)?;
            if record.generation != env.process_generation
                || record.role != old_record.role
                || record.muted != old_record.muted
                || record.swarm != old_record.swarm
                || record.location != old_record.location
            {
                continue;
            }
            let proof = self.proof(&env.role_grant_id)?;
            let grant = &proof.grant.payload;
            if (self.clock)() < grant.not_before + (grant.expires_at - grant.not_before) / 2 {
                continue;
            }
            let tree = crate::messaging::tree::Tree::open(
                SqliteStore::open(&object_dir(root, &format!("swarm-{}", record.swarm))?)?,
                record.swarm,
            )?;
            let base = env
                .context
                .strip_suffix("/**")
                .or_else(|| env.context.strip_suffix("/*"))
                .unwrap_or(&env.context);
            if tree
                .check(
                    crate::messaging::tree::Actor::Seat {
                        seat: env.seat,
                        class: record.role.class,
                    },
                    base,
                    crate::messaging::tree::Right::Write,
                )
                .is_err()
            {
                continue;
            }
            let mut db = self.account.connection()?;
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            self.finalize_chain(&mut env, &record, &tx)?;
            tx.commit()?;
            renewed += 1;
        }
        Ok(renewed)
    }
    /// Signature/history verification never admits a delivery by itself.
    pub fn verify_history(&self, env: &Envelope) -> Result<()> {
        if env.issuer_kind == IssuerKind::LocalDev {
            return self.verify_local(env);
        }
        self.verify_chain(env, env.timestamp, false)
    }
    fn verify_local(&self, env: &Envelope) -> Result<()> {
        let state =
            transition::read(&self.account)?.ok_or_else(|| invalid("missing issuer state"))?;
        let entry = state.legacy.iter().find(|e| {
            e.seat == env.seat && e.generation == env.process_generation && e.issuer == env.issuer
        });
        if let Some(entry) = entry {
            let mut before = entry.before;
            // Fence immediately when the registry advances, even if the new
            // generation has not sent its first chain message yet.
            if before.is_none()
                && crate::registry::Registry::new(self.account.clone())
                    .get(env.seat)
                    .is_ok_and(|r| r.generation > entry.generation)
            {
                let last: Option<u64> = self
                    .account
                    .connection()?
                    .query_row(
                        "SELECT sequence FROM msg_counters WHERE seat=?1 AND generation=?2",
                        params![
                            env.seat.to_string(),
                            crate::store::sqlint(entry.generation)?
                        ],
                        |r| crate::messaging::ledger::u64_at(r, 0),
                    )
                    .optional()?;
                before = Some(
                    last.unwrap_or(0)
                        .checked_add(1)
                        .ok_or_else(|| invalid("legacy sequence exhausted"))?,
                );
            }
            if before.is_some_and(|limit| env.sender_sequence >= limit) {
                return Err(invalid("local-dev sender sequence is after cutover"));
            }
            if entry.author.as_ref() != Some(&env.author)
                || env.account != env.issuer
                || env.device != env.issuer
            {
                return Err(invalid("untrusted historical local-dev key"));
            }
            return env.verify_signature();
        }
        Err(invalid(
            "local-dev issuer or generation was not admitted before cutover",
        ))
    }
    fn verify_chain(&self, env: &Envelope, now: u64, current: bool) -> Result<()> {
        if env.issuer_kind != IssuerKind::GpgRoot
            || env.issuer != self.identity.grant.payload.root_pubkey
            || env.account != env.issuer
        {
            return Err(invalid("foreign root issuer"));
        }
        if env.device != self.identity.grant.payload.device_pubkey {
            return Err(invalid("wrong device in envelope"));
        }
        if current {
            self.device_current(now)?;
        } else {
            self.identity
                .verify(&self.pinned, &self.initial_head, now)?;
        }
        let proof = self.proof(&env.role_grant_id)?;
        let grant = &proof.grant.payload;
        if grant.expires_at <= now || env.timestamp >= grant.expires_at {
            return Err(invalid("expired SeatGrant"));
        }
        if grant.not_before > now || env.timestamp < grant.not_before {
            return Err(invalid("SeatGrant not yet valid"));
        }
        let epoch = transition::counter(&grant.grant_epoch)?;
        if env.role_epoch != epoch {
            return Err(invalid("stale grant_epoch"));
        }
        if current
            && let Some(bytes) = self
                .account
                .get(&format!("identity/seat-floor/{}", env.seat))?
        {
            let floor: Signed<Revocation> = serde_json::from_slice(&bytes)?;
            floor.verify(
                "SeatRevocation",
                &ed25519_dalek::VerifyingKey::from_bytes(&parse_public(&env.device)?)?,
            )?;
            if epoch < transition::counter(&floor.payload.grant_floor)? {
                return Err(invalid("revoked grant_epoch"));
            }
        }
        if grant.root_pubkey != env.account
            || grant.device_grant_id != self.identity.grant.digest
            || grant.device_id != self.identity.grant.payload.device_id
            || grant.device_epoch != self.identity.grant.payload.device_epoch
            || grant.seat_id != env.seat
            || grant.seat_pubkey != env.author
            || proof.process_generation != env.process_generation
            || grant.swarm_id != env.swarm_id
            || grant.role != env.role
            || grant.role.class != env.priority_class
            || grant.expires_at > self.identity.grant.payload.expires_at
            || grant.expires_at <= grant.not_before
            || grant.expires_at - grant.not_before > 24 * 3600
            || !grant.control_scopes.is_empty()
            || grant.destinations != [env.context.clone()]
        {
            return Err(invalid("SeatGrant scope or chain mismatch"));
        }
        let class = match grant.role.class {
            crate::registry::Class::Oracle => "oracle",
            crate::registry::Class::Worker => "worker",
        };
        if !self
            .identity
            .grant
            .payload
            .role_ceiling
            .iter()
            .any(|c| c == class)
        {
            return Err(invalid("seat class exceeds device ceiling"));
        }
        let key_epoch = transition::counter(&grant.key_epoch)?;
        if current && let Some(roster) = self.roster()? {
            let latest = roster
                .payload
                .entries
                .iter()
                .filter(|e| e.seat_id == env.seat)
                .map(|e| transition::counter(&e.key_epoch))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .max()
                .unwrap_or(0);
            if key_epoch < latest {
                return Err(invalid("stale key_epoch"));
            }
        }
        let cache = (
            env.seat,
            key_epoch,
            epoch,
            digest(&serde_json::to_vec(&proof)?),
        );
        let mut memory = self
            .memory
            .lock()
            .map_err(|_| invalid("seat-key memory poisoned"))?;
        if !memory.verified.contains(&cache) {
            let device = ed25519_dalek::VerifyingKey::from_bytes(&parse_public(&env.device)?)?;
            proof.grant.verify("SeatGrant", &device)?;
            certificate::verify_seat(
                &self.identity.public_device_certificate()?,
                &B64.decode(&proof.certificate)?,
                &parse_public(&env.author)?,
                env.seat,
                key_epoch,
                now,
            )?;
            memory.verified.insert(cache);
        }
        env.verify_signature()
    }
    pub fn revoke_seat(&self, seat: Ulid) -> Result<u64> {
        self.device_current((self.clock)())?;
        let key = self.device_key()?;
        let mut db = self.account.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let max: Option<u64> = tx
            .query_row(
                "SELECT epoch FROM msg_policies WHERE seat=?1",
                [seat.to_string()],
                |r| crate::messaging::ledger::u64_at(r, 0),
            )
            .optional()?;
        let old_floor: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("identity/seat-floor/{seat}")],
                |r| r.get(0),
            )
            .optional()?;
        let old_floor = old_floor
            .map(|bytes| -> Result<u64> {
                let old: Signed<Revocation> = serde_json::from_slice(&bytes)?;
                old.verify("SeatRevocation", &key.verifying_key())?;
                transition::counter(&old.payload.grant_floor)
            })
            .transpose()?
            .unwrap_or(0);
        let floor = max
            .unwrap_or(1)
            .max(old_floor)
            .checked_add(1)
            .ok_or_else(|| invalid("seat grant epoch exhausted"))?;
        let revocation = Signed::issue_device(
            "SeatRevocation",
            Revocation {
                seat_id: seat,
                device_id: self.identity.grant.payload.device_id,
                grant_floor: floor.to_string(),
                issued_at: (self.clock)(),
            },
            &key,
        )?;
        tx.execute(
            "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![
                format!("identity/seat-floor/{seat}"),
                serde_json::to_vec(&revocation)?
            ],
        )?;
        if let Some(mut roster) = roster_tx(&tx)? {
            roster.verify("SeatRoster", &key.verifying_key())?;
            for entry in roster
                .payload
                .entries
                .iter_mut()
                .filter(|e| e.seat_id == seat)
            {
                entry.status = KeyStatus::Revoked;
            }
            write_roster(
                &tx,
                roster.payload,
                Some(roster.digest),
                &key,
                (self.clock)(),
            )?;
        }
        tx.commit()?;
        Ok(floor)
    }
    fn finalize_chain(
        &self,
        env: &mut Envelope,
        record: &SeatRecord,
        tx: &Transaction<'_>,
    ) -> Result<()> {
        let now = (self.clock)();
        self.device_current(now)?;
        transition::fence(tx, env.seat, env.process_generation)?;
        let device = self.device_key()?;
        let floor: Option<Vec<u8>> = tx
            .query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("identity/seat-floor/{}", env.seat)],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(floor) = floor {
            let floor: Signed<Revocation> = serde_json::from_slice(&floor)?;
            floor.verify("SeatRevocation", &device.verifying_key())?;
            if env.role_epoch < transition::counter(&floor.payload.grant_floor)? {
                return Err(invalid("revoked grant_epoch"));
            }
        }
        let mut memory = self
            .memory
            .lock()
            .map_err(|_| invalid("seat-key memory poisoned"))?;
        let leaf = memory
            .leaves
            .get(&(env.seat, env.process_generation))
            .ok_or_else(|| invalid("seat leaf not generated"))?;
        if public(&leaf.key.verifying_key().to_bytes()) != env.author {
            return Err(invalid("seat generation key mismatch"));
        }
        let prior = roster_tx(tx)?;
        if let Some(prior) = &prior {
            prior.verify("SeatRoster", &device.verifying_key())?;
        }
        let mut roster = prior
            .as_ref()
            .map(|r| r.payload.clone())
            .unwrap_or(SeatRoster {
                version: 1,
                root_pubkey: self.identity.grant.payload.root_pubkey.clone(),
                device_grant_id: self.identity.grant.digest.clone(),
                device_epoch: self.identity.grant.payload.device_epoch.clone(),
                roster_sequence: "0".into(),
                previous_roster_id: None,
                issued_at: now,
                entries: vec![],
            });
        let current = roster
            .entries
            .iter()
            .find(|e| e.seat_id == env.seat && e.seat_pubkey == env.author);
        let (epoch, certificate) = if let Some(entry) = current {
            (
                transition::counter(&entry.key_epoch)?,
                entry.certificate.clone(),
            )
        } else {
            let epoch = roster
                .entries
                .iter()
                .filter(|e| e.seat_id == env.seat)
                .map(|e| transition::counter(&e.key_epoch))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| invalid("seat key epoch exhausted"))?;
            (
                epoch,
                B64.encode(certificate::certify_seat(
                    &self.identity.public_device_certificate()?,
                    &device,
                    &leaf.key,
                    env.seat,
                    epoch,
                    now.try_into()?,
                )?),
            )
        };
        let alias = format!(
            "identity/seat-current/{}/{}/{}/{}",
            env.seat,
            env.process_generation,
            env.role_epoch,
            digest(env.context.as_bytes())
        );
        let previous: Option<Vec<u8>> = tx
            .query_row("SELECT value FROM kv WHERE key=?1", [&alias], |r| r.get(0))
            .optional()?;
        let old = previous
            .as_ref()
            .map(|b| std::str::from_utf8(b))
            .transpose()?;
        if let Some(old) = old {
            let existing: Vec<u8> = tx.query_row(
                "SELECT value FROM kv WHERE key=?1",
                [format!("identity/seat-grant/{old}")],
                |r| r.get(0),
            )?;
            let proof: SeatProof = serde_json::from_slice(&existing)?;
            let grant = &proof.grant.payload;
            if grant.seat_pubkey == env.author
                && now < grant.not_before + (grant.expires_at - grant.not_before) / 2
                && now < grant.expires_at
            {
                env.device = self.identity.grant.payload.device_pubkey.clone();
                env.role_grant_id = old.into();
                return Ok(());
            }
        }
        let signed = Signed::issue_device(
            "SeatGrant",
            SeatGrant {
                version: 1,
                root_pubkey: self.identity.grant.payload.root_pubkey.clone(),
                device_grant_id: self.identity.grant.digest.clone(),
                device_id: self.identity.grant.payload.device_id,
                device_epoch: self.identity.grant.payload.device_epoch.clone(),
                seat_id: env.seat,
                seat_pubkey: env.author.clone(),
                key_epoch: epoch.to_string(),
                grant_epoch: env.role_epoch.to_string(),
                previous_grant_id: old.map(str::to_string),
                swarm_id: env.swarm_id,
                harness_kind: record.harness_kind.clone(),
                role: env.role.clone(),
                destinations: vec![env.context.clone()],
                control_scopes: vec![],
                muted: record.muted,
                location: record.location.clone(),
                not_before: now,
                expires_at: (now + 24 * 3600).min(self.identity.grant.payload.expires_at),
            },
            &device,
        )?;
        let reference = signed.digest.clone();
        let proof = SeatProof {
            certificate: certificate.clone(),
            process_generation: env.process_generation,
            grant: signed,
        };
        tx.execute(
            "INSERT INTO kv VALUES(?1,?2)",
            params![
                format!("identity/seat-grant/{reference}"),
                serde_json::to_vec(&proof)?
            ],
        )?;
        tx.execute(
            "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![alias, reference.as_bytes()],
        )?;
        for entry in roster.entries.iter_mut().filter(|e| {
            e.seat_id == env.seat && e.seat_pubkey != env.author && e.status == KeyStatus::Current
        }) {
            entry.status = KeyStatus::Retired;
        }
        let cert = certificate::parse(&B64.decode(&certificate)?)?;
        let entry = RosterEntry {
            seat_id: env.seat,
            seat_pubkey: env.author.clone(),
            key_epoch: epoch.to_string(),
            character_name: record.name.clone(),
            harness_kind: record.harness_kind.clone(),
            role: env.role.clone(),
            valid_from: now,
            valid_until: proof.grant.payload.expires_at,
            status: KeyStatus::Current,
            certificate,
            certificate_fingerprint: cert.fingerprint().to_string(),
            grant_reference: reference.clone(),
        };
        if let Some(old) = roster
            .entries
            .iter_mut()
            .find(|e| e.seat_id == env.seat && e.seat_pubkey == env.author)
        {
            *old = entry;
        } else {
            roster.entries.push(entry);
        }
        write_roster(tx, roster, prior.map(|r| r.digest), &device, now)?;
        env.device = self.identity.grant.payload.device_pubkey.clone();
        env.role_grant_id = reference;
        let mut template = env.clone();
        template.body = crate::messaging::wire::Body::Text {
            text: String::new(),
        };
        template.attachments.clear();
        template.signature.clear();
        memory.renewal.insert(
            (env.seat, env.process_generation, env.context.clone()),
            (template, record.clone()),
        );
        // The immutable proof is cached only after verification in verify().
        memory.verified.clear();
        Ok(())
    }
}
impl SeatKeys for ChainSeatKeys {
    fn issuer(&self) -> Issuer {
        self.issuer.clone()
    }
    fn leaf(&self, seat: Ulid, generation: u64) -> Result<SigningKey> {
        if seat == Ulid::nil() || generation == 0 {
            return Err(invalid("invalid seat generation"));
        }
        let state =
            transition::read(&self.account)?.ok_or_else(|| invalid("missing issuer journal"))?;
        if transition::legacy(&state, seat, generation) {
            return self.local.leaf(seat, generation);
        }
        let mut memory = self
            .memory
            .lock()
            .map_err(|_| invalid("seat-key memory poisoned"))?;
        if let Some(leaf) = memory.leaves.get(&(seat, generation)) {
            return Ok(leaf.key.clone());
        }
        let mut seed = [0; 32];
        getrandom::fill(&mut seed).map_err(std::io::Error::other)?;
        let key = SigningKey::from_bytes(&seed);
        seed.fill(0);
        memory
            .leaves
            .insert((seat, generation), Leaf { key: key.clone() });
        Ok(key)
    }
    fn verify(&self, env: &Envelope) -> Result<()> {
        if env.seat == Ulid::nil() {
            if env.issuer_kind != IssuerKind::GpgRoot {
                return Err(invalid(
                    "certified receivers refuse local-dev operator messages",
                ));
            }
            self.device_current((self.clock)())?;
            let state = transition::read(&self.account)?
                .ok_or_else(|| invalid("missing issuer journal"))?;
            let grant = &self.identity.grant.payload;
            if state.kind != IssuerKind::GpgRoot
                || env.issuer != grant.root_pubkey
                || env.account != grant.root_pubkey
                || env.device != grant.device_pubkey
                || env.author != grant.device_pubkey
                || env.role_grant_id != self.identity.grant.digest
                || env.role_epoch != transition::counter(&grant.device_epoch)?
                || env.timestamp < grant.not_before
                || env.timestamp >= grant.expires_at
            {
                return Err(invalid("operator envelope does not match DeviceGrant"));
            }
            return env.verify_signature();
        }
        if env.issuer_kind == IssuerKind::LocalDev {
            return self.verify_local(env);
        }
        let state =
            transition::read(&self.account)?.ok_or_else(|| invalid("missing issuer journal"))?;
        if state.kind != IssuerKind::GpgRoot {
            return Err(invalid("gpg-root issuer retired by identity reset"));
        }
        self.verify_chain(env, (self.clock)(), true)
    }
    fn finalize(
        &self,
        env: &mut Envelope,
        record: &SeatRecord,
        tx: &Transaction<'_>,
    ) -> Result<()> {
        let state = transition::get(tx)?.ok_or_else(|| invalid("missing issuer journal"))?;
        if transition::legacy(&state, env.seat, env.process_generation) {
            let issuer = self.local.issuer();
            if state.kind == IssuerKind::LocalDev && state.key != issuer.key {
                return Err(invalid("identity reset requires the new local-dev key"));
            }
            env.issuer_kind = issuer.kind;
            env.issuer = issuer.key.clone();
            env.account = issuer.key.clone();
            env.device = issuer.key;
            return transition::remember_local(tx, env);
        }
        self.finalize_chain(env, record, tx)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Revocation {
    seat_id: Ulid,
    device_id: Ulid,
    grant_floor: String,
    issued_at: u64,
}
fn read<T: serde::de::DeserializeOwned>(account: &SqliteStore, key: &str) -> Result<T> {
    Ok(serde_json::from_slice(
        &account
            .get(key)?
            .ok_or_else(|| invalid(format!("missing {key}")))?,
    )?)
}
fn roster_tx(tx: &Transaction<'_>) -> Result<Option<Signed<SeatRoster>>> {
    let bytes: Vec<u8> = tx.query_row(
        "SELECT value FROM kv WHERE key='identity/roster'",
        [],
        |r| r.get(0),
    )?;
    if bytes == b"[]" {
        Ok(None)
    } else {
        Ok(Some(serde_json::from_slice(&bytes)?))
    }
}
fn write_roster(
    tx: &Transaction<'_>,
    mut roster: SeatRoster,
    previous: Option<String>,
    key: &SigningKey,
    now: u64,
) -> Result<()> {
    let old: u64 = roster.roster_sequence.parse()?;
    roster.roster_sequence = old
        .checked_add(1)
        .ok_or_else(|| invalid("roster sequence exhausted"))?
        .to_string();
    roster.previous_roster_id = previous;
    roster.issued_at = now;
    let signed = Signed::issue_device("SeatRoster", roster, key)?;
    let bytes = serde_json::to_vec(&signed)?;
    tx.execute(
        "INSERT INTO kv VALUES(?1,?2)",
        params![format!("identity/roster-history/{}", signed.digest), &bytes],
    )?;
    tx.execute(
        "UPDATE kv SET value=?1 WHERE key='identity/roster'",
        [&bytes],
    )?;
    Ok(())
}

pub fn seat_keys(
    run_dir: &Path,
    local: SigningKey,
    handle: tokio::runtime::Handle,
) -> Result<Arc<dyn SeatKeys>> {
    Ok(super::IdentityRuntime::open(run_dir, local, handle)?.keys())
}
impl super::IdentityRuntime {
    pub fn open(run_dir: &Path, local: SigningKey, handle: tokio::runtime::Handle) -> Result<Self> {
        Self::open_with_store(run_dir, local, handle, None)
    }
    pub(super) fn open_with_store(
        run_dir: &Path,
        local: SigningKey,
        handle: tokio::runtime::Handle,
        custody: Option<Arc<dyn DeviceKeyStore + Send + Sync>>,
    ) -> Result<Self> {
        let account = SqliteStore::open(&object_dir(run_dir, "account")?)?;
        let state = transition::read(&account)?;
        if state
            .as_ref()
            .is_none_or(|s| s.kind == IssuerKind::LocalDev)
        {
            if state.is_none() && account.get("identity/head")?.is_some() {
                return Err(invalid("bootstrapped identity lacks its issuer journal"));
            }
            return Ok(Self {
                account,
                keys: Arc::new(LocalSeatKeys::new(local.clone())),
                chain: None,
                retry: None,
                pending: None,
                bus_key: Some(local),
            });
        }
        let state = state.ok_or_else(|| invalid("missing issuer state"))?;
        let pinned = state
            .root_fingerprint
            .as_deref()
            .ok_or_else(|| invalid("missing root fingerprint"))?;
        let head: Signed<AccountHead> = read(&account, "identity/head")?;
        let id = head
            .payload
            .active_devices
            .first()
            .ok_or_else(|| invalid("no active device"))?
            .device_id;
        let identity: DeviceIdentity = read(&account, &format!("identity/device/{id}"))?;
        let custody = match custody {
            Some(custody) => custody,
            None => Arc::new(KeychainDeviceKeys::resume(identity.custody)?),
        };
        let chain = Arc::new(ChainSeatKeys::open(
            account.clone(),
            custody,
            pinned,
            local,
        )?);
        let weak = Arc::downgrade(&chain);
        let task = handle.spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let Some(chain) = weak.upgrade() else {
                    break;
                };
                let job = chain.clone();
                let result = tokio::task::spawn_blocking(move || job.renew_due()).await;
                if let Ok(mut hold) = chain.renewal_hold.lock() {
                    *hold = match result {
                        Ok(Ok(_)) => None,
                        Ok(Err(e)) => Some(e.to_string()),
                        Err(_) => Some("identity renewal worker unavailable".into()),
                    };
                }
            }
        });
        Ok(Self {
            account,
            keys: Arc::new(MaintainedKeys {
                chain: chain.clone(),
                task,
            }),
            chain: Some(chain),
            retry: None,
            pending: None,
            bus_key: None,
        })
    }
}

struct MaintainedKeys {
    chain: Arc<ChainSeatKeys>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for MaintainedKeys {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl SeatKeys for MaintainedKeys {
    fn issuer(&self) -> Issuer {
        self.chain.issuer()
    }
    fn signing_hold(&self) -> Option<String> {
        self.chain
            .signing_hold(crate::store::now_ms() / 1000)
            .map(|reason| reason.to_string())
    }
    fn leaf(&self, seat: Ulid, generation: u64) -> Result<SigningKey> {
        self.chain.leaf(seat, generation)
    }
    fn verify(&self, env: &Envelope) -> Result<()> {
        self.chain.verify(env)
    }
    fn finalize(
        &self,
        env: &mut Envelope,
        record: &SeatRecord,
        tx: &Transaction<'_>,
    ) -> Result<()> {
        self.chain.finalize(env, record, tx)
    }
}
