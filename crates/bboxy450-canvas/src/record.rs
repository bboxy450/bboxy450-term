// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Interim Markdown record codec. Log and model know nothing about Markdown.
use crate::CanvasRecord;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    #[serde(rename = "type")]
    pub record_type: String,
    pub id: String,
    pub number: u64,
    pub slug: String,
    pub title: String,
    pub swarm: String,
    pub tags: Vec<String>,
    pub creator: String,
    pub task: Option<String>,
}
impl Record {
    pub(crate) fn new(id: String, number: u64, mut record: CanvasRecord) -> Self {
        record.tags.sort();
        record.tags.dedup();
        Self {
            record_type: "canvas".into(),
            id,
            number,
            slug: format!("canvas-{number}"),
            title: record.title,
            swarm: record.swarm,
            tags: record.tags,
            creator: record.creator,
            task: record.task,
        }
    }
}
pub fn encode(record: &Record) -> io::Result<Vec<u8>> {
    let yaml = serde_yaml::to_string(record).map_err(io::Error::other)?;
    Ok(format!("---\n{yaml}---\n").into_bytes())
}
pub fn decode(bytes: &[u8]) -> io::Result<Record> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid canvas frontmatter");
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let (yaml, _) = text
        .strip_prefix("---\n")
        .and_then(|s| s.split_once("\n---\n"))
        .ok_or_else(invalid)?;
    let record: Record = serde_yaml::from_str(yaml).map_err(io::Error::other)?;
    if record.record_type != "canvas"
        || record.slug != format!("canvas-{}", record.number)
        || record.title.trim().is_empty()
        || record.swarm.is_empty()
        || record.creator.is_empty()
    {
        return Err(invalid());
    }
    Ok(record)
}
