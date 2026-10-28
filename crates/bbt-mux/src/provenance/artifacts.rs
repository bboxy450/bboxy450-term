// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Artifact lineages keep responsibility separate from observed file changes.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    Ulid,
    adapters::Grade,
    index::{Index, Update},
    mcp::{Call, SeatActor as Actor, work_items::Plan},
    store::{Result, events::Envelope, invalid},
};

pub(crate) const INLINE_HISTORY: usize = 32;
pub(crate) const CANDIDATES: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Attribution {
    pub seat: Option<Ulid>,
    pub generation: Option<u64>,
    pub grade: Grade,
    pub candidates: Vec<Ulid>,
    pub reason: Option<String>,
}
impl Attribution {
    pub fn inferred(mut candidates: Vec<(Ulid, u64)>) -> Self {
        candidates.sort_unstable();
        candidates.dedup();
        let owner = if candidates.len() == 1 {
            candidates.first().copied()
        } else {
            None
        };
        let reason = if candidates.is_empty() {
            Some("no-active-seat".into())
        } else if candidates.len() > 1 {
            Some("ambiguous-active-seats".into())
        } else {
            None
        };
        Self {
            seat: owner.map(|p| p.0),
            generation: owner.map(|p| p.1),
            grade: Grade::Inferred,
            candidates: candidates
                .into_iter()
                .take(CANDIDATES)
                .map(|p| p.0)
                .collect(),
            reason,
        }
    }
    pub fn reported(seat: Ulid, generation: u64) -> Self {
        Self {
            seat: Some(seat),
            generation: Some(generation),
            grade: Grade::Reported,
            candidates: vec![],
            reason: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Change {
    pub artifact_id: Ulid,
    pub kind: String,
    pub reference: String,
    pub previous_ref: Option<String>,
    pub digest: Option<String>,
    pub before_digest: Option<String>,
    pub bytes: Option<u64>,
    pub attribution: Attribution,
    pub source: String,
    pub reason: Option<String>,
    pub commit: Option<String>,
}

fn push_history(metadata: &mut Value, row: Value) -> Result<()> {
    let items = metadata
        .as_object_mut()
        .ok_or_else(|| invalid("artifact metadata is not a mapping"))?
        .entry("history")
        .or_insert(json!([]))
        .as_array_mut()
        .ok_or_else(|| invalid("artifact history is not a list"))?;
    if items.iter().any(|old| old["id"] == row["id"]) {
        return Ok(());
    }
    items.push(row);
    if items.len() > INLINE_HISTORY {
        items.drain(..items.len() - INLINE_HISTORY);
    }
    Ok(())
}
fn file_kind(reference: &str) -> &'static str {
    if reference.starts_with("git:") {
        return "other";
    }
    let extension = std::path::Path::new(reference)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    match extension.to_ascii_lowercase().as_str() {
        "md" | "markdown" => "markdown",
        "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" => "image",
        _ => "file",
    }
}

/// Apply only a committed observation. Its original signer and sequence remain
/// in the history; the Markdown writer separately identifies the observer key.
pub(crate) fn materialize(index: &Index, envelope: &Envelope, change: &Change) -> Result<Update> {
    let path = format!("artifacts/{}.md", change.artifact_id);
    let old = index.current(&path)?;
    let mut metadata=old.as_ref().map(|d|d.metadata.clone()).unwrap_or_else(||json!({
        "id":change.artifact_id,"type":"artifact","kind":file_kind(&change.reference),"ref":change.reference,
        "claimed":false,"produced_by":null,"grade":"inferred","for":[],"derived_from":[],"aliases":[],"history":[]
    }));
    if metadata["id"] != json!(change.artifact_id) || metadata["type"] != "artifact" {
        return Err(invalid(
            "artifact record identity conflicts with observation",
        ));
    }
    if let Some(previous) = &change.previous_ref {
        let aliases = metadata
            .as_object_mut()
            .unwrap()
            .entry("aliases")
            .or_insert(json!([]))
            .as_array_mut()
            .ok_or_else(|| invalid("artifact aliases are not a list"))?;
        let value = json!(previous);
        if !aliases.contains(&value) {
            aliases.push(value);
        }
        // Aliases are locations, never content identity. Older locations remain in the log.
        if aliases.len() > INLINE_HISTORY {
            aliases.drain(..aliases.len() - INLINE_HISTORY);
        }
    }
    metadata["ref"] = json!(change.reference);
    metadata["observed"] = json!(true);
    metadata["last_observation_sequence"] = json!(envelope.seq.to_string());
    metadata["digest_grade"] = json!(if change.digest.is_some() {
        "observed"
    } else {
        "unavailable"
    });
    metadata["digest"] = json!(change.digest);
    metadata["bytes"] = json!(change.bytes);
    metadata["deleted"] = json!(change.kind == "deleted");
    metadata["observation_reason"] = json!(change.reason);
    if metadata["claimed"] != true {
        metadata["produced_by"] = json!(change.attribution.seat.map(|seat| format!("seat:{seat}")));
        metadata["grade"] = json!(change.attribution.grade);
        metadata["attribution_candidates"] = json!(change.attribution.candidates);
        metadata["attribution_reason"] = json!(change.attribution.reason);
    }
    use base64::Engine;
    push_history(
        &mut metadata,
        json!({
            "id":envelope.event.id,"sequence":envelope.seq.to_string(),"kind":change.kind,"ref":change.reference,"previous_ref":change.previous_ref,
            "before_digest":change.before_digest,"digest":change.digest,"ts":envelope.event.ts.to_string(),
            "seat":change.attribution.seat,"generation":change.attribution.generation.map(|v|v.to_string()),"grade":change.attribution.grade,
            "source":envelope.event.source,"reason":change.reason,"signer":base64::engine::general_purpose::STANDARD.encode(envelope.provenance.signer),"event_hash":envelope.hash()?,"commit":change.commit
        }),
    )?;
    Ok(Update {path,metadata,body:old.as_ref().map(|d|d.body.clone()).unwrap_or_else(||"# Observed artifact\n\nResponsibility and observation evidence are recorded in the frontmatter.\n".into()),expected:old.map(|d|d.digest)})
}

/// Reconcile the already-authorized MCP plan, without changing its Call/op_id.
/// Task links and the result use the lineage ID when detection came first.
pub(crate) fn reconcile_claim(
    index: &Index,
    plan: &mut Plan,
    actor: &Actor,
    call: &Call,
    reference: &str,
) -> std::result::Result<(), String> {
    let Some(first) = plan.writes.first_mut() else {
        return Err("artifact plan has no write".into());
    };
    if first.metadata["type"] != "artifact" {
        return Err("artifact plan type mismatch".into());
    }
    let desired_digest = first.metadata.get("digest").cloned();
    let matches: Vec<_> = index
        .documents
        .values()
        .filter(|doc| {
            doc.metadata["type"] == "artifact"
                && (doc.metadata["ref"] == reference
                    || doc.metadata["aliases"]
                        .as_array()
                        .is_some_and(|aliases| aliases.iter().any(|alias| alias == reference)))
        })
        .collect();
    let live: Vec<_> = matches
        .iter()
        .copied()
        .filter(|doc| doc.metadata["deleted"] != true)
        .collect();
    let matches = if live.is_empty() { matches } else { live };
    if matches.len() > 1 {
        return Err("artifact reference resolves to multiple records".into());
    }
    let old_op_id = first.metadata["id"].clone();
    if let Some(existing) = matches.first() {
        let current = index
            .current(&existing.path)
            .map_err(|e| e.to_string())?
            .ok_or("artifact record disappeared")?;
        if current.metadata["claimed"] == true
            && current.metadata["produced_by"] != format!("seat:{}", actor.seat_id)
        {
            return Err("artifact responsibility is held by another seat".into());
        }
        if let (Some(desired), Some(observed)) = (
            &desired_digest,
            current
                .metadata
                .get("digest")
                .filter(|v| current.metadata["observed"] == true && !v.is_null()),
        ) && desired != observed
        {
            return Err("artifact digest conflicts with observed revision".into());
        }
        let task_links = first.metadata["for"].clone();
        let mut metadata = current.metadata.clone();
        metadata["claimed"] = json!(true);
        metadata["produced_by"] = json!(format!("seat:{}", actor.seat_id));
        metadata["grade"] = json!("reported");
        metadata["kind"] = first.metadata["kind"].clone();
        if let Some(links) = task_links.as_array() {
            for field in ["for", "derived_from"] {
                let mut all = metadata[field].as_array().cloned().unwrap_or_default();
                for link in links {
                    if !all.contains(link) {
                        all.push(link.clone());
                    }
                }
                if all.len() > 128 {
                    return Err("artifact task-link bound exceeded".into());
                }
                metadata[field] = json!(all);
            }
        }
        if let Some(digest) = desired_digest {
            if metadata["observed"] == true {
                metadata["claimed_digest"] = digest;
            } else {
                metadata["digest"] = digest;
            }
        }
        *first = Update {
            path: current.path,
            metadata,
            body: current.body,
            expected: Some(current.digest),
        };
    } else {
        first.metadata["ref"] = json!(reference);
    }
    push_history(&mut first.metadata,json!({"id":call.op_id,"kind":"claimed","seat":actor.seat_id,"generation":actor.process_generation.to_string(),"grade":"reported","source":"mcp/artifact_claim","ts":crate::store::now_ms().to_string()})).map_err(|e|e.to_string())?;
    if first.metadata["observed"] != true {
        first.metadata["digest_grade"] = json!("reported-unverified");
    }
    let id = first.metadata["id"].clone();
    let path = first.path.clone();
    if id != old_op_id {
        let old_link = format!(
            "[[{}]]",
            old_op_id.as_str().ok_or("invalid original artifact ID")?
        );
        let new_link = format!("[[{}]]", id.as_str().ok_or("invalid artifact lineage ID")?);
        for update in plan.writes.iter_mut().skip(1) {
            if let Some(links) = update.metadata["artifacts"].as_array_mut() {
                for link in links {
                    if link == &old_link {
                        *link = json!(new_link);
                    }
                }
            }
        }
    }
    plan.data["artifact_id"] = id;
    plan.data["path"] = json!(format!(".bboxy/{path}"));
    Ok(())
}

/// A bounded public summary, included in the signed prepared/applied activity.
/// The terminal never copies arbitrary frontmatter or tool bodies into this field.
pub(crate) fn summary(metadata: &Value) -> Option<Value> {
    if metadata["type"] != "artifact" {
        return None;
    }
    let latest = metadata["history"]
        .as_array()
        .and_then(|h| h.last())
        .map(|row| {
            let fields = [
                "id",
                "kind",
                "seat",
                "generation",
                "grade",
                "source",
                "ts",
                "sequence",
                "digest",
                "before_digest",
                "commit",
            ];
            let mut safe = serde_json::Map::new();
            for field in fields {
                if let Some(value) = row.get(field) {
                    safe.insert(field.into(), value.clone());
                }
            }
            Value::Object(safe)
        });
    Some(
        json!({"id":metadata["id"],"kind":metadata["kind"],"ref":metadata["ref"],"digest":metadata["digest"],"claimed":metadata["claimed"],"produced_by":metadata["produced_by"],"grade":metadata["grade"],"latest":latest}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        messaging::wire::IssuerKind,
        registry::{Class, Role},
        store::{events::Draft, sqlite::SqliteStore},
    };
    use ed25519_dalek::SigningKey;
    fn actor() -> Actor {
        Actor {
            seat_id: Ulid::new(),
            process_generation: 1,
            swarm_id: Ulid::new(),
            role: Role {
                name: "executor".into(),
                class: Class::Worker,
            },
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        }
    }
    fn plan(id: Ulid, task: Ulid) -> Plan {
        Plan {
            writes: vec![
                Update {
                    path: format!("artifacts/{id}.md"),
                    metadata: json!({"id":id,"type":"artifact","kind":"file","ref":"out.txt","claimed":true,"produced_by":"test","for":[]}),
                    body: "claim".into(),
                    expected: None,
                },
                Update {
                    path: "task.md".into(),
                    metadata: json!({"id":task,"type":"task","artifacts":[format!("[[{id}]]")]}),
                    body: "fixture".into(),
                    expected: None,
                },
            ],
            wakes: vec![],
            data: json!({"artifact_id":id}),
        }
    }
    #[test]
    fn ambiguous_detection_then_claim_keeps_one_lineage_and_rewrites_task_link() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let store = SqliteStore::open(&temp.path().join("index")).unwrap();
        let mut index = Index::open(&root, store).unwrap();
        let owner = actor();
        let key = SigningKey::from_bytes(&[75; 32]);
        let id = Ulid::new();
        let attribution = Attribution::inferred(vec![(owner.seat_id, 1), (Ulid::new(), 1)]);
        assert!(attribution.seat.is_none());
        assert_eq!(
            attribution.reason.as_deref(),
            Some("ambiguous-active-seats")
        );
        let change = Change {
            artifact_id: id,
            kind: "created".into(),
            reference: "out.txt".into(),
            previous_ref: None,
            digest: Some(format!("sha256:{}", "ab".repeat(32))),
            before_digest: None,
            bytes: Some(4),
            attribution,
            source: "filesystem".into(),
            reason: None,
            commit: None,
        };
        let event = Envelope::signed(
            Draft::new(
                format!("artifact/{id}/created"),
                "filesystem",
                json!(change),
            ),
            1,
            String::new(),
            &key,
        )
        .unwrap();
        let update = materialize(&index, &event, &change).unwrap();
        assert!(update.metadata["produced_by"].is_null());
        index.apply(update, owner.seat_id, &key).unwrap();
        index.scan().unwrap();
        let op = Ulid::new();
        let task = Ulid::new();
        let mut writes = plan(op, task);
        let call = Call {
            op_id: op,
            tool: "artifact_claim".into(),
            args: json!({"kind":"file","ref":"out.txt"}),
        };
        reconcile_claim(&index, &mut writes, &owner, &call, "out.txt").unwrap();
        assert_eq!(writes.data["artifact_id"], json!(id));
        assert_eq!(
            writes.writes[1].metadata["artifacts"][0],
            format!("[[{id}]]")
        );
        let update = writes.writes.remove(0);
        assert_eq!(
            update.metadata["produced_by"],
            format!("seat:{}", owner.seat_id)
        );
        assert_eq!(update.metadata["grade"], "reported");
        index.apply(update, owner.seat_id, &key).unwrap();
        index.scan().unwrap();
        assert_eq!(
            index
                .documents
                .values()
                .filter(|d| d.metadata["type"] == "artifact")
                .count(),
            1
        );
        let mut other = owner.clone();
        other.seat_id = Ulid::new();
        assert!(
            reconcile_claim(
                &index,
                &mut plan(Ulid::new(), task),
                &other,
                &call,
                "out.txt"
            )
            .unwrap_err()
            .contains("another seat")
        );
    }
    #[test]
    fn inline_history_is_bounded_and_older_rows_stay_out_of_the_document() {
        let mut metadata = json!({});
        for n in 0..80 {
            push_history(&mut metadata, json!({"id":n,"kind":"modified"})).unwrap();
        }
        assert_eq!(metadata["history"].as_array().unwrap().len(), 32);
        assert_eq!(metadata["history"][0]["id"], 48);
        push_history(&mut metadata, json!({"id":79,"kind":"modified"})).unwrap();
        assert_eq!(metadata["history"].as_array().unwrap().len(), 32);
    }
}
