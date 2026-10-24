//! Knowledge contract inside the IO owner. This is the in-process default;
//! transport, registry/Settings and foreign providers are later checkpoints.
mod bboxy;
pub use bboxy::BboxySource;
pub use bbt_plugin::data_source;

use crate::{Ulid, messaging::wire::IssuerKind, provenance::author::Author};
use data_source as d;
use ed25519_dalek::SigningKey;

/// An already-authorized host mutation and its actual signer. This is never
/// deserialized from a plugin call. Task acceptance remains a host decision.
pub struct WriteContext<'a> {
    pub(crate) author: Author<'a>,
}
impl<'a> WriteContext<'a> {
    /// Bind the host's actual seat generation and key after policy admission.
    pub fn seat(seat: Ulid, generation: Option<u64>, key: &'a SigningKey) -> Self {
        Self {
            author: Author::seat(seat, generation, key),
        }
    }
    /// Bind an admitted operator mutation to the actual device/local issuer.
    pub fn operator(device: Ulid, issuer: IssuerKind, key: &'a SigningKey) -> Self {
        Self {
            author: Author::operator(device, issuer, key),
        }
    }
}

/// The eight data_source methods, using the same portable shapes as stdio
/// will use. Calls run on IO; no method may block the UI thread. Read methods
/// never acknowledge bus delivery or operator read state.
pub trait DataSource {
    /// Resolve one unambiguous record; candidates accompany ambiguity errors.
    fn resolve(&self, request: d::ResolveRequest) -> d::Result<d::RecordRef>;
    /// Fetch frontmatter, unrendered Markdown and confined resource links.
    fn fetch(&self, request: d::FetchRequest) -> d::Result<d::Record>;
    /// Rank plain text/filters, frontmatter first, in a query-bound page.
    fn search(&self, request: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>>;
    /// Page references by explicit frontmatter filters.
    fn list(&self, request: d::ListRequest) -> d::Result<d::Page<d::RecordRef>>;
    /// Return the existing weighted links in both directions.
    fn links(&self, request: d::FetchRequest) -> d::Result<d::Links>;
    /// Page a document-owned graph, never an execution/messaging graph.
    fn graph(&self, request: d::GraphRequest) -> d::Result<d::GraphPage>;
    /// Apply an admitted mutation through the Index's signed, checked writer.
    fn write(
        &self,
        request: d::WriteRequest,
        context: &WriteContext<'_>,
    ) -> d::Result<d::WriteResult>;
    /// Poll the existing activity log with no new ledger or consumer.
    fn changes_since(&self, request: d::ChangesRequest) -> d::Result<d::Changes>;
}

/// Existing document-facing host projections, expressed only through the
/// provider port. Callers derive visibility from an admitted actor, not JSON.
pub mod views {
    use super::{DataSource, data_source as d};
    use crate::registry::SeatRecord;
    use serde_json::{Value, json};
    use std::collections::{BTreeMap, BTreeSet};

    pub(crate) struct Catalogue {
        pub nodes: BTreeMap<String, d::Node>,
        pub edges: Vec<d::Edge>,
        pub truncated: bool,
    }
    pub(crate) fn catalogue(
        source: &dyn DataSource,
        graph: d::DocumentGraph,
    ) -> d::Result<Catalogue> {
        bbt_rt::assert_not_ui_thread();
        let mut result = Catalogue {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            truncated: false,
        };
        let mut cursor = None;
        let mut seen = BTreeSet::new();
        loop {
            let page = source.graph(d::GraphRequest {
                scope: d::Scope::All { graph },
                cursor,
                limit: Some(d::MAX_PAGE),
            })?;
            if page.nodes.len() > d::MAX_PAGE
                || page.edges.len() > 512
                || serde_json::to_vec(&page)
                    .map_err(|e| d::CommandError::new("invalid_provider_result", e.to_string()))?
                    .len()
                    > d::MAX_PAGE_BYTES
            {
                return Err(d::CommandError::new(
                    "invalid_provider_result",
                    "graph page exceeds contract bounds",
                ));
            }
            for node in page.nodes {
                if result.nodes.get(&node.id).is_some_and(|old| old != &node) {
                    return Err(d::CommandError::new(
                        "ambiguous_reference",
                        "provider changed a node inside one graph read",
                    ));
                }
                result.nodes.insert(node.id.clone(), node);
            }
            result.edges.extend(page.edges);
            result.truncated |= page.edges_truncated;
            cursor = page.next;
            let Some(next) = &cursor else { break };
            if !seen.insert(next.clone()) {
                return Err(d::CommandError::new(
                    "invalid_cursor",
                    "provider repeated a graph cursor",
                ));
            }
        }
        Ok(result)
    }
    /// The current assignment overrides stale historical registry task lists.
    pub fn owned(metadata: &Value, id: &str, seat: &SeatRecord) -> bool {
        if let Some(assignee) = metadata.get("assignee") {
            return assignee
                .as_str()
                .is_some_and(|id| id == seat.id.to_string() || id == format!("seat:{}", seat.id));
        }
        seat.tasks.iter().any(|task| task.to_string() == id)
    }
    /// Preserve the legacy bounded MCP graph projection and its actor filter.
    /// Execution graphs remain outside this document-provider adapter.
    pub fn graph(
        source: &dyn DataSource,
        seat: &SeatRecord,
        family: d::DocumentGraph,
        after: Option<&str>,
        limit: usize,
        own_only: bool,
    ) -> d::Result<Value> {
        if !(1..=d::MAX_PAGE).contains(&limit) || after.is_some_and(|s| s.len() > 256) {
            return Err(d::CommandError::new(
                "invalid_page",
                "invalid graph limit or cursor",
            ));
        }
        let all = catalogue(source, family)?;
        let own = seat.id.to_string();
        let seat_ref = format!("seat:{}", seat.id);
        let eligible: Vec<_> = all
            .nodes
            .values()
            .filter(|node| {
                (!own_only
                    || if node.reference.is_some() {
                        owned(&node.frontmatter, &node.id, seat)
                    } else {
                        node.id == own || node.id == seat_ref
                    })
                    && after.is_none_or(|after| node.id.as_str() > after)
            })
            .collect();
        let more = eligible.len() > limit;
        let selected: Vec<_> = eligible.into_iter().take(limit).collect();
        let ids: BTreeSet<_> = selected.iter().map(|node| node.id.as_str()).collect();
        let nodes:Vec<_>=selected.iter().map(|node|json!({"id":node.id,"path":node.reference.as_ref().map(|r|&r.path),"title":node.title.as_deref().map(|s|crate::mcp::tools::clip(s,512)),"type":node.record_type})).collect();
        let matching: Vec<_> = all
            .edges
            .iter()
            .filter(|edge| ids.contains(edge.from.as_str()) && ids.contains(edge.to.as_str()))
            .collect();
        let edges:Vec<_>=matching.iter().take(512).map(|edge|json!({"from":edge.from,"to":edge.to,"relation":edge.rel,"weight":edge.weight})).collect();
        let family = match family {
            d::DocumentGraph::Markdown => "markdown",
            d::DocumentGraph::Task => "task",
            d::DocumentGraph::Artifact => "artifact",
        };
        Ok(
            json!({"graph":family,"nodes":nodes,"edges":edges,"next":if more {selected.last().map(|n|&n.id)}else{None},"edges_truncated":all.truncated||matching.len()>512}),
        )
    }
    /// Own-task briefs retain the current limits and explicit truncation flags.
    pub fn orient(source: &dyn DataSource, seat: &SeatRecord) -> d::Result<Value> {
        let all = catalogue(source, d::DocumentGraph::Markdown)?;
        let mut selected: Vec<_> = all
            .nodes
            .values()
            .filter(|node| node.reference.is_some() && owned(&node.frontmatter, &node.id, seat))
            .collect();
        selected.sort_by_key(|node| &node.reference.as_ref().expect("selected records").path);
        let mut items = Vec::new();
        for node in selected.iter().take(8) {
            let record = source.fetch(d::FetchRequest {
                reference: node.reference.clone().expect("selected records"),
            })?;
            let verify = &record.frontmatter["verify"];
            items.push(json!({"id":record.reference.id,"path":record.reference.path,"title":record.frontmatter["title"].as_str().map(|s|crate::mcp::tools::clip(s,512)),"status":record.frontmatter["status"],
                "verify":if verify.to_string().len()<=4096 {verify.clone()}else{json!({"unavailable":"verify exceeds card bound; read the document"})},
                "body":crate::mcp::tools::clip(&record.body,16384),"body_truncated":record.body.len()>16384,"parse_error":record.parse_error}));
        }
        Ok(json!({"items":items,"truncated":selected.len()>8}))
    }
    /// Revalidate cached read rows against the current provider snapshot.
    pub(crate) fn retained(
        source: &dyn DataSource,
        seat: &SeatRecord,
        items: &Value,
        allow_virtual: bool,
    ) -> bool {
        let Ok(all) = catalogue(source, d::DocumentGraph::Markdown) else {
            return false;
        };
        items.as_array().is_some_and(|items| {
            items.iter().all(|item| {
                if item["path"].is_null() {
                    return allow_virtual
                        && (item["id"] == seat.id.to_string()
                            || item["id"] == format!("seat:{}", seat.id));
                }
                item["id"]
                    .as_str()
                    .and_then(|id| all.nodes.get(id))
                    .is_some_and(|node| {
                        node.reference
                            .as_ref()
                            .is_some_and(|reference| item["path"] == reference.path)
                            && owned(&node.frontmatter, &node.id, seat)
                    })
            })
        })
    }
}
