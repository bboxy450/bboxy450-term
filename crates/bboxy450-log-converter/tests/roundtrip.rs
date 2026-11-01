// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-log-converter: https://github.com/bboxy450/bboxy450-log-converter/blob/17ef2a173d7873c1c881544fd69f40245e0b2fa9/tests/codex.rs
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

use bboxy450_log_converter::{
    ir::*,
    lower::codex::write,
    parse::codex::{Cursor as Checkpoint, Parser},
};
use serde_json::json;
use std::io::{BufReader, Cursor};
#[path = "support/codex_fixture.rs"]
mod fixture;
fn id() -> SessionId {
    SessionId(ulid::Ulid::from(4u128))
}
fn parse(bytes: &[u8]) -> (Session, Vec<Event>, Checkpoint) {
    let mut parser = Parser::new(BufReader::with_capacity(7, Cursor::new(bytes)), id()).unwrap();
    let mut events = vec![];
    while let Some(batch) = parser.next_record().unwrap() {
        events.extend(batch);
    }
    (parser.session().clone(), events, parser.cursor().clone())
}
#[test]
fn codex_roundtrip_raw_and_ir_are_identical() {
    let bytes = fixture::comprehensive();
    let (session, events, cursor) = parse(&bytes);
    let mut lowered = vec![];
    write(&session, &events, &mut lowered).unwrap();
    assert_eq!(lowered, bytes);
    assert_eq!(parse(&lowered), (session.clone(), events.clone(), cursor));
    assert_eq!(session.source.native_id, fixture::SID);
    assert_eq!(session.cwd.unwrap().to_str(), Some("/synthetic"));
    for (seq, event) in events.iter().enumerate() {
        assert_eq!(event.seq, seq as u64);
    }
    assert_eq!(events[0].parent, None);
    for pair in events.windows(2) {
        assert_eq!(
            pair[1].parent,
            if pair[1].record == pair[0].record {
                pair[0].parent
            } else {
                Some(pair[0].record)
            }
        );
    }
    assert_eq!(
        events
            .iter()
            .map(|e| e.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        events.len()
    );
}
#[test]
fn codex_maps_the_full_fixture() {
    let (_, e, _) = parse(&fixture::comprehensive());
    assert!(e.iter().any(|e| matches!(
        &e.kind,
        EventKind::Message {
            role: Role::System,
            ..
        }
    )));
    assert!(e.iter().any(|e| matches!(
        &e.kind,
        EventKind::Message {
            role: Role::Developer,
            ..
        }
    )));
    assert!(
        e.iter().any(
            |e| matches!(&e.kind,EventKind::Context{changes} if changes["model"]=="synthetic")
        )
    );
    assert!(e.iter().any(|e|matches!(&e.kind,EventKind::Message{blocks,..} if blocks.iter().any(|b|matches!(b,Block::Image{data,..} if data.len==3 && data.blake3==*blake3::hash(b"abc").as_bytes())))));
    let reason = e
        .iter()
        .find(|e| {
            matches!(
                &e.kind,
                EventKind::Reasoning {
                    sealed: Some(_),
                    ..
                }
            )
        })
        .unwrap();
    assert!(
        matches!(&reason.kind,EventKind::Reasoning{text:Some(t),summary:Some(s),sealed:Some(z)} if t=="readable reasoning" && s=="readable summary" && z.provider==Provider::Codex && z.form==SealedForm::EncryptedReasoning && z.bytes==b"LC_SYNTHETIC_SEALED_REASONING")
    );
    assert_eq!(
        reason.id,
        EventId::tagged(Provider::Codex, fixture::SID, "rs_synthetic", "block-0")
    );
    let call = e
        .iter()
        .find(|e| matches!(&e.kind,EventKind::ToolCall{call_id,..} if call_id=="call_fn"))
        .unwrap();
    let output = e
        .iter()
        .find(|e| matches!(&e.kind,EventKind::ToolResult{call_id,..} if call_id=="call_fn"))
        .unwrap();
    assert_eq!(
        call.id,
        EventId::tagged(Provider::Codex, fixture::SID, "call_fn", "block-0")
    );
    assert_ne!(call.id, output.id); // An output's call_id refers to its call.
    assert!(
        matches!(&call.kind,EventKind::ToolCall{input:ToolInput::Json(v),namespace:Some(n),tool_kind:ToolKind::Execute,..} if v["cmd"]=="synthetic" && n=="functions")
    );
    assert!(e.iter().any(|e|matches!(&e.kind,EventKind::ToolCall{input:ToolInput::Text(t),..} if t=="synthetic JavaScript text")));
    assert!(
        e.iter()
            .any(|e| matches!(&e.kind, EventKind::ToolResult { is_error: true, .. }))
    );
    assert!(e.iter().any(|e| matches!(&e.kind, EventKind::Native { record_type, .. } if record_type == "event_msg/token_count")));
    assert_eq!(
        e.iter()
            .filter(|e| matches!(e.kind, EventKind::Unknown))
            .count(),
        2
    );
    for event in &e {
        if let EventKind::Compaction {
            replacement,
            sealed: Some(sealed),
            ..
        } = &event.kind
        {
            assert_eq!(sealed.provider, Provider::Codex);
            assert_eq!(sealed.form, SealedForm::EncryptedCompaction);
            assert_eq!(event.fidelity, Fidelity::Opaque);
            for id in replacement {
                assert!(
                    e.iter()
                        .any(|p| p.id == *id && p.fidelity == Fidelity::Partial && p.raw.is_none())
                );
            }
        }
    }
    let starts = e
        .iter()
        .filter(|e| matches!(e.kind, EventKind::TurnStart { .. }))
        .collect::<Vec<_>>();
    assert_eq!(starts[0].turn, Some(TurnId("turn-one".into())));
    assert_eq!(starts[1].turn, Some(TurnId("turn-two".into())));
    assert!(e.iter().any(|e| matches!(
        e.kind,
        EventKind::TurnEnd {
            outcome: TurnOutcome::Interrupted
        }
    )));
}
#[test]
fn codex_determinism_includes_cbor() {
    let bytes = fixture::comprehensive();
    let a = parse(&bytes);
    let b = parse(&bytes);
    assert_eq!(a, b);
    assert_eq!(
        a.1.iter()
            .map(|e| cbor::encode(e).unwrap())
            .collect::<Vec<_>>(),
        b.1.iter()
            .map(|e| cbor::encode(e).unwrap())
            .collect::<Vec<_>>()
    );
}
#[test]
fn codex_checkpoint_at_every_byte_boundary_tolerates_truncated_utf8_and_json() {
    let bytes = fixture::comprehensive();
    let expected = parse(&bytes);
    for cut in 0..bytes.len() {
        let (session, mut prefix, cursor) = parse(&bytes[..cut]);
        let through = bytes[..cut]
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i + 1);
        assert_eq!(cursor.offset, through as u64);
        // Serialize the checkpoint as P7 will, then reconstruct a fresh reader.
        let restored = serde_json::from_slice(&serde_json::to_vec(&cursor).unwrap()).unwrap();
        let mut resumed = Parser::resume(Cursor::new(&bytes), session, restored).unwrap();
        while let Some(batch) = resumed.next_record().unwrap() {
            prefix.extend(batch);
        }
        assert_eq!(
            (resumed.session().clone(), prefix, resumed.cursor().clone()),
            expected,
            "cut {cut}"
        );
    }
}
#[test]
fn codex_partial_tail_does_not_advance_an_open_reader() {
    let bytes = fixture::resumable("/synthetic");
    let mut parser = Parser::new(Cursor::new(bytes[..bytes.len() - 1].to_vec()), id()).unwrap();
    while parser.next_record().unwrap().is_some() {}
    let before = parser.cursor().clone();
    assert!(parser.next_record().unwrap().is_none());
    assert_eq!(parser.cursor(), &before);
    let (mut reader, session, cursor) = parser.into_parts();
    reader.get_mut().push(b'\n');
    let mut parser = Parser::resume(reader, session, cursor).unwrap();
    assert!(parser.next_record().unwrap().is_some());
    assert_eq!(parser.cursor().offset, bytes.len() as u64);
}
#[test]
fn codex_malformed_and_unknown_lines_are_retained_without_panics() {
    let mut bytes = fixture::encode(vec![fixture::header("/synthetic")]);
    bytes.extend_from_slice(b"not json\n\xff\n42\n{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"arguments\":\"{\"}}\n");
    let (session, events, cursor) = parse(&bytes);
    assert_eq!(cursor.parse_errors, 4);
    assert!(matches!(
        events[1].kind,
        EventKind::Notice {
            level: Level::Warn,
            ..
        }
    ));
    assert!(matches!(events[3].kind, EventKind::Unknown));
    let mut lowered = vec![];
    write(&session, &events, &mut lowered).unwrap();
    assert_eq!(lowered, bytes);
    for byte in 0..=255 {
        let mut candidate = fixture::encode(vec![fixture::header("/synthetic")]);
        candidate.extend([byte, b'\n']);
        let _ = parse(&candidate);
    }
}
#[test]
fn codex_no_raw_preserves_sealed_and_readable_reasoning() {
    let mut parser = Parser::new(Cursor::new(fixture::comprehensive()), id())
        .unwrap()
        .without_raw();
    let mut sealed = 0;
    while let Some(events) = parser.next_record().unwrap() {
        for event in events {
            assert!(event.raw.is_none());
            if let EventKind::Reasoning {
                summary: Some(s),
                sealed: Some(z),
                ..
            } = event.kind
            {
                assert_eq!(s, "readable summary");
                assert!(!z.bytes.is_empty());
                sealed += 1;
            }
        }
    }
    assert_eq!(sealed, 1);
}
#[test]
fn codex_lowering_refuses_missing_meta_nonraw_foreign_and_reordering() {
    let (session, mut events, _) = parse(&fixture::comprehensive());
    assert!(write(&session, &events[1..], vec![]).is_err());
    events.swap(1, 2);
    assert!(write(&session, &events, vec![]).is_err());
    events.swap(1, 2);
    events
        .iter_mut()
        .skip(1)
        .find(|event| event.raw.is_some())
        .unwrap()
        .raw = None;
    assert!(write(&session, &events, vec![]).is_err());
    let mut foreign = session.clone();
    foreign.source.provider = Provider::Claude;
    assert!(write(&foreign, &events, vec![]).is_err());
    assert!(Parser::resume(Cursor::new(vec![]), foreign, Checkpoint::default()).is_err());
}
#[test]
fn codex_summary_without_encryption_and_native_token_counts() {
    let bytes = fixture::encode(vec![
        fixture::header("/synthetic"),
        json!({"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"summary only"}]}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":5}}}}),
    ]);
    let (_, events, _) = parse(&bytes);
    assert!(
        matches!(&events[1].kind,EventKind::Reasoning{summary:Some(s),sealed:None,..} if s=="summary only")
    );
    assert!(
        matches!(&events[2].kind, EventKind::Native { record_type, .. } if record_type == "event_msg/token_count")
    );
}

#[test]
fn codex_event_messages_do_not_duplicate_response_items_or_break_tool_pairs() {
    let bytes = fixture::encode(vec![
        fixture::header("/synthetic"),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"once"}]}}),
        json!({"type":"event_msg","payload":{"type":"user_message","message":"once"}}),
        json!({"type":"response_item","payload":{"type":"reasoning","summary":[{"text":"reason once"}]}}),
        json!({"type":"event_msg","payload":{"type":"agent_reasoning","text":"reason once"}}),
        json!({"type":"response_item","payload":{"type":"function_call","call_id":"call_bad","name":"test","arguments":"not JSON"}}),
        json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"call_bad","output":"done"}}),
        json!({"type":"event_msg","payload":{"type":"token_count","info":null}}),
        json!({"type":"event_msg","payload":{"type":"item_completed"}}),
    ]);
    let (session, events, cursor) = parse(&bytes);
    assert_eq!(cursor.parse_errors, 0);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e.kind,
                EventKind::Message {
                    role: Role::User,
                    ..
                }
            ))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::Reasoning { .. }))
            .count(),
        1
    );
    assert_eq!(events.iter().filter(|e|matches!(&e.kind, EventKind::Native { record_type, .. } if record_type.starts_with("event_msg/"))).count(),4);
    assert!(events.iter().any(|e| matches!(&e.kind, EventKind::ToolCall { call_id, input: ToolInput::Text(s), .. } if call_id == "call_bad" && s == "not JSON")));
    assert!(events.iter().any(
        |e| matches!(&e.kind, EventKind::ToolResult { call_id, .. } if call_id == "call_bad")
    ));
    let mut out = vec![];
    write(&session, &events, &mut out).unwrap();
    assert_eq!(out, bytes);
}

#[test]
fn codex_replacement_tags_use_element_positions_and_empty_tags_collide() {
    let bytes = fixture::encode(vec![
        fixture::header("/synthetic"),
        json!({"type":"compacted","payload":{"replacement_history":[
            {"type":"message","role":"user","content":[{"type":"input_text","text":"one"}]},
            {"type":"compaction","encrypted_content":"LC_SYNTHETIC"},
            {"type":"message","role":"assistant","content":[{"type":"output_text","text":"two"}]}
        ]}}),
    ]);
    let (session, events, _) = parse(&bytes);
    let EventKind::Compaction { replacement, .. } = &events[1].kind else {
        panic!("compaction");
    };
    assert_eq!(
        replacement,
        &vec![
            EventId::tagged(Provider::Codex, fixture::SID, "1", "replacement-0"),
            EventId::tagged(Provider::Codex, fixture::SID, "1", "replacement-2")
        ]
    );
    assert_ne!(replacement[0], replacement[1]);
    assert_eq!(parse(&bytes).1, events);
    let broken = [
        EventId::tagged(Provider::Codex, fixture::SID, "1", ""),
        EventId::tagged(Provider::Codex, fixture::SID, "1", ""),
    ];
    assert_eq!(broken[0], broken[1]);
    assert_eq!(broken[0], events[1].id); // Empty tags also collide with the retained record.
    let mut out = vec![];
    write(&session, &events, &mut out).unwrap();
    assert_eq!(out, bytes);
}

#[test]
fn codex_malformed_keys_cannot_alias_noncontiguous_native_ordinals() {
    let mut header = fixture::header("/synthetic");
    header["ordinal"] = json!(9);
    let bytes = format!(
        "{}\nnot JSON\n{}\n",
        header,
        json!({"type":"turn_context","ordinal":1,"payload":{"cwd":"/synthetic"}})
    )
    .into_bytes();
    let (session, events, _) = parse(&bytes);
    assert_eq!(
        events[1].id,
        EventId::native(Provider::Codex, fixture::SID, "malformed-line:1")
    );
    assert_eq!(
        events[2].id,
        EventId::native(Provider::Codex, fixture::SID, "1")
    );
    assert_ne!(events[1].id, events[2].id);
    let mut out = vec![];
    write(&session, &events, &mut out).unwrap();
    assert_eq!(out, bytes);
}

#[test]
fn codex_finish_tail_reports_and_consumes_terminal_incomplete_bytes_once() {
    let mut bytes = fixture::encode(vec![fixture::header("/synthetic")]);
    bytes.extend_from_slice(b"{\"broken\":");
    let mut parser = Parser::new(Cursor::new(&bytes), id()).unwrap();
    assert!(parser.next_record().unwrap().is_some());
    let before = parser.cursor().offset;
    assert!(parser.next_record().unwrap().is_none());
    assert_eq!(parser.cursor().offset, before);
    let events = parser.finish_tail().unwrap().unwrap();
    assert!(
        matches!(&events[0].kind,EventKind::Notice{level:Level::Warn,text} if text.contains("Truncated"))
    );
    assert_eq!(parser.cursor().offset, bytes.len() as u64);
    assert_eq!(parser.cursor().parse_errors, 1);
    assert!(events[0].raw.is_none()); // No complete raw line can be replayed.
    assert!(parser.finish_tail().unwrap().is_none());
}

#[test]
fn codex_oversized_complete_line_is_skipped_and_later_records_progress() {
    use bboxy450_log_converter::parse::codex::MAX_LINE_BYTES;
    let mut bytes = fixture::encode(vec![fixture::header("/synthetic")]);
    bytes.resize(bytes.len() + MAX_LINE_BYTES + 2, b'x');
    bytes.push(b'\n');
    let end = bytes.len();
    bytes.extend_from_slice(b"{\"type\":\"turn_context\",\"payload\":{}}\n");
    let mut parser = Parser::new(Cursor::new(bytes), id()).unwrap();
    parser.next_record().unwrap().unwrap();
    let events = parser.next_record().unwrap().unwrap();
    assert!(
        matches!(&events[0].kind,EventKind::Notice{level:Level::Warn,text} if text.contains("oversized"))
    );
    assert_eq!(parser.cursor().offset, end as u64);
    assert_eq!(parser.cursor().parse_errors, 1);
    assert!(
        parser
            .next_record()
            .unwrap()
            .unwrap()
            .iter()
            .any(|event| matches!(event.kind, EventKind::Context { .. }))
    );
}

struct FaultOnce {
    inner: Cursor<Vec<u8>>,
    at: u64,
    failed: bool,
}
impl std::io::Read for FaultOnce {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let data = std::io::BufRead::fill_buf(self)?;
        let n = data.len().min(out.len());
        out[..n].copy_from_slice(&data[..n]);
        std::io::BufRead::consume(self, n);
        Ok(n)
    }
}
impl std::io::BufRead for FaultOnce {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let pos = self.inner.position();
        if !self.failed && pos >= self.at {
            self.failed = true;
            return Err(std::io::Error::other("synthetic read failure"));
        }
        let limit = if self.failed {
            usize::MAX
        } else {
            (self.at - pos) as usize
        };
        let data = std::io::BufRead::fill_buf(&mut self.inner)?;
        Ok(&data[..data.len().min(limit)])
    }
    fn consume(&mut self, n: usize) {
        std::io::BufRead::consume(&mut self.inner, n);
    }
}
impl std::io::Seek for FaultOnce {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.inner, pos)
    }
}
#[test]
fn codex_io_error_restores_reader_and_leaves_checkpoint_and_session_unchanged() {
    let source = FaultOnce {
        inner: Cursor::new(fixture::resumable("/synthetic")),
        at: 20,
        failed: false,
    };
    let mut parser = Parser::new(source, id()).unwrap();
    let before = (parser.session().clone(), parser.cursor().clone());
    assert!(parser.next_record().is_err());
    assert_eq!((parser.session().clone(), parser.cursor().clone()), before);
    assert!(parser.next_record().unwrap().is_some());
    assert_eq!(parser.session().source.native_id, fixture::SID);
}
