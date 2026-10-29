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
use std::{sync::Mutex, time::Instant};

#[derive(Default)]
struct Calls {
    lists: Vec<d::Filter>,
    graphs: usize,
    fetches: Vec<String>,
    resolves: usize,
}
struct Corpus {
    rows: Vec<d::Record>,
    calls: Mutex<Calls>,
    native: bool,
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
        },
        frontmatter,
        body: format!("Intent for document {n}"),
        etag: format!("etag-{n}"),
        resources: vec![],
        parse_error: None,
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
        let end = (offset + request.limit.unwrap_or(128).min(2)).min(rows.len());
        Ok(d::Page {
            items: rows[offset..end]
                .iter()
                .map(|r| r.reference.clone())
                .collect(),
            next: (end < rows.len()).then(|| end.to_string()),
        })
    }
    fn graph(&self, request: d::GraphRequest) -> d::Result<d::GraphPage> {
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

// keeps: scoped task inventory must not perform body IO proportional to unrelated Markdown/tasks.
#[test]
fn task_inventory_large_corpus_pushes_scope_and_fetches_only_page_and_lookahead() {
    let f = Fixture::new();
    let started = Instant::now();
    let page = f.read(&Fixture::actor(&f.oracle), "task_list", json!({"limit":2}));
    assert_eq!(page["items"].as_array().unwrap().len(), 2);
    assert_eq!(page["next"], Ulid::from(20_002_u128).to_string());
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
    assert_eq!(page["next"], Ulid::from(20_005_u128).to_string());
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
    assert_eq!(page["next"], Ulid::from(10_002_u128).to_string());
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
