// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

#[path = "support/claude.rs"]
mod support;
use bboxy450_log_converter::{ir::*, lower, parse::claude::Parser};
use std::io::Cursor;
#[test]
fn claude_raw_roundtrip_and_mapping() {
    let input = support::fixture();
    let (session, events) = support::parse(&input);
    let mut output = vec![];
    lower::claude::write(&session, &events, &mut output).unwrap();
    assert_eq!(input, output, "ClaudeRawRoundTrip");
    assert_eq!(
        events.iter().filter(|e| e.raw.is_some()).count(),
        input.iter().filter(|&&b| b == b'\n').count()
    );
    assert!(events.iter().any(|e| matches!(
        e.kind,
        EventKind::Message {
            role: Role::User,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e.kind,
        EventKind::Reasoning {
            sealed: Some(Sealed {
                form: SealedForm::ThinkingSignature,
                ..
            }),
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e.kind,
        EventKind::Reasoning {
            sealed: Some(Sealed {
                form: SealedForm::RedactedThinking,
                ..
            }),
            ..
        }
    )));
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::ToolResult { is_error: true, .. }))
    );
    assert!(events.iter().any(
        |e| matches!(e.kind, EventKind::Message { .. }) && e.fidelity == Fidelity::SummaryOnly
    ));
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::Usage { cache_write: 3, .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e.kind, EventKind::Subagent { .. }))
    );
    for name in ["Bash", "Edit", "Write", "MultiEdit", "TodoWrite", "Task"] {
        assert!(
            events
                .iter()
                .any(|e| matches!(&e.kind,EventKind::ToolCall{name:n,..}if n==name))
        );
    }
    assert!(
        events
            .iter()
            .filter_map(|e| e.raw.as_ref())
            .any(|r| String::from_utf8_lossy(&r.bytes).contains("oldStart\":17"))
    );
}
#[test]
fn claude_checkpoint_dag_metadata_and_block_ids() {
    let bytes = support::fixture();
    let mut parser = Parser::new(
        Cursor::new(bytes.clone()),
        SessionId(ulid::Ulid::from(1u128)),
    )
    .unwrap();
    let mut prefix = vec![];
    for _ in 0..4 {
        prefix.extend(parser.next_record().unwrap().unwrap());
    }
    let (_, session, cursor) = parser.into_parts();
    let encoded = serde_json::to_vec(&cursor).unwrap();
    let mut resumed = Parser::resume(
        Cursor::new(bytes.clone()),
        session,
        serde_json::from_slice(&encoded).unwrap(),
    )
    .unwrap();
    while let Some(events) = resumed.next_record().unwrap() {
        prefix.extend(events)
    }
    while let Some(events) = resumed.finish_tail().unwrap() {
        prefix.extend(events)
    }
    let (_, all) = support::parse(&bytes);
    assert_eq!(prefix, all, "ClaudeOffsetResume");
    let ids = all
        .iter()
        .map(|e| e.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), all.len(), "ClaudeDistinctDerivedIds");
    let source = &resumed.session().source.native_id;
    let key = "10000000-0000-4000-8000-000000000001";
    assert!(
        all.iter()
            .any(|e| e.id == EventId::inferred(Provider::Claude, source, key, "block-1"))
    );
    let tagged = all
        .iter()
        .filter(|e| {
            e.id == EventId::inferred(Provider::Claude, source, key, "usage")
                || e.id == EventId::inferred(Provider::Claude, source, key, "context")
        })
        .map(|e| &e.kind)
        .collect::<Vec<_>>();
    assert!(matches!(tagged[0], EventKind::Usage { .. }));
    assert!(matches!(tagged[1], EventKind::Context { .. }));
    let native_parent = RecordId::native(
        Provider::Claude,
        source,
        "10000000-0000-4000-8000-000000000000",
    );
    assert!(
        all.iter()
            .filter(|e| e.id == EventId::inferred(Provider::Claude, source, key, "block-0"))
            .all(|e| e.parent == Some(native_parent))
    );
}
#[test]
fn claude_unknown_malformed_and_truncated_are_safe() {
    for input in [
        b"{\"type\":\"future\"}\n".as_slice(),
        b"not-json\n",
        b"{\"type\":",
    ] {
        let result = std::panic::catch_unwind(|| support::parse(input));
        assert!(result.is_ok(), "ClaudeUnknownNeverPanics");
        let (_, events) = result.unwrap();
        assert!(!events.is_empty(), "ClaudeUnknownNeverPanics");
        assert!(
            events
                .iter()
                .any(|e| matches!(e.kind, EventKind::Native { .. } | EventKind::Notice { .. })),
            "ClaudeUnknownNeverPanics"
        );
    }
}
#[test]
fn claude_partial_tail_retries_without_advancing() {
    let mut bytes = b"{\"type\":\"future\"}".to_vec();
    let mut parser = Parser::new(
        Cursor::new(bytes.clone()),
        SessionId(ulid::Ulid::from(1u128)),
    )
    .unwrap();
    assert!(parser.next_record().unwrap().is_none());
    assert_eq!(parser.cursor().offset, 0);
    let (_, session, cursor) = parser.into_parts();
    bytes.push(b'\n');
    let mut resumed = Parser::resume(Cursor::new(bytes), session, cursor).unwrap();
    assert!(resumed.next_record().unwrap().is_some());
    assert_eq!(resumed.cursor().parse_errors, 0);
}
#[test]
fn claude_large_inputs_and_repaired_new_turns_stay_distinct() {
    use serde_json::json;
    let large = "x".repeat(300_000);
    let rows = [
        json!({"type":"user","uuid":"u","message":{"content":[{"type":"tool_result","tool_use_id":"orphan","content":"old"},{"type":"text","text":"new"}]}}),
        json!({"type":"user","uuid":"new-request","message":{"content":"new request"}}),
        json!({"type":"assistant","uuid":"a","message":{"content":[{"type":"tool_use","id":"write","name":"Write","input":{"file_path":"target","content":large}}]}}),
    ];
    let bytes = rows.iter().map(|v| format!("{v}\n")).collect::<String>();
    let (s, events) = support::parse(bytes.as_bytes());
    let ids = events
        .iter()
        .map(|e| e.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(events.len(), ids.len());
    assert!(events.iter().any(|e|matches!(&e.kind,EventKind::ToolCall{input:ToolInput::Json(v),..}if v["content"].as_str().unwrap().len()==300_000)));
    assert!(matches!(
        events.last().unwrap().kind,
        EventKind::TurnEnd {
            outcome: TurnOutcome::Unknown
        }
    ));
    let mut out = vec![];
    lower::claude::write(&s, &events, &mut out).unwrap();
    assert_eq!(out, bytes.as_bytes());
}

#[test]
fn claude_metadata_vector_and_raw_placement() {
    use serde_json::json;
    let row = json!({"type":"assistant","sessionId":"synthetic-vector","uuid":"vector","isSidechain":true,"agentId":"agent","message":{"model":"synthetic-model","usage":{"input_tokens":1},"content":[{"type":"text","text":"text"},{"type":"thinking","thinking":"thought","signature":"opaque"},{"type":"tool_use","id":"toolu_vector","name":"Bash","input":{"command":"true"}}]}});
    let bytes = format!("{row}\n");
    let (_, events) = support::parse(bytes.as_bytes());
    let ids = events
        .iter()
        .map(|e| e.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids.len(), events.len(), "ClaudeDistinctDerivedIds");
    let tags = [
        "block-0", "block-1", "block-2", "usage", "subagent", "context",
    ];
    assert_eq!(events.len(), tags.len());
    for (event, tag) in events.iter().zip(tags) {
        assert_eq!(
            event.id,
            EventId::inferred(Provider::Claude, "synthetic-vector", "vector", tag),
            "ClaudeDerivedTagOrder"
        );
    }
    assert_eq!(
        events.iter().filter(|e| e.raw.is_some()).count(),
        1,
        "ClaudeOneRawPerRecord"
    );
    assert!(events[0].raw.is_some(), "ClaudeFirstEventOwnsRaw");
    let results = json!({"type":"user","sessionId":"synthetic-vector","uuid":"results","message":{"content":[{"type":"tool_result","tool_use_id":"a","content":"a"},{"type":"tool_result","tool_use_id":"b","content":"b"},{"type":"tool_result","tool_use_id":"c","content":"c"}]}});
    let bytes = format!("{results}\n");
    let (_, events) = support::parse(bytes.as_bytes());
    assert_eq!(events.len(), 3);
    for (index, event) in events.iter().enumerate() {
        assert_eq!(
            event.id,
            EventId::inferred(
                Provider::Claude,
                "synthetic-vector",
                "results",
                &format!("block-{index}")
            )
        );
    }
    assert_eq!(
        events.iter().filter(|e| e.raw.is_some()).count(),
        1,
        "ClaudeOneRawPerRecord"
    );
}
#[test]
fn claude_appending_a_turn_end_preserves_delivered_ids() {
    use serde_json::json;
    let first = json!({"type":"user","sessionId":"append-stable","uuid":"user","message":{"content":"question"}});
    let last = json!({"type":"assistant","sessionId":"append-stable","uuid":"assistant","parentUuid":"user","message":{"content":"reply"}});
    let source = format!("{first}\n{last}\n");
    let mut parser = Parser::new(
        Cursor::new(source.as_bytes()),
        SessionId(ulid::Ulid::from(1u128)),
    )
    .unwrap();
    let mut before = vec![];
    while let Some(batch) = parser.next_record().unwrap() {
        before.extend(batch)
    }
    let (_, after) = support::parse(source.as_bytes());
    assert_eq!(before, after[..before.len()], "ClaudeIdsStableAtEof");
    let next = json!({"type":"user","sessionId":"append-stable","uuid":"next","parentUuid":"assistant","message":{"content":"next question"}});
    let (_, extended) = support::parse(format!("{source}{next}\n").as_bytes());
    assert_eq!(
        before,
        extended[..before.len()],
        "ClaudeIdsStableUnderAppend"
    );
}

#[test]
fn claude_record_dag_and_compaction_summary_join_without_identity_maps() {
    use serde_json::json;
    let rows = [
        json!({"type":"assistant","uuid":"a","sessionId":"dag","message":{"content":[{"type":"text","text":"spoken"},{"type":"thinking","thinking":"thought"}]}}),
        json!({"type":"user","uuid":"side","sessionId":"dag","parentUuid":"a","isSidechain":true,"agentId":"child","message":{"content":[{"type":"tool_result","tool_use_id":"t","content":"output"}]}}),
        json!({"type":"system","uuid":"boundary","sessionId":"dag","parentUuid":"side","subtype":"compact_boundary","compactMetadata":{"logicalParentUuid":"a"}}),
        json!({"type":"user","uuid":"summary","sessionId":"dag","parentUuid":"side","isCompactSummary":true,"message":{"content":"summary text"}}),
        json!({"type":"last-prompt","uuid":"leaf","sessionId":"dag","leafUuid":"side"}),
    ];
    let input = rows.iter().map(|v| format!("{v}\n")).collect::<String>();
    let (session, events) = support::parse(input.as_bytes());
    let anchor = |key| RecordId::native(Provider::Claude, "dag", key);
    let first = events
        .iter()
        .filter(|e| e.record == anchor("a"))
        .collect::<Vec<_>>();
    assert_eq!(first.len(), 2);
    assert!(first.iter().all(|e| e.id.0 != e.record.0));
    assert!(
        events
            .iter()
            .filter(|e| e.record == anchor("side"))
            .all(|e| e.parent == Some(anchor("a")))
    );
    let boundary = events
        .iter()
        .find(|e| e.record == anchor("boundary"))
        .unwrap();
    assert!(
        matches!(boundary.kind,EventKind::Compaction{replaced_through:Some(id),..}if id==anchor("a"))
    );
    assert_eq!(boundary.id.0, boundary.record.0);
    let summary = events
        .iter()
        .find(|e| e.record == anchor("summary"))
        .unwrap();
    assert_eq!(summary.parent, Some(anchor("boundary")));
    assert_eq!(summary.fidelity, Fidelity::SummaryOnly);
    assert!(matches!(
        summary.kind,
        EventKind::Message {
            role: Role::User,
            ..
        }
    ));
    assert!(
        events
            .iter()
            .filter(|e| e.record == anchor("leaf"))
            .all(|e| e.parent == Some(anchor("side")))
    );
    for key in ["a", "side", "boundary", "summary", "leaf"] {
        assert_eq!(
            events
                .iter()
                .filter(|e| e.record == anchor(key) && e.raw.is_some())
                .count(),
            1,
            "ClaudeOneRawPerRecord"
        );
    }
    let mut output = vec![];
    lower::claude::write(&session, &events, &mut output).unwrap();
    assert_eq!(output, input.as_bytes());
}
#[test]
fn claude_no_raw_retains_readable_and_sealed_reasoning() {
    let source = support::fixture();
    let mut parser = Parser::new(Cursor::new(source), SessionId(ulid::Ulid::from(1u128)))
        .unwrap()
        .without_raw();
    let mut seen = false;
    while let Some(events) = parser.next_record().unwrap() {
        for e in events {
            assert!(e.raw.is_none());
            if let EventKind::Reasoning {
                text: Some(text),
                sealed: Some(sealed),
                ..
            } = e.kind
            {
                assert_eq!(text, "A separate thought");
                assert_eq!(sealed.bytes, b"synthetic-signature");
                seen = true;
            }
        }
    }
    assert!(seen);
}

#[test]
fn claude_native_identity_for_metadata_prefix_and_standalone_summary_parent() {
    use serde_json::json;
    let rows = [
        json!({"type":"file-history-snapshot","uuid":"prefix"}),
        json!({"type":"user","uuid":"summary","sessionId":"known-native","parentUuid":"external-boundary","isCompactSummary":true,"message":{"content":"summary"}}),
    ];
    let input = rows.iter().map(|v| format!("{v}\n")).collect::<String>();
    let mut parser = Parser::with_native_id(
        Cursor::new(input.as_bytes()),
        SessionId(ulid::Ulid::from(1u128)),
        "known-native",
    )
    .unwrap();
    let mut events = vec![];
    while let Some(batch) = parser.next_record().unwrap() {
        events.extend(batch)
    }
    assert_eq!(parser.session().source.native_id, "known-native");
    assert_eq!(
        events[0].record,
        RecordId::native(Provider::Claude, "known-native", "prefix")
    );
    assert_eq!(
        events[1].parent,
        Some(RecordId::native(
            Provider::Claude,
            "known-native",
            "external-boundary"
        ))
    );
    let mut output = vec![];
    lower::claude::write(parser.session(), &events, &mut output).unwrap();
    assert_eq!(output, input.as_bytes());
}
