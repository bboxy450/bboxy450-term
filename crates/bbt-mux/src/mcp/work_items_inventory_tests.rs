// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Count provider work, not just individual RPC deadlines, with a large unrelated corpus.
use super::*;
use crate::{
    knowledge::{DataSource, WriteContext},
    mcp::{Door, SeatActor},
    messaging::wire::IssuerKind,
    store::sqlite::SqliteStore,
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Calls {
    lists: Vec<d::Filter>,
    graphs: usize,
    fetches: Vec<String>,
    resolves: usize,
    /// The operation deadline each bounded provider call received.
    deadlines: Vec<Instant>,
}
struct Corpus {
    rows: Vec<d::Record>,
    calls: Mutex<Calls>,
    native: bool,
    list_page_size: usize,
    /// A slow provider: each graph page takes this long, clipped by a deadline.
    delay: Option<Duration>,
}
fn record(n: u128, kind: &str, assignee: Option<Value>) -> d::Record {
    let id = Ulid::from(n).to_string();
    let mut frontmatter = json!({"id":id,"type":kind,"title":format!("Document {n}"),"status":"open","verify":"true","acceptance":["checked"]});
    if let Some(assignee) = assignee {
        frontmatter["assignee"] = assignee;
    }
    d::Record {
        reference: d::RecordRef {
            id,
            external_id: Some(format!("native-{n}")),
            number: None,
            slug: format!("tasks/doc-{n}"),
            path: format!("tasks/doc-{n}.md"),
            digest: format!("digest-{n}"),
            updated_at: None,
        },
        frontmatter,
        body: format!("Intent for document {n}"),
        etag: format!("etag-{n}"),
        resources: vec![],
        parse_error: None,
        relationships: None,
    }
}
impl Corpus {
    fn new(worker: Ulid, peer: Ulid, foreign: Ulid) -> Self {
        let mut rows: Vec<_> = (1..=10_000)
            .map(|n| record(n, "note", Some(json!(format!("seat:{worker}")))))
            .collect();
        rows.extend(
            (10_001..=11_000).map(|n| record(n, "task", Some(json!(format!("seat:{foreign}"))))),
        );
        rows.extend([
            record(20_001, "task", Some(json!(format!("seat:{worker}")))),
            // Legacy bare assignment and absent swarm frontmatter must survive pushdown.
            record(20_002, "task", Some(json!(worker))),
            record(20_003, "task", Some(json!(format!("seat:{peer}")))),
            record(20_004, "task", None),
            record(20_005, "bug", Some(Value::Null)),
            record(20_006, "epic", Some(json!("none"))),
        ]);
        Self {
            rows,
            calls: Mutex::default(),
            native: false,
            list_page_size: 2,
            delay: None,
        }
    }
    fn reset(&self) {
        *self.calls.lock().unwrap() = Calls::default();
    }
}
impl DataSource for Corpus {
    fn import_root(&self) -> d::Result<std::path::PathBuf> {
        if self.native {
            Ok("/private/native-task-fixture/.bboxy".into())
        } else {
            Err(d::CommandError::new(
                "unsupported_operation",
                "external corpus",
            ))
        }
    }
    fn resolve(&self, request: d::ResolveRequest) -> d::Result<d::RecordRef> {
        self.calls.lock().unwrap().resolves += 1;
        self.rows
            .iter()
            .find(|r| {
                r.reference.slug == request.locator
                    || r.reference.path == request.locator
                    || r.reference.id == request.locator
            })
            .map(|r| r.reference.clone())
            .ok_or_else(|| d::CommandError::new("not_found", "fixture locator"))
    }
    fn fetch(&self, request: d::FetchRequest) -> d::Result<d::Record> {
        self.calls
            .lock()
            .unwrap()
            .fetches
            .push(request.reference.id.clone());
        self.rows
            .iter()
            .find(|r| r.reference == request.reference)
            .cloned()
            .ok_or_else(|| d::CommandError::new("stale_ref", "fixture reference"))
    }
    fn list(&self, request: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        assert!(
            request.filter.record_type.is_some(),
            "task type must be pushed down"
        );
        assert!(
            request.filter.assignee.is_some(),
            "ordinary inventory must push assignment"
        );
        assert!(
            request.filter.swarm.is_none(),
            "legacy tasks have no swarm frontmatter"
        );
        self.calls
            .lock()
            .unwrap()
            .lists
            .push(request.filter.clone());
        let rows: Vec<_> = self
            .rows
            .iter()
            .filter(|r| {
                request
                    .filter
                    .record_type
                    .as_ref()
                    .is_none_or(|k| r.frontmatter["type"] == *k)
                    && request
                        .filter
                        .status
                        .as_ref()
                        .is_none_or(|s| r.frontmatter["status"] == *s)
                    && request.filter.assignee.as_deref().is_none_or(|a| {
                        r.frontmatter["assignee"].as_str().is_some_and(|v| {
                            v.trim_start_matches("seat:") == a.trim_start_matches("seat:")
                        })
                    })
            })
            .collect();
        let offset = request
            .cursor
            .as_deref()
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap();
        let end = (offset + request.limit.unwrap_or(128).min(self.list_page_size)).min(rows.len());
        Ok(d::Page {
            items: rows[offset..end]
                .iter()
                .map(|r| r.reference.clone())
                .collect(),
            next: (end < rows.len()).then(|| end.to_string()),
        })
    }
    fn graph(&self, request: d::GraphRequest) -> d::Result<d::GraphPage> {
        if let Some(delay) = self.delay {
            std::thread::sleep(delay);
        }
        self.pages(request)
    }
    // Like DataSourceProvider::request_before: the deadline clips the call,
    // and an exhausted call fails as a transport timeout.
    fn graph_until(&self, request: d::GraphRequest, deadline: Instant) -> d::Result<d::GraphPage> {
        self.calls.lock().unwrap().deadlines.push(deadline);
        if let Some(delay) = self.delay {
            std::thread::sleep(delay.min(deadline.saturating_duration_since(Instant::now())));
        }
        if Instant::now() >= deadline {
            return Err(d::CommandError::new(
                "plugin_timeout",
                "fixture call timed out",
            ));
        }
        self.pages(request)
    }
    fn list_until(
        &self,
        request: d::ListRequest,
        deadline: Instant,
    ) -> d::Result<d::Page<d::RecordRef>> {
        self.calls.lock().unwrap().deadlines.push(deadline);
        self.list(request)
    }
    fn fetch_until(&self, request: d::FetchRequest, deadline: Instant) -> d::Result<d::Record> {
        self.calls.lock().unwrap().deadlines.push(deadline);
        if let Some(delay) = self.delay {
            std::thread::sleep(delay.min(deadline.saturating_duration_since(Instant::now())));
        }
        if Instant::now() >= deadline {
            return Err(d::CommandError::new(
                "plugin_timeout",
                "fixture call timed out",
            ));
        }
        self.fetch(request)
    }
    fn search(&self, _: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
        unreachable!()
    }
    fn links(&self, _: d::FetchRequest) -> d::Result<d::Links> {
        unreachable!()
    }
    fn write(&self, _: d::WriteRequest, _: &WriteContext<'_>) -> d::Result<d::WriteResult> {
        unreachable!()
    }
    fn changes_since(&self, _: d::ChangesRequest) -> d::Result<d::Changes> {
        unreachable!()
    }
}
impl Corpus {
    fn pages(&self, request: d::GraphRequest) -> d::Result<d::GraphPage> {
        self.calls.lock().unwrap().graphs += 1;
        let d::Scope::Type {
            graph: d::DocumentGraph::Markdown,
            record_type,
        } = request.scope
        else {
            panic!("inventory must request an explicit document-type scope")
        };
        assert!(["task", "bug", "epic"].contains(&record_type.as_str()));
        let rows: Vec<_> = self
            .rows
            .iter()
            .filter(|r| r.frontmatter["type"] == record_type)
            .collect();
        let offset = request
            .cursor
            .as_deref()
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap();
        let end = (offset + request.limit.unwrap_or(128)).min(rows.len());
        Ok(d::GraphPage {
            nodes: rows[offset..end]
                .iter()
                .map(|r| d::Node {
                    id: r.reference.id.clone(),
                    reference: Some(r.reference.clone()),
                    title: None,
                    record_type: Some(record_type.clone()),
                    frontmatter: r.frontmatter.clone(),
                })
                .collect(),
            edges: vec![],
            next: (end < rows.len()).then(|| end.to_string()),
            edges_truncated: false,
        })
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    registry: Registry,
    oracle: SeatRecord,
    worker: SeatRecord,
    peer: SeatRecord,
    foreign: SeatRecord,
    source: Corpus,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let registry = Registry::new(SqliteStore::open(&temp.path().join("account")).unwrap());
        let swarm = Ulid::from(100_u128);
        let mut oracle = SeatRecord::new(Ulid::from(11_u128), swarm, "oracle", "zsh");
        oracle.role.class = Class::Oracle;
        let worker = SeatRecord::new(Ulid::from(12_u128), swarm, "worker", "zsh");
        let peer = SeatRecord::new(Ulid::from(13_u128), swarm, "peer", "zsh");
        let foreign = SeatRecord::new(Ulid::from(14_u128), Ulid::from(200_u128), "foreign", "zsh");
        for seat in [&oracle, &worker, &peer, &foreign] {
            registry.insert(seat).unwrap();
        }
        let source = Corpus::new(worker.id, peer.id, foreign.id);
        Self {
            _temp: temp,
            registry,
            oracle,
            worker,
            peer,
            foreign,
            source,
        }
    }
    fn actor(record: &SeatRecord) -> Actor {
        Actor::Seat(SeatActor {
            seat_id: record.id,
            process_generation: record.generation,
            swarm_id: record.swarm,
            role: record.role.clone(),
            grant_epoch: 1,
            issuer_kind: IssuerKind::LocalDev,
        })
    }
    fn read(&self, actor: &Actor, tool: &str, args: Value) -> Value {
        let tasks = ProviderTasks(&self.source, Some(self.oracle.swarm));
        let Prepared::Read(value) = prepare(
            &tasks,
            &self.registry,
            actor,
            &Call {
                op_id: Ulid::new(),
                tool: tool.into(),
                args,
            },
        )
        .unwrap() else {
            panic!("expected read")
        };
        value
    }
}

// keeps: 160 large task bodies/history never poison an MCP page. The next
// cursor is the last emitted ID, including when bytes cut before the item limit.
#[test]
fn task_list_byte_pages_cover_large_bodies_and_reject_uncut_control() {
    let mut f = Fixture::new();
    f.source.list_page_size = 128;
    f.source.rows = (1..=160)
        .map(|n| {
            let mut row = record(n, "task", Some(json!(format!("seat:{}", f.worker.id))));
            row.body = "body ".repeat(3276) + "done"; // Exactly 16 KiB.
            row.frontmatter["title"] = json!("\"\\é".repeat(1024)); // 4 KiB, escaped twice.
            row.frontmatter["history"] = json!(["historical evidence".repeat(1024)]);
            row.frontmatter["verify_result"] =
                json!({"command":"true","exit_code":0,"evidence":["x".repeat(8192)]});
            row
        })
        .collect();
    let actor = Fixture::actor(&f.worker);
    let mut after = Value::Null;
    let mut ids = Vec::new();
    let mut pages = 0;
    loop {
        let page = f.read(&actor, "task_list", json!({"limit":128,"after":after}));
        let op = Ulid::new();
        let result =
            crate::mcp::knowledge::bounded(op, crate::mcp::success(op, page.clone()), false);
        assert_eq!(result["isError"], false, "{result}");
        assert!(serde_json::to_vec(&result).unwrap().len() <= TASK_PAGE_BYTES);
        let items = page["items"].as_array().unwrap();
        assert!(
            !items.is_empty() && items.len() < 128,
            "bytes must cut the page"
        );
        for item in items {
            assert_eq!(item["body"], "");
            assert_eq!(item["body_truncated"], true);
            assert!(item["metadata"].get("history").is_none());
            assert_eq!(item["metadata"]["verify_result"], json!({"exit_code":0}));
            ids.push(item["id"].as_str().unwrap().to_owned());
        }
        let crate::mcp::work_items::TaskResponse::Items { items: cards, next } =
            response("task_list", result).unwrap()
        else {
            panic!("GUI/ctl summary adapter must accept the lean page");
        };
        assert_eq!(cards.len(), items.len());
        assert_eq!(cards[0].verify.as_ref().unwrap().exit_code, 0);
        assert_eq!(json!(next), page["next"]);
        pages += 1;
        if page["next"].is_null() {
            break;
        }
        // No updated_at here: the cursor is the last item's ULID time and ID.
        let last = items.last().unwrap()["id"].as_str().unwrap();
        assert_eq!(page["next"], format!("0:{last}"));
        assert_ne!(page["next"], after);
        after = page["next"].clone();
        assert!(pages < 160);
    }
    assert!(pages > 1);
    assert_eq!(
        ids,
        (1..=160)
            .rev()
            .map(|n| Ulid::from(n as u128).to_string())
            .collect::<Vec<_>>(),
        "newest first, every task once"
    );

    let full = f.read(
        &actor,
        "task_status",
        json!({"task_id":ids.last().unwrap()}),
    );
    assert_eq!(full["body"], f.source.rows[0].body);
    assert_eq!(full["body_truncated"], false);
    assert_eq!(full["metadata"], f.source.rows[0].frontmatter);

    let docs = ProviderTasks(&f.source, Some(f.worker.swarm))
        .documents(
            &f.registry,
            &actor,
            &serde_json::from_value(json!({"limit":128})).unwrap(),
        )
        .unwrap();
    // Deliberately remove the byte cut while retaining the same projection.
    let uncut = list_page(&docs, 128, usize::MAX).unwrap();
    let op = Ulid::new();
    let broken = crate::mcp::knowledge::bounded(op, crate::mcp::success(op, uncut), false);
    assert_eq!(
        broken["structuredContent"]["error"]["code"],
        "response_bound"
    );
}

#[test]
fn task_list_projects_one_oversized_item_and_counts_escaped_envelope() {
    let mut f = Fixture::new();
    let mut row = record(1, "task", Some(json!(format!("seat:{}", f.worker.id))));
    row.body = "x".repeat(crate::mcp::MAX_RESULT + 1);
    row.frontmatter["title"] = json!("é\"\\\u{1}".repeat(crate::mcp::MAX_RESULT));
    row.frontmatter["report"] = json!({"summary":"x".repeat(crate::mcp::MAX_RESULT)});
    f.source.rows = vec![row];
    let page = f.read(
        &Fixture::actor(&f.worker),
        "task_list",
        json!({"limit":128}),
    );
    assert!(page["next"].is_null());
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["body_truncated"], true);
    let op = Ulid::new();
    let result = crate::mcp::knowledge::bounded(op, crate::mcp::success(op, page), false);
    assert_eq!(result["isError"], false);
    assert!(serde_json::to_vec(&result).unwrap().len() < TASK_PAGE_BYTES);
}

// keeps: scoped task inventory must not perform body IO proportional to unrelated Markdown/tasks.
#[test]
fn task_inventory_large_corpus_pushes_scope_and_fetches_only_page_and_lookahead() {
    let f = Fixture::new();
    let started = Instant::now();
    let page = f.read(&Fixture::actor(&f.oracle), "task_list", json!({"limit":2}));
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    assert_eq!(page["next"], format!("0:{}", Ulid::from(20_002_u128)));
    assert_eq!(page["items"][0]["id"], Ulid::from(20_003_u128).to_string());
    let calls = f.source.calls.lock().unwrap();
    assert!((2..=3).contains(&calls.fetches.len()));
    assert_eq!(
        calls.resolves, 0,
        "list references must not be resolved a second time"
    );
    assert_eq!(calls.graphs, 0);
    assert!(!calls.lists.is_empty() && calls.lists.len() <= 18);
    assert!(
        calls
            .lists
            .iter()
            .any(|q| q.assignee.as_deref() == Some(format!("seat:{}", f.peer.id).as_str()))
    );
    assert!(
        calls
            .fetches
            .iter()
            .all(|id| id >= &Ulid::from(20_001_u128).to_string())
    );
    assert!(calls.lists.iter().all(|q| {
        q.assignee
            .as_deref()
            .is_some_and(|a| a.trim_start_matches("seat:") != f.foreign.id.to_string())
    }));
    eprintln!(
        "large task corpus={} list_calls={} body_fetches={} elapsed={:?}",
        f.source.rows.len(),
        calls.lists.len(),
        calls.fetches.len(),
        started.elapsed()
    );
    drop(calls);
    f.source.reset();
    let next = f.read(
        &Fixture::actor(&f.oracle),
        "task_list",
        json!({"after":page["next"],"limit":2}),
    );
    assert_eq!(next["items"].as_array().unwrap().len(), 1);
    assert!(next["next"].is_null());
    assert_eq!(f.source.calls.lock().unwrap().fetches.len(), 1);
}

#[test]
fn native_inventory_keeps_scoped_unassigned_tasks_without_foreign_body_reads() {
    let mut f = Fixture::new();
    f.source.native = true;
    for row in &mut f.source.rows {
        if (20_004_u128..=20_006).any(|id| row.reference.id == Ulid::from(id).to_string()) {
            row.frontmatter["delegated_by"] = json!(format!("seat:{}", f.oracle.id));
        }
    }
    let mut foreign = record(20_007, "task", None);
    foreign.frontmatter["delegated_by"] = json!(format!("seat:{}", f.foreign.id));
    f.source.rows.push(foreign);
    let page = f.read(
        &Fixture::actor(&f.oracle),
        "task_list",
        json!({"limit":128}),
    );
    let ids: Vec<_> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        ids,
        (20_001_u128..=20_006)
            .rev()
            .map(|id| Ulid::from(id).to_string())
            .collect::<Vec<_>>()
    );
    let calls = f.source.calls.lock().unwrap();
    assert_eq!(calls.fetches.len(), 6);
    assert!(calls.fetches.iter().all(|id| ids.contains(id)));
    assert!(calls.lists.is_empty());
    assert!(calls.graphs > 0 && calls.graphs <= 12);
}

#[test]
fn task_inventory_worker_keeps_legacy_assignment_and_only_queries_self() {
    let f = Fixture::new();
    let page = f.read(
        &Fixture::actor(&f.worker),
        "task_list",
        json!({"limit":128,"status":"open"}),
    );
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    let calls = f.source.calls.lock().unwrap();
    assert_eq!(calls.fetches.len(), 2);
    assert!(!calls.lists.is_empty() && calls.lists.len() <= 6);
    assert!(
        calls
            .lists
            .iter()
            .all(|q| q.status.as_deref() == Some("open")
                && q.assignee.as_deref().unwrap().trim_start_matches("seat:")
                    == f.worker.id.to_string())
    );
}

#[test]
fn task_inventory_unassigned_classifies_bulk_metadata_without_unrelated_bodies() {
    let f = Fixture::new();
    let actor = Actor::Operator {
        device_id: Ulid::new(),
        door: Door::Gui,
        issued_by: None,
    };
    let started = Instant::now();
    let page = f.read(&actor, "task_list", json!({"unassigned":true,"limit":2}));
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    assert_eq!(page["next"], format!("0:{}", Ulid::from(20_005_u128)));
    let calls = f.source.calls.lock().unwrap();
    assert!((2..=3).contains(&calls.fetches.len()));
    assert!(calls.lists.is_empty());
    assert!(calls.graphs > 0 && calls.graphs <= 10);
    assert!(
        calls
            .fetches
            .iter()
            .all(|id| id >= &Ulid::from(20_004_u128).to_string())
    );
    eprintln!(
        "unassigned corpus={} metadata_pages={} body_fetches={} elapsed={:?}",
        f.source.rows.len(),
        calls.graphs,
        calls.fetches.len(),
        started.elapsed()
    );
}

#[test]
fn task_inventory_status_and_delegation_by_slug_do_not_list_or_graph() {
    let f = Fixture::new();
    f.read(
        &Fixture::actor(&f.worker),
        "task_status",
        json!({"task_path":"tasks/doc-20001"}),
    );
    let tasks = ProviderTasks(&f.source, Some(f.oracle.swarm));
    assert!(matches!(
        prepare(
            &tasks,
            &f.registry,
            &Fixture::actor(&f.oracle),
            &Call {
                op_id: Ulid::new(),
                tool: "task_delegate".into(),
                args: json!({"task_path":"tasks/doc-20004","seat_id":f.worker.id})
            }
        )
        .unwrap(),
        Prepared::Effect(_)
    ));
    let calls = f.source.calls.lock().unwrap();
    assert!(calls.lists.is_empty());
    assert_eq!(calls.graphs, 0);
    assert_eq!(calls.resolves, 2);
    assert_eq!(calls.fetches.len(), 2);
}

#[test]
fn task_inventory_unscoped_operator_uses_metadata_not_per_seat_queries() {
    let f = Fixture::new();
    let actor = Actor::Operator {
        device_id: Ulid::new(),
        door: Door::Gui,
        issued_by: None,
    };
    let tasks = ProviderTasks(&f.source, None);
    let Prepared::Read(page) = prepare(
        &tasks,
        &f.registry,
        &actor,
        &Call {
            op_id: Ulid::new(),
            tool: "task_list".into(),
            args: json!({"limit":2}),
        },
    )
    .unwrap() else {
        panic!("expected read")
    };
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    assert_eq!(page["next"], format!("0:{}", Ulid::from(20_002_u128)));
    let calls = f.source.calls.lock().unwrap();
    assert!(calls.lists.is_empty());
    assert!(calls.graphs > 0 && calls.graphs <= 10);
    assert!((2..=3).contains(&calls.fetches.len()));
}

#[test]
fn task_inventory_still_rejects_cross_swarm_ownership_and_honors_captured_scope() {
    let mut f = Fixture::new();
    let mut poisoned = record(20_000, "task", Some(json!(format!("seat:{}", f.worker.id))));
    poisoned.frontmatter["delegated_by"] = json!(format!("seat:{}", f.foreign.id));
    f.source.rows.push(poisoned);
    let page = f.read(
        &Fixture::actor(&f.oracle),
        "task_list",
        json!({"limit":128}),
    );
    assert_eq!(page["items"].as_array().unwrap().len(), 3);
    assert!(
        page["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["id"] != Ulid::from(20_000_u128).to_string())
    );
    let tasks = ProviderTasks(&f.source, Some(f.oracle.swarm));
    assert!(
        prepare(
            &tasks,
            &f.registry,
            &Fixture::actor(&f.oracle),
            &Call {
                op_id: Ulid::new(),
                tool: "task_list".into(),
                args: json!({"swarm_id":f.foreign.swarm})
            }
        )
        .is_err()
    );
}

#[test]
fn task_inventory_historical_records_do_not_poison_live_work_or_gain_task_authority() {
    let mut f = Fixture::new();
    f.source.native = true;
    for (n, state) in [
        (19_001, None),
        (19_002, Some("legacy-complete")),
        (19_003, Some("")),
    ] {
        let mut row = record(n, "task", Some(json!(format!("seat:{}", f.worker.id))));
        if let Some(state) = state {
            row.frontmatter["status"] = json!(state);
        } else {
            row.frontmatter.as_object_mut().unwrap().remove("status");
        }
        f.source.rows.push(row);
    }
    let actor = Fixture::actor(&f.oracle);
    let result = f.read(&actor, "task_list", json!({"limit":128}));
    let ids: Vec<_> = result["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&Ulid::from(20_001_u128).to_string().as_str()));
    for n in 19_001..=19_003 {
        let tasks = ProviderTasks(&f.source, Some(f.oracle.swarm));
        assert!(!ids.contains(&Ulid::from(n as u128).to_string().as_str()));
        let error = prepare(
            &tasks,
            &f.registry,
            &actor,
            &Call {
                op_id: Ulid::new(),
                tool: "task_status".into(),
                args: json!({"task_id":Ulid::from(n as u128)}),
            },
        )
        .err()
        .expect("historical status cannot gain native task authority");
        assert_eq!(error, "invalid task status");
    }
}

// keeps: G6 — one task_list read answers inside the seat bridge's 10 s socket
// timeout with a named deadline error, however slow its provider pages are.
#[test]
fn task_inventory_stops_at_one_overall_deadline_with_a_named_error() {
    let mut f = Fixture::new();
    // Ten graph pages at 0.9 s each take 9 s without an overall deadline.
    f.source.delay = Some(Duration::from_millis(900));
    let actor = Actor::Operator {
        device_id: Ulid::new(),
        door: Door::Gui,
        issued_by: None,
    };
    let tasks = ProviderTasks(&f.source, None);
    let started = Instant::now();
    let refused = prepare(
        &tasks,
        &f.registry,
        &actor,
        &Call {
            op_id: Ulid::new(),
            tool: "task_list".into(),
            args: json!({"limit":2}),
        },
    )
    .err()
    .expect("a slow provider must exhaust the task_list budget");
    let elapsed = started.elapsed();
    assert_eq!(failure_code(&refused), DEADLINE_EXCEEDED, "{refused}");
    assert!(
        refused.contains("after"),
        "the refusal names how to narrow or page: {refused}"
    );
    assert!(
        elapsed >= TASK_LIST_BUDGET && elapsed < TASK_LIST_BUDGET + Duration::from_millis(500),
        "{elapsed:?}"
    );
    let calls = f.source.calls.lock().unwrap();
    assert!(!calls.deadlines.is_empty(), "provider calls were unbounded");
    assert!(
        calls.deadlines.iter().all(|at| *at == calls.deadlines[0]
            && *at > started
            && *at <= started + TASK_LIST_BUDGET + Duration::from_millis(100)),
        "every call shares the one operation deadline"
    );
}

// keeps: G6 — replaying a retained task_list rechecks its items under the
// same one overall deadline; what it cannot recheck in time is not replayed.
#[test]
fn task_list_replay_recheck_stops_at_the_same_overall_deadline() {
    let mut f = Fixture::new();
    let actor = Actor::Operator {
        device_id: Ulid::new(),
        door: Door::Gui,
        issued_by: None,
    };
    let call = Call {
        op_id: Ulid::new(),
        tool: "task_list".into(),
        args: json!({"limit":12}),
    };
    let items: Vec<_> = f.source.rows[10_000..10_012]
        .iter()
        .map(|row| json!({"path": row.reference.path}))
        .collect();
    let retained = json!({"isError": false, "structuredContent": {"items": items}});
    assert!(retained_allowed(
        &ProviderTasks(&f.source, None),
        &f.registry,
        &actor,
        &call,
        &retained
    ));
    // Twelve fetches at 0.9 s each take 10.8 s without the replay deadline.
    f.source.delay = Some(Duration::from_millis(900));
    f.source.calls.lock().unwrap().deadlines.clear();
    let started = Instant::now();
    let allowed = retained_allowed(
        &ProviderTasks(&f.source, None),
        &f.registry,
        &actor,
        &call,
        &retained,
    );
    let elapsed = started.elapsed();
    assert!(!allowed, "an unverified retained item must not be replayed");
    assert!(
        elapsed < TASK_LIST_BUDGET + Duration::from_millis(500),
        "{elapsed:?}"
    );
    let calls = f.source.calls.lock().unwrap();
    assert!(!calls.deadlines.is_empty(), "replay fetches were unbounded");
    assert!(
        calls.deadlines.iter().all(|at| *at == calls.deadlines[0]),
        "every replay fetch shares the one deadline"
    );
}

/// Rows whose changed times deliberately disagree with their ID order,
/// including ties that only the ID breaks.
fn changed_rows(f: &Fixture) -> Vec<d::Record> {
    [
        (1, 30),
        (2, 10),
        (3, 30),
        (4, 20),
        (5, 50),
        (6, 10),
        (7, 40),
        (8, 20),
        (9, 30),
    ]
    .into_iter()
    .map(|(n, at)| {
        let mut row = record(n, "task", Some(json!(format!("seat:{}", f.worker.id))));
        row.reference.updated_at = Some(at);
        row
    })
    .collect()
}
fn page_ids(f: &Fixture, actor: &Actor, args: Value) -> (Vec<u128>, Value) {
    let page = f.read(actor, "task_list", args);
    let ids = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap().parse::<Ulid>().unwrap().0)
        .collect();
    (ids, page["next"].clone())
}

// keeps: task_list lists the most recently changed tasks first (updated_at,
// then ID) and `after` continues exactly with strictly older ones, through
// the per-seat provider path and the native wide path; a change moves a task
// to the top.
#[test]
fn task_list_pages_most_recently_changed_first_exactly() {
    for native in [false, true] {
        let mut f = Fixture::new();
        f.source.native = native;
        f.source.list_page_size = 3;
        f.source.rows = changed_rows(&f);
        let actor = Fixture::actor(&f.oracle);
        let (mut seen, mut after, mut pages) = (Vec::new(), Value::Null, 0);
        loop {
            let (ids, next) = page_ids(&f, &actor, json!({"limit":2,"after":after}));
            assert!(!ids.is_empty() && ids.len() <= 2);
            seen.extend(ids);
            pages += 1;
            if next.is_null() {
                break;
            }
            let last = seen.last().unwrap();
            let at = f.source.rows[*last as usize - 1]
                .reference
                .updated_at
                .unwrap();
            assert_eq!(next, format!("{at}:{}", Ulid(*last)), "native={native}");
            after = next;
        }
        assert_eq!(seen, [5, 7, 9, 3, 1, 8, 4, 6, 2], "native={native}");
        assert_eq!(pages, 5);
        // An update to the oldest change moves it to the top.
        f.source.rows[1].reference.updated_at = Some(99);
        f.source.rows[1].reference.digest = "digest-2-updated".into();
        let (ids, _) = page_ids(&f, &actor, json!({"limit":2}));
        assert_eq!(ids, [2, 5], "native={native}");
    }
}

// keeps: the operator's one Tasks view merges the swarm's assigned and
// delegated tasks with the account's open unassigned ones, recent first.
#[test]
fn task_list_include_unassigned_merges_one_recent_first_operator_view() {
    let mut f = Fixture::new();
    let operator = Actor::Operator {
        device_id: Ulid::new(),
        door: Door::Gui,
        issued_by: None,
    };
    let mut rows = vec![
        record(1, "task", Some(json!(format!("seat:{}", f.worker.id)))),
        record(2, "task", None),
        record(3, "task", Some(json!(format!("seat:{}", f.peer.id)))),
        record(4, "task", Some(json!(format!("seat:{}", f.foreign.id)))),
        record(5, "task", None),
    ];
    rows[2].frontmatter["delegated_by"] = json!(format!("seat:{}", f.oracle.id));
    rows[4].frontmatter["status"] = json!("blocked");
    for (row, at) in rows.iter_mut().zip([40, 50, 30, 60, 70]) {
        row.reference.updated_at = Some(at);
    }
    f.source.rows = rows;
    let swarm = f.oracle.swarm;
    let (ids, next) = page_ids(
        &f,
        &operator,
        json!({"include_unassigned":true,"swarm_id":swarm,"limit":2}),
    );
    // The foreign swarm's task and a blocked unassigned task stay out.
    assert_eq!(ids, [2, 1]);
    let (rest, end) = page_ids(
        &f,
        &operator,
        json!({"include_unassigned":true,"swarm_id":swarm,"limit":2,"after":next}),
    );
    assert_eq!(rest, [3]);
    assert!(end.is_null());
    for refused in [
        json!({"include_unassigned":true,"unassigned":true}),
        json!({"include_unassigned":true,"assignee":f.worker.id}),
    ] {
        let tasks = ProviderTasks(&f.source, Some(swarm));
        let call = Call {
            op_id: Ulid::new(),
            tool: "task_list".into(),
            args: refused,
        };
        assert!(prepare(&tasks, &f.registry, &operator, &call).is_err());
    }
    let tasks = ProviderTasks(&f.source, Some(swarm));
    let call = Call {
        op_id: Ulid::new(),
        tool: "task_list".into(),
        args: json!({"include_unassigned":true}),
    };
    let error = prepare(&tasks, &f.registry, &Fixture::actor(&f.oracle), &call)
        .err()
        .expect("only the account operator merges the unassigned inventory");
    assert!(error.contains("account operator"), "{error}");
}

#[test]
fn task_list_after_accepts_its_cursor_or_a_bare_id_and_refuses_others() {
    let id = Ulid::from_parts(1_793_498_400_000, 7);
    assert_eq!(
        Recency::parse(&format!("12:{id}")).unwrap(),
        Recency(12, id)
    );
    assert_eq!(
        Recency::parse(&id.to_string()).unwrap(),
        Recency(1_793_498_400_000, id)
    );
    assert_eq!(
        Recency::parse(&Recency(12, id).cursor()).unwrap(),
        Recency(12, id)
    );
    for bad in [
        "#1",
        ":x",
        "12:",
        "x:01M6AD23JS424P4FGVQCQFEDZT",
        "-1:01M6AD23JS424P4FGVQCQFEDZT",
    ] {
        assert!(Recency::parse(bad).is_err(), "{bad}");
    }
}
