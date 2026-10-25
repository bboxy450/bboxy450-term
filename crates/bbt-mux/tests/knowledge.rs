#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use bbt_mux::{
    Ulid,
    index::Index,
    knowledge::{BboxySource, DataSource, WriteContext, data_source as d},
    registry::{Registry, SeatRecord},
    store::{Events, events::EventLog, object_dir, sqlite::SqliteStore},
};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf, time::Instant};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    store: SqliteStore,
    log: EventLog,
    key: SigningKey,
    index: Index,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let directory = object_dir(temp.path(), "index").unwrap();
        let store = SqliteStore::open(&directory).unwrap();
        let index = Index::open(&root, store.clone()).unwrap();
        let log = EventLog::open(&directory).unwrap();
        Self {
            _temp: temp,
            root,
            store,
            index,
            log,
            key: SigningKey::from_bytes(&[37; 32]),
        }
    }
    fn seed(&mut self, id: u128, path: &str, extra: Value, body: &str) {
        let mut metadata = json!({"id":Ulid(id).to_string(),"type":"note","title":path,"updated":"2026-10-20T00:00:00Z","unknown_metadata":{"keep":true}});
        for (key, value) in extra.as_object().unwrap() {
            metadata[key] = value.clone();
        }
        self.index
            .write(path, metadata, body, Ulid(900), &self.key)
            .unwrap();
        self.sync();
    }
    fn sync(&mut self) {
        for event in self.index.scan().unwrap() {
            let id = event.id;
            self.log.append(event, &self.key).unwrap();
            self.index.acknowledge(id).unwrap();
        }
    }
    fn resolve(&self, locator: &str) -> d::RecordRef {
        BboxySource::new(&self.index)
            .resolve(d::ResolveRequest {
                locator: locator.into(),
            })
            .unwrap()
    }
}

// keeps: spec knowledge-base §6
#[test]
fn resolution_fetch_and_images_use_one_root_and_the_existing_record_handles() {
    let mut f = Fixture::new();
    std::fs::create_dir_all(f.root.join("images")).unwrap();
    let image = include_bytes!("../../bbt-plugin/tests/fixtures/data_source/seed/images/pixel.png");
    std::fs::write(f.root.join("images/pixel space.png"), image).unwrap();
    let body = "# Alpha\n\n![[images/pixel space.png]]\n\n`![[missing.png]]`\n\n```text\n![[also-missing.png]]\n```\n";
    f.seed(
        70,
        "notes/alpha.md",
        json!({"type":"spec","title":"Alpha"}),
        body,
    );
    let reference = f.resolve("#1");
    for locator in [
        "alpha",
        "notes/alpha.md",
        ".bboxy/notes/alpha.md",
        &Ulid(70).to_string(),
        "[[#1|Alpha]]",
    ] {
        assert_eq!(f.resolve(locator), reference);
    }
    let source = BboxySource::new(&f.index);
    let record = source
        .fetch(d::FetchRequest {
            reference: reference.clone(),
        })
        .unwrap();
    assert_eq!(record.body, body);
    assert_eq!(record.frontmatter["unknown_metadata"], json!({"keep":true}));
    assert_eq!(record.etag, reference.digest);
    assert!(record.frontmatter["provenance"]["sig"].is_string());
    assert_eq!(record.resources.len(), 1);
    let resource = &record.resources[0];
    assert!(resource.uri.ends_with("/images/pixel%20space.png"));
    assert_eq!(resource.mime_type, "image/png");
    assert_eq!(source.read_resource(&resource.uri).unwrap(), image);
    assert_eq!(
        std::fs::read_dir(f.root.join("images")).unwrap().count(),
        1,
        "no resource copies"
    );
    assert!(
        source
            .resolve(d::ResolveRequest {
                locator: "#01".into()
            })
            .is_err()
    );
    assert!(
        source
            .resolve(d::ResolveRequest {
                locator: "missing".into()
            })
            .is_err()
    );
}

// keeps: spec knowledge-base §6
#[test]
fn shared_contract_root_answers_all_eight_methods_for_future_stdio_conformance() {
    let mut f = Fixture::new();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../bbt-plugin/tests/fixtures/data_source/seed");
    for path in ["tasks/brief.md", "tasks/work.md", "images/pixel.png"] {
        let destination = f.root.join(path);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(seed.join(path), destination).unwrap();
    }
    f.sync();
    let source = BboxySource::new(&f.index);
    let reference = f.resolve("#7");
    for locator in ["brief", "tasks/brief.md", &Ulid(7).to_string()] {
        assert_eq!(f.resolve(locator), reference);
    }
    let record = source
        .fetch(d::FetchRequest {
            reference: reference.clone(),
        })
        .unwrap();
    assert_eq!(record.frontmatter["unknown_metadata"], json!({"kept":true}));
    assert_eq!(
        source.read_resource(&record.resources[0].uri).unwrap(),
        include_bytes!("../../bbt-plugin/tests/fixtures/data_source/seed/images/pixel.png")
    );
    let page = source
        .search(d::SearchRequest {
            query: "fixture type:task".into(),
            cursor: None,
            limit: Some(1),
        })
        .unwrap();
    assert!(page.next.is_some());
    assert_eq!(page.items[0].reference.id, Ulid(7).to_string());
    let listed = source
        .list(d::ListRequest {
            filter: d::Filter {
                record_type: Some("task".into()),
                ..Default::default()
            },
            cursor: None,
            limit: None,
        })
        .unwrap();
    assert_eq!(listed.items.len(), 2);
    let links = source.links(d::FetchRequest { reference }).unwrap();
    assert!(
        links
            .outgoing
            .iter()
            .any(|e| e.to == Ulid(8).to_string() && e.rel == "depends" && e.weight == 2.)
    );
    assert!(
        source
            .links(d::FetchRequest {
                reference: f.resolve("#8")
            })
            .unwrap()
            .incoming
            .iter()
            .any(|e| e.from == Ulid(7).to_string())
    );
    let graph = source
        .graph(d::GraphRequest {
            scope: d::Scope::All {
                graph: d::DocumentGraph::Task,
            },
            cursor: None,
            limit: Some(1),
        })
        .unwrap();
    assert!(graph.next.is_some());
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(
        graph.edges.len(),
        1,
        "cross-page edge travels with its source node"
    );
    assert_eq!(graph.edges[0].to, Ulid(8).to_string());
    let before = source
        .changes_since(d::ChangesRequest {
            cursor: "0".into(),
            limit: None,
        })
        .unwrap()
        .next;
    let context = WriteContext::seat(Ulid(900), Some(1), &f.key);
    let written = source
        .write(
            d::WriteRequest {
                reference: None,
                path: Some("notes/new.md".into()),
                frontmatter: json!({"id":Ulid(9).to_string(),"type":"note"}),
                body: "new fixture".into(),
                etag: None,
            },
            &context,
        )
        .unwrap();
    assert_eq!(written.record.reference.number, Some(9));
    f.sync();
    let changes = BboxySource::new(&f.index)
        .changes_since(d::ChangesRequest {
            cursor: before,
            limit: None,
        })
        .unwrap();
    assert_eq!(changes.changed, [written.record.reference]);
}

// keeps: spec knowledge-base §6
#[test]
fn ambiguity_names_candidates_and_fresh_number_checks_do_not_pick_a_first_hit() {
    let mut f = Fixture::new();
    f.seed(1, "notes/same.md", json!({}), "one");
    f.seed(2, "tasks/same.md", json!({"type":"task"}), "two");
    let error = BboxySource::new(&f.index)
        .resolve(d::ResolveRequest {
            locator: "same".into(),
        })
        .unwrap_err();
    assert_eq!(error.code, "ambiguous_reference");
    assert_eq!(
        error
            .candidates
            .iter()
            .map(|r| r.path.as_str())
            .collect::<Vec<_>>(),
        ["notes/same.md", "tasks/same.md"]
    );
    let path = f.root.join("tasks/same.md");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, text.replace("number: 2", "number: 1")).unwrap();
    // Deliberately do not scan: NUM2's fresh disk check must still catch this.
    let error = BboxySource::new(&f.index)
        .resolve(d::ResolveRequest {
            locator: "#1".into(),
        })
        .unwrap_err();
    assert!(error.message.contains("duplicate record number 1"));
    assert!(error.message.contains("notes/same.md") && error.message.contains("tasks/same.md"));
}

// keeps: spec knowledge-base §6
#[test]
fn search_filters_ranking_weighted_links_and_graph_pages_share_index_data() {
    let mut f = Fixture::new();
    f.seed(1,"tasks/front.md",json!({"type":"task","title":"Needle in frontmatter","status":"open","assignee":format!("seat:{}",Ulid(900))}),"plain body");
    f.seed(
        2,
        "notes/body.md",
        json!({"title":"body match"}),
        "Needle in body",
    );
    f.seed(
        3,
        "notes/link.md",
        json!({"links":[{"to":Ulid(2).to_string(),"rel":"depends","weight":3.5}]}),
        "Needle too",
    );
    f.seed(4, "notes/quiet.md", json!({}), "Needle without degree");
    let source = BboxySource::new(&f.index);
    let hits = source
        .search(d::SearchRequest {
            query: "needle".into(),
            cursor: None,
            limit: Some(10),
        })
        .unwrap();
    assert_eq!(hits.items[0].reference.path, "tasks/front.md");
    assert!(
        hits.items
            .iter()
            .position(|h| h.reference.path == "notes/body.md")
            < hits
                .items
                .iter()
                .position(|h| h.reference.path == "notes/quiet.md")
    );
    let filtered = source
        .search(d::SearchRequest {
            query: "needle type:task status:open since:2026-10-19".into(),
            cursor: None,
            limit: None,
        })
        .unwrap();
    assert_eq!(filtered.items.len(), 1);
    let listed = source
        .list(d::ListRequest {
            filter: d::Filter {
                assignee: Some(Ulid(900).to_string()),
                ..Default::default()
            },
            cursor: None,
            limit: None,
        })
        .unwrap();
    assert_eq!(listed.items[0].path, "tasks/front.md");
    assert!(
        source
            .search(d::SearchRequest {
                query: "since:not-a-date".into(),
                cursor: None,
                limit: None
            })
            .is_err()
    );
    let body = f.resolve("#2");
    let links = source
        .links(d::FetchRequest {
            reference: body.clone(),
        })
        .unwrap();
    assert_eq!(links.incoming.len(), 1);
    assert_eq!(links.incoming[0].rel, "depends");
    assert_eq!(links.incoming[0].weight, 3.5);
    let first = source
        .graph(d::GraphRequest {
            scope: d::Scope::All {
                graph: d::DocumentGraph::Task,
            },
            cursor: None,
            limit: Some(1),
        })
        .unwrap();
    assert_eq!(first.nodes.len(), 1);
    let next = source
        .graph(d::GraphRequest {
            scope: d::Scope::All {
                graph: d::DocumentGraph::Task,
            },
            cursor: first.next,
            limit: Some(10),
        })
        .unwrap();
    assert_eq!(next.nodes.len(), 3);
    let neighborhood = source
        .graph(d::GraphRequest {
            scope: d::Scope::Neighborhood {
                graph: d::DocumentGraph::Task,
                reference: body,
                depth: 1,
            },
            cursor: None,
            limit: None,
        })
        .unwrap();
    assert_eq!(neighborhood.nodes.len(), 2);
    assert_eq!(neighborhood.edges.len(), 1);
    assert_eq!(neighborhood.edges[0].weight, 3.5);
}

// keeps: spec knowledge-base §6
#[test]
fn cursor_is_root_query_and_snapshot_bound_and_reopens_without_restarting_the_page() {
    let mut f = Fixture::new();
    f.seed(1, "a.md", json!({}), "first");
    f.seed(2, "b.md", json!({}), "second");
    let request = |cursor| d::ListRequest {
        filter: Default::default(),
        cursor,
        limit: Some(1),
    };
    let first = BboxySource::new(&f.index).list(request(None)).unwrap();
    let cursor = first.next.unwrap();
    f.index = Index::open(&f.root, f.store.clone()).unwrap();
    f.sync();
    let page = BboxySource::new(&f.index)
        .list(request(Some(cursor.clone())))
        .unwrap();
    assert_eq!(page.items[0].path, "b.md");
    assert!(page.next.is_none());
    let other = Fixture::new();
    assert_eq!(
        BboxySource::new(&other.index)
            .list(request(Some(cursor.clone())))
            .unwrap_err()
            .code,
        "invalid_cursor"
    );
    assert_eq!(
        BboxySource::new(&f.index)
            .search(d::SearchRequest {
                query: "first".into(),
                cursor: Some(cursor.clone()),
                limit: Some(1)
            })
            .unwrap_err()
            .code,
        "invalid_cursor"
    );
    f.seed(3, "c.md", json!({}), "third");
    assert_eq!(
        BboxySource::new(&f.index)
            .list(request(Some(cursor)))
            .unwrap_err()
            .code,
        "stale_cursor"
    );
}

// keeps: spec knowledge-base §6
#[test]
fn uneven_search_pages_defer_a_row_that_fits_a_fresh_quantum_without_clipping() {
    let mut f = Fixture::new();
    f.seed(
        1,
        "a.md",
        json!({"title":format!("needle {}","a".repeat(240*1024))}),
        "one",
    );
    f.seed(
        2,
        "b.md",
        json!({"title":format!("needle {}","b".repeat(32*1024))}),
        "two",
    );
    let source = BboxySource::new(&f.index);
    let request = |cursor| d::SearchRequest {
        query: "needle".into(),
        cursor,
        limit: Some(128),
    };
    let first = source.search(request(None)).unwrap();
    assert_eq!(first.items.len(), 1);
    assert!(first.items[0].title.len() > 240 * 1024);
    assert!(serde_json::to_vec(&first).unwrap().len() <= 256 * 1024);
    let second = source.search(request(first.next)).unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.items[0].title.len() > 32 * 1024);
    assert!(second.next.is_none());
}

// keeps: decision #266
#[test]
fn decision_birth_names_the_allocated_number_and_refusal_consumes_nothing() {
    let mut f = Fixture::new();
    let key = f.key.clone();
    let context = WriteContext::seat(Ulid(900), Some(3), &key);
    let request = |path: &str, id| d::WriteRequest {
        reference: None,
        path: Some(path.into()),
        frontmatter: json!({"id":Ulid(id).to_string(),"type":"decision","title":"Named at birth","unknown":{"keep":true}}),
        body: "The original decision brief.\n".into(),
        etag: None,
    };
    let first = BboxySource::new(&f.index)
        .write(request("decisions/first.md", 1), &context)
        .unwrap();
    assert_eq!(first.record.reference.number, Some(1));
    assert_eq!(first.record.reference.path, "decisions/d1-first.md");
    assert_eq!(first.record.body, "The original decision brief.\n");
    assert_eq!(first.record.frontmatter["unknown"], json!({"keep":true}));
    assert!(!f.root.join("decisions/first.md").exists());
    assert!(f.root.join(&first.record.reference.path).exists());
    assert!(f.log.after(0, 128).unwrap().iter().any(|row| {
        row.event.subject == format!("doc/{}/write_prepared", Ulid(1))
            && row.event.payload["path"] == first.record.reference.path
    }));
    f.sync();
    let before = f.log.last_sequence().unwrap();
    let refused = BboxySource::new(&f.index)
        .write(request("decisions/d999-second.md", 2), &context)
        .unwrap_err();
    assert_eq!(refused.code, "decision_filename_required");
    assert!(refused.message.contains("decisions/d2-second.md"));
    assert!(!f.root.join("decisions/d999-second.md").exists());
    assert!(!f.root.join("decisions/d2-second.md").exists());
    assert_eq!(f.log.last_sequence().unwrap(), before);
    let second = BboxySource::new(&f.index)
        .write(request("decisions/d2-second.md", 2), &context)
        .unwrap();
    assert_eq!(second.record.reference.number, Some(2));
    assert_eq!(second.record.reference.path, "decisions/d2-second.md");
    f.sync();
    let updated = BboxySource::new(&f.index)
        .write(
            d::WriteRequest {
                reference: Some(first.record.reference.clone()),
                path: None,
                frontmatter: first.record.frontmatter,
                body: "Edited without renaming.\n".into(),
                etag: Some(first.record.etag),
            },
            &context,
        )
        .unwrap();
    assert_eq!(updated.record.reference.path, "decisions/d1-first.md");
    assert_eq!(updated.record.reference.number, Some(1));
    assert_eq!(updated.record.body, "Edited without renaming.\n");
    assert!(f.log.verify_chain().unwrap() > 0);
}

// keeps: spec knowledge-base §6
#[test]
fn checked_signed_write_allocates_once_and_changes_only_after_existing_outbox_publication() {
    let mut f = Fixture::new();
    f.seed(1, "a.md", json!({}), "original");
    let before = BboxySource::new(&f.index)
        .changes_since(d::ChangesRequest {
            cursor: "0".into(),
            limit: None,
        })
        .unwrap()
        .next;
    let key = f.key.clone();
    let context = WriteContext::seat(Ulid(900), Some(3), &key);
    let sequence_before = f.log.last_sequence().unwrap();
    assert!(
        BboxySource::new(&f.index)
            .write(
                d::WriteRequest {
                    reference: None,
                    path: Some("bad-image.md".into()),
                    frontmatter: json!({"id":Ulid(9).to_string(),"type":"note"}),
                    body: "![[images/not-present.png]]".into(),
                    etag: None
                },
                &context
            )
            .is_err()
    );
    assert!(!f.root.join("bad-image.md").exists());
    assert_eq!(
        f.log.last_sequence().unwrap(),
        sequence_before,
        "resource admission precedes write intent"
    );
    let result=BboxySource::new(&f.index).write(d::WriteRequest {reference:None,path:Some("notes/new.md".into()),frontmatter:json!({"id":Ulid(2).to_string(),"type":"note","title":"new","unknown":{"kept":true}}),body:"new body\n".into(),etag:None},&context).unwrap();
    assert!(!result.lossy);
    assert_eq!(result.record.reference.number, Some(2));
    assert_eq!(result.record.frontmatter["unknown"], json!({"kept":true}));
    let interim = BboxySource::new(&f.index)
        .changes_since(d::ChangesRequest {
            cursor: before.clone(),
            limit: None,
        })
        .unwrap();
    assert!(
        interim.changed.is_empty(),
        "prepared/written activity is not a published doc change"
    );
    f.sync();
    let changes = BboxySource::new(&f.index)
        .changes_since(d::ChangesRequest {
            cursor: before,
            limit: Some(128),
        })
        .unwrap();
    assert_eq!(changes.changed.len(), 1);
    assert_eq!(changes.changed[0], result.record.reference);
    let path = f.root.join("notes/new.md");
    let bytes = std::fs::read(&path).unwrap();
    let source = BboxySource::new(&f.index);
    let update = d::WriteRequest {
        reference: Some(result.record.reference.clone()),
        path: None,
        frontmatter: result.record.frontmatter.clone(),
        body: "changed".into(),
        etag: Some("stale".into()),
    };
    let mut path_on_update = update.clone();
    path_on_update.path = Some(result.record.reference.path.clone());
    assert_eq!(
        source.write(path_on_update, &context).unwrap_err().code,
        "invalid_path"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        source.write(update.clone(), &context).unwrap_err().code,
        "etag_mismatch"
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut update = update;
    update.etag = Some(result.record.etag.clone());
    update.frontmatter["number"] = json!(1);
    assert!(source.write(update.clone(), &context).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    update.frontmatter["number"] = json!(2);
    let updated = source.write(update, &context).unwrap();
    assert_eq!(updated.record.reference.number, Some(2));
    f.sync();
    assert!(f.log.verify_chain().unwrap() > 0);
    let cursor = BboxySource::new(&f.index)
        .changes_since(d::ChangesRequest {
            cursor: changes.next,
            limit: None,
        })
        .unwrap()
        .next;
    f.index = Index::open(&f.root, f.store.clone()).unwrap();
    f.sync();
    assert!(
        BboxySource::new(&f.index)
            .changes_since(d::ChangesRequest {
                cursor,
                limit: None
            })
            .unwrap()
            .changed
            .is_empty()
    );
    let source = BboxySource::new(&f.index);
    assert!(
        source
            .write(
                d::WriteRequest {
                    reference: None,
                    path: Some("../escape.md".into()),
                    frontmatter: json!({"id":Ulid(9).to_string(),"type":"note"}),
                    body: "no".into(),
                    etag: None
                },
                &context
            )
            .is_err()
    );
    assert!(!f.root.parent().unwrap().join("escape.md").exists());
}

// keeps: spec knowledge-base §6
#[test]
fn resource_symlinks_escapes_nonfiles_and_oversize_are_refused_without_copies() {
    use std::os::unix::fs::symlink;
    let mut f = Fixture::new();
    std::fs::create_dir_all(f.root.join("images")).unwrap();
    let outside = f.root.parent().unwrap().join("outside.png");
    std::fs::write(&outside, b"owned outside fixture").unwrap();
    symlink(&outside, f.root.join("images/link.png")).unwrap();
    f.seed(1, "bad.md", json!({}), "![[images/link.png]]");
    let source = BboxySource::new(&f.index);
    assert_eq!(
        source
            .fetch(d::FetchRequest {
                reference: f.resolve("#1")
            })
            .unwrap_err()
            .code,
        "resource_outside_data_dir"
    );
    assert_eq!(
        source
            .read_resource(&format!(
                "file://{}",
                outside.canonicalize().unwrap().display()
            ))
            .unwrap_err()
            .code,
        "resource_outside_data_dir"
    );
    let root = f.root.canonicalize().unwrap();
    assert_eq!(
        source
            .read_resource(&format!("file://{}/../outside.png", root.display()))
            .unwrap_err()
            .code,
        "resource_outside_data_dir"
    );
    let huge = std::fs::File::create(root.join("images/huge.png")).unwrap();
    huge.set_len(d::MAX_RESOURCE_BYTES + 1).unwrap();
    assert_eq!(
        source
            .read_resource(&format!("file://{}/images/huge.png", root.display()))
            .unwrap_err()
            .code,
        "resource_too_large"
    );
    assert!(
        source
            .read_resource(&format!("file://{}/images", root.display()))
            .is_err()
    );
}

// keeps: spec knowledge-base §6
#[test]
fn swarm_graph_uses_registry_references_and_refuses_cross_swarm_ambiguity() {
    let mut f = Fixture::new();
    let registry = Registry::new(f.store.clone());
    registry
        .insert(&SeatRecord::new(Ulid(900), Ulid(500), "one", "zsh"))
        .unwrap();
    registry
        .insert(&SeatRecord::new(Ulid(901), Ulid(501), "two", "zsh"))
        .unwrap();
    f.seed(
        1,
        "one.md",
        json!({"type":"task","assignee":format!("seat:{}",Ulid(900))}),
        "one",
    );
    f.seed(
        2,
        "two.md",
        json!({"type":"task","assignee":format!("seat:{}",Ulid(901))}),
        "two",
    );
    let request = || d::GraphRequest {
        scope: d::Scope::Swarm {
            graph: d::DocumentGraph::Task,
            swarm: Ulid(500).to_string(),
        },
        cursor: None,
        limit: None,
    };
    assert_eq!(
        BboxySource::new(&f.index)
            .graph(request())
            .unwrap_err()
            .code,
        "scope_unavailable"
    );
    let page = BboxySource::new(&f.index)
        .with_registry(&registry)
        .graph(request())
        .unwrap();
    assert_eq!(page.nodes.len(), 1);
    assert_eq!(page.nodes[0].reference.as_ref().unwrap().path, "one.md");
    f.seed(3,"mixed.md",json!({"type":"task","assignee":format!("seat:{}",Ulid(900)),"delegated_by":format!("seat:{}",Ulid(901))}),"ambiguous");
    assert_eq!(
        BboxySource::new(&f.index)
            .with_registry(&registry)
            .graph(request())
            .unwrap_err()
            .code,
        "ambiguous_scope"
    );
}

// keeps: spec knowledge-base §6
#[test]
fn numbered_208_and_1000_record_fixtures_page_on_io_with_stable_unique_results() {
    for count in [208, 1000] {
        let mut f = Fixture::new();
        // Existing numbered files are fixture input, not a provider allocator.
        // Production writes above use the Index's one locked allocator.
        std::fs::create_dir_all(f.root.join("notes")).unwrap();
        for n in 1..=count {
            std::fs::write(f.root.join(format!("notes/{n:04}.md")),format!("---\nid: \"{}\"\nnumber: {n}\ntype: note\ntitle: Fixture {n}\n---\nneedle body {n}\n",Ulid(n as u128))).unwrap();
        }
        f.sync();
        let started = Instant::now();
        let source = BboxySource::new(&f.index);
        let mut cursor = None;
        let mut ids = BTreeSet::new();
        let mut pages = 0;
        loop {
            let page = source
                .search(d::SearchRequest {
                    query: "needle type:note".into(),
                    cursor,
                    limit: Some(128),
                })
                .unwrap();
            assert!(page.items.len() <= 128);
            assert!(serde_json::to_vec(&page).unwrap().len() <= 256 * 1024);
            for hit in &page.items {
                assert!(ids.insert(hit.reference.id.clone()));
            }
            pages += 1;
            cursor = page.next;
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(ids.len(), count);
        eprintln!(
            "knowledge paging: {count} records, {pages} pages, {:?} IO elapsed",
            started.elapsed()
        );
    }
}
