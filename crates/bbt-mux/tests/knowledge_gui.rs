use bbt_mux::knowledge::{DataSource, WriteContext, data_source as d, read};
use serde_json::{Value, json};

struct Source {
    forbid_calls: bool,
    oversized_page: bool,
    oversized_body: bool,
}
impl Source {
    fn new() -> Self {
        Self {
            forbid_calls: false,
            oversized_page: false,
            oversized_body: false,
        }
    }
    fn reference() -> d::RecordRef {
        d::RecordRef {
            id: "record".into(),
            number: Some(1),
            slug: "one".into(),
            path: "e2e/one.md".into(),
            digest: "a".repeat(64),
            external_id: None,
        }
    }
    fn forbidden<T>(&self) -> d::Result<T> {
        panic!("read transport called an unexposed provider method")
    }
}
impl DataSource for Source {
    fn resolve(&self, _: d::ResolveRequest) -> d::Result<d::RecordRef> {
        assert!(!self.forbid_calls, "invalid input reached the provider");
        Ok(Self::reference())
    }
    fn fetch(&self, _: d::FetchRequest) -> d::Result<d::Record> {
        assert!(!self.forbid_calls, "invalid input reached the provider");
        Ok(d::Record {
            reference: Self::reference(),
            frontmatter: json!({"number":1,"type":"e2e"}),
            body: if self.oversized_body {
                "x".repeat(d::MAX_PAGE_BYTES)
            } else {
                "body".into()
            },
            etag: "a".repeat(64),
            resources: vec![],
            parse_error: None,
        })
    }
    fn list(&self, _: d::ListRequest) -> d::Result<d::Page<d::RecordRef>> {
        assert!(!self.forbid_calls, "invalid input reached the provider");
        Ok(d::Page {
            items: vec![
                Self::reference();
                if self.oversized_page {
                    d::MAX_PAGE + 1
                } else {
                    1
                }
            ],
            next: None,
        })
    }
    fn changes_since(&self, _: d::ChangesRequest) -> d::Result<d::Changes> {
        assert!(!self.forbid_calls, "invalid input reached the provider");
        Ok(d::Changes {
            changed: vec![Self::reference()],
            next: "next".into(),
        })
    }
    fn search(&self, _: d::SearchRequest) -> d::Result<d::Page<d::SearchHit>> {
        assert!(!self.forbid_calls, "invalid input reached the provider");
        Ok(d::Page {
            items: vec![
                d::SearchHit {
                    reference: Self::reference(),
                    title: "one".into(),
                    record_type: "e2e".into(),
                    number: Some(1),
                    snippet: "body".into(),
                    score: 1.0,
                };
                if self.oversized_page {
                    d::MAX_PAGE + 1
                } else {
                    1
                }
            ],
            next: None,
        })
    }
    fn links(&self, _: d::FetchRequest) -> d::Result<d::Links> {
        self.forbidden()
    }
    fn graph(&self, _: d::GraphRequest) -> d::Result<d::GraphPage> {
        self.forbidden()
    }
    fn write(&self, _: d::WriteRequest, _: &WriteContext<'_>) -> d::Result<d::WriteResult> {
        self.forbidden()
    }
}

// keeps: spec knowledge-base §6.1, portable read-only provider contracts.
#[test]
fn admitted_read_dispatch_reuses_portable_dtos_and_cannot_write_or_ack() {
    let source = Source::new();
    let reference = read(&source, "resolve", json!({"locator":"#1"})).unwrap();
    let record = read(&source, "fetch", json!({"ref":reference})).unwrap();
    assert_eq!(record["body"], "body");
    let search = read(&source, "search", json!({"query":"one","limit":1})).unwrap();
    assert_eq!(search["items"][0]["ref"]["number"], 1);
    let page = read(&source, "list", json!({"limit":1})).unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        read(&source, "changes_since", json!({"cursor":"0"})).unwrap()["next"],
        "next"
    );
    for name in [
        "write",
        "links",
        "graph",
        "read_resource",
        "ack",
        "mark_read",
    ] {
        assert_eq!(
            read(&source, name, Value::Null).unwrap_err().code,
            "read_only_method"
        );
    }
}

// keeps: spec knowledge-base §6.1, portable read-only provider contracts.
#[test]
fn oversized_requests_pages_and_record_bodies_are_named_refusals() {
    let mut source = Source::new();
    source.forbid_calls = true;
    assert_eq!(
        read(
            &source,
            "resolve",
            json!({"locator":"x".repeat(d::MAX_PAGE_BYTES)})
        )
        .unwrap_err()
        .code,
        "request_too_large"
    );
    assert_eq!(
        read(&source, "fetch", json!({"ref":7})).unwrap_err().code,
        "invalid_request"
    );
    source.forbid_calls = false;
    source.oversized_page = true;
    assert_eq!(
        read(&source, "search", json!({"query":"one"}))
            .unwrap_err()
            .code,
        "invalid_provider_result"
    );
    assert_eq!(
        read(&source, "list", json!({})).unwrap_err().code,
        "invalid_provider_result"
    );
    source.oversized_body = true;
    assert_eq!(
        read(&source, "fetch", json!({"ref":Source::reference()}))
            .unwrap_err()
            .code,
        "response_too_large"
    );
}
