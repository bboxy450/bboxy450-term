// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

//! Claude JSONL -> lc-ir. Adapted from bboxy450-harness PR #21, mapping/repair
//! logic: https://github.com/bboxy450/bboxy450-harness/blob/04edebdc/src/import/claude/engine.rs
//! Its respectful-slop origin policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
//! Unlike the presentation adapter, raw native records and sealed signatures
//! remain verbatim. Bash/Edit/Write/MultiEdit/TodoWrite retain complete inputs;
//! their Terminal/diff/Plan presentation is the P5 harness adapter's job.
//!
//! | Native | IR |
//! | --- | --- |
//! | user / assistant text | Message(User / Assistant) |
//! | thinking / redacted_thinking | Reasoning + provider-tagged Sealed |
//! | tool_use / tool_result | ToolCall / ToolResult (errors retained) |
//! | images | Image metadata; original payload only in raw |
//! | sidechain / Task / Agent | Subagent + lineage |
//! | compact_boundary / isCompactSummary | Compaction / Message(SummaryOnly), joined by parent |
//! | usage / context | Usage / Context |
//! | requests / turn_duration / final EOF | inferred TurnStart / TurnEnd |
//! | other / malformed / truncated | Native / Notice / Notice |
//!
//! next_record never consumes an incomplete tail. Checkpoint its whole batch
//! with Session and Cursor; resume seeks to that byte offset. finish_tail is
//! for a final, closed file only. Memory is one <=64 MiB native line and its
//! projections, plus <=4096 hashed open calls (overflow prevents safe boundary).
use crate::ir::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    io::{self, BufRead, Read, Seek, SeekFrom},
};

pub const PARSER_REVISION: u32 = 1;
pub const MAX_LINE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_OPEN_CALLS: usize = 4096;
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub offset: u64,
    pub line_ordinal: u64,
    pub next_seq: u64,
    pub previous: Option<RecordId>,
    pub turn: Option<TurnId>,
    pub parse_errors: u64,
    anchor: Option<String>,
    at: Option<Timestamp>,
    anchor_parent: Option<RecordId>,
    pending: BTreeSet<[u8; 32]>,
    pending_overflow: bool,
    seen_response: bool,
    compaction: Option<RecordId>,
}
pub struct Parser<R> {
    reader: R,
    session: Session,
    cursor: Cursor,
    retain_raw: bool,
    current_record: RecordId,
}
impl<R: BufRead + Seek> Parser<R> {
    pub fn new(reader: R, id: SessionId) -> io::Result<Self> {
        Self::resume(
            reader,
            Session {
                id,
                source: Source {
                    provider: Provider::Claude,
                    native_id: id.0.to_string(),
                    cli_version: None,
                    parser_revision: PARSER_REVISION,
                },
                lineage: vec![],
                cwd: None,
                git: None,
                created_at: None,
            },
            Cursor::default(),
        )
    }
    pub fn resume(mut reader: R, session: Session, cursor: Cursor) -> io::Result<Self> {
        if session.source.provider != Provider::Claude
            || session.source.parser_revision != PARSER_REVISION
        {
            return Err(invalid("checkpoint is not this Claude parser revision"));
        }
        reader.seek(SeekFrom::Start(cursor.offset))?;
        Ok(Self {
            reader,
            session,
            cursor,
            retain_raw: true,
            current_record: RecordId([0; 16]),
        })
    }
    pub fn without_raw(mut self) -> Self {
        self.retain_raw = false;
        self
    }
    pub fn session(&self) -> &Session {
        &self.session
    }
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }
    pub fn into_parts(self) -> (R, Session, Cursor) {
        (self.reader, self.session, self.cursor)
    }
    pub fn next_record(&mut self) -> io::Result<Option<Vec<Event>>> {
        self.read_record(false)
    }
    /// Complete a final incomplete tail, then emit a final inferred turn end.
    /// A turn with unmatched calls has Unknown outcome, never a safe switch point.
    pub fn finish_tail(&mut self) -> io::Result<Option<Vec<Event>>> {
        self.read_record(true)
    }
    fn id(&self, key: &str, tag: &str) -> EventId {
        EventId::tagged(Provider::Claude, &self.session.source.native_id, key, tag)
    }
    fn record(&self, key: &str) -> RecordId {
        RecordId::native(Provider::Claude, &self.session.source.native_id, key)
    }
    fn emit(
        &mut self,
        kind: EventKind,
        id: EventId,
        parent: Option<RecordId>,
        at: Option<Timestamp>,
        raw: Option<Raw>,
        fidelity: Fidelity,
    ) -> Event {
        let event = Event {
            seq: self.cursor.next_seq,
            id,
            record: self.current_record,
            parent,
            turn: self.cursor.turn.clone(),
            at,
            fidelity,
            raw,
            kind,
        };
        self.cursor.next_seq += 1;
        self.cursor.previous = Some(self.current_record);
        event
    }
    fn end(&mut self) -> Option<Event> {
        self.cursor.turn.as_ref()?;
        let key = self.cursor.anchor.as_deref().unwrap_or("0");
        let id = self.id(key, "turn-end");
        let outcome = if self.cursor.seen_response
            && self.cursor.pending.is_empty()
            && !self.cursor.pending_overflow
        {
            TurnOutcome::Completed
        } else {
            TurnOutcome::Unknown
        };
        let current = self.current_record;
        self.current_record = self.record(key);
        let event = self.emit(
            EventKind::TurnEnd { outcome },
            id,
            self.cursor.anchor_parent,
            self.cursor.at.clone(),
            None,
            Fidelity::Inferred,
        );
        self.current_record = current;
        self.cursor.turn = None;
        self.cursor.pending.clear();
        self.cursor.pending_overflow = false;
        self.cursor.seen_response = false;
        Some(event)
    }
    fn read_record(&mut self, final_eof: bool) -> io::Result<Option<Vec<Event>>> {
        let (mut line, count, oversized, terminated) = match read_line(&mut self.reader) {
            Ok(v) => v,
            Err(e) => {
                self.reader.seek(SeekFrom::Start(self.cursor.offset))?;
                return Err(e);
            }
        };
        if count == 0 {
            return Ok(if final_eof {
                self.end().map(|e| vec![e])
            } else {
                None
            });
        }
        if !terminated && !final_eof {
            self.reader.seek(SeekFrom::Start(self.cursor.offset))?;
            return Ok(None);
        }
        let ordinal = self.cursor.line_ordinal;
        self.current_record = self.record(&ordinal.to_string());
        if terminated {
            line.pop();
        }
        if oversized || !terminated {
            self.cursor.parse_errors += 1;
            let event = self.emit(
                EventKind::Notice {
                    level: Level::Warn,
                    text: if oversized {
                        "Skipped oversized Claude line (64 MiB limit)".into()
                    } else {
                        "Truncated Claude final line".into()
                    },
                },
                self.id(&ordinal.to_string(), ""),
                self.cursor.previous,
                None,
                None,
                Fidelity::Partial,
            );
            self.advance(count)?;
            return Ok(Some(vec![event]));
        }
        let row = serde_json::from_slice::<Value>(&line);
        let mut events = vec![];
        let mut raw = self.retain_raw.then_some(Raw {
            provider: Provider::Claude,
            bytes: line,
        });
        let row = match row {
            Ok(v) if v.is_object() => v,
            _ => {
                self.cursor.parse_errors += 1;
                events.push(self.emit(
                    EventKind::Notice {
                        level: Level::Warn,
                        text: format!("Malformed Claude JSON record at line {ordinal}"),
                    },
                    self.id(&ordinal.to_string(), ""),
                    self.cursor.previous,
                    None,
                    raw,
                    Fidelity::Partial,
                ));
                self.advance(count)?;
                return Ok(Some(events));
            }
        };
        if self.cursor.line_ordinal == 0 {
            if let Some(id) = text(&row, "sessionId") {
                self.session.source.native_id = id;
            }
            self.session.created_at = text(&row, "timestamp");
        }
        self.session.source.cli_version =
            text(&row, "version").or(self.session.source.cli_version.take());
        if let Some(cwd) = text(&row, "cwd") {
            self.session.cwd = Some(cwd.into());
        }
        if let Some(branch) = text(&row, "gitBranch") {
            self.session.git = Some(Git {
                branch: Some(branch),
                commit: None,
                repo_url: None,
            });
        }
        let key = text(&row, "uuid").unwrap_or_else(|| ordinal.to_string());
        self.current_record = self.record(&key);
        let at = text(&row, "timestamp");
        let parent = if row["isCompactSummary"] == true {
            self.cursor.compaction
        } else {
            text(&row, "parentUuid")
                .or_else(|| text(&row, "leafUuid"))
                .map(|p| self.record(&p))
        };
        let typ = row["type"].as_str().unwrap_or("unknown");
        let content = &row["message"]["content"];
        let user = typ == "user";
        let summary = row["isCompactSummary"] == true;
        let request = user && row["isMeta"] != true && !summary;
        let role = if user { Role::User } else { Role::Assistant };
        let blocks = content.as_array().cloned().unwrap_or_else(|| {
            if let Some(s) = content.as_str() {
                vec![serde_json::json!({"type":"text","text":s})]
            } else {
                vec![]
            }
        });
        let request_at = if request && !blocks.iter().any(|b| b["type"] == "tool_result") {
            blocks.iter().position(|b| b["type"] == "text")
        } else {
            None
        };
        let mut mapped = vec![];
        if matches!(typ, "user" | "assistant") {
            for (index, block) in blocks.iter().enumerate() {
                mapped.push((index, block_kind(block, role)));
            }
        } else if (typ == "system" && row["subtype"] == "compact_boundary") || typ == "summary" {
            let replaced = text(&row["compactMetadata"], "logicalParentUuid")
                .or_else(|| text(&row, "logicalParentUuid"))
                .map(|p| self.record(&p));
            self.cursor.compaction = Some(self.record(&key));
            mapped.push((
                0,
                EventKind::Compaction {
                    summary: if summary || typ == "summary" {
                        Some(if let Some(s) = row["summary"].as_str() {
                            vec![Block::Text { text: s.into() }]
                        } else {
                            native_blocks(content)
                        })
                    } else {
                        None
                    },
                    replaced_through: replaced,
                    replacement: vec![],
                    sealed: None,
                },
            ));
        }
        if mapped.is_empty() {
            mapped.push((0, native(typ)));
        }
        // A tool-result record is not a new request, even if it also carries
        // contextual text (lc-ir §2). All content remains represented.
        for (index, kind) in mapped {
            if request_at == Some(index) {
                if let Some(end) = self.end() {
                    events.push(end);
                }
                self.cursor.turn = Some(TurnId(format!("claude:{key}")));
                events.push(self.emit(
                    EventKind::TurnStart {
                        origin: TurnOrigin::User,
                    },
                    self.id(&key, "turn-start"),
                    parent,
                    at.clone(),
                    None,
                    Fidelity::Inferred,
                ));
            }
            if typ == "assistant" || matches!(kind, EventKind::ToolResult { .. }) {
                self.cursor.seen_response = true;
            }
            match &kind {
                EventKind::ToolCall { call_id, .. } => {
                    if self.cursor.pending.len() < MAX_OPEN_CALLS {
                        self.cursor
                            .pending
                            .insert(*blake3::hash(call_id.as_bytes()).as_bytes());
                    } else {
                        self.cursor.pending_overflow = true;
                    }
                }
                EventKind::ToolResult { call_id, .. } => {
                    self.cursor
                        .pending
                        .remove(blake3::hash(call_id.as_bytes()).as_bytes());
                }
                _ => {}
            }
            let tag = if matches!(
                kind,
                EventKind::Native { .. } | EventKind::Compaction { .. }
            ) && !matches!(typ, "user" | "assistant")
            {
                String::new()
            } else {
                format!("block-{index}")
            };
            let fidelity = if summary {
                Fidelity::SummaryOnly
            } else if self.retain_raw {
                if matches!(kind, EventKind::Native { .. }) {
                    Fidelity::Opaque
                } else {
                    Fidelity::Partial
                }
            } else {
                Fidelity::Partial
            };
            events.push(self.emit(
                kind,
                self.id(&key, &tag),
                parent,
                at.clone(),
                raw.take(),
                fidelity,
            ));
        }
        // Oracle ruling 01M6CQ1F8413AH6T8FWNR0KSX3: fixed metadata order.
        if let Some(usage) = row["message"].get("usage").or_else(|| row.get("usage")) {
            let n = |key: &str| usage[key].as_u64().unwrap_or(0);
            events.push(self.emit(
                EventKind::Usage {
                    input: n("input_tokens"),
                    output: n("output_tokens"),
                    cache_read: n("cache_read_input_tokens"),
                    cache_write: n("cache_creation_input_tokens"),
                    reasoning: n("reasoning_tokens"),
                },
                self.id(&key, "usage"),
                parent,
                at.clone(),
                None,
                Fidelity::Inferred,
            ));
        }
        let agent = blocks
            .iter()
            .find(|b| {
                b["type"] == "tool_use" && matches!(b["name"].as_str(), Some("Task" | "Agent"))
            })
            .and_then(|b| text(b, "id"))
            .or_else(|| {
                if row["isSidechain"] == true {
                    Some(
                        text(&row, "agentId")
                            .unwrap_or_else(|| self.session.source.native_id.clone()),
                    )
                } else {
                    None
                }
            });
        if let Some(agent) = agent {
            let bytes = self.id(&agent, "subagent").0;
            let child = SessionId(ulid::Ulid::from_bytes(bytes));
            if row["isSidechain"] == true && self.session.lineage.is_empty() {
                self.session.lineage.push(Lineage {
                    relation: Relation::SubagentOf,
                    session: child,
                    native: Some((Provider::Claude, agent)),
                    at: Some(self.id(&key, "subagent")),
                });
            }
            events.push(self.emit(
                EventKind::Subagent {
                    session: child,
                    relation: Relation::SubagentOf,
                },
                self.id(&key, "subagent"),
                parent,
                at.clone(),
                None,
                Fidelity::Inferred,
            ));
        }
        let mut changes = std::collections::BTreeMap::new();
        for name in ["cwd", "permissionMode", "model"] {
            if let Some(v) = row.get(name).or_else(|| {
                if name == "model" {
                    row["message"].get(name)
                } else {
                    None
                }
            }) {
                changes.insert(name.into(), v.clone());
            }
        }
        if !changes.is_empty() {
            events.push(self.emit(
                EventKind::Context { changes },
                self.id(&key, "context"),
                parent,
                at.clone(),
                None,
                Fidelity::Inferred,
            ));
        }
        // The first emitted event owns raw unless a whole-record event exists.
        if let Some(start) = events
            .iter()
            .position(|e| e.id == self.id(&key, "turn-start"))
            && let Some(owner) = events.iter().position(|e| e.raw.is_some())
        {
            if start < owner {
                let raw = events[owner].raw.take();
                events[start].raw = raw;
            }
        }
        if typ == "assistant"
            || request_at.is_some()
            || blocks.iter().any(|b| b["type"] == "tool_result")
            || (typ == "system" && row["subtype"] == "turn_duration")
        {
            self.cursor.anchor_parent = parent;
            self.cursor.anchor = Some(key);
            self.cursor.at = at;
        }
        if typ == "system"
            && row["subtype"] == "turn_duration"
            && let Some(end) = self.end()
        {
            events.push(end);
        }
        self.advance(count)?;
        Ok(Some(events))
    }
    fn advance(&mut self, count: u64) -> io::Result<()> {
        self.cursor.offset = self
            .cursor
            .offset
            .checked_add(count)
            .ok_or_else(|| invalid("offset overflow"))?;
        self.cursor.line_ordinal += 1;
        Ok(())
    }
}
fn native(typ: &str) -> EventKind {
    EventKind::Native {
        provider: Provider::Claude,
        record_type: typ.into(),
    }
}
fn block_kind(block: &Value, role: Role) -> EventKind {
    match block["type"].as_str().unwrap_or("unknown") {
        "text" => EventKind::Message {
            role,
            blocks: vec![Block::Text {
                text: text(block, "text").unwrap_or_default(),
            }],
        },
        "thinking" => EventKind::Reasoning {
            text: text(block, "thinking"),
            summary: None,
            sealed: text(block, "signature").map(|s| Sealed {
                provider: Provider::Claude,
                form: SealedForm::ThinkingSignature,
                bytes: s.into_bytes(),
            }),
        },
        "redacted_thinking" => EventKind::Reasoning {
            text: None,
            summary: None,
            sealed: text(block, "data").map(|s| Sealed {
                provider: Provider::Claude,
                form: SealedForm::RedactedThinking,
                bytes: s.into_bytes(),
            }),
        },
        "tool_use" => {
            let name = text(block, "name").unwrap_or_default();
            EventKind::ToolCall {
                call_id: text(block, "id").unwrap_or_default(),
                tool_kind: match name.as_str() {
                    "Bash" => ToolKind::Execute,
                    "Edit" | "Write" | "MultiEdit" => ToolKind::Edit,
                    "Read" => ToolKind::Read,
                    "Grep" | "Glob" => ToolKind::Search,
                    _ => ToolKind::Other,
                },
                name,
                namespace: None,
                input: ToolInput::Json(block.get("input").cloned().unwrap_or(Value::Null)),
            }
        }
        "tool_result" => EventKind::ToolResult {
            call_id: text(block, "tool_use_id").unwrap_or_default(),
            is_error: block["is_error"] == true,
            blocks: native_blocks(&block["content"]),
        },
        "image" => EventKind::Message {
            role,
            blocks: vec![image(block)],
        },
        typ => native(typ),
    }
}
fn native_blocks(content: &Value) -> Vec<Block> {
    if let Some(s) = content.as_str() {
        return vec![Block::Text { text: s.into() }];
    }
    content
        .as_array()
        .map(|a| {
            a.iter()
                .map(|v| match v["type"].as_str() {
                    Some("text") => Block::Text {
                        text: text(v, "text").unwrap_or_default(),
                    },
                    Some("image") => image(v),
                    _ => Block::Unknown,
                })
                .collect()
        })
        .unwrap_or_default()
}
fn image(value: &Value) -> Block {
    let source = &value["source"];
    let body = source["data"]
        .as_str()
        .and_then(|s| STANDARD.decode(s).ok())
        .unwrap_or_default();
    Block::Image {
        mime: text(source, "media_type").unwrap_or_else(|| "application/octet-stream".into()),
        data: BlobRef {
            blake3: *blake3::hash(&body).as_bytes(),
            len: body.len() as u64,
        },
    }
}
fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(str::to_owned)
}
fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
fn read_line<R: BufRead>(reader: &mut R) -> io::Result<(Vec<u8>, u64, bool, bool)> {
    let mut bytes = vec![];
    let mut count = reader
        .by_ref()
        .take((MAX_LINE_BYTES + 2) as u64)
        .read_until(b'\n', &mut bytes)? as u64;
    let mut terminated = bytes.last() == Some(&b'\n');
    let oversized = bytes.len() - usize::from(terminated) > MAX_LINE_BYTES;
    if oversized {
        bytes.clear();
        while !terminated {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                break;
            }
            let used = available
                .iter()
                .position(|&b| b == b'\n')
                .map_or(available.len(), |p| {
                    terminated = true;
                    p + 1
                });
            count = count
                .checked_add(used as u64)
                .ok_or_else(|| invalid("line size overflow"))?;
            reader.consume(used);
        }
    }
    Ok((bytes, count, oversized, terminated))
}
