use bbt_mux::{
    Ulid,
    knowledge::{DataSource, WriteContext, data_source as d, views},
    registry::SeatRecord,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap};

struct Stub {
    nodes: Vec<d::Node>,
    records: BTreeMap<String, d::Record>,
    calls: RefCell<Vec<String>>,
    repeat_cursor: bool,
    oversize: bool,
}
impl Stub {
    fn new(own: Ulid, other: Ulid) -> Self {
        let mut nodes = Vec::new();
        let mut records = BTreeMap::new();
        for (id, assignee) in [
            (1, Some(json!(format!("seat:{own}")))),
            (2, Some(json!(format!("seat:{other}")))),
            (3, None),
            (4, Some(Value::Null)),
        ] {
            let reference = d::RecordRef {
                id: Ulid(id).to_string(),
                number: Some(id as u64),
                slug: format!("task-{id}"),
                path: format!("tasks/{id}.md"),
                digest: "a".repeat(64),
                external_id: None,
            };
            let mut metadata = json!({"id":reference.id,"number":id,"type":"task","title":format!("from provider {id}"),"status":"open"});
            if let Some(assignee) = assignee {
                metadata["assignee"] = assignee;
            }
            nodes.push(d::Node {
                id: reference.id.clone(),
                reference: Some(reference.clone()),
                title: Some(format!("from provider {id}")),
                record_type: Some("task".into()),
                frontmatter: metadata.clone(),
            });
            records.insert(
                reference.id.clone(),
                d::Record {
                    reference,
                    frontmatter: metadata,
                    body: format!("FROM PROVIDER {id}"),
                    etag: "a".repeat(64),
                    resources: Vec::new(),
                    parse_error: None,
                },
            );
        }
        for seat in [own, other] {
            nodes.push(d::Node {
                id: format!("seat:{seat}"),
                reference: None,
                title: None,
                record_type: None,
                frontmatter: json!({}),
            });
        }
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        Self {
            nodes,
            records,
            calls: RefCell::new(Vec::new()),
            repeat_cursor: false,
            oversize: false,
        }
    }
    fn no<T>(&self, method: &str) -> d::Result<T> {
        self.calls.borrow_mut().push(method.into());
        Err(d::CommandError::new("unexpected_call", method))
    }
}
impl DataSource for Stub {
    fn resolve(&self, _: d::ResolveRequest) -> d::Result<d::RecordRef> {
        self.no("resolve")
    }
    fn fetch(&self, request: d::FetchRequest) -> d::Result<d::Record> {
        self.calls.borrow_mut().push("fetch".into());
        self.records
            .get(&request.reference.id)
            .cloned()
            .ok_or_else(|| d::CommandError::new("not_found", "stub record missing"))
    }
    fn search(&self, _: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
        self.no("search")
    }
    fn list(&self, _: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        self.no("list")
    }
    fn links(&self, _: d::FetchRequest) -> d::Result<d::Links> {
        self.no("links")
    }
    fn graph(&self, request: d::GraphRequest) -> d::Result<d::GraphPage> {
        self.calls.borrow_mut().push("graph".into());
        let start = if self.repeat_cursor {
            0
        } else {
            request
                .cursor
                .as_deref()
                .unwrap_or("0")
                .parse::<usize>()
                .unwrap()
        };
        let end = (start + 2).min(self.nodes.len());
        let nodes = if self.oversize {
            vec![self.nodes[0].clone(); d::MAX_PAGE + 1]
        } else {
            self.nodes[start..end].to_vec()
        };
        let mut edges = Vec::new();
        if nodes.iter().any(|node| node.id == Ulid(1).to_string()) {
            // Both a foreign edge and a cross-page owned edge arrive from
            // the provider; visibility and selection belong to the host.
            for target in [Ulid(2), Ulid(3)] {
                edges.push(d::Edge {
                    from: Ulid(1).to_string(),
                    to: target.to_string(),
                    rel: "depends".into(),
                    weight: 2.,
                });
            }
        }
        Ok(d::GraphPage {
            nodes,
            edges,
            next: if self.repeat_cursor {
                Some("repeat".into())
            } else {
                (end < self.nodes.len()).then(|| end.to_string())
            },
            edges_truncated: false,
        })
    }
    fn write(&self, _: d::WriteRequest, _: &WriteContext<'_>) -> d::Result<d::WriteResult> {
        self.no("write")
    }
    fn changes_since(&self, _: d::ChangesRequest) -> d::Result<d::Changes> {
        self.no("changes_since")
    }
}

#[test]
fn alternative_provider_drives_graph_and_briefs_without_an_index_or_policy_upgrade() {
    let own = Ulid(900);
    let other = Ulid(901);
    let source = Stub::new(own, other);
    let mut seat = SeatRecord::new(own, Ulid(500), "fixture", "zsh");
    seat.tasks = vec![Ulid(2), Ulid(3), Ulid(4)];
    let graph = views::graph(&source, &seat, d::DocumentGraph::Task, None, 32, true).unwrap();
    let ids: Vec<_> = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            Ulid(1).to_string(),
            Ulid(3).to_string(),
            format!("seat:{own}")
        ]
    );
    assert_eq!(graph["edges"].as_array().unwrap().len(), 1);
    assert_eq!(graph["edges"][0]["to"], Ulid(3).to_string());
    assert!(!graph.to_string().contains(&format!("seat:{other}")));
    let brief = views::orient(&source, &seat).unwrap();
    assert_eq!(brief["items"].as_array().unwrap().len(), 2);
    assert_eq!(brief["items"][0]["body"], "FROM PROVIDER 1");
    assert_eq!(brief["items"][1]["body"], "FROM PROVIDER 3");
    assert!(!brief.to_string().contains("FROM PROVIDER 2"));
    assert!(!brief.to_string().contains("FROM PROVIDER 4"));
    assert!(source.calls.borrow().contains(&"graph".into()));
    assert_eq!(
        source
            .calls
            .borrow()
            .iter()
            .filter(|call| *call == "fetch")
            .count(),
        2
    );
    assert!(!source.calls.borrow().iter().any(|call| matches!(
        call.as_str(),
        "write" | "resolve" | "search" | "list" | "links" | "changes_since"
    )));
    let full = views::graph(&source, &seat, d::DocumentGraph::Task, None, 32, false).unwrap();
    assert_eq!(full["nodes"].as_array().unwrap().len(), 6);
    let next = views::graph(&source, &seat, d::DocumentGraph::Task, None, 1, true).unwrap();
    assert_eq!(next["next"], Ulid(1).to_string());
    assert!(
        next["edges"].as_array().unwrap().is_empty(),
        "legacy MCP view edges stay inside selected nodes"
    );
}

#[test]
fn a_nonconforming_provider_cannot_make_a_view_loop_or_admit_an_oversize_page() {
    let seat = SeatRecord::new(Ulid(900), Ulid(500), "fixture", "zsh");
    let mut source = Stub::new(seat.id, Ulid(901));
    source.repeat_cursor = true;
    assert_eq!(
        views::graph(&source, &seat, d::DocumentGraph::Markdown, None, 32, true)
            .unwrap_err()
            .code,
        "invalid_cursor"
    );
    source.repeat_cursor = false;
    source.oversize = true;
    assert_eq!(
        views::orient(&source, &seat).unwrap_err().code,
        "invalid_provider_result"
    );
}

#[test]
fn document_view_paths_cannot_bypass_the_provider_port() {
    let module = include_str!("../src/knowledge/mod.rs")
        .split_once("pub mod views")
        .unwrap()
        .1;
    let tools = include_str!("../src/mcp/tools.rs");
    let retained = tools
        .split_once("pub(crate) fn retained_documents")
        .unwrap()
        .1
        .split_once("pub(crate) fn clip")
        .unwrap()
        .0;
    let graph = tools
        .split_once("pub(crate) fn graph(")
        .unwrap()
        .1
        .split_once("fn execution_graph")
        .unwrap()
        .0;
    let orient = tools
        .split_once("pub(crate) fn orient_documents")
        .unwrap()
        .1;
    for (name, source) in [
        ("provider view projections", module),
        ("retained reads", retained),
        ("document graph dispatch", graph),
        ("own briefs", orient),
    ] {
        for bypass in [
            "crate::index",
            "Index::",
            "index.documents",
            "index.graphs",
            "index.current",
            "index.by_number",
        ] {
            assert!(
                !source.contains(bypass),
                "{name} bypasses DataSource via {bypass}"
            );
        }
    }
    let logic = include_str!("../src/logic.rs");
    assert!(!logic.contains("tools::orient_documents(&self.index"));
    assert!(!logic.contains("tools::retained_documents(&self.index"));
    assert!(!logic.contains("tools::graph(&self.index, actor"));
}
