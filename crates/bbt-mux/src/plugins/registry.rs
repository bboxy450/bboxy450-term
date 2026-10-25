//! Production account authority. The logic owner supplies its already-open store.
//!
//! Filesystem/store methods run on an IO worker, not the UI or logic command
//! loop. Descriptor IO is a separate per-plugin job; its completion must still
//! match the current account generation. No login, business calls or hooks here.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use bbt_plugin::{
    Refusal,
    contracts::Contract,
    registry::{Activation, Approval, Candidate, Entry, Inspection, State},
    trust::TrustRecord,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use super::{
    check::parse_descriptor,
    client::Client,
    discovery::DiscoveryRoots,
    failure,
    trust::{hash_file, open_regular},
};
use crate::{mcp::Actor, store::sqlite::SqliteStore};

const ACTIVATIONS: &str = "plugins/activation/";

/// Host-owned deadlines, never supplied by an untrusted descriptor or wire call.
#[derive(Debug, Clone, Copy)]
pub struct RegistryConfig {
    pub descriptor_timeout: Duration,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            descriptor_timeout: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Registry {
    account: SqliteStore,
    root: PathBuf,
    roots: DiscoveryRoots,
    config: RegistryConfig,
}

fn activation_key(id: &str) -> String {
    format!("{ACTIVATIONS}{id}")
}

fn trust_key(sha256: &str) -> String {
    format!("plugins/trust/{sha256}")
}

fn load<T: DeserializeOwned>(db: &Connection, key: &str) -> Result<Option<T>, Refusal> {
    let value: Option<Vec<u8>> = db
        .query_row("SELECT value FROM kv WHERE key=?1", [key], |row| row.get(0))
        .optional()
        .map_err(|e| failure("plugin_store_unavailable", e))?;
    value
        .map(|bytes| {
            serde_json::from_slice(&bytes).map_err(|e| failure("invalid_plugin_record", e))
        })
        .transpose()
}

fn put(db: &Connection, key: &str, value: &impl Serialize) -> Result<(), Refusal> {
    let bytes = serde_json::to_vec(value).map_err(|e| failure("invalid_plugin_record", e))?;
    db.execute(
        "INSERT INTO kv VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, bytes],
    )
    .map_err(|e| failure("plugin_store_unavailable", e))?;
    Ok(())
}

fn next_generation(current: u64) -> Result<u64, Refusal> {
    current.checked_add(1).ok_or_else(|| {
        failure(
            "plugin_generation_exhausted",
            "new account generation required",
        )
    })
}

fn operator(actor: &Actor) -> Result<(), Refusal> {
    if matches!(actor, Actor::Operator { device_id, .. } if !device_id.is_nil()) {
        Ok(())
    } else {
        Err(failure(
            "operator_required",
            "plugin trust and activation require the admitted operator device",
        ))
    }
}

fn invalidate(activation: &mut Activation) -> Result<(), Refusal> {
    if !activation.needs_retrust || activation.desired_enabled {
        activation.generation = next_generation(activation.generation)?;
        activation.desired_enabled = false;
        activation.needs_retrust = true;
        activation.inspection_started = false;
        activation.inspection = None;
    }
    Ok(())
}

fn same_binding(candidate: &Candidate, activation: &Activation) -> bool {
    candidate.id == activation.candidate
        && candidate.source == activation.source
        && candidate.scope == activation.scope
        && candidate.name.as_ref() == Some(&activation.name)
        && candidate.sha256.as_ref() == Some(&activation.sha256)
}

fn same_trust(record: &TrustRecord, activation: &Activation) -> bool {
    record.sha256 == activation.sha256
        && record.source == activation.source
        && record.name == activation.name
        && record.scope == activation.scope.name()
}

fn private_dir(path: &Path) -> Result<(), Refusal> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(failure("plugin_directory_refused", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            match fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let metadata = fs::symlink_metadata(path)
                        .map_err(|e| failure("plugin_store_unavailable", e))?;
                    if metadata.is_dir() && !metadata.file_type().is_symlink() {
                        Ok(())
                    } else {
                        Err(failure("plugin_directory_refused", path.display()))
                    }
                }
                Err(e) => Err(failure("plugin_store_unavailable", e)),
            }
        }
        Err(e) => Err(failure("plugin_store_unavailable", e)),
    }
}

impl Registry {
    /// The caller is the logic/account owner, not another account opener.
    pub fn new(account: SqliteStore, roots: DiscoveryRoots) -> Result<Self, Refusal> {
        Self::with_config(account, roots, RegistryConfig::default())
    }

    pub fn with_config(
        account: SqliteStore,
        roots: DiscoveryRoots,
        config: RegistryConfig,
    ) -> Result<Self, Refusal> {
        bbt_rt::assert_not_ui_thread();
        if config.descriptor_timeout.is_zero() {
            return Err(failure(
                "invalid_plugin_config",
                "descriptor timeout must be positive",
            ));
        }
        let root = account
            .path
            .parent()
            .expect("object store has a parent")
            .canonicalize()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        Ok(Self {
            account,
            root,
            roots,
            config,
        })
    }

    /// Rescan observations and durably revoke changed/missing/colliding bindings.
    /// The caller publishes only when this projection differs from its last one.
    pub fn refresh(&self) -> Result<Vec<Entry>, Refusal> {
        self.project(None)
    }

    fn project(&self, only_digest: Option<&str>) -> Result<Vec<Entry>, Refusal> {
        bbt_rt::assert_not_ui_thread();
        let mut candidates = self.roots.scan(only_digest)?;
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let activations: Vec<Activation> = {
            let mut statement = tx
                .prepare("SELECT value FROM kv WHERE key >= ?1 AND key < ?2 ORDER BY key")
                .map_err(|e| failure("plugin_store_unavailable", e))?;
            let rows = statement
                .query_map([ACTIVATIONS, "plugins/activation0"], |row| {
                    row.get::<_, Vec<u8>>(0)
                })
                .map_err(|e| failure("plugin_store_unavailable", e))?;
            rows.map(|row| {
                let bytes = row.map_err(|e| failure("plugin_store_unavailable", e))?;
                serde_json::from_slice(&bytes).map_err(|e| failure("invalid_plugin_record", e))
            })
            .collect::<Result<_, _>>()?
        };
        for activation in &activations {
            if !candidates.iter().any(|c| c.id == activation.candidate) {
                candidates.push(Candidate {
                    id: activation.candidate.clone(),
                    source: activation.source.clone(),
                    scope: activation.scope,
                    name: Some(activation.name.clone()),
                    sha256: None,
                    refusal: Some(failure(
                        "plugin_missing",
                        "previously trusted source is missing",
                    )),
                });
            }
        }
        let mut entries = Vec::new();
        for candidate in candidates {
            let mut activation = activations
                .iter()
                .find(|a| a.candidate == candidate.id)
                .cloned();
            if let Some(activation) = &mut activation
                && only_digest.is_none_or(|selected| selected == candidate.id)
                && (candidate.refusal.is_some() || !same_binding(&candidate, activation))
            {
                let before = activation.clone();
                invalidate(activation)?;
                if *activation != before {
                    put(&tx, &activation_key(&candidate.id), activation)?;
                }
            }
            let trust: Option<TrustRecord> = if let Some(activation) = &activation {
                load::<TrustRecord>(&tx, &trust_key(&activation.sha256))?
                    .filter(|r| same_trust(r, activation))
            } else {
                None
            };
            entries.push(Entry {
                candidate,
                activation,
                trust,
            });
        }
        // Descriptors may add contracts beyond the filename's primary one.
        // Detect those host-qualified collisions too, without executing anything.
        let mut tools: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            if let Some(name) = &entry.candidate.name {
                let mut contracts = BTreeSet::from([name.system]);
                if let Some(shape) = entry.trust.as_ref().and_then(|t| t.shape.as_ref()) {
                    for system in &shape.systems {
                        contracts.insert(Contract::named(&system.contract)?);
                    }
                }
                for contract in contracts {
                    for method in contract.methods() {
                        if let Ok(tool) = name.tool(contract, method) {
                            tools.entry(tool).or_default().insert(i);
                        }
                    }
                }
            }
        }
        for indices in tools.values().filter(|v| v.len() > 1) {
            for i in indices {
                let entry = &mut entries[*i];
                entry.candidate.refusal = Some(failure(
                    "plugin_name_collision",
                    "host-qualified tools collide; all candidates are refused",
                ));
                if let Some(activation) = &mut entry.activation {
                    let before = activation.clone();
                    invalidate(activation)?;
                    if *activation != before {
                        put(&tx, &activation_key(&activation.candidate), activation)?;
                    }
                }
            }
        }
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        entries.sort_by(|a, b| a.candidate.id.cmp(&b.candidate.id));
        Ok(entries)
    }

    fn entry(&self, id: &str) -> Result<Entry, Refusal> {
        self.project(Some(id))?
            .into_iter()
            .find(|e| e.candidate.id == id)
            .ok_or_else(|| failure("plugin_not_found", id))
    }

    fn authority<'a>(
        &self,
        entry: &'a Entry,
    ) -> Result<(&'a Activation, &'a TrustRecord), Refusal> {
        if let Some(refusal) = &entry.candidate.refusal {
            return Err(refusal.clone());
        }
        let activation = entry
            .activation
            .as_ref()
            .ok_or_else(|| failure("untrusted_plugin", "no account activation binding"))?;
        if activation.needs_retrust || !same_binding(&entry.candidate, activation) {
            return Err(failure(
                "plugin_reapproval_required",
                "source binding changed; explicit operator trust is required",
            ));
        }
        let record = entry.trust.as_ref().ok_or_else(|| {
            failure(
                "untrusted_plugin",
                "no matching operator account trust record",
            )
        })?;
        if !same_trust(record, activation) {
            return Err(failure(
                "invalid_trust_record",
                "account trust binding differs",
            ));
        }
        Ok((activation, record))
    }

    fn directories(&self) -> Result<(PathBuf, PathBuf), Refusal> {
        let plugins = self.root.join("plugins");
        private_dir(&plugins)?;
        let bins = plugins.join("bin");
        let data = plugins.join("data");
        private_dir(&bins)?;
        private_dir(&data)?;
        Ok((bins, data))
    }

    fn pin(&self, source: &Path, expected: &str) -> Result<(PathBuf, PathBuf), Refusal> {
        let (bins, data) = self.directories()?;
        let mut input = open_regular(source)?;
        let mut output = tempfile::NamedTempFile::new_in(&bins)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let mut digest = Sha256::new();
        let mut bytes = [0; 64 * 1024];
        loop {
            let count = input
                .read(&mut bytes)
                .map_err(|e| failure("plugin_file_unavailable", e))?;
            if count == 0 {
                break;
            }
            output
                .write_all(&bytes[..count])
                .map_err(|e| failure("plugin_store_unavailable", e))?;
            digest.update(&bytes[..count]);
        }
        if format!("{:x}", digest.finalize()) != expected {
            return Err(failure(
                "plugin_digest_changed",
                "source changed before approval; review the new digest",
            ));
        }
        output
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o500))
            .and_then(|()| output.as_file().sync_all())
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let copy = bins.join(expected);
        match output.persist_noclobber(&copy) {
            Ok(_) => {
                File::open(&bins)
                    .and_then(|f| f.sync_all())
                    .map_err(|e| failure("plugin_store_unavailable", e))?;
            }
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(failure("plugin_store_unavailable", e.error)),
        }
        let data_dir = data.join(expected);
        private_dir(&data_dir)?;
        Ok((copy, data_dir))
    }

    fn verify_copy(&self, record: &TrustRecord) -> Result<PathBuf, Refusal> {
        if record.sha256.len() != 64
            || !record
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(failure("invalid_digest", "expected lowercase SHA-256"));
        }
        let (bins, data) = self.directories()?;
        let copy = bins.join(&record.sha256);
        let data_dir = data.join(&record.sha256);
        if Path::new(&record.copy) != copy || Path::new(&record.data_dir) != data_dir {
            return Err(failure(
                "invalid_trust_record",
                "pinned paths differ from the logic account",
            ));
        }
        private_dir(&data_dir)?;
        let file = open_regular(&copy)?;
        let mode = file
            .metadata()
            .map_err(|e| failure("plugin_file_unavailable", e))?
            .permissions()
            .mode();
        if mode & 0o222 != 0 {
            return Err(failure(
                "plugin_copy_writable",
                "pinned copy must be read-only",
            ));
        }
        if mode & 0o100 == 0 {
            return Err(failure(
                "plugin_not_executable",
                "pinned copy is not executable",
            ));
        }
        if hash_file(&copy)? != record.sha256 {
            return Err(failure(
                "plugin_digest_changed",
                "pinned bytes changed; explicit operator trust is required",
            ));
        }
        Ok(copy)
    }

    /// Effect API for the admitted operator path. Its journal is owned by logic.
    /// This commits the record before a caller can obtain an inspection job.
    pub fn trust(
        &self,
        actor: &Actor,
        id: &str,
        expected: &str,
        approval: Option<Approval>,
    ) -> Result<Entry, Refusal> {
        bbt_rt::assert_not_ui_thread();
        operator(actor)?;
        let entry = self.entry(id)?;
        if let Some(reason) = entry.candidate.refusal {
            return Err(reason);
        }
        if entry.candidate.sha256.as_deref() != Some(expected) {
            return Err(failure(
                "plugin_digest_changed",
                "review the current candidate digest",
            ));
        }
        let name = entry
            .candidate
            .name
            .as_ref()
            .ok_or_else(|| failure("invalid_plugin_name", "missing parsed name"))?;
        for method in name.system.methods() {
            name.tool(name.system, method)?;
        }
        let approval = approval.unwrap_or_else(|| Approval::minimum(name));
        if approval.tier < name.system.tier() {
            return Err(failure(
                "tier_not_approved",
                "approval is below the fixed contract minimum",
            ));
        }
        let (copy, data_dir) = self.pin(Path::new(&entry.candidate.source), expected)?;
        let mut record = TrustRecord {
            sha256: expected.into(),
            source: entry.candidate.source.clone(),
            copy: copy.to_string_lossy().into_owned(),
            data_dir: data_dir.to_string_lossy().into_owned(),
            name: name.clone(),
            scope: entry.candidate.scope.name().into(),
            approved_tier: approval.tier,
            grants: approval.grants,
            shape: None,
        };
        self.verify_copy(&record)?;
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let old: Option<Activation> = load(&tx, &activation_key(id))?;
        if let Some(existing) = load::<TrustRecord>(&tx, &trust_key(expected))?
            && (existing.source != record.source
                || existing.name != record.name
                || existing.scope != record.scope)
        {
            return Err(failure(
                "plugin_trust_binding_conflict",
                "this digest is bound to a different canonical source/name",
            ));
        }
        let mut inspection = old
            .as_ref()
            .filter(|a| same_binding(&entry.candidate, a))
            .and_then(|a| a.inspection.clone());
        if let Some(inspection) = &mut inspection
            && let Some(shape) = &inspection.shape
        {
            inspection.refusal = record.validate_shape(shape).err();
            if inspection.refusal.is_none() {
                record.shape = Some(shape.clone());
            }
        }
        // Failed IO can be explicitly retried by a new trust operation; a valid
        // observed shape is reused for explicit grant changes without execution.
        if inspection.as_ref().is_some_and(|i| i.shape.is_none()) {
            inspection = None;
        }
        let activation = Activation {
            candidate: id.into(),
            source: entry.candidate.source.clone(),
            scope: entry.candidate.scope,
            name: name.clone(),
            sha256: expected.into(),
            generation: next_generation(old.as_ref().map_or(0, |a| a.generation))?,
            desired_enabled: false,
            needs_retrust: false,
            inspection_started: false,
            inspection,
        };
        put(&tx, &trust_key(expected), &record)?;
        put(&tx, &activation_key(id), &activation)?;
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        self.entry(id)
    }

    /// Returns no job when this generation already has an observed descriptor.
    pub fn inspection(&self, id: &str) -> Result<Option<InspectionJob>, Refusal> {
        let entry = self.entry(id)?;
        let (activation, record) = self.authority(&entry)?;
        self.verify_copy(record)?;
        if activation.inspection.is_some() || activation.inspection_started {
            return Ok(None);
        }
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let mut current: Activation = load(&tx, &activation_key(id))?
            .ok_or_else(|| failure("untrusted_plugin", "activation disappeared"))?;
        if &current != activation {
            return Err(failure(
                "stale_plugin_operation",
                "activation changed before inspection admission",
            ));
        }
        current.inspection_started = true;
        put(&tx, &activation_key(id), &current)?;
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        Ok(Some(InspectionJob {
            registry: self.clone(),
            activation: current,
            record: record.clone(),
        }))
    }

    pub fn finish_inspection(&self, result: InspectionResult) -> Result<Entry, Refusal> {
        bbt_rt::assert_not_ui_thread();
        // Observe a source change before accepting an otherwise matching job.
        let observed = self.entry(&result.activation.candidate)?;
        let (activation, _) = self.authority(&observed)?;
        if activation != &result.activation {
            return Err(failure(
                "stale_plugin_completion",
                "source or activation changed while inspection ran",
            ));
        }
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let mut current: Activation = load(&tx, &activation_key(&result.activation.candidate))?
            .ok_or_else(|| failure("untrusted_plugin", "activation disappeared"))?;
        if current != result.activation || current.needs_retrust {
            return Err(failure(
                "stale_plugin_completion",
                "account activation changed while inspection ran",
            ));
        }
        let mut record: TrustRecord = load(&tx, &trust_key(&current.sha256))?
            .ok_or_else(|| failure("untrusted_plugin", "trust disappeared"))?;
        if record != result.record || !same_trust(&record, &current) {
            return Err(failure(
                "stale_plugin_completion",
                "account trust changed while inspection ran",
            ));
        }
        let mut inspection = result.inspection;
        if let Some(shape) = &inspection.shape {
            inspection.refusal = record.validate_shape(shape).err();
            if inspection.refusal.is_none() {
                record.shape = Some(shape.clone());
            }
        }
        current.inspection = Some(inspection);
        current.inspection_started = false;
        put(&tx, &trust_key(&record.sha256), &record)?;
        put(&tx, &activation_key(&current.candidate), &current)?;
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        self.entry(&current.candidate)
    }

    pub fn set_enabled(&self, actor: &Actor, id: &str, enabled: bool) -> Result<Entry, Refusal> {
        bbt_rt::assert_not_ui_thread();
        operator(actor)?;
        let entry = self.entry(id)?;
        let activation = if enabled {
            let (activation, record) = self.authority(&entry)?;
            if record.shape.is_none() {
                return Err(activation
                    .inspection
                    .as_ref()
                    .and_then(|i| i.refusal.clone())
                    .unwrap_or_else(|| {
                        failure(
                            "shape_not_cached",
                            "inspect a trusted descriptor before enabling",
                        )
                    }));
            }
            self.verify_copy(record)?;
            activation
        } else {
            entry
                .activation
                .as_ref()
                .ok_or_else(|| failure("untrusted_plugin", "no activation to disable"))?
        };
        let mut db = self
            .account
            .connection()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let tx = db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        let mut current: Activation = load(&tx, &activation_key(id))?
            .ok_or_else(|| failure("untrusted_plugin", "activation disappeared"))?;
        if &current != activation {
            return Err(failure(
                "stale_plugin_operation",
                "activation changed before commit",
            ));
        }
        current.generation = next_generation(current.generation)?;
        current.desired_enabled = enabled;
        put(&tx, &activation_key(id), &current)?;
        tx.commit()
            .map_err(|e| failure("plugin_store_unavailable", e))?;
        self.entry(id)
    }

    /// Each future business launch must use this fresh check, not a view row.
    pub fn executable(&self, id: &str) -> Result<PathBuf, Refusal> {
        let entry = self.entry(id)?;
        let (_, record) = self.authority(&entry)?;
        if entry.state() != State::Enabled {
            return Err(failure("plugin_disabled", "explicit enable is required"));
        }
        self.verify_copy(record)
    }
}

/// A per-plugin metadata job, not a long-lived global plugin worker.
pub struct InspectionJob {
    registry: Registry,
    activation: Activation,
    record: TrustRecord,
}

pub struct InspectionResult {
    activation: Activation,
    record: TrustRecord,
    inspection: Inspection,
    pid: Option<u32>,
}

impl InspectionResult {
    /// Direct child owned and reaped by this job, when it reached spawn.
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }
    pub fn observation(&self) -> &Inspection {
        &self.inspection
    }
}

impl InspectionJob {
    /// The host runs this off its command loop. It cannot call login or any
    /// mutating plugin method, and normal failure paths explicitly reap children.
    pub async fn run(self) -> InspectionResult {
        let started = Instant::now();
        let mut pid = None;
        let mut stderr = String::new();
        let mut truncated = false;
        let run = async {
            let entry = self.registry.entry(&self.activation.candidate)?;
            let (activation, record) = self.registry.authority(&entry)?;
            if activation != &self.activation || record != &self.record {
                return Err(failure(
                    "stale_plugin_operation",
                    "account binding changed before spawn",
                ));
            }
            let copy = self.registry.verify_copy(record)?;
            let mut child = Client::spawn_verified(
                copy,
                Path::new(&record.data_dir),
                true,
                self.registry.config.descriptor_timeout,
            )?;
            pid = Some(child.pid);
            let value = child.shape().await;
            let output = child.shutdown().await;
            stderr = output.text;
            truncated = output.truncated;
            parse_descriptor(value?, &record.name)
        }
        .await;
        let (shape, refusal) = match run {
            Ok(shape) => (Some(shape), None),
            Err(refusal) => (None, Some(refusal)),
        };
        InspectionResult {
            activation: self.activation,
            record: self.record,
            inspection: Inspection {
                shape,
                refusal,
                elapsed_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
                stderr,
                stderr_truncated: truncated,
            },
            pid,
        }
    }
}
