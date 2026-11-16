// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Den at the plugin boundary (`specs/spec-language.md`, "In a Markdown-only
//! provider: GitKB"). A `data_source` plugin stores Markdown only, so the host
//! lowers a `.den` record before it crosses: the provider gets `x.md` whose
//! frontmatter is the record's metadata plus two host keys, and whose body is
//! the record's body.
//!
//! | Key | Value |
//! |---|---|
//! | `generated_from` | `{module, source_sha256}` of the Den source |
//! | `den_source` | the whole Den source, verbatim |
//!
//! The contract carries frontmatter as arbitrary JSON, so the source travels
//! in a key, not in a fenced block that would change the body. [`raise`]
//! reads a stored record back into the same Den record and names any field
//! edited outside the host (a forked projection) instead of accepting it.
use crate::knowledge::data_source as d;
use serde_json::{Map, Value, json};

/// The keys only this module writes into a lowered record.
const LOWERING: [&str; 3] = ["generated_from", "den_source", "den_state"];
/// Fields a provider derives and the signed write strips before dispatch.
const DERIVED: [&str; 7] = [
    "bboxy_signed_record",
    "bboxy_status",
    "gitkb",
    "provenance",
    "created",
    "updated",
    "slug",
];

fn error(code: &str, message: impl Into<String>) -> d::CommandError {
    d::CommandError::new(code, message)
}

/// The Markdown path a provider stores for a `.den` path.
pub(crate) fn markdown_path(den: &str) -> Option<String> {
    den.strip_suffix(".den").map(|stem| format!("{stem}.md"))
}

/// The Den source a lowered record carries, if it is one.
pub(crate) fn source(frontmatter: &Value) -> Option<&str> {
    frontmatter.get("den_source").and_then(Value::as_str)
}

/// Refuses lowering keys in a write that is not a Den record: a Markdown
/// record never claims to be a projection it is not.
pub(crate) fn refuse_claim(frontmatter: &Value) -> d::Result<()> {
    match LOWERING.iter().find(|key| frontmatter.get(**key).is_some()) {
        Some(key) => Err(error(
            "den_field_reserved",
            format!("`{key}` is written by the host when it lowers a .den record; remove it"),
        )),
        None => Ok(()),
    }
}

/// Lowers `(metadata, body)` of the Den record at `den_path` in place:
/// renders the Den source (keeping `previous` author text byte for byte) and
/// adds `generated_from` and `den_source`. A pair Den cannot hold exactly is
/// refused before anything reaches the provider.
pub(crate) fn lower(
    den_path: &str,
    frontmatter: &mut Map<String, Value>,
    body: &str,
    previous: Option<&str>,
) -> d::Result<()> {
    for key in LOWERING {
        frontmatter.remove(key);
    }
    let metadata = Value::Object(frontmatter.clone());
    let lossy = |reason: String| {
        error(
            "den_lossy",
            format!("{den_path} cannot cross to a Markdown provider without loss: {reason}"),
        )
    };
    let source =
        crate::knowledge::den::render(den_path, &metadata, body, previous).map_err(lossy)?;
    if crate::knowledge::den::parse(&source).map_err(lossy)? != (metadata, body.to_owned()) {
        return Err(lossy(
            "the Den source does not read back as the record".into(),
        ));
    }
    let module = bboxy450_den::load(&source)
        .ok()
        .and_then(|records| records.into_iter().next())
        .map(|record| record.module.join("."))
        .unwrap_or_default();
    frontmatter.insert(
        "generated_from".into(),
        json!({"module": module, "source_sha256": bboxy450_den::render::source_sha256(&source)}),
    );
    frontmatter.insert("den_source".into(), json!(source));
    Ok(())
}

/// One Den record read back from a provider's Markdown.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Raised {
    pub(crate) path: String,
    pub(crate) metadata: Value,
    pub(crate) body: String,
    pub(crate) source: String,
}

/// Reads a lowered record back as its Den record. `Ok(None)` is an ordinary
/// Markdown record; `Err` names what was edited outside the host.
pub(crate) fn raise(record: &d::Record) -> Result<Option<Raised>, String> {
    let Some(text) = source(&record.frontmatter) else {
        return Ok(None);
    };
    let declared = record.frontmatter["generated_from"]["source_sha256"].as_str();
    if declared != Some(bboxy450_den::render::source_sha256(text).as_str()) {
        return Err("den_source differs from generated_from.source_sha256".into());
    }
    let (metadata, body) = crate::knowledge::den::parse(text)?;
    if body != record.body {
        return Err("the Markdown body differs from the Den source".into());
    }
    let empty = Map::new();
    let stored = record.frontmatter.as_object().unwrap_or(&empty);
    let fields = metadata.as_object().unwrap_or(&empty);
    let changed = fields
        .iter()
        .filter(|(key, value)| stored.get(*key) != Some(value))
        .map(|(key, _)| key.as_str())
        .chain(stored.keys().map(String::as_str).filter(|key| {
            !fields.contains_key(*key) && !LOWERING.contains(key) && !DERIVED.contains(key)
        }))
        .collect::<Vec<_>>();
    if !changed.is_empty() {
        return Err(format!(
            "frontmatter differs from the Den source: {}",
            changed.join(", ")
        ));
    }
    let path = record.reference.path.strip_suffix(".md").map_or_else(
        || record.reference.path.clone(),
        |stem| format!("{stem}.den"),
    );
    Ok(Some(Raised {
        path,
        metadata,
        body,
        source: text.to_owned(),
    }))
}

/// A fetched record as the host shows it: an edited projection says so in
/// `den_state: forked`, never silently accepted.
pub(crate) fn mark(mut record: d::Record) -> d::Record {
    if let (Err(_), Some(map)) = (raise(&record), record.frontmatter.as_object_mut()) {
        map.insert("den_state".into(), json!("forked"));
    }
    record
}
