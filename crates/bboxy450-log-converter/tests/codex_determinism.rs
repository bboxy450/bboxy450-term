// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::{ir::*, lower::codex, parse::codex::Parser};
use serde_json::json;
use std::{collections::BTreeMap, io::Cursor};
#[allow(dead_code)]
#[path = "support/codex_fixture.rs"]
mod fixture;
fn parse(bytes: &[u8]) -> (Session, Vec<Event>) {
    let mut parser = Parser::new(Cursor::new(bytes), SessionId(ulid::Ulid::from(4u128))).unwrap();
    let mut events = vec![];
    while let Some(batch) = parser.next_record().unwrap() {
        events.extend(batch);
    }
    (parser.session().clone(), events)
}
#[test]
fn codex_append_does_not_reidentify_content_and_each_record_owns_raw_once() {
    let before = fixture::comprehensive();
    let (session, events) = parse(&before);
    let mut after = before.clone();
    after.extend_from_slice(
        b"{\"type\":\"event_msg\",\"ordinal\":1000,\"payload\":{\"type\":\"task_complete\"}}\n",
    );
    let (_, appended) = parse(&after);
    assert_eq!(&appended[..events.len()], events.as_slice());
    let mut groups: BTreeMap<RecordId, Vec<&Event>> = BTreeMap::new();
    for event in &events {
        groups.entry(event.record).or_default().push(event);
    }
    for (record, group) in &groups {
        assert_eq!(group.iter().filter(|e| e.raw.is_some()).count(), 1);
        assert!(group.iter().filter(|e| e.id.0 == record.0).count() <= 1);
        assert!(group.iter().all(|e| e.parent == group[0].parent));
        for event in group {
            if matches!(
                event.kind,
                EventKind::Message { .. }
                    | EventKind::Reasoning { .. }
                    | EventKind::ToolCall { .. }
                    | EventKind::ToolResult { .. }
            ) {
                assert_ne!(event.id.0, record.0);
            }
        }
    }
    let start = events
        .windows(2)
        .find(|p| matches!(p[0].kind, EventKind::TurnStart { .. }))
        .unwrap();
    assert_eq!(start[0].record, start[1].record);
    assert!(matches!(start[1].kind, EventKind::Native { .. }));
    assert!(start[0].raw.is_none() && start[1].raw.is_some());
    let mut duplicated = events.clone();
    for event in &mut duplicated {
        event.raw = groups[&event.record].iter().find_map(|e| e.raw.clone());
    }
    assert!(codex::write(&session, &duplicated, vec![]).is_err()); // Broken raw-owner control.
    let mut output = vec![];
    codex::write(&session, &events, &mut output).unwrap();
    assert_eq!(output, before);
}
#[test]
fn codex_usage_context_tags_and_item_reference_parents_are_record_based() {
    let bytes = fixture::encode(vec![
        fixture::header("/synthetic"),
        json!({"type":"response_item","payload":{"type":"function_call","call_id":"call_ref","name":"synthetic","arguments":"{}"}}),
        json!({"type":"turn_context","payload":{"model":"synthetic"}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"call_ref","output":"done"}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":9}}}}),
    ]);
    let (_, events) = parse(&bytes);
    let call = events
        .iter()
        .find(|e| matches!(e.kind, EventKind::ToolCall { .. }))
        .unwrap();
    let result = events
        .iter()
        .find(|e| matches!(e.kind, EventKind::ToolResult { .. }))
        .unwrap();
    assert_eq!(result.parent, Some(call.record));
    for (key, tag) in [("2", "context"), ("4", "usage")] {
        let record = RecordId(EventId::native(Provider::Codex, fixture::SID, key).0);
        let group = events
            .iter()
            .filter(|e| e.record == record)
            .collect::<Vec<_>>();
        assert_eq!(group.len(), 2);
        assert_eq!(group[0].id.0, record.0);
        assert!(matches!(group[0].kind, EventKind::Native { .. }));
        assert_eq!(
            group[1].id,
            EventId::tagged(Provider::Codex, fixture::SID, key, tag)
        );
        assert!(group[0].raw.is_some() && group[1].raw.is_none());
        assert_ne!(group[0].id, group[1].id);
        // Empty derived tags collide with the retained Native event.
        assert_eq!(
            EventId::tagged(Provider::Codex, fixture::SID, key, "").0,
            group[0].id.0
        );
    }
    assert_eq!(parse(&bytes).1, events);
}
