// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use bboxy450_term::knowledge::{E2eLoader, Watch, e2e_lines, promises};
use bbt_mux::{
    Ulid,
    index::Index,
    knowledge::{BboxySource, data_source as d, read},
    store::{Events, events::EventLog, object_dir, sqlite::SqliteStore},
};
use bbt_ui::{self as ui, components as c, modules::e2e_checklist::Tone};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

struct Fixture {
    _temp: tempfile::TempDir,
    index: Index,
    log: EventLog,
    key: SigningKey,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let directory = object_dir(temp.path(), "index").unwrap();
        let index = Index::open(
            &temp.path().join(".bboxy"),
            SqliteStore::open(&directory).unwrap(),
        )
        .unwrap();
        let log = EventLog::open(&directory).unwrap();
        Self {
            _temp: temp,
            index,
            log,
            key: SigningKey::from_bytes(&[29; 32]),
        }
    }
    fn write(&mut self, path: &str, metadata: Value) {
        self.index
            .write(path, metadata, "fixture only", Ulid(800), &self.key)
            .unwrap();
        for event in self.index.scan().unwrap() {
            let id = event.id;
            self.log.append(event, &self.key).unwrap();
            self.index.acknowledge(id).unwrap();
        }
    }
    fn update(&mut self, path: &str, status: &str) {
        let source = BboxySource::new(&self.index);
        use bbt_mux::knowledge::DataSource;
        let reference = source
            .resolve(d::ResolveRequest {
                locator: path.into(),
            })
            .unwrap();
        let mut record = source.fetch(d::FetchRequest { reference }).unwrap();
        record.frontmatter["status"] = json!(status);
        self.write(path, record.frontmatter);
    }
}
fn drive(loader: &mut E2eLoader, fixture: &Fixture) -> Vec<&'static str> {
    let source = BboxySource::new(&fixture.index);
    let mut calls = vec![];
    while let Some(call) = loader.next_request() {
        assert!(calls.len() < 1024, "loader did not bound its work");
        assert!(
            loader.next_request().is_none(),
            "more than one outstanding read"
        );
        calls.push(call.method);
        loader.respond(read(&source, call.method, call.args));
    }
    calls
}
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn labels(layout: &c::Layout) -> String {
    let mut out = if let c::LayoutKind::Text { runs, .. } = &layout.kind {
        runs.iter().map(|r| r.text.as_str()).collect()
    } else {
        String::new()
    };
    for child in &layout.children {
        out += &labels(child);
    }
    out
}
fn screen(loader: &E2eLoader) -> String {
    let context = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let view = loader.snapshot().element_width(context, &Metrics, 1218.);
    labels(
        &view
            .layout(Rect::new(0., 0., 1218., 0.), context, &Metrics)
            .unwrap(),
    )
}

// keeps: spec e2e-acceptance §1
#[test]
fn one_signed_corpus_drives_the_screen_and_ctl_projection_through_changes() {
    let mut fixture = Fixture::new();
    fixture.write(
        "tasks/fix.md",
        json!({"id":Ulid(1).to_string(),"type":"task","title":"fix T1","status":"running"}),
    );
    let canonical = promises().unwrap();
    for (i, promise) in canonical.iter().enumerate() {
        fixture.write(&format!("e2e/{}.md", promise.id), json!({
            "id":Ulid(100+i as u128).to_string(),"type":"e2e","function":promise.id,"group":promise.group,
            "title":promise.text,"status":if promise.id=="T1" {"fail"}else if promise.id=="S3" {"manual"}else{"pass"},"coverage":"full",
            "last_run":"2026-10-24T03:12:00Z","run_by":"seat:fixture",
            "links":if promise.id=="T1" {json!([{"to":"[[#1]]","rel":"tracked-by"}])}else{json!([])}
        }));
    }
    let mut loader = E2eLoader::new(canonical);
    let calls = drive(&mut loader, &fixture);
    assert_eq!(calls[0], "changes_since");
    assert_eq!(loader.snapshot().project().summary.tone, Tone::Amber);
    let lines = e2e_lines(loader.snapshot(), None).unwrap();
    // One row per promise, then the summary.
    let n = promises().unwrap().len();
    assert_eq!(lines.len(), n + 1);
    assert_eq!(lines[n]["state"], "amber");
    assert_eq!(lines[n]["passed"], n - 2);
    assert_eq!(lines[n]["manual"], 1);
    assert_eq!(
        lines[0]["tracked_tasks"],
        json!([{"ref":"#1","status":"running"}])
    );
    assert_eq!(lines[0]["run_by"], "seat:fixture");
    let drawn = screen(&loader);
    assert!(drawn.contains("AMBER") && drawn.contains("1 failure tracked"));
    assert!(drawn.contains("fixture") && drawn.contains("03:12"));
    let group = e2e_lines(loader.snapshot(), Some("terminal")).unwrap();
    let terminal = promises()
        .unwrap()
        .iter()
        .filter(|p| p.group == "terminal")
        .count();
    assert_eq!(group.len(), terminal + 1);
    assert_eq!(
        group.last(),
        lines.last(),
        "group must not change the global verdict"
    );
    assert!(e2e_lines(loader.snapshot(), Some("missing group")).is_err());
    assert!(loader.next_request().is_none(), "stable view must not poll");

    fixture.update("tasks/fix.md", "accepted");
    loader.changed();
    assert_eq!(drive(&mut loader, &fixture)[0], "changes_since");
    assert_eq!(loader.snapshot().project().summary.tone, Tone::Red);
    assert_eq!(
        e2e_lines(loader.snapshot(), None).unwrap()[n]["untracked"],
        1
    );
    assert!(screen(&loader).contains("untracked"));

    fixture.update("e2e/T1.md", "pass");
    loader.changed();
    drive(&mut loader, &fixture);
    assert_eq!(loader.snapshot().project().summary.tone, Tone::Green);
    assert_eq!(
        e2e_lines(loader.snapshot(), None).unwrap()[n]["state"],
        "green"
    );
    assert!(screen(&loader).contains("GREEN"));

    loader.changed();
    assert_eq!(loader.next_request().unwrap().method, "changes_since");
    loader.respond(Err(d::CommandError::new(
        "provider_offline",
        "fixture offline",
    )));
    assert_eq!(
        loader.snapshot().project().summary.tone,
        Tone::Red,
        "cached pass rows cannot keep a green verdict"
    );
    assert_eq!(
        e2e_lines(loader.snapshot(), None).unwrap()[n]["state"],
        "red"
    );
    assert!(loader.snapshot().faults[0].contains("fixture offline"));
    assert!(screen(&loader).contains("fault · Acceptance records unavailable:"));
    loader.reconnect();
    let call = loader.next_request().unwrap();
    assert_eq!(call.args["cursor"], "0");
}

// keeps: spec e2e-acceptance §1
#[test]
fn malformed_repeated_and_oversized_pages_fail_closed_instead_of_publishing_partial_rows() {
    let mut loader = E2eLoader::new(promises().unwrap());
    assert_eq!(loader.next_request().unwrap().method, "changes_since");
    loader.respond(Ok(json!({"changed":[],"next":"cursor"})));
    assert_eq!(loader.next_request().unwrap().method, "list");
    loader.respond(Ok(json!({"items":[],"next":"same"})));
    assert_eq!(loader.next_request().unwrap().args["cursor"], "same");
    loader.respond(Ok(json!({"items":[],"next":"same"})));
    assert!(loader.next_request().is_none());
    assert!(loader.snapshot().faults[0].contains("cursor"));
    assert!(loader.snapshot().records.is_empty());

    loader.reconnect();
    loader.next_request().unwrap();
    loader.respond(Ok(json!({"changed":[],"next":"cursor"})));
    loader.next_request().unwrap();
    loader.respond(Ok(json!({"items":42,"next":null})));
    assert_eq!(loader.snapshot().project().summary.tone, Tone::Red);
    assert!(!loader.snapshot().faults.is_empty());

    loader.reconnect();
    loader.next_request().unwrap();
    loader.respond(Ok(
        json!({"changed":[],"next":"x".repeat(d::MAX_PAGE_BYTES)}),
    ));
    assert!(loader.snapshot().faults[0].contains("bound"));
    assert!(loader.next_request().is_none());
}

// keeps: spec e2e-acceptance §1
#[test]
fn observer_coalesces_wakes_stops_when_hidden_and_refuses_previous_epoch_replies() {
    use bbt_proto::GuiAction;
    let mut fixture = Fixture::new();
    let promises = promises().unwrap();
    for promise in &promises {
        fixture.write(
            &format!("e2e/{}.md", promise.id),
            json!({
                "type":"e2e", "function":promise.id, "status":"pass", "coverage":"full"
            }),
        );
    }
    let records = promises.len();
    let mut watch = Watch::new(promises);
    assert!(watch.next_action().is_none());
    watch.observe(true);
    assert!(
        watch.next_action().is_none(),
        "disconnected reads never queue"
    );
    watch.connection(true);
    let source = BboxySource::new(&fixture.index);
    let drive = |watch: &mut Watch| {
        let mut calls = 0;
        while let Some(GuiAction::Knowledge { id, method, args }) = watch.next_action() {
            assert!(calls < 1024);
            assert!(watch.next_action().is_none());
            watch.response(id, read(&source, &method, args));
            calls += 1;
        }
        calls
    };
    // The changes page, the list page, then one fetch per record.
    assert_eq!(drive(&mut watch), records + 2);
    let snapshot = watch.publication().unwrap();
    assert_eq!(snapshot.project().summary.tone, Tone::Green);
    assert!(watch.next_action().is_none(), "idle never polls");
    for _ in 0..1000 {
        watch.changed();
    }
    assert_eq!(
        drive(&mut watch),
        records + 2,
        "wakes coalesce to one bounded refresh"
    );
    assert!(
        watch.publication().is_none(),
        "identical rows do not dirty UI"
    );

    watch.changed();
    let GuiAction::Knowledge { id: old, .. } = watch.next_action().unwrap() else {
        unreachable!()
    };
    watch.connection(false);
    assert_eq!(
        watch.publication().unwrap().project().summary.tone,
        Tone::Red
    );
    watch.connection(true);
    let GuiAction::Knowledge {
        id: fresh,
        method,
        args,
    } = watch.next_action().unwrap()
    else {
        unreachable!()
    };
    assert_eq!(args["cursor"], "0");
    watch.response(old, Ok(json!({"changed":[],"next":"stale"})));
    assert!(
        watch.next_action().is_none(),
        "stale reply cannot release current read"
    );
    watch.response(fresh, read(&source, &method, args));
    drive(&mut watch);
    assert_eq!(
        watch.publication().unwrap().project().summary.tone,
        Tone::Green
    );

    watch.changed();
    let GuiAction::Knowledge { id, method, args } = watch.next_action().unwrap() else {
        unreachable!()
    };
    watch.observe(false);
    watch.response(id, read(&source, &method, args));
    assert!(watch.next_action().is_none());
    assert!(watch.publication().is_none());
    watch.observe(true);
    assert_eq!(
        drive(&mut watch),
        records + 2,
        "reopen forces a fresh snapshot"
    );
    assert_eq!(
        watch.publication().unwrap().project().summary.tone,
        Tone::Green
    );
}
