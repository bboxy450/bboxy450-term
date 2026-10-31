// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Example `data_source` plugin: the Markdown files of its root, each with a
//! `---` frontmatter of `key: value` lines (a value is JSON when it parses,
//! else a string). Lists and searches read the root afresh; writes create or
//! replace one file and report `lossy` when the stored form differs.
#![allow(clippy::disallowed_methods)] // A plugin process: no UI thread.

use std::path::{Path, PathBuf};

use bbt_plugin::data_source as ds;
use bbt_plugin_sdk::contracts::{AsDataSource, Base, DataSource};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

struct Notes {
    root: PathBuf,
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn parse(text: &str) -> (Map<String, Value>, String) {
    let mut fields = Map::new();
    let Some(rest) = text.strip_prefix("---\n") else {
        return (fields, text.into());
    };
    let Some((head, body)) = rest.split_once("\n---\n") else {
        return (fields, text.into());
    };
    for line in head.lines() {
        if let Some((key, value)) = line.split_once(": ") {
            let value = serde_json::from_str(value).unwrap_or_else(|_| Value::from(value));
            fields.insert(key.trim().into(), value);
        }
    }
    (fields, body.into())
}

fn render(fields: &Map<String, Value>, body: &str) -> String {
    let mut text = String::from("---\n");
    for (key, value) in fields {
        text.push_str(&format!("{key}: {value}\n"));
    }
    text + "---\n" + body
}

impl Notes {
    fn paths(&self, dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                self.paths(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                out.push(path);
            }
        }
    }

    fn record(&self, path: &Path) -> Result<ds::Record, Refusal> {
        let bytes = std::fs::read(path).map_err(|e| Refusal::new("not_found", e.to_string()))?;
        let text = String::from_utf8_lossy(&bytes);
        let (frontmatter, body) = parse(&text);
        let relative = path
            .strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let slug = relative.trim_end_matches(".md").to_owned();
        let digest = hex(&bytes);
        Ok(ds::Record {
            reference: ds::RecordRef {
                id: frontmatter
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(&slug)
                    .to_owned(),
                number: frontmatter.get("number").and_then(Value::as_u64),
                slug,
                path: relative,
                digest: digest.clone(),
                external_id: None,
            },
            frontmatter: Value::Object(frontmatter),
            body,
            etag: digest,
            resources: Vec::new(),
            parse_error: None,
        })
    }

    fn records(&self, ctx: &Ctx) -> Result<Vec<ds::Record>, Refusal> {
        let mut paths = Vec::new();
        self.paths(&self.root, &mut paths);
        paths
            .iter()
            .map(|path| {
                ctx.check()?;
                self.record(path)
            })
            .collect()
    }

    // A simple numeric cursor over the sorted records.
    fn page<T>(items: Vec<T>, cursor: Option<&str>, limit: Option<usize>) -> ds::Page<T> {
        let start: usize = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);
        let limit = limit.unwrap_or(ds::DEFAULT_PAGE).clamp(1, ds::MAX_PAGE);
        let total = items.len();
        let items: Vec<T> = items.into_iter().skip(start).take(limit).collect();
        let end = start + items.len();
        ds::Page {
            items,
            next: (end < total).then(|| end.to_string()),
        }
    }
}

impl Base for Notes {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-data_source-example",
            "Example Markdown notes",
            "The Markdown files of one folder",
        )
    }
    fn initialize(init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        let root = init
            .root
            .ok_or_else(|| Refusal::new("invalid_root", "root is required"))?;
        let root =
            std::fs::canonicalize(root).map_err(|e| Refusal::new("invalid_root", e.to_string()))?;
        Ok(Self { root })
    }
}

impl DataSource for Notes {
    fn resolve(&mut self, req: ds::ResolveRequest, ctx: &Ctx) -> Result<ds::RecordRef, Refusal> {
        let number = req
            .locator
            .strip_prefix('#')
            .and_then(|n| n.parse::<u64>().ok());
        let found: Vec<_> = self
            .records(ctx)?
            .into_iter()
            .map(|record| record.reference)
            .filter(|r| {
                number.is_some_and(|n| r.number == Some(n))
                    || r.slug == req.locator
                    || r.path == req.locator
                    || r.id == req.locator
            })
            .collect();
        match found.as_slice() {
            [one] => Ok(one.clone()),
            [] => Err(Refusal::new("not_found", "no record matches")),
            _ => Err(Refusal::new("ambiguous_locator", "several records match")
                .with("candidates", serde_json::to_value(&found)?)),
        }
    }
    fn fetch(&mut self, req: ds::FetchRequest, _ctx: &Ctx) -> Result<ds::Record, Refusal> {
        self.record(&self.root.join(&req.reference.path))
    }
    fn search(
        &mut self,
        req: ds::SearchRequest,
        ctx: &Ctx,
    ) -> Result<ds::Page<ds::SearchHit>, Refusal> {
        let needle = req.query.to_lowercase();
        let hits = self
            .records(ctx)?
            .into_iter()
            .filter(|record| {
                needle.split_whitespace().any(|word| {
                    record.body.to_lowercase().contains(word)
                        || record.frontmatter.to_string().to_lowercase().contains(word)
                })
            })
            .map(|record| ds::SearchHit {
                title: record.frontmatter["title"]
                    .as_str()
                    .unwrap_or(&record.reference.slug)
                    .into(),
                record_type: record.frontmatter["type"].as_str().unwrap_or("note").into(),
                number: record.reference.number,
                snippet: record.body.chars().take(120).collect(),
                score: 1.,
                reference: record.reference,
            })
            .collect();
        Ok(Self::page(hits, req.cursor.as_deref(), req.limit))
    }
    fn list(
        &mut self,
        req: ds::ListRequest,
        ctx: &Ctx,
    ) -> Result<ds::Page<ds::RecordRef>, Refusal> {
        let wanted = |field: &str, value: &Option<String>, record: &ds::Record| {
            value
                .as_ref()
                .is_none_or(|v| record.frontmatter.get(field).and_then(Value::as_str) == Some(v))
        };
        let refs = self
            .records(ctx)?
            .into_iter()
            .filter(|r| {
                wanted("type", &req.filter.record_type, r)
                    && wanted("status", &req.filter.status, r)
            })
            .map(|record| record.reference)
            .collect();
        Ok(Self::page(refs, req.cursor.as_deref(), req.limit))
    }
    // These notes carry no links.
    fn links(&mut self, _req: ds::FetchRequest, _ctx: &Ctx) -> Result<ds::Links, Refusal> {
        Ok(ds::Links {
            outgoing: Vec::new(),
            incoming: Vec::new(),
        })
    }
    fn graph(&mut self, req: ds::GraphRequest, ctx: &Ctx) -> Result<ds::GraphPage, Refusal> {
        let nodes = self
            .records(ctx)?
            .into_iter()
            .map(|record| ds::Node {
                id: record.reference.id.clone(),
                title: record.frontmatter["title"].as_str().map(str::to_owned),
                record_type: record.frontmatter["type"].as_str().map(str::to_owned),
                frontmatter: record.frontmatter,
                reference: Some(record.reference),
            })
            .collect();
        let page = Self::page(nodes, req.cursor.as_deref(), req.limit);
        Ok(ds::GraphPage {
            nodes: page.items,
            edges: Vec::new(),
            next: page.next,
            edges_truncated: false,
        })
    }
    fn write(&mut self, req: ds::WriteRequest, _ctx: &Ctx) -> Result<ds::WriteResult, Refusal> {
        let relative = match (&req.reference, &req.path) {
            (Some(reference), _) => reference.path.clone(),
            (None, Some(path)) => path.clone(),
            (None, None) => {
                return Err(Refusal::new(
                    "invalid_arguments",
                    "a path or ref is required",
                ));
            }
        };
        if relative.contains("..") || Path::new(&relative).is_absolute() {
            return Err(Refusal::new(
                "invalid_path",
                "the path must stay inside the root",
            ));
        }
        let path = self.root.join(&relative);
        match &req.reference {
            None if path.exists() => {
                return Err(Refusal::new("path_exists", "the path already exists"));
            }
            Some(_) => {
                let current = self.record(&path)?;
                if req.etag.as_ref().is_some_and(|etag| *etag != current.etag) {
                    return Err(Refusal::new(
                        "stale_etag",
                        "the record changed since it was read",
                    ));
                }
            }
            None => (),
        }
        let fields = req.frontmatter.as_object().cloned().unwrap_or_default();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Refusal::new("write_failed", e.to_string()))?;
        }
        std::fs::write(&path, render(&fields, &req.body))
            .map_err(|e| Refusal::new("write_failed", e.to_string()))?;
        let record = self.record(&path)?;
        let lossy = record.frontmatter != Value::Object(fields) || record.body != req.body;
        Ok(ds::WriteResult { record, lossy })
    }
    // No change log: every record is reported once, from cursor 0.
    fn changes_since(
        &mut self,
        req: ds::ChangesRequest,
        ctx: &Ctx,
    ) -> Result<ds::Changes, Refusal> {
        let changed = if req.cursor == "0" {
            self.records(ctx)?
                .into_iter()
                .map(|r| r.reference)
                .take(req.limit.unwrap_or(ds::MAX_PAGE))
                .collect()
        } else {
            Vec::new()
        };
        Ok(ds::Changes {
            changed,
            next: "1".into(),
        })
    }
}

fn main() {
    run::<AsDataSource<Notes>>();
}
