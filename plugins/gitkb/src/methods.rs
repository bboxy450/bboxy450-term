// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The data_source 1.0 methods plus explicit `commit`. Translated from
//! bboxy450-term `plugins/gitkb/adapter.py` (resolve, listing, search_page,
//! links, graph, write, changes, commit). Pages now fill up to `limit` while
//! the call budget lasts instead of examining one record per call. Uses (does
//! not copy) GitKB's workspace.write.v1 contract,
//! https://github.com/gitkb/gitkb-core crates/gitkb-cli/src/commands/workspace.rs.
use std::collections::BTreeSet;

use bbt_plugin_sdk::{Refusal, encoded};
use serde_json::{Map, Value, json};

use crate::native::{
    digest, native, native_statuses, number, portable, portable_to_native, random_hex, shell_words,
    slug_path, spaced_json, yaml_key,
};
use crate::store::{Catalog, GitKb, PAGE_BYTES, PER_DOC_MS, RESERVE, Summary, reference, truthy};

const MAX_PAGE: u64 = 128;
const MAX_EDGES: usize = 512;

fn invalid(message: &str) -> Refusal {
    Refusal::new("invalid_arguments", message.to_owned())
}

fn required<'a>(args: &'a Value, key: &str) -> Result<&'a Value, Refusal> {
    args.get(key)
        .ok_or_else(|| invalid(&format!("missing argument {key}")))
}

fn string<'a>(value: &'a Value, what: &str) -> Result<&'a str, Refusal> {
    value
        .as_str()
        .ok_or_else(|| invalid(&format!("{what} must be a string")))
}

/// A page item's size, accumulated against the page byte budget.
struct Budget(usize, usize);

impl Budget {
    fn admit(&mut self, item: &Value, empty: bool) -> bool {
        let size = encoded(item).len() + 1;
        if self.0 + size > self.1 && !empty {
            return false;
        }
        self.0 += size;
        true
    }
}

impl GitKb {
    pub fn check_ref(&mut self, reference: &Value) -> Result<Value, Refusal> {
        if !reference.is_object() {
            return Err(Refusal::new("invalid_ref", "ref must be an object"));
        }
        let record = self.read(&reference["slug"], None)?;
        if ["id", "slug", "path", "digest"]
            .iter()
            .any(|k| record[*k] != reference[*k])
        {
            return Err(Refusal::new(
                "stale_ref",
                "record or source changed; resolve again",
            ));
        }
        if !reference["external_id"].is_null() && reference["external_id"] != record["external_id"]
        {
            return Err(Refusal::new("stale_ref", "native identity changed"));
        }
        Ok(record)
    }

    pub fn resolve(&mut self, args: &Value) -> Result<Value, Refusal> {
        let locator = required(args, "locator")?
            .as_str()
            .filter(|l| !l.is_empty() && l.chars().count() <= 4096)
            .ok_or_else(|| {
                Refusal::new(
                    "invalid_locator",
                    "locator must be a bounded nonempty string",
                )
            })?
            .to_owned();
        let records = if let Some(digits) = locator.strip_prefix('#') {
            if digits.is_empty()
                || digits.starts_with('0')
                || !digits.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(Refusal::new(
                    "invalid_number",
                    "expected # followed by a positive number",
                ));
            }
            let target: Value = digits.parse::<u64>().map_or(Value::Null, Value::from);
            let catalog = self.catalog(&[])?;
            let mut candidates: BTreeSet<String> = catalog
                .rows
                .keys()
                .filter(|s| number(None, s).is_ok_and(|n| n == target))
                .cloned()
                .collect();
            // Explicit native numbers take precedence within each record.
            let rows = self.cli(&["list", "--where", &format!("number = {digits}"), "--json"])?;
            for row in rows.as_array().into_iter().flatten() {
                if let Some(slug) = row["slug"].as_str() {
                    candidates.insert(slug.into());
                }
            }
            let mut records = Vec::new();
            for slug in candidates {
                let record = self.read(&slug.into(), None)?;
                if record["number"] == target && !target.is_null() {
                    records.push(record);
                }
            }
            records
        } else if let Ok(identifier) = native(&locator) {
            let catalog = self.catalog(&[])?;
            let mut records = Vec::new();
            for slug in &catalog.order {
                if catalog.rows[slug]["id"] == identifier {
                    records.push(self.read(&slug.as_str().into(), None)?);
                }
            }
            records
        } else {
            let slug = slug_path(&locator.clone().into())?;
            match self.read(&slug.clone().into(), None) {
                Ok(record) => vec![record],
                Err(refusal) if refusal.code == "not_found" => {
                    let catalog = self.catalog(&[])?;
                    let mut records = Vec::new();
                    for candidate in &catalog.order {
                        if candidate.rsplit('/').next() == Some(slug.as_str()) {
                            records.push(self.read(&candidate.as_str().into(), None)?);
                        }
                    }
                    records
                }
                Err(refusal) => return Err(refusal),
            }
        };
        match records.as_slice() {
            [] => Err(Refusal::new(
                "not_found",
                "no document matches this root-scoped locator",
            )),
            [one] => Ok(reference(one)),
            many => Err(
                Refusal::new("ambiguous_locator", "use the full GitKB slug or native ID").with(
                    "candidates",
                    Value::Array(many.iter().map(reference).collect()),
                ),
            ),
        }
    }

    /// `(offset, limit, cursor state)` for a key/snapshot-bound page.
    fn position(
        &self,
        args: &Value,
        key: &str,
        snapshot: &str,
    ) -> Result<(usize, usize, Option<Value>), Refusal> {
        let limit = match args.get("limit") {
            None | Some(Value::Null) => 32,
            Some(value) => value
                .as_u64()
                .filter(|n| (1..=MAX_PAGE).contains(n))
                .ok_or_else(|| Refusal::new("invalid_limit", "limit must be from 1 through 128"))?,
        } as usize;
        let cursor = args.get("cursor").unwrap_or(&Value::Null);
        if !truthy(cursor) {
            return Ok((0, limit, None));
        }
        let state = cursor
            .as_str()
            .and_then(|token| self.cursors.get(token))
            .filter(|old| old["key"] == key && old["snapshot"] == snapshot)
            .ok_or_else(|| {
                Refusal::new(
                    "stale_cursor",
                    "cursor is expired, belongs to another query/root, or the source changed",
                )
            })?
            .clone();
        let offset = state["offset"].as_u64().unwrap_or(0) as usize;
        Ok((offset, limit, Some(state)))
    }

    /// How many records to read next: what the page still needs, bounded by
    /// the time left before the reply reserve.
    fn chunk(&self, needed: usize, left: usize) -> usize {
        let spare = self.ctx.remaining().saturating_sub(RESERVE).as_millis() / PER_DOC_MS;
        needed
            .clamp(8, 64)
            .min(spare.max(1) as usize)
            .min(left)
            .max(1)
    }

    /// Fill a page from `slugs[offset..]` with accepted items, until `limit`,
    /// the byte budget or the time reserve. Always examines at least one record.
    fn fill(
        &mut self,
        catalog: &Catalog,
        slugs: &[String],
        offset: &mut usize,
        limit: usize,
        accept: impl Fn(&Summary) -> Option<Value>,
    ) -> Result<Vec<Value>, Refusal> {
        let mut items = Vec::new();
        let mut budget = Budget(0, PAGE_BYTES);
        let mut examined = 0;
        while *offset < slugs.len() && items.len() < limit {
            if examined > 0 && self.ctx.remaining() < RESERVE {
                break;
            }
            let n = self.chunk(limit - items.len(), slugs.len() - *offset);
            let summaries = self.summaries(catalog, &slugs[*offset..*offset + n])?;
            if summaries.is_empty() {
                break;
            }
            for summary in summaries {
                if let Some(item) = summary.as_ref().and_then(&accept) {
                    if !budget.admit(&item, items.is_empty()) {
                        return Ok(items);
                    }
                    items.push(item);
                }
                *offset += 1;
                examined += 1;
                if items.len() >= limit {
                    break;
                }
            }
        }
        Ok(items)
    }

    pub fn listing(&mut self, args: &Value, search: bool) -> Result<Value, Refusal> {
        let mut filters = match args.get("filter") {
            None | Some(Value::Null) => Map::new(),
            Some(Value::Object(map)) => map.clone(),
            Some(_) => return Err(invalid("filter must be an object")),
        };
        let mut words = Vec::new();
        if search {
            for word in shell_words(string(required(args, "query")?, "query")?)? {
                match word.split_once(':') {
                    Some((name, value)) if matches!(name, "type" | "status" | "since") => {
                        filters.insert(name.into(), value.into());
                    }
                    _ => words.push(word),
                }
            }
        }
        let wanted = |field: &str| -> Result<Option<String>, Refusal> {
            match filters.get(field) {
                Some(v) if truthy(v) => Ok(Some(string(v, field)?.to_owned())),
                _ => Ok(None),
            }
        };
        let mut flags = Vec::new();
        for field in ["type", "since"] {
            if let Some(value) = wanted(field)? {
                flags.extend([format!("--{field}"), value]);
            }
        }
        for field in ["assignee", "swarm"] {
            if let Some(value) = wanted(field)? {
                flags.extend(["--where".into(), format!("{field} = {value}")]);
            }
        }
        let catalog = if let Some(status) = wanted("status")? {
            let mut native_flags = flags.clone();
            native_flags.extend(["--status".into(), native_statuses(&status).join(",")]);
            let mut catalog = self.catalog(&native_flags)?;
            let mut namespaced = flags.clone();
            namespaced.extend(["--where".into(), format!("bboxy_status = {status}")]);
            catalog.merge(self.catalog(&namespaced)?);
            catalog
        } else {
            self.catalog(&flags)?
        };
        if !words.is_empty() {
            return self.search_page(&catalog, &words, &filters, args);
        }
        let slugs: Vec<String> = catalog.rows.keys().cloned().collect();
        let key = encoded(&json!([
            if search { "search" } else { "list" },
            filters,
            words
        ]));
        let (mut offset, limit, _) = self.position(args, &key, &catalog.snapshot)?;
        let items = self.fill(&catalog, &slugs, &mut offset, limit, |s| {
            s.matches(&filters).then(|| {
                if search {
                    search_item(s, None)
                } else {
                    s.reference.clone()
                }
            })
        })?;
        let next = if offset < slugs.len() {
            self.cursors
                .token(json!({"key": key, "snapshot": catalog.snapshot, "offset": offset}))
                .into()
        } else {
            Value::Null
        };
        Ok(json!({"items": items, "next": next}))
    }

    /// Native strict FTS first, then a paged supplement of workspace sources
    /// that native FTS does not index.
    fn search_page(
        &mut self,
        catalog: &Catalog,
        words: &[String],
        filters: &Map<String, Value>,
        args: &Value,
    ) -> Result<Value, Refusal> {
        let key = encoded(&json!(["search", filters, words]));
        let (mut offset, limit, state) = self.position(args, &key, &catalog.snapshot)?;
        let state = state.unwrap_or_default();
        let mut native_phase = state["phase"].as_str().unwrap_or("native") == "native";
        let mut seen: BTreeSet<String> = state["seen"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|s| s.as_str().map(str::to_owned))
            .collect();
        let lowered: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
        let text_matches = |summary: &Summary| {
            summary
                .haystack
                .as_ref()
                .is_none_or(|text| lowered.iter().all(|w| text.contains(w.as_str())))
        };
        let mut items = Vec::new();
        let mut budget = Budget(0, PAGE_BYTES);
        let mut examined = 0;
        let mut done = false;
        'page: while items.len() < limit {
            if examined > 0 && self.ctx.remaining() < RESERVE {
                break;
            }
            if native_phase {
                let want = (limit - items.len()).to_string();
                let from = offset.to_string();
                let query = words.join(" ");
                let mut cli = vec![
                    "search",
                    &query,
                    "--all-terms",
                    "--limit",
                    &want,
                    "--offset",
                    &from,
                ];
                let kind = filters
                    .get("type")
                    .filter(|v| truthy(v))
                    .and_then(Value::as_str);
                if let Some(kind) = kind {
                    cli.extend(["--type", kind]);
                }
                cli.push("--json");
                let result = self.cli(&cli)?;
                let hits = result["results"].as_array().cloned().unwrap_or_default();
                if hits.is_empty() {
                    native_phase = false;
                    offset = 0;
                    continue;
                }
                let wanted: Vec<String> = hits
                    .iter()
                    .filter_map(|h| h["slug"].as_str())
                    .filter(|s| catalog.rows.contains_key(*s))
                    .map(str::to_owned)
                    .collect();
                let summaries = self.summaries(catalog, &wanted)?;
                let mut ready = wanted.iter().zip(summaries);
                let mut processed = 0;
                for hit in &hits {
                    let slug = hit["slug"].as_str().unwrap_or("");
                    if catalog.rows.contains_key(slug) {
                        let Some((_, summary)) = ready.next() else {
                            break;
                        };
                        if catalog.workspace(slug) {
                            seen.insert(slug.into());
                        }
                        if let Some(summary) = summary
                            && summary.matches(filters)
                            && text_matches(&summary)
                        {
                            let item = search_item(&summary, Some(hit));
                            if !budget.admit(&item, items.is_empty()) {
                                break 'page;
                            }
                            items.push(item);
                        }
                    }
                    offset += 1;
                    processed += 1;
                    examined += 1;
                    if items.len() >= limit {
                        break;
                    }
                }
                if processed == hits.len() && result["pagination"]["next_offset"].is_null() {
                    native_phase = false;
                    offset = 0;
                }
            } else {
                let candidates: Vec<String> = catalog
                    .rows
                    .keys()
                    .filter(|s| catalog.workspace(s) && !seen.contains(*s))
                    .cloned()
                    .collect();
                if offset >= candidates.len() {
                    done = true;
                    break;
                }
                let n = self.chunk(limit - items.len(), candidates.len() - offset);
                let summaries = self.summaries(catalog, &candidates[offset..offset + n])?;
                if summaries.is_empty() {
                    break;
                }
                for summary in summaries {
                    if let Some(summary) = summary
                        && summary.matches(filters)
                        && text_matches(&summary)
                    {
                        let item = search_item(&summary, None);
                        if !budget.admit(&item, items.is_empty()) {
                            break 'page;
                        }
                        items.push(item);
                    }
                    offset += 1;
                    examined += 1;
                    if items.len() >= limit {
                        break;
                    }
                }
                if offset
                    >= catalog
                        .rows
                        .keys()
                        .filter(|s| catalog.workspace(s) && !seen.contains(*s))
                        .count()
                {
                    done = true;
                    break;
                }
            }
        }
        let next = if done {
            Value::Null
        } else {
            self.cursors
                .token(
                    json!({"phase": if native_phase { "native" } else { "workspace" },
                    "offset": offset, "key": key, "snapshot": catalog.snapshot, "seen": seen}),
                )
                .into()
        };
        Ok(json!({"items": items, "next": next}))
    }

    fn edges(&mut self, slugs: &[String], depth: u64) -> Result<(Vec<Value>, Vec<Value>), Refusal> {
        if slugs.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let depth = depth.to_string();
        let mut args = vec!["graph", "--json", "--depth", &depth, "--"];
        args.extend(slugs.iter().map(String::as_str));
        let result = self.cli(&args)?;
        let mut edges = Vec::new();
        for edge in result["edges"].as_array().into_iter().flatten() {
            // Document edges only: GitKB also reports code-symbol targets
            // (`code:path::symbol`), which have no portable record identity.
            let (Ok(from), Ok(to)) = (
                portable(edge["from"].as_str().unwrap_or("")),
                portable(edge["to"].as_str().unwrap_or("")),
            ) else {
                continue;
            };
            edges.push(json!({
                "from": from,
                "to": to,
                "rel": edge.get("rel_type").cloned().unwrap_or_else(|| "references".into()),
                "weight": edge["weight"].as_f64().unwrap_or(1.0),
            }));
        }
        let nodes = result["nodes"].as_array().cloned().unwrap_or_default();
        Ok((edges, nodes))
    }

    pub fn links(&mut self, args: &Value) -> Result<Value, Refusal> {
        let record = self.check_ref(required(args, "ref")?)?;
        let slug = record["slug"].as_str().unwrap_or("").to_owned();
        let (edges, _) = self.edges(&[slug], 1)?;
        let id = &record["id"];
        let outgoing: Vec<&Value> = edges.iter().filter(|e| &e["from"] == id).collect();
        let incoming: Vec<&Value> = edges.iter().filter(|e| &e["to"] == id).collect();
        Ok(json!({"outgoing": outgoing, "incoming": incoming}))
    }

    pub fn graph(&mut self, args: &Value) -> Result<Value, Refusal> {
        let scope = required(args, "scope")?;
        if !scope.is_object() {
            return Err(invalid("scope must be an object"));
        }
        let graph = scope["graph"].as_str().unwrap_or("");
        if !matches!(graph, "markdown" | "task" | "artifact") {
            return Err(Refusal::new(
                "unsupported_graph",
                "execution, channel and messaging graphs belong to the host",
            ));
        }
        let catalog = self.catalog(&[])?;
        let typed = |slug: &String, kind: &str| catalog.rows[slug]["type"] == kind;
        let mut selected: BTreeSet<String> = catalog
            .rows
            .keys()
            .filter(|s| graph == "markdown" || typed(s, graph))
            .cloned()
            .collect();
        let kind = scope["kind"].as_str().unwrap_or("");
        match kind {
            "neighborhood" => {
                let depth = scope["depth"].as_u64().filter(|d| *d <= 8).ok_or_else(|| {
                    Refusal::new("invalid_depth", "neighborhood depth must be 0 through 8")
                })?;
                let record = self.check_ref(&scope["ref"])?;
                let slug = record["slug"].as_str().unwrap_or("").to_owned();
                let (_, nodes) = self.edges(&[slug], depth)?;
                let near: BTreeSet<&str> =
                    nodes.iter().filter_map(|n| n["slug"].as_str()).collect();
                selected.retain(|s| near.contains(s.as_str()));
            }
            "type" => {
                let kind = scope["type"].as_str().unwrap_or("");
                selected.retain(|s| typed(s, kind));
            }
            "swarm" => {
                let swarm = string(&scope["swarm"], "scope.swarm")?;
                let rows = self.cli(&["list", "--where", &format!("swarm = {swarm}"), "--json"])?;
                let member: BTreeSet<&str> = rows
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|r| r["slug"].as_str())
                    .collect();
                selected.retain(|s| member.contains(s.as_str()));
            }
            "all" => {}
            _ => return Err(Refusal::new("invalid_scope", "unknown graph scope")),
        }
        let slugs: Vec<String> = selected.into_iter().collect();
        let key = encoded(&json!(["graph", scope]));
        let (mut offset, limit, _) = self.position(args, &key, &catalog.snapshot)?;
        let swarm_filter: Map<String, Value> = if kind == "swarm" {
            [("swarm".to_owned(), scope["swarm"].clone())]
                .into_iter()
                .collect()
        } else {
            Map::new()
        };
        // Nodes carry the frontmatter an exact read returns: committed-only
        // records build it from one batched read, workspace records read
        // exactly. Half the result budget stays for up to 512 edges.
        let mut nodes = Vec::new();
        let mut members = Vec::new();
        let mut budget = Budget(0, PAGE_BYTES / 2);
        let mut examined = 0;
        'page: while offset < slugs.len() && nodes.len() < limit {
            // Twice the reserve: the native edge read still follows.
            if examined > 0 && self.ctx.remaining() < RESERVE * 2 {
                break;
            }
            let n = self.chunk(limit - nodes.len(), slugs.len() - offset);
            let found = self.nodes(&catalog, &slugs[offset..offset + n], RESERVE * 2)?;
            if found.is_empty() {
                break;
            }
            for found in found {
                if let Some((summary, frontmatter)) = found
                    && summary.matches(&swarm_filter)
                {
                    let node = json!({"id": summary.reference["id"], "ref": summary.reference,
                        "title": frontmatter["title"], "type": frontmatter["type"],
                        "frontmatter": frontmatter});
                    if !budget.admit(&node, nodes.is_empty()) {
                        break 'page;
                    }
                    members.push(slugs[offset].clone());
                    nodes.push(node);
                }
                offset += 1;
                examined += 1;
                if nodes.len() >= limit {
                    break;
                }
            }
        }
        let (edges, _) = self.edges(&members, 1)?;
        let ids: BTreeSet<&str> = nodes.iter().filter_map(|n| n["id"].as_str()).collect();
        let outgoing: Vec<&Value> = edges
            .iter()
            .filter(|e| e["from"].as_str().is_some_and(|id| ids.contains(id)))
            .collect();
        let next = if offset < slugs.len() {
            self.cursors
                .token(json!({"key": key, "snapshot": catalog.snapshot, "offset": offset}))
                .into()
        } else {
            Value::Null
        };
        Ok(
            json!({"nodes": nodes, "edges": &outgoing[..outgoing.len().min(MAX_EDGES)],
            "next": next, "edges_truncated": outgoing.len() > MAX_EDGES}),
        )
    }

    pub fn write(&mut self, args: &Value) -> Result<Value, Refusal> {
        let mut signed = args.get("signed_record").cloned().unwrap_or(Value::Null);
        if !truthy(&signed) && self.fixture {
            signed = "disposable unsigned conformance fixture".into();
        }
        let Some(signed) = signed.as_str().filter(|s| !s.is_empty()).map(str::to_owned) else {
            return Err(Refusal::new(
                "signed_writer_unavailable",
                "write requires a host-admitted signed_record; use the host knowledge writer",
            ));
        };
        let requested = required(args, "frontmatter")?
            .as_object()
            .ok_or_else(|| invalid("frontmatter must be an object"))?
            .clone();
        let body = required(args, "body")?;
        let mut fm = requested.clone();
        if !body.is_string()
            || !fm.get("title").is_some_and(truthy)
            || !fm.get("type").is_some_and(truthy)
        {
            return Err(Refusal::new(
                "invalid_record",
                "title, type and Markdown body are required",
            ));
        }
        let body = body.as_str().unwrap_or("").to_owned();
        let reference = args.get("ref").filter(|r| truthy(r));
        let mut create_workspace = false;
        let slug;
        let generation: Value;
        if let Some(reference) = reference {
            if !args.get("path").unwrap_or(&Value::Null).is_null() {
                return Err(Refusal::new(
                    "invalid_write",
                    "updates use ref with no path",
                ));
            }
            let old = self.check_ref(reference)?;
            if args.get("etag").unwrap_or(&Value::Null) != &old["etag"] {
                return Err(Refusal::new(
                    "etag_mismatch",
                    "update requires the current source etag",
                ));
            }
            slug = old["slug"].as_str().unwrap_or("").to_owned();
            let id = fm.get("id").unwrap_or(&old["id"]);
            if id != &old["id"] && id != &old["external_id"] {
                return Err(Refusal::new(
                    "immutable_id",
                    "record identity cannot change",
                ));
            }
            if old["gitkb"]["source"]["kind"] == "committed" {
                // Native CAS binds the absent workspace generation to the
                // current committed revision. Materialize+write in one native
                // transaction, rather than checkout/read/write CLI sequences.
                generation = old["gitkb"]["workspace_generation"].clone();
                create_workspace = true;
            } else {
                generation = old["etag"].clone();
            }
            fm.insert("id".into(), old["external_id"].clone());
        } else {
            let path = args
                .get("path")
                .and_then(Value::as_str)
                .filter(|p| p.ends_with(".md"))
                .ok_or_else(|| {
                    Refusal::new("path_required", "create requires a relative .md path")
                })?;
            slug = slug_path(&path.into())?;
            if self.catalog(&[])?.rows.contains_key(&slug) {
                return Err(Refusal::new(
                    "path_exists",
                    "a GitKB document already occupies this slug",
                ));
            }
            generation = Value::Null;
            if let Some(id) = fm.get("id").filter(|v| truthy(v)) {
                let id = native(string(id, "frontmatter.id")?)?;
                fm.insert("id".into(), id.into());
            }
        }
        if fm.get("slug").is_some_and(|s| s != slug.as_str()) {
            return Err(Refusal::new(
                "slug_mismatch",
                "frontmatter.slug must match the addressed path",
            ));
        }
        fm.insert("slug".into(), slug.clone().into());
        if fm["type"] == "task" {
            let status = fm.get("status").cloned().unwrap_or_else(|| "open".into());
            let native_status = status
                .as_str()
                .and_then(portable_to_native)
                .map_or_else(|| status.clone(), Value::from);
            fm.insert("bboxy_status".into(), status);
            fm.insert("status".into(), native_status);
        }
        fm.insert("bboxy_signed_record".into(), signed.clone().into());
        // Flow-style JSON values are YAML 1.2. Plain top-level keys also let
        // GitKB's exact ID/slug guard see the supplied identity before parsing.
        let header: Vec<String> = fm
            .iter()
            .map(|(k, v)| format!("{}: {}", yaml_key(k), spaced_json(v)))
            .collect();
        let raw = format!("---\n{}\n---\n{body}", header.join("\n"));
        let origin = format!("bboxy-{}", random_hex(12));
        let mut cli = vec![
            "workspace",
            "write",
            &slug,
            "--stdin",
            "--origin-id",
            &origin,
        ];
        let generation_text = generation.as_str().unwrap_or("").to_owned();
        if truthy(&generation) {
            cli.extend(["--if-generation", &generation_text]);
        } else {
            cli.push("--create-if-missing");
        }
        if create_workspace {
            cli.push("--create-if-missing");
        }
        cli.push("--json");
        self.cli_stdin(&cli, &raw)?;
        self.forget(&slug);
        let record = self.read(&slug.clone().into(), Some("workspace"))?;
        let lossy = record["body"] != body
            || requested
                .iter()
                .any(|(k, v)| record["frontmatter"].get(k) != Some(v));
        Ok(json!({"record": record, "lossy": lossy, "signed_record": signed}))
    }

    pub fn changes(&mut self, args: &Value) -> Result<Value, Refusal> {
        let catalog = self.catalog(&[])?;
        let manifest: Map<String, Value> = catalog
            .rows
            .iter()
            .map(|(slug, row)| {
                (
                    slug.clone(),
                    json!({"id": row["id"], "state": digest(&encoded(row))}),
                )
            })
            .collect();
        let cursor = string(required(args, "cursor")?, "cursor")?;
        let (_, limit, _) = self.position(
            &json!({"limit": args.get("limit").cloned().unwrap_or(Value::Null)}),
            "",
            "",
        )?;
        let baseline = || json!({"rows": manifest, "snapshot": catalog.snapshot, "changes": true});
        if cursor.is_empty() || cursor == "0" {
            let token = self.cursors.token(baseline());
            return Ok(json!({"changed": [], "next": token}));
        }
        let old = self
            .cursors
            .get(cursor)
            .filter(|old| old["changes"] == true)
            .cloned()
            .ok_or_else(|| {
                Refusal::new(
                    "stale_cursor",
                    "unknown change cursor; start a new baseline with cursor 0",
                )
            })?;
        let empty = Map::new();
        let old_rows = old["rows"].as_object().unwrap_or(&empty);
        let pending: Vec<String> = if let Some(pending) = old["pending"].as_array() {
            if old["snapshot"] != catalog.snapshot.as_str() {
                return Err(Refusal::new(
                    "stale_cursor",
                    "source changed while paging changes; resync the root",
                ));
            }
            pending
                .iter()
                .filter_map(|s| s.as_str().map(str::to_owned))
                .collect()
        } else {
            manifest
                .keys()
                .chain(old_rows.keys())
                .filter(|s| manifest.get(*s) != old_rows.get(*s))
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect()
        };
        let tombstone = |slug: &str| -> Result<Value, Refusal> {
            let id = old_rows
                .get(slug)
                .or_else(|| manifest.get(slug))
                .and_then(|r| r["id"].as_str())
                .unwrap_or("");
            Ok(json!({"id": portable(id)?, "external_id": id, "slug": slug,
                "path": format!("{slug}.md"), "number": number(None, slug)?,
                "digest": digest(&format!("deleted:{id}"))}))
        };
        let mut changed = Vec::new();
        let mut processed = 0;
        while processed < pending.len() && changed.len() < limit {
            if processed > 0 && self.ctx.remaining() < RESERVE {
                break;
            }
            let n = self.chunk(limit - changed.len(), pending.len() - processed);
            let chunk = &pending[processed..processed + n];
            let live: Vec<String> = chunk
                .iter()
                .filter(|s| catalog.rows.contains_key(*s))
                .cloned()
                .collect();
            let summaries = self.summaries(&catalog, &live)?;
            let mut ready = summaries.into_iter();
            for slug in chunk {
                if catalog.rows.contains_key(slug) {
                    let Some(summary) = ready.next() else {
                        break;
                    };
                    changed.push(match summary {
                        Some(summary) => summary.reference,
                        None => tombstone(slug)?,
                    });
                } else {
                    changed.push(tombstone(slug)?);
                }
                processed += 1;
            }
        }
        let mut following = baseline();
        if processed < pending.len() {
            following["pending"] = json!(pending[processed..]);
            following["rows"] = old["rows"].clone();
        }
        Ok(json!({"changed": changed, "next": self.cursors.token(following)}))
    }

    pub fn commit(&mut self, args: &Value) -> Result<Value, Refusal> {
        let refs = args
            .get("refs")
            .and_then(Value::as_array)
            .filter(|r| (1..=32).contains(&r.len()))
            .ok_or_else(|| {
                Refusal::new(
                    "invalid_commit",
                    "commit requires from 1 through 32 explicit references",
                )
            })?
            .clone();
        let message = args
            .get("message")
            .and_then(Value::as_str)
            .filter(|m| !m.trim().is_empty() && m.chars().count() <= 4096)
            .ok_or_else(|| {
                Refusal::new(
                    "invalid_commit",
                    "commit requires a nonempty message of at most 4096 characters",
                )
            })?
            .to_owned();
        let mut slugs = Vec::new();
        for reference in &refs {
            let record = self.check_ref(reference)?;
            slugs.push(record["slug"].as_str().unwrap_or("").to_owned());
        }
        if slugs.iter().collect::<BTreeSet<_>>().len() != slugs.len() {
            return Err(Refusal::new(
                "invalid_commit",
                "duplicate document references",
            ));
        }
        let mut cli = vec!["commit", "--json", "-m", &message, "--"];
        cli.extend(slugs.iter().map(String::as_str));
        let result = self.cli(&cli)?;
        if result["changes"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|c| !slugs.iter().any(|s| c["slug"] == s.as_str()))
        {
            return Err(Refusal::new(
                "unexpected_commit_scope",
                "GitKB returned an unexpected document; inspect the native commit",
            ));
        }
        self.call_records.clear();
        for slug in &slugs {
            self.forget(slug);
        }
        Ok(json!({"commit": result["commit_id"], "committed": refs}))
    }
}

fn search_item(summary: &Summary, hit: Option<&Value>) -> Value {
    let snippet = hit
        .and_then(|h| h.get("snippet"))
        .cloned()
        .unwrap_or_else(|| summary.snippet.clone().into());
    let score = hit.and_then(|h| h["score"].as_f64()).unwrap_or(1.0);
    json!({"ref": summary.reference, "title": summary.title, "type": summary.kind,
        "number": summary.reference["number"], "snippet": snippet, "score": score})
}
