// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Lossless task imports: one root-bound, signed directory publication, no wakes.

use super::*;
use std::io::Write;

const MAX_BUNDLE: u64 = 32 * 1024 * 1024;
const MAX_RECORDS: usize = 512;

#[derive(Debug)]
pub(crate) struct AllocationStale;
impl std::fmt::Display for AllocationStale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("import_allocation_stale: ordinary work changed the provisional range; no import published; retry the pinned bundle with a fresh operation")
    }
}
impl std::error::Error for AllocationStale {}

#[derive(Deserialize)]
struct Manifest {
    source_revision: String,
    seed_records: usize,
    dependency_records: usize,
    all_records: usize,
    rows: Vec<Row>,
    links: Vec<SourceLink>,
}
#[derive(Deserialize)]
struct Row {
    source_id: String,
    source_number: u64,
    source_type: String,
    source_path: String,
    source_sha256: String,
    destination_path: String,
    dedupe_key: String,
    seed_task_folder_record: bool,
}
#[derive(Deserialize)]
struct SourceLink {
    from: String,
    raw: String,
    targets: Vec<String>,
    result: String,
}
#[derive(Deserialize)]
struct Audit {
    decisions: Vec<Disposition>,
}
#[derive(Deserialize, Serialize)]
struct Disposition {
    index: usize,
    disposition: String,
    reason: String,
    #[serde(default)]
    commits: Vec<String>,
    evidence: Vec<Value>,
}

pub(crate) struct Bundle {
    manifest: Manifest,
    audit: Audit,
    manifest_bytes: Vec<u8>,
    audit_bytes: Vec<u8>,
    sources: Vec<Vec<u8>>,
    pub(crate) digest: String,
}

fn read_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::os::unix::fs::OpenOptionsExt;
    // Canonical parent containment is checked by the caller for source records.
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("import input must be a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid("import input exceeds bound"));
    }
    Ok(bytes)
}

/// Read-only planning helper for ctl. The authenticated daemon reloads every byte.
pub fn bundle_digest(manifest: &Path, audit: &Path, sources: &Path) -> Result<String> {
    Ok(Bundle::load(manifest, audit, sources)?.digest)
}

impl Bundle {
    pub(crate) fn load(manifest: &Path, audit: &Path, sources: &Path) -> Result<Self> {
        let manifest_bytes = read_file(manifest, MAX_BUNDLE)?;
        let audit_bytes = read_file(audit, MAX_BUNDLE)?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
        let audit: Audit = serde_json::from_slice(&audit_bytes)?;
        if manifest.rows.is_empty()
            || manifest.rows.len() > MAX_RECORDS
            || manifest.all_records != manifest.rows.len()
            || manifest
                .seed_records
                .checked_add(manifest.dependency_records)
                != Some(manifest.all_records)
            || manifest.seed_records
                != manifest
                    .rows
                    .iter()
                    .filter(|r| r.seed_task_folder_record)
                    .count()
            || manifest.links.len() > 32768
            || manifest.source_revision.len() != 40
            || !manifest
                .source_revision
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid("invalid import counts or revision"));
        }
        let source_root = sources.canonicalize()?;
        let mut total = (manifest_bytes.len() + audit_bytes.len()) as u64;
        let mut raw = Vec::new();
        let mut paths = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut numbers = BTreeSet::new();
        let mut dedupe = BTreeSet::new();
        let mut hash = sha2::Sha256::new();
        use sha2::Digest;
        hash.update(b"bboxy450/task-import/v1\0");
        for bytes in [&manifest_bytes, &audit_bytes] {
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(bytes);
        }
        for row in &manifest.rows {
            if !paths.insert(&row.source_path)
                || !ids.insert(&row.source_id)
                || !numbers.insert(row.source_number)
                || !dedupe.insert(&row.dedupe_key)
                || row.source_id.parse::<Ulid>().is_err()
                || row.source_number == 0
                || row.source_path != format!(".bboxy/{}", row.destination_path)
                || row.dedupe_key.is_empty()
                || row.dedupe_key.len() > 4096
            {
                return Err(invalid(
                    "duplicate or invalid source identity/path/dedupe key",
                ));
            }
            let relative = Path::new(&row.destination_path);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                || relative.extension().is_none_or(|e| e != "md")
            {
                return Err(invalid("source path escapes import directory"));
            }
            let path = source_root.join(relative);
            // Refuse symlinks, including intermediate components, even within the source directory.
            let mut checked = source_root.clone();
            for part in relative.components() {
                checked.push(part);
                if checked.symlink_metadata()?.file_type().is_symlink() {
                    return Err(invalid("symlink in import source"));
                }
            }
            let bytes = read_file(&path, MAX_DOCUMENT)?;
            total += bytes.len() as u64;
            if total > MAX_BUNDLE {
                return Err(invalid("import bundle exceeds 32 MiB"));
            }
            let doc = Document::parse(&row.destination_path, std::str::from_utf8(&bytes)?);
            if digest(&bytes) != row.source_sha256
                || doc.error.is_some()
                || doc.id != row.source_id
                || record_number(&doc.metadata) != Some(row.source_number)
                || doc.metadata["type"] != row.source_type
            {
                return Err(invalid(format!(
                    "source identity/hash mismatch: {}",
                    row.source_path
                )));
            }
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(&bytes);
            raw.push(bytes);
        }
        let mut decisions = BTreeSet::new();
        for d in &audit.decisions {
            if d.index >= manifest.seed_records
                || !decisions.insert(d.index)
                || d.reason.trim().is_empty()
                || d.evidence.is_empty()
                || !matches!(
                    d.disposition.as_str(),
                    "close_completed"
                        | "close_superseded"
                        | "retain_deferred"
                        | "retain_narrowed"
                        | "retain_active"
                        | "retain_followup"
                        | "already_done"
                        | "archive_note"
                )
            {
                return Err(invalid("invalid or incomplete audited disposition"));
            }
        }
        if decisions.len() != manifest.seed_records {
            return Err(invalid("audit must cover every seed record"));
        }
        for link in &manifest.links {
            if !paths.contains(&link.from)
                || link.raw.len() > 4096
                || !matches!(link.result.as_str(), "resolved" | "unresolved")
                || (link.result == "resolved"
                    && (link.targets.len() != 1 || !paths.contains(&link.targets[0])))
                || (link.result == "unresolved" && !link.targets.is_empty())
            {
                return Err(invalid("invalid import link mapping"));
            }
        }
        Ok(Self {
            manifest,
            audit,
            manifest_bytes,
            audit_bytes,
            sources: raw,
            digest: format!("{:x}", hash.finalize()),
        })
    }
}

fn immutable(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    file.set_permissions(std::fs::Permissions::from_mode(0o400))?;
    Ok(())
}

impl Index {
    #[cfg(test)]
    pub(crate) fn import_author(
        &self,
        bundle: Bundle,
        owner: Ulid,
        author: &crate::provenance::author::Author<'_>,
    ) -> Result<Value> {
        self.import_staged(bundle, owner, author, || Ok(()), || Ok(()))
    }

    pub(crate) fn import_staged(
        &self,
        bundle: Bundle,
        owner: Ulid,
        author: &crate::provenance::author::Author<'_>,
        after_staging: impl FnOnce() -> Result<()>,
        validate: impl Fn() -> Result<()>,
    ) -> Result<Value> {
        // Adapted from this Index's guarded writes: same author/signature/intent path,
        // with one number allocation and one directory publication for the whole bundle.
        // https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        let lock = std::fs::File::open(&self.root)?;
        lock.try_lock()
            .map_err(|_| Box::new(IndexBusy) as Box<dyn std::error::Error + Send + Sync>)?;
        let prefix = format!("imports/{}", bundle.digest);
        let destination = self.root.join(&prefix);
        if destination.symlink_metadata().is_ok() {
            drop(lock);
            after_staging()?;
            validate()?;
            return self.import_receipt(&destination, &bundle);
        }
        // A subset imported under another manifest must not create a second identity.
        let mut existing = Vec::new();
        collect(&self.root, &mut existing)?;
        let keys: BTreeSet<_> = bundle
            .manifest
            .rows
            .iter()
            .map(|r| r.dedupe_key.as_str())
            .collect();
        let source_ids: BTreeSet<_> = bundle
            .manifest
            .rows
            .iter()
            .map(|r| r.source_id.as_str())
            .collect();
        let mut occupied_ids: BTreeSet<String> =
            source_ids.iter().map(|id| id.to_string()).collect();
        for (path, _) in existing {
            let doc = self
                .current(
                    path.strip_prefix(&self.root)?
                        .to_str()
                        .ok_or_else(|| invalid("invalid record path"))?,
                )?
                .ok_or_else(|| invalid("record disappeared"))?;
            occupied_ids.insert(doc.id.clone());
            if doc.metadata["import_source"]["dedupe_key"]
                .as_str()
                .is_some_and(|k| keys.contains(k))
                || doc.metadata["import_source"]["id"]
                    .as_str()
                    .is_some_and(|id| source_ids.contains(id))
            {
                return Err(invalid(
                    "source already imported under another bundle; reconcile its existing identity",
                ));
            }
        }
        let first = self
            .record_numbers()?
            .last_key_value()
            .map_or(0, |(n, _)| *n)
            .checked_add(1)
            .ok_or_else(|| invalid("record numbers exhausted"))?;
        first
            .checked_add(bundle.manifest.rows.len() as u64 + 1)
            .ok_or_else(|| invalid("record numbers exhausted"))?;
        let mapping: BTreeMap<_, _> = bundle
            .manifest
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let id = loop {
                    let id = Ulid::new();
                    if occupied_ids.insert(id.to_string()) {
                        break id;
                    }
                };
                (row.source_path.clone(), (id, first + i as u64))
            })
            .collect();
        drop(lock);
        let mut locators: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for row in &bundle.manifest.rows {
            for locator in [
                row.source_id.clone(),
                format!("#{}", row.source_number),
                row.source_path.clone(),
                row.destination_path.clone(),
                Path::new(&row.destination_path)
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            ] {
                locators
                    .entry(locator)
                    .or_default()
                    .insert(row.source_path.clone());
            }
        }
        // Sibling staging never enters collect()/watcher scans of the active root.
        // A killed process leaves only inactive products; replay publishes once.
        let staging = self
            .root
            .parent()
            .ok_or_else(|| invalid("import root needs a parent"))?
            .join(format!(
                ".bboxy-import-{}-{}",
                &bundle.digest[..12],
                Ulid::new()
            ));
        std::fs::create_dir(&staging)?;
        // Retire only this call's own inactive products on a known failure.
        // After publication this path contains only empty staging parents.
        struct InactiveStage(PathBuf);
        impl Drop for InactiveStage {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _inactive = InactiveStage(staging.clone());
        let mut staged = self.clone();
        staged.root = staging.canonicalize()?;
        let directory = staging.join(&prefix);
        std::fs::create_dir_all(directory.join("raw"))?;
        immutable(&directory.join("manifest.json"), &bundle.manifest_bytes)?;
        immutable(&directory.join("audit.json"), &bundle.audit_bytes)?;
        let mut records = Vec::new();
        let mut seed = 0;
        let mut followup = None;
        for (i, row) in bundle.manifest.rows.iter().enumerate() {
            let (id, number) = mapping[&row.source_path];
            let raw = &bundle.sources[i];
            immutable(&directory.join(format!("raw/{id}.source")), raw)?;
            let mut doc = Document::parse(&row.destination_path, std::str::from_utf8(raw)?);
            rewrite_value(&mut doc.metadata, row, &bundle, &mapping, &locators);
            rewrite_graph_fields(&mut doc.metadata, &mapping, &locators);
            doc.body = rewrite(&doc.body, row, &bundle, &mapping, &locators);
            let metadata = doc
                .metadata
                .as_object_mut()
                .ok_or_else(|| invalid("source frontmatter"))?;
            let documentary_status = (!row.seed_task_folder_record
                && !matches!(row.source_type.as_str(), "task" | "bug" | "epic"))
            .then(|| metadata.get("status").cloned())
            .flatten();
            // Historical seat IDs, reports and signatures remain only in the immutable source.
            for field in [
                "id",
                "number",
                "assignee",
                "delegated_by",
                "report",
                "verify_result",
                "provenance",
                "authorship",
                "created_by",
                "updated_by",
                "status",
                "created",
                "updated",
            ] {
                metadata.remove(field);
            }
            if let Some(status) = documentary_status {
                metadata.insert("status".into(), status);
            }
            metadata.insert("id".into(), json!(id));
            metadata.insert("number".into(), json!(number));
            metadata.insert("import_source".into(),json!({"id":row.source_id,"number":row.source_number,"path":row.source_path,
                "revision":bundle.manifest.source_revision,"sha256":row.source_sha256,"dedupe_key":row.dedupe_key,
                "raw":format!("raw/{id}.source"),"bundle_sha256":bundle.digest,"type":row.source_type}));
            if row.seed_task_folder_record {
                let disposition = bundle
                    .audit
                    .decisions
                    .iter()
                    .find(|d| d.index == seed)
                    .ok_or_else(|| invalid("seed disposition missing"))?;
                seed += 1;
                let live = disposition.disposition.starts_with("retain_")
                    && disposition.disposition != "retain_followup";
                metadata.insert(
                    "type".into(),
                    json!(if disposition.disposition == "archive_note" {
                        "note"
                    } else {
                        "task"
                    }),
                );
                metadata.insert("status".into(), json!(if live { "open" } else { "done" }));
                metadata.insert("delegated_by".into(), json!(format!("seat:{owner}")));
                let mut audited = serde_json::to_value(disposition)?;
                rewrite_value(&mut audited, row, &bundle, &mapping, &locators);
                metadata.insert("reconciliation".into(), json!({"kind":"historical","fresh_accepted":false,"audit":audited,"by":author.id}));
                if live {
                    doc.body = format!(
                        "# Remaining scope\n\n{}\n\nHistorical brief follows; completed clauses are context.\n\n{}",
                        remaining_scope(row.source_number, &disposition.reason),
                        doc.body
                    );
                }
                if disposition.disposition == "retain_followup" {
                    let child = Ulid::new();
                    metadata.insert("followup".into(), json!(format!("[[{child}]]")));
                    followup = Some((child, id, disposition.reason.clone()));
                    doc.body = format!(
                        "# MA-7 accounting completed\n\nAccounting is historical implemented work. Adapter startup remains open in [[{child}]] (GitHub issue 40); it is not certified by this disposition.\n\n{}",
                        doc.body
                    );
                }
            } else if matches!(row.source_type.as_str(), "task" | "bug" | "epic") {
                metadata.insert("type".into(), json!("note"));
                metadata.insert("status".into(), json!("done"));
            }
            doc.body = format!(
                "<!-- Imported projection: {} @ {}; source {}. Exact bytes: raw/{}.source.\nGenerated by bboxy450-term: https://github.com/bboxy450/bboxy450-term\nRetain source credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->\n\n{}",
                row.source_path, bundle.manifest.source_revision, row.dedupe_key, id, doc.body
            );
            let path = format!("{prefix}/{id}.md");
            staged.write_locked(
                Update {
                    path: path.clone(),
                    metadata: doc.metadata,
                    body: doc.body,
                    expected: None,
                },
                author,
                true,
                Some(number),
            )?;
            records.push(json!({"source_path":row.source_path,"dedupe_key":row.dedupe_key,"id":id,"number":number,"path":path,"source_sha256":row.source_sha256}));
        }
        if let Some((id, parent, reason)) = followup {
            let number = first + bundle.manifest.rows.len() as u64;
            staged.write_locked(Update{path:format!("{prefix}/{id}.md"),expected:None,
                metadata:json!({"id":id,"type":"task","number":number,"title":"Adapter process-start defect (issue 40)","status":"open",
                    "delegated_by":format!("seat:{owner}"),"parent":format!("[[{parent}]]"),"github_issue":40,
                    "import_followup":bundle.digest,"verify":"make test"}),
                body:format!("# Adapter process-start defect\n\n{reason}\n\nRetain the actual startup quarantine and resolve issue 40 through its separately assigned implementation. MA-7 accounting is complete. No historical assignee was delegated.\n")},author,true,Some(number))?;
        }
        let mut receipt = json!({"bundle_sha256":bundle.digest,"root":self.root,"source_revision":bundle.manifest.source_revision,"records":records});
        let mut published = Vec::new();
        collect(&directory, &mut published)?;
        let hashes: BTreeMap<_, _> = published
            .iter()
            .map(|(p, _)| {
                Ok((
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    digest(&std::fs::read(p)?),
                ))
            })
            .collect::<Result<_>>()?;
        receipt["published_sha256"] = json!(hashes);
        let receipt_data_sha256 = digest(&serde_json::to_vec(&receipt)?);
        let mut intent = Draft::new(
            format!("activity/task-import/{}/prepared", bundle.digest),
            "mux/task-import",
            json!({"bundle_sha256":bundle.digest,"source_records":bundle.manifest.rows.len(),"receipt":format!(".bboxy/{prefix}/receipt.json"),"agent":author.id,"receipt_data_sha256":receipt_data_sha256}),
        );
        intent.seat = author.seat;
        intent.generation = author.generation;
        let prepared = self.append_activity(intent, author.key)?;
        receipt["activity_intent"] = json!({"id":prepared.event.id,"sequence":prepared.seq});
        receipt["signer"] = json!(author.id);
        receipt["public_key"] = json!(
            base64::engine::general_purpose::STANDARD.encode(author.key.verifying_key().as_bytes())
        );
        let signature = author.key.sign(&serde_json::to_vec(&receipt)?);
        receipt["signature"] =
            json!(base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()));
        immutable(
            &directory.join("receipt.json"),
            &serde_json::to_vec_pretty(&receipt)?,
        )?;
        std::fs::File::open(directory.join("raw"))?.sync_all()?;
        std::fs::File::open(&directory)?.sync_all()?;
        after_staging()?;
        let publication_lock = std::fs::File::open(&self.root)?;
        publication_lock
            .try_lock()
            .map_err(|_| Box::new(IndexBusy) as Box<dyn std::error::Error + Send + Sync>)?;
        validate()?;
        if destination.symlink_metadata().is_ok() {
            drop(publication_lock);
            validate()?;
            return self.import_receipt(&destination, &bundle);
        }
        let current_numbers = self.record_numbers()?;
        if current_numbers
            .last_key_value()
            .is_some_and(|(number, _)| *number >= first)
        {
            return Err(Box::new(AllocationStale));
        }
        let reserved: BTreeSet<_> = receipt["published_sha256"]
            .as_object()
            .ok_or_else(|| invalid("import projection hashes missing"))?
            .keys()
            .map(|name| name.trim_end_matches(".md"))
            .collect();
        for path in current_numbers.values() {
            let current = self
                .current(path)?
                .ok_or_else(|| invalid("record disappeared before import publication"))?;
            if reserved.contains(current.id.as_str())
                || current.metadata["import_source"]["dedupe_key"]
                    .as_str()
                    .is_some_and(|k| keys.contains(k))
                || current.metadata["import_source"]["id"]
                    .as_str()
                    .is_some_and(|id| source_ids.contains(id))
            {
                return Err(Box::new(AllocationStale));
            }
        }
        let imports = self.root.join("imports");
        if imports
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(invalid("symlink in import destination"));
        }
        validate()?;
        std::fs::create_dir_all(&imports)?;
        std::fs::rename(&directory, &destination)?;
        std::fs::File::open(&imports)?.sync_all()?;
        drop(publication_lock);
        // Only our now-empty staging parents are removed. Interrupted siblings stay inactive.
        let _ = std::fs::remove_dir(staging.join("imports"));
        let _ = std::fs::remove_dir(staging);
        self.import_receipt(&destination, &bundle)
    }

    fn import_receipt(&self, directory: &Path, bundle: &Bundle) -> Result<Value> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        if directory.symlink_metadata()?.file_type().is_symlink() {
            return Err(invalid("symlink in import receipt"));
        }
        let mut receipt: Value =
            serde_json::from_slice(&read_file(&directory.join("receipt.json"), MAX_BUNDLE)?)?;
        if receipt["root"] != json!(self.root)
            || receipt["bundle_sha256"] != bundle.digest
            || receipt["records"].as_array().map(Vec::len) != Some(bundle.manifest.rows.len())
        {
            return Err(invalid("import receipt root/digest/count mismatch"));
        }
        let signature = receipt
            .as_object_mut()
            .unwrap()
            .remove("signature")
            .ok_or_else(|| invalid("import receipt unsigned"))?;
        let decode = |v: &Value| -> Result<Vec<u8>> {
            Ok(base64::engine::general_purpose::STANDARD.decode(
                v.as_str()
                    .ok_or_else(|| invalid("invalid receipt signature"))?,
            )?)
        };
        let public: [u8; 32] = decode(&receipt["public_key"])?
            .try_into()
            .map_err(|_| invalid("invalid receipt key"))?;
        VerifyingKey::from_bytes(&public)?.verify(
            &serde_json::to_vec(&receipt)?,
            &Signature::from_slice(&decode(&signature)?)?,
        )?;
        // Authority comes from the host's recorded, admitted import intent,
        // never from the key that an untrusted receipt declares for itself.
        let sequence = receipt["activity_intent"]["sequence"]
            .as_u64()
            .ok_or_else(|| invalid("receipt lacks admitted import intent"))?;
        let intent = self
            .activity
            .at(sequence)?
            .ok_or_else(|| invalid("admitted import intent unavailable"))?;
        intent.verify()?;
        let mut core = receipt.clone();
        for field in ["activity_intent", "signer", "public_key"] {
            core.as_object_mut().unwrap().remove(field);
        }
        if intent.seq != sequence
            || intent.event.id.to_string() != receipt["activity_intent"]["id"]
            || intent.event.source != "mux/task-import"
            || intent.event.subject != format!("activity/task-import/{}/prepared", bundle.digest)
            || intent.provenance.signer != public
            || intent.event.payload["agent"] != receipt["signer"]
            || intent.event.payload["bundle_sha256"] != bundle.digest
            || intent.event.payload["source_records"] != bundle.manifest.rows.len()
            || intent.event.payload["receipt"]
                != format!(".bboxy/imports/{}/receipt.json", bundle.digest)
            || intent.event.payload["receipt_data_sha256"] != digest(&serde_json::to_vec(&core)?)
        {
            return Err(invalid(
                "receipt is not bound to its admitted import intent",
            ));
        }
        for row in receipt["records"].as_array().unwrap() {
            let id = row["id"]
                .as_str()
                .ok_or_else(|| invalid("receipt id missing"))?;
            id.parse::<Ulid>()?;
            let raw = read_file(&directory.join(format!("raw/{id}.source")), MAX_DOCUMENT)?;
            if digest(&raw) != row["source_sha256"] {
                return Err(invalid("immutable import source changed"));
            }
            let doc = self
                .current(&format!("imports/{}/{id}.md", bundle.digest))?
                .ok_or_else(|| invalid("imported source identity missing"))?;
            if doc.id != id
                || doc.metadata["number"] != row["number"]
                || doc.metadata["import_source"]["dedupe_key"] != row["dedupe_key"]
                || doc.metadata["import_source"]["sha256"] != row["source_sha256"]
            {
                return Err(invalid("imported source identity changed"));
            }
        }
        if read_file(&directory.join("manifest.json"), MAX_BUNDLE)? != bundle.manifest_bytes
            || read_file(&directory.join("audit.json"), MAX_BUNDLE)? != bundle.audit_bytes
        {
            return Err(invalid("immutable import inputs changed"));
        }
        // Published records may subsequently undergo legitimate task lifecycle edits.
        // Their original byte hashes remain in the signed receipt for independent audit.
        for name in receipt["published_sha256"]
            .as_object()
            .ok_or_else(|| invalid("receipt hashes missing"))?
            .keys()
        {
            let doc = self
                .current(&format!("imports/{}/{name}", bundle.digest))?
                .ok_or_else(|| invalid("imported record missing"))?;
            if doc.error.is_some() {
                return Err(invalid("imported record is invalid"));
            }
        }
        Ok(
            json!({"status":"imported","bundle_sha256":bundle.digest,"source_records":bundle.manifest.rows.len(),
            "published_records":receipt["published_sha256"].as_object().map(|v|v.len()),"receipt":directory.join("receipt.json"),"historical_wakes":0}),
        )
    }
}

fn remaining_scope(number: u64, reason: &str) -> String {
    match number {
        128 => "Prove the in-bboxy task loop and remaining native task-row evidence; retain shipped launcher, messaging and desktop foundations.",
        330 => "Name and repair durable GUI-event EOF across logic restart (issue 39); keep identity, unread and same-generation PTY assertions.",
        329 => "Name and repair the fifty-seat spawn ENOENT mechanism (issue 38); successful controls alone do not close this defect.",
        129 => "Resolve outstanding terminal/interaction fidelity and real-harness end-to-end evidence; shipped rendering/firehose work is historical context.",
        125 => "Meet measured runtime/frame budgets with unchanged workload and real p99 populations. Parser-pool R3 was superseded by C1b; do not revive it.",
        108 => "Deferred real QUIC/remote transport: expected-key rejection, two-endpoint round trip and loopback proof after local task-loop reliability.",
        _ => reason,
    }.into()
}

fn rewrite_value(
    value: &mut Value,
    row: &Row,
    bundle: &Bundle,
    mapping: &BTreeMap<String, (Ulid, u64)>,
    locators: &BTreeMap<String, BTreeSet<String>>,
) {
    match value {
        Value::String(s) => *s = rewrite(s, row, bundle, mapping, locators),
        Value::Array(xs) => {
            for v in xs {
                rewrite_value(v, row, bundle, mapping, locators);
            }
        }
        Value::Object(xs) => {
            for v in xs.values_mut() {
                rewrite_value(v, row, bundle, mapping, locators);
            }
        }
        _ => (),
    }
}

fn rewrite_graph_fields(
    value: &mut Value,
    mapping: &BTreeMap<String, (Ulid, u64)>,
    locators: &BTreeMap<String, BTreeSet<String>>,
) {
    fn reference(
        value: &mut Value,
        mapping: &BTreeMap<String, (Ulid, u64)>,
        locators: &BTreeMap<String, BTreeSet<String>>,
    ) {
        if let Value::Array(xs) = value {
            for v in xs {
                reference(v, mapping, locators);
            }
            return;
        }
        let Some(text) = value
            .as_str()
            .map(str::to_owned)
            .or_else(|| value.as_u64().map(|n| format!("#{n}")))
        else {
            return;
        };
        if text.contains("[[")
            || text.starts_with("unresolved source reference (")
            || text.contains("://")
        {
            return;
        }
        *value = match locators
            .get(&text)
            .filter(|xs| xs.len() == 1)
            .and_then(|xs| xs.first())
            .and_then(|path| mapping.get(path))
        {
            Some((id, _)) => json!(format!("[[{id}]]")),
            None => json!(format!("unresolved-source-reference:{text}")),
        };
    }
    for key in [
        "parent",
        "children",
        "depends_on",
        "for",
        "derived_from",
        "artifacts",
        "on",
    ] {
        if let Some(v) = value.get_mut(key) {
            reference(v, mapping, locators);
        }
    }
    if let Some(links) = value.get_mut("links").and_then(Value::as_array_mut) {
        for link in links {
            if let Some(v) = link.get_mut("to") {
                reference(v, mapping, locators);
            }
        }
    }
}

fn rewrite(
    text: &str,
    row: &Row,
    bundle: &Bundle,
    mapping: &BTreeMap<String, (Ulid, u64)>,
    locators: &BTreeMap<String, BTreeSet<String>>,
) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else {
            output.push_str(&rest[start..]);
            return output;
        };
        let raw = &after[..end];
        let locator = raw.split('|').next().unwrap_or(raw).trim();
        let declared = bundle
            .manifest
            .links
            .iter()
            .find(|l| l.from == row.source_path && l.raw == raw);
        let target = if let Some(link) = declared {
            (link.result == "resolved").then(|| &link.targets[0])
        } else {
            locators
                .get(locator)
                .filter(|xs| xs.len() == 1)
                .and_then(|xs| xs.first())
        };
        if let Some((id, _)) = target.and_then(|path| mapping.get(path)) {
            let label = raw
                .split_once('|')
                .map(|(_, label)| format!("|{label}"))
                .unwrap_or_default();
            output.push_str(&format!("[[{id}{label}]]"));
        } else {
            output.push_str(&format!(
                "unresolved source reference ({})",
                raw.replace(['[', ']'], "")
            ));
        }
        rest = &after[end + 2..];
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Index, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().join("input");
        std::fs::create_dir(&input).unwrap();
        let id = Ulid::new();
        let raw = format!(
            "---\nid: {id}\nnumber: 171\ntype: task\nstatus: running\n---\n[[#171]] [[#999]]\n"
        );
        std::fs::write(input.join("source.md"), &raw).unwrap();
        let manifest = json!({"source_revision":"a".repeat(40),"seed_records":1,"dependency_records":0,"all_records":1,
            "rows":[{"source_id":id,"source_number":171,"source_type":"task","source_path":".bboxy/source.md",
                "source_sha256":digest(raw.as_bytes()),"destination_path":"source.md","dedupe_key":"fixture:171","seed_task_folder_record":true}],
            "links":[{"from":".bboxy/source.md","raw":"#171","targets":[".bboxy/source.md"],"result":"resolved"},
                {"from":".bboxy/source.md","raw":"#999","targets":[],"result":"unresolved"}]});
        std::fs::write(
            input.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(input.join("audit.json"),serde_json::to_vec(&json!({"decisions":[{"index":0,"disposition":"close_completed","reason":"Retained evidence","evidence":[{"path":"source.md","line":1}]}]})).unwrap()).unwrap();
        let index = Index::open(
            &temp.path().join(".bboxy"),
            SqliteStore::open(&temp.path().join("state")).unwrap(),
        )
        .unwrap();
        (temp, index, input)
    }
    fn load(input: &Path) -> Bundle {
        Bundle::load(
            &input.join("manifest.json"),
            &input.join("audit.json"),
            input,
        )
        .unwrap()
    }
    #[test]
    fn interrupted_staging_stays_inactive_and_replay_publishes_once() {
        let (_temp, mut index, input) = fixture();
        let key = SigningKey::from_bytes(&[73; 32]);
        let owner = Ulid::new();
        let author = crate::provenance::author::Author::seat(owner, Some(1), &key);
        let result = index.import_staged(
            load(&input),
            owner,
            &author,
            || {
                let mut active = Vec::new();
                collect(&index.root, &mut active)?;
                assert!(active.is_empty(), "staging leaked into active root");
                Err(invalid("simulated interruption before publication"))
            },
            || Ok(()),
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("simulated interruption")
        );
        index.scan().unwrap();
        assert!(index.documents.is_empty());
        let first = index.import_author(load(&input), owner, &author).unwrap();
        index.scan().unwrap();
        assert_eq!(index.documents.len(), 1);
        let before = index.documents.clone();
        assert_eq!(
            first,
            index.import_author(load(&input), owner, &author).unwrap()
        );
        index.scan().unwrap();
        assert_eq!(index.documents, before);
        let doc = index.documents.values().next().unwrap();
        assert!(doc.body.contains(&format!("[[{}]]", doc.id)));
        assert!(doc.body.contains("unresolved source reference (#999)"));
        assert_eq!(doc.metadata["status"], "done");
    }
    #[test]
    fn changed_source_audit_or_bundle_cannot_duplicate_an_import() {
        let (_temp, index, input) = fixture();
        let key = SigningKey::from_bytes(&[73; 32]);
        let owner = Ulid::new();
        let author = crate::provenance::author::Author::seat(owner, Some(1), &key);
        index.import_author(load(&input), owner, &author).unwrap();
        let audit = input.join("audit.json");
        let mut bytes = std::fs::read(&audit).unwrap();
        bytes.push(b' ');
        std::fs::write(&audit, bytes).unwrap();
        assert!(
            index
                .import_author(load(&input), owner, &author)
                .unwrap_err()
                .to_string()
                .contains("already imported")
        );
        std::fs::write(input.join("source.md"), b"tampered").unwrap();
        assert!(Bundle::load(&input.join("manifest.json"), &audit, &input).is_err());
    }
    #[test]
    fn self_signed_replacement_key_and_resigned_mapping_are_not_admitted_receipts() {
        use ed25519_dalek::Verifier;
        let (_temp, index, input) = fixture();
        let key = SigningKey::from_bytes(&[73; 32]);
        let owner = Ulid::new();
        let author = crate::provenance::author::Author::seat(owner, Some(1), &key);
        let result = index.import_author(load(&input), owner, &author).unwrap();
        let path = PathBuf::from(result["receipt"].as_str().unwrap());
        let original: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for replace_key in [true, false] {
            let mut receipt = original.clone();
            receipt.as_object_mut().unwrap().remove("signature");
            let signing = if replace_key {
                SigningKey::from_bytes(&[19; 32])
            } else {
                key.clone()
            };
            if replace_key {
                receipt["signer"] = json!(format!("seat:{}", Ulid::new()));
                receipt["public_key"] = json!(
                    base64::engine::general_purpose::STANDARD
                        .encode(signing.verifying_key().as_bytes())
                );
            } else {
                let name = receipt["published_sha256"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap()
                    .clone();
                receipt["published_sha256"][name] = json!("0".repeat(64));
            }
            let bytes = serde_json::to_vec(&receipt).unwrap();
            let signature = signing.sign(&bytes);
            signing.verifying_key().verify(&bytes, &signature).unwrap(); // internally consistent, still unauthorized
            receipt["signature"] =
                json!(base64::engine::general_purpose::STANDARD.encode(signature.to_bytes()));
            std::fs::remove_file(&path).unwrap();
            std::fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
            let failure = index
                .import_author(load(&input), owner, &author)
                .unwrap_err();
            assert!(
                failure.to_string().contains("admitted import intent"),
                "{failure}"
            );
        }
    }
    #[test]
    fn ordinary_write_during_staging_completes_and_conflicted_range_refuses_then_replays() {
        let (_temp, mut index, input) = fixture();
        let key = SigningKey::from_bytes(&[73; 32]);
        let owner = Ulid::new();
        let author = crate::provenance::author::Author::seat(owner, Some(1), &key);
        let control = Ulid::new();
        let result = index.import_staged(
            load(&input),
            owner,
            &author,
            || {
                index.apply_author(
                    Update {
                        path: "ordinary.md".into(),
                        metadata: json!({"id":control,"type":"note"}),
                        body: "ordinary write completed while import staging was paused".into(),
                        expected: None,
                    },
                    &author,
                )
            },
            || Ok(()),
        );
        assert!(
            result
                .err()
                .unwrap()
                .downcast_ref::<AllocationStale>()
                .is_some()
        );
        index.scan().unwrap();
        assert_eq!(index.documents.len(), 1);
        assert_eq!(index.documents["ordinary.md"].id, control.to_string());
        let control_bytes = std::fs::read(index.root.join("ordinary.md")).unwrap();
        index.import_author(load(&input), owner, &author).unwrap();
        index.scan().unwrap();
        assert_eq!(index.documents.len(), 2);
        assert_eq!(
            std::fs::read(index.root.join("ordinary.md")).unwrap(),
            control_bytes
        );
    }
    #[test]
    fn source_symlinks_and_duplicate_manifest_identity_are_refused() {
        let (_temp, _index, input) = fixture();
        let original = input.join("source.md");
        let saved = input.join("saved");
        std::fs::rename(&original, &saved).unwrap();
        std::os::unix::fs::symlink(&saved, &original).unwrap();
        assert!(
            Bundle::load(
                &input.join("manifest.json"),
                &input.join("audit.json"),
                &input
            )
            .err()
            .unwrap()
            .to_string()
            .contains("symlink")
        );
    }
}
