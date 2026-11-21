// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Artifact content lives in Blobs. Only signed root activity publishes a revision.
use crate::{
    Ulid,
    index::{Index, Update},
    provenance::author::Author,
    store::{
        Result, digest,
        events::{Draft, Envelope},
        invalid,
    },
};
use bbt_supervisor::store::{Blobs, blobs::LocalBlobs};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(crate) const MAX_INLINE: usize = 262_144;
pub(crate) const MAX_FILE: usize = 16_777_216;
pub(crate) const MAX_PINS: usize = 32;
pub(crate) const SVG: &str = "image/svg+xml";
pub(crate) const PNG: &str = "image/png";

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub(crate) struct Address {
    pub(crate) id: Ulid,
    pub(crate) revision: u64,
}

impl Address {
    pub(crate) fn validate(self) -> Result<()> {
        if self.id == Ulid::nil() || self.revision == 0 {
            return Err(invalid("artifact ID and revision must be nonzero"));
        }
        Ok(())
    }

    pub(crate) fn target(target: &str) -> Result<(Ulid, Option<u64>)> {
        bboxy450_canvas::artifact_address(target.strip_prefix("artifact:").unwrap_or(target))
            .ok_or_else(|| invalid("invalid stored artifact address"))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Pin {
    #[serde(flatten)]
    address: Address,
    digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Capture {
    observed: Vec<Pin>,
    declared: Vec<Pin>,
    missing: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Revision {
    #[serde(flatten)]
    pub(crate) address: Address,
    pub(crate) media_type: String,
    pub(crate) digest: String,
    bytes: usize,
    capture: Capture,
}

#[derive(Deserialize, Serialize)]
struct Content {
    root: String,
    owner: Ulid,
    operation: Option<Ulid>,
    request: Option<String>,
    origin: Option<Ulid>,
    producer: String,
    revision: Revision,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Captured {
    content: Content,
    expected: Option<Address>,
}

impl Captured {
    pub(crate) fn matches(&self, owner: Ulid, operation: Ulid, fingerprint: &str) -> Result<()> {
        if self.content.owner != owner
            || self.content.operation != Some(operation)
            || self.content.request.as_deref() != Some(fingerprint)
        {
            return Err(invalid(
                "op_id_reused: captured bytes belong to another artifact request",
            ));
        }
        Ok(())
    }
}

pub(crate) enum Attempt {
    Start,
    Captured(Box<Captured>),
    Published(Revision),
}

impl Revision {
    fn pin(&self) -> Pin {
        Pin {
            address: self.address,
            digest: self.digest.clone(),
        }
    }

    pub(crate) fn value(&self, write: bool) -> Value {
        let Address { id, revision } = self.address;
        let image = matches!(self.media_type.as_str(), SVG | PNG);
        let prefix = if image { "!" } else { "" };
        let reference = format!("{prefix}[[artifact:{id}]]");
        let snapshot = format!("{prefix}[[artifact:{id}:{revision}]]");
        let mut value = json!({
            "id": id, "media_type": self.media_type, "revision": revision,
            "digest": self.digest, "bytes": self.bytes, "reference": reference,
            "snapshot_reference": snapshot, "capture": self.capture, "custody": "stored",
        });
        if image && write {
            value["display_hint"] = json!(format!(
                "Print {reference} verbatim outside a code fence to show this image. Use {snapshot} for an unchanged historical view."
            ));
        }
        value
    }
}

impl Content {
    fn provenance(&self) -> Value {
        let mut value = self.revision.value(false);
        value["type"] = json!("artifact");
        value["owner"] = json!(self.owner);
        value["produced_by"] = json!(self.producer);
        value["origin_execution"] = json!(self.origin);
        value["pin"] = json!(format!(
            "artifact:{}:{}",
            self.revision.address.id, self.revision.address.revision
        ));
        value["basis"] = json!("signed_artifact_revision");
        value
    }

    fn decode(row: &Envelope) -> Result<Self> {
        let content: Self = serde_json::from_value(row.event.payload.clone())?;
        let Address { id, revision } = content.revision.address;
        if row.event.source != "artifact/content"
            || row.event.subject != subject(id, revision)
            || revision == 0
        {
            return Err(invalid(
                "artifact content address disagrees with the signed event",
            ));
        }
        Ok(content)
    }
}

fn subject(id: Ulid, revision: u64) -> String {
    format!("artifact/{id}/content/{revision:020}")
}

fn event_id(purpose: &str, owner: Ulid, operation: Ulid) -> Ulid {
    let hash = Sha256::digest(format!("bboxy/artifact/{purpose}/v1/{owner}/{operation}"));
    Ulid::from_bytes(hash[..16].try_into().expect("digest contains 16 bytes"))
}

pub(crate) struct Store<'a> {
    index: &'a Index,
    root: &'a str,
    objects: PathBuf,
}

pub(crate) struct ExecutionSource<'a> {
    pub(crate) id: Ulid,
    pub(crate) bytes: &'a [u8],
    pub(crate) observer: &'a Author<'a>,
}

pub(crate) struct Write<'a> {
    pub(crate) operation: Ulid,
    pub(crate) request_digest: String,
    pub(crate) expected: Option<Address>,
    pub(crate) media_type: &'a str,
    pub(crate) inputs: &'a [Address],
    pub(crate) execution: Option<ExecutionSource<'a>>,
    pub(crate) reader: Option<Ulid>,
}

impl<'a> Store<'a> {
    pub(crate) fn new(index: &'a Index, root: &'a str, runtime: &Path) -> Self {
        Self {
            index,
            root,
            objects: crate::artifact_images::object_path(runtime, root),
        }
    }

    fn validate(&self, row: &Envelope) -> Result<Content> {
        let content = Content::decode(row)?;
        if content.root != self.root {
            return Err(invalid("signed artifact belongs to another root"));
        }
        Ok(content)
    }

    fn content(&self, id: Ulid, revision: Option<u64>) -> Result<Option<Content>> {
        self.index
            .activity()
            .artifact_content(id, revision)?
            .as_ref()
            .map(|event| self.validate(event))
            .transpose()
    }

    fn verified_revision(&self, id: Ulid, revision: Option<u64>) -> Result<Content> {
        if let Some(content) = self.content(id, revision)? {
            return Ok(content);
        }
        if revision.is_some() && self.content(id, None)?.is_some() {
            return Err(invalid(
                "artifact_revision_not_found: use an existing exact revision",
            ));
        }
        Err(invalid(
            "not_stored: use provenance_get for reference custody",
        ))
    }

    pub(crate) fn read(
        &self,
        id: Ulid,
        revision: Option<u64>,
        caller: Option<Ulid>,
    ) -> Result<(Revision, Vec<u8>)> {
        let content = self.verified_revision(id, revision)?;
        Self::authorize(&content, caller)?;
        let bytes = self.read_bytes(&content.revision)?;
        Ok((content.revision, bytes))
    }

    fn authorize(content: &Content, caller: Option<Ulid>) -> Result<()> {
        if caller.is_some_and(|caller| caller != content.owner) {
            Err(invalid(
                "artifact_forbidden: content belongs to another seat",
            ))
        } else {
            Ok(())
        }
    }

    /// Read revision metadata from the signed stream without opening the Blob.
    pub(crate) fn provenance(
        &self,
        id: Ulid,
        revision: Option<u64>,
        caller: Option<Ulid>,
    ) -> Result<Option<Value>> {
        let Some(latest) = self.content(id, None)? else {
            return Ok(None);
        };
        Self::authorize(&latest, caller)?;
        let content =
            if revision.is_none_or(|revision| revision == latest.revision.address.revision) {
                latest
            } else {
                self.verified_revision(id, revision)?
            };
        Self::authorize(&content, caller)?;
        Ok(Some(content.provenance()))
    }

    pub(crate) fn descriptor(
        &self,
        id: Ulid,
        revision: Option<u64>,
        caller: Option<Ulid>,
    ) -> Result<crate::artifact_images::Resolved> {
        Address {
            id,
            revision: revision.unwrap_or(1),
        }
        .validate()?;
        let content = self.verified_revision(id, revision)?;
        Self::authorize(&content, caller)?;
        let revision = content.revision;
        Ok(crate::artifact_images::Resolved {
            root: self.root.to_owned(),
            id,
            revision: revision.address.revision,
            digest: revision.digest,
            media_type: revision.media_type,
            bytes: revision.bytes,
        })
    }

    #[allow(
        clippy::disallowed_methods,
        reason = "Blob directory durability on the publication worker"
    )]
    fn open_blobs_for_write(&self) -> Result<LocalBlobs> {
        let blobs = LocalBlobs::open(&self.objects)?;
        // LocalBlobs flushes the file and its directory. Also flush the three
        // parent entries that may have been created for this native root.
        for directory in self.objects.ancestors().take(3) {
            std::fs::File::open(directory)?.sync_all()?;
        }
        Ok(blobs)
    }

    fn read_bytes(&self, revision: &Revision) -> Result<Vec<u8>> {
        crate::artifact_images::payload(&self.objects, &revision.digest, revision.bytes)
    }

    fn append(&self, id: Ulid, content: &Content, author: &Author<'_>) -> Result<()> {
        let addr = content.revision.address;
        let mut draft = Draft::new(
            subject(addr.id, addr.revision),
            "artifact/content",
            serde_json::to_value(content)?,
        );
        draft.id = id;
        draft.seat = author.seat;
        draft.generation = author.generation;
        self.index.append_activity(draft, author.key)?;
        Ok(())
    }

    fn execution_pin(
        &self,
        owner: Ulid,
        source: &ExecutionSource<'_>,
        blobs: &LocalBlobs,
    ) -> Result<Pin> {
        let id = event_id("execution-artifact", owner, source.id);
        let expected_digest = digest(source.bytes);
        if let Some(previous) = self.content(id, Some(1))? {
            if previous.owner != owner
                || previous.origin != Some(source.id)
                || previous.revision.media_type != "text/x-python"
                || previous.revision.digest != expected_digest
            {
                return Err(invalid(
                    "execution_source_changed: immutable source binding differs",
                ));
            }
            self.read_bytes(&previous.revision)?;
            return Ok(previous.revision.pin());
        }
        self.refuse_reference(id)?;
        let revision = Revision {
            address: Address { id, revision: 1 },
            media_type: "text/x-python".into(),
            digest: blobs.put(source.bytes)?,
            bytes: source.bytes.len(),
            capture: Capture {
                observed: vec![],
                declared: vec![],
                missing: vec![],
            },
        };
        let pin = revision.pin();
        self.append(
            event_id("execution-event", owner, source.id),
            &Content {
                root: self.root.into(),
                owner,
                operation: None,
                request: None,
                origin: Some(source.id),
                producer: source.observer.id.clone(),
                revision,
            },
            source.observer,
        )?;
        Ok(pin)
    }

    fn refuse_reference(&self, id: Ulid) -> Result<()> {
        if self.index.current(&format!("artifacts/{id}.md"))?.is_some() {
            return Err(invalid(
                "artifact_custody_conflict: ID already has reference custody",
            ));
        }
        Ok(())
    }

    /// Recheck the operation before loading a staged path, so a replay
    /// returns the original payload even when the staging file has changed.
    pub(crate) fn replay(
        &self,
        owner: Ulid,
        operation: Ulid,
        fingerprint: &str,
    ) -> Result<Option<Revision>> {
        let Some(row) =
            self.index
                .activity()
                .artifact_operation(event_id("operation", owner, operation))?
        else {
            return Ok(None);
        };
        let prior = self.validate(&row)?;
        if prior.owner != owner
            || prior.operation != Some(operation)
            || prior.request.as_deref() != Some(fingerprint)
        {
            return Err(invalid(
                "op_id_reused: operation arguments differ from the retained artifact write",
            ));
        }
        self.read_bytes(&prior.revision)?;
        Ok(Some(prior.revision))
    }

    pub(crate) fn inspect(
        &self,
        owner: Ulid,
        operation: Ulid,
        fingerprint: &str,
    ) -> Result<Attempt> {
        if let Some(revision) = self.replay(owner, operation, fingerprint)? {
            return Ok(Attempt::Published(revision));
        }
        let attempt = event_id("capture", owner, operation);
        let Some(event) = self.index.activity().artifact_attempt(attempt)? else {
            return Ok(Attempt::Start);
        };
        let captured: Captured = serde_json::from_value(event.event.payload.clone())?;
        if captured.content.root != self.root {
            return Err(invalid(
                "captured artifact is outside the current native root",
            ));
        }
        captured.matches(owner, operation, fingerprint)?;
        Ok(Attempt::Captured(Box::new(captured)))
    }

    pub(crate) fn check_expected(&self, owner: Ulid, expected: Address) -> Result<()> {
        let current = self.verified_revision(expected.id, None)?;
        Self::authorize(&current, Some(owner))?;
        if current.revision.media_type != SVG {
            return Err(invalid(
                "artifact_immutable: only an SVG image admits updates",
            ));
        }
        if current.revision.address.revision != expected.revision {
            return Err(invalid(
                "artifact_conflict: read the current revision and reconcile before updating",
            ));
        }
        Ok(())
    }

    fn pin_input(&self, address: Address, caller: Option<Ulid>) -> Result<Pin> {
        address.validate()?;
        let input = self.verified_revision(address.id, Some(address.revision))?;
        Self::authorize(&input, caller)?;
        self.read_bytes(&input.revision)?;
        Ok(input.revision.pin())
    }

    fn capture(
        &self,
        request: &Write<'_>,
        author: &Author<'_>,
        owner: Ulid,
        bytes: &[u8],
    ) -> Result<Captured> {
        if bytes.len() > MAX_FILE {
            return Err(invalid("artifact_too_large: staged bytes exceed 16 MiB"));
        }
        let (address, media_type) = if let Some(expected) = request.expected {
            self.check_expected(owner, expected)?;
            let revision = expected
                .revision
                .checked_add(1)
                .ok_or_else(|| invalid("artifact revision exhausted"))?;
            (
                Address {
                    id: expected.id,
                    revision,
                },
                SVG,
            )
        } else {
            let id = Ulid::new();
            self.refuse_reference(id)?;
            (Address { id, revision: 1 }, request.media_type)
        };
        if request.inputs.len() > MAX_PINS {
            return Err(invalid("artifact_input_limit: at most 32 pins"));
        }
        let declared = request
            .inputs
            .iter()
            .map(|address| self.pin_input(*address, request.reader))
            .collect::<Result<Vec<_>>>()?;
        let blobs = self.open_blobs_for_write()?;
        let observed = request
            .execution
            .as_ref()
            .map(|source| self.execution_pin(owner, source, &blobs))
            .transpose()?;
        let missing = if observed.is_some() {
            vec![]
        } else {
            vec!["execution_source".into()]
        };
        let content = Content {
            root: self.root.into(),
            owner,
            operation: Some(request.operation),
            request: Some(request.request_digest.clone()),
            origin: request.execution.as_ref().map(|source| source.id),
            producer: author.id.clone(),
            revision: Revision {
                address,
                media_type: media_type.into(),
                digest: blobs.put(bytes)?,
                bytes: bytes.len(),
                capture: Capture {
                    observed: observed.into_iter().collect(),
                    declared,
                    missing,
                },
            },
        };
        let captured = Captured {
            content,
            expected: request.expected,
        };
        let id = event_id("capture", owner, request.operation);
        let mut draft = Draft::new(
            format!("artifact/{id}/capture"),
            "artifact/capture",
            serde_json::to_value(&captured)?,
        );
        draft.id = id;
        draft.seat = author.seat;
        draft.generation = author.generation;
        self.index.append_activity(draft, author.key).map_err(|e| {
            invalid(format!(
                "unknown: artifact capture receipt unavailable: {e}"
            ))
        })?;
        Ok(captured)
    }

    /// Resume only a signed capture on an admitted original-operation retry.
    /// Missing captures never cause a staging path or a later script to be read.
    pub(crate) fn write(
        &self,
        request: &Write<'_>,
        author: &Author<'_>,
        bytes: Option<&[u8]>,
    ) -> Result<Revision> {
        if !self.index.owns_projection() {
            return Err(invalid(
                "provider_busy: the root publication lease is not held",
            ));
        }
        let owner = author
            .seat
            .ok_or_else(|| invalid("artifact write requires an admitted seat"))?;
        let captured = match self.inspect(owner, request.operation, &request.request_digest)? {
            Attempt::Published(revision) => return Ok(revision),
            Attempt::Captured(capture) => *capture,
            Attempt::Start => {
                let bytes = bytes.ok_or_else(|| {
                    invalid("unknown: original operation has no durable byte capture")
                })?;
                self.capture(request, author, owner, bytes)?
            }
        };
        let revision = &captured.content.revision;
        for pin in revision
            .capture
            .declared
            .iter()
            .chain(&revision.capture.observed)
        {
            if self.pin_input(pin.address, request.reader)?.digest != pin.digest {
                return Err(invalid(
                    "artifact_unavailable: an exact input digest changed",
                ));
            }
        }
        self.read_bytes(revision)?;
        let id = revision.address.id;
        self.index.with_publication(|| {
            if let Some(expected) = captured.expected {
                self.check_expected(owner, expected)?;
            } else {
                self.refuse_reference(id)?;
            }
            self.append(
                event_id("operation", owner, request.operation),
                &captured.content,
                author,
            )
            .map_err(|e| {
                invalid(format!(
                    "unknown: artifact publication outcome unavailable: {e}"
                ))
            })?;
            Ok(captured.content.revision)
        })
        .map_err(|error| {
            if error.is::<crate::index::IndexBusy>() {
                invalid("unknown: artifact captured; retry the original operation to publish")
            } else {
                error
            }
        })
    }
}

pub(crate) fn event_provenance(row: &Envelope) -> Result<Value> {
    Ok(Content::decode(row)?.provenance())
}

pub(crate) fn projection(index: &Index, row: &Envelope) -> Result<Option<Update>> {
    if row.event.source != "artifact/content" {
        return Ok(None);
    }
    let content = Content::decode(row)?;
    let revision = &content.revision;
    let id = revision.address.id;
    let path = format!("artifacts/{id}.md");
    let old = index.current(&path)?;
    if let Some(old) = &old {
        if old.metadata["custody"] != "stored" || old.metadata["id"] != json!(id) {
            return Err(invalid(
                "artifact content cannot overwrite reference custody",
            ));
        }
        if old.metadata["last_content_sequence"]
            .as_u64()
            .is_some_and(|n| n >= row.seq)
        {
            return Ok(None);
        }
    }
    let mut metadata = revision.value(false);
    metadata["type"] = json!("artifact");
    metadata["kind"] = json!(if matches!(revision.media_type.as_str(), SVG | PNG) {
        "image"
    } else {
        "file"
    });
    metadata["ref"] = json!(format!("artifact:{id}"));
    let owner = format!("seat:{}", content.owner);
    let observed = content.producer != owner;
    metadata["owner"] = json!(owner);
    metadata["produced_by"] = json!(content.producer);
    metadata["grade"] = json!(if observed { "observed" } else { "reported" });
    metadata["observed"] = json!(observed);
    metadata["last_content_sequence"] = json!(row.seq);
    metadata["origin_execution"] = json!(content.origin);
    Ok(Some(Update {
        path,
        metadata,
        body: "# Stored artifact

Revisions and capture pins are available through artifact_read.
"
        .into(),
        expected: old.map(|doc| doc.digest),
    }))
}

#[cfg(test)]
mod tests;
