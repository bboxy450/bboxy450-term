// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-log-converter: https://github.com/bboxy450/bboxy450-log-converter/blob/17ef2a173d7873c1c881544fd69f40245e0b2fa9/src/parse/codex.rs
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Incremental Codex JSONL reader. Checkpoints are complete native-record boundaries.
//! The caller owns the IR SessionId. Native ids and timestamps come from metadata.

use crate::ir::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Read, Seek, SeekFrom};

pub const PARSER_REVISION: u32 = 3;
pub const MAX_LINE_BYTES: usize = 64 * 1024 * 1024;

/// Save together with `Parser::session()`. Resume rereads only an incomplete tail.
/// A batch from `next_record` is atomic: checkpoint only after consuming the batch.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub offset: u64,
    pub line_ordinal: u64,
    pub next_seq: u64,
    pub previous: Option<RecordId>,
    pub turn: Option<TurnId>,
    pub parse_errors: u64,
}

pub struct Parser<R> {
    reader: R,
    session: Session,
    cursor: Cursor,
    retain_raw: bool,
}

impl<R: BufRead + Seek> Parser<R> {
    pub fn new(reader: R, id: SessionId) -> io::Result<Self> {
        Self::resume(
            reader,
            Session {
                id,
                source: Source {
                    provider: Provider::Codex,
                    native_id: String::new(),
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
        if session.source.provider != Provider::Codex
            || session.source.parser_revision != PARSER_REVISION
        {
            return Err(invalid("checkpoint is not from this Codex parser revision"));
        }
        reader.seek(SeekFrom::Start(cursor.offset))?;
        Ok(Self {
            reader,
            session,
            cursor,
            retain_raw: true,
        })
    }

    pub fn without_raw(mut self) -> Self {
        self.retain_raw = false;
        self
    }
    pub fn into_parts(self) -> (R, Session, Cursor) {
        (self.reader, self.session, self.cursor)
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Return one complete native record and any derived compaction messages.
    /// EOF or a non-newline-terminated tail returns None without advancing cursor.
    /// CR in CRLF is retained in Raw so the writer reproduces the exact bytes.
    pub fn next_record(&mut self) -> io::Result<Option<Vec<Event>>> {
        self.read_record(false)
    }

    /// Report a terminal incomplete tail once. Only call when append has ended.
    /// The discarded bytes are counted and hashed in a Partial Notice; no raw
    /// record is invented. Same-provider replay requires complete native records.
    pub fn finish_tail(&mut self) -> io::Result<Option<Vec<Event>>> {
        self.read_record(true)
    }

    fn read_record(&mut self, final_eof: bool) -> io::Result<Option<Vec<Event>>> {
        let read = read_line(&mut self.reader);
        let (mut line, count, oversized, terminated) = match read {
            Ok(result) => result,
            Err(error) => {
                self.reader.seek(SeekFrom::Start(self.cursor.offset))?;
                return Err(error);
            }
        };
        if count == 0 {
            return Ok(None);
        }
        if !terminated && !final_eof {
            self.reader.seek(SeekFrom::Start(self.cursor.offset))?;
            return Ok(None);
        }
        if oversized || !terminated {
            let key = format!("malformed-line:{}", self.cursor.line_ordinal);
            let id = EventId::native(Provider::Codex, &self.session.source.native_id, &key);
            let record = RecordId::native(Provider::Codex, &self.session.source.native_id, &key);
            let text = if oversized {
                format!("Skipped oversized Codex line: {count} bytes exceeds 64 MiB")
            } else {
                format!(
                    "Truncated Codex tail: {count} bytes, blake3 {}",
                    blake3::hash(&line)
                )
            };
            let event = self.event(
                id,
                (record, self.cursor.previous),
                None,
                Fidelity::Partial,
                None,
                EventKind::Notice {
                    level: Level::Warn,
                    text,
                },
            );
            self.cursor.parse_errors += 1;
            self.cursor.previous = Some(record);
            self.advance(count)?;
            return Ok(Some(vec![event]));
        }
        line.pop();
        let row = serde_json::from_slice::<Value>(&line);
        let referenced_parent = row.as_ref().ok().and_then(|row| {
            let item = &row["payload"];
            if row["type"] == "response_item"
                && matches!(
                    item["type"].as_str(),
                    Some("function_call_output" | "custom_tool_call_output")
                )
            {
                item.get("call_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            } else {
                None
            }
        });
        let ordinal = self.cursor.line_ordinal;
        let mut derived = vec![];
        let (kind, fidelity, at, key) = match row {
            Ok(row) => {
                let key = native_key(&row, ordinal);
                let at = row
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let mapped = self.map(&row, &key, &mut derived);
                match mapped {
                    Some((kind, fidelity)) => (kind, fidelity, at, key),
                    None => {
                        self.cursor.parse_errors += 1;
                        (EventKind::Unknown, Fidelity::Opaque, at, key)
                    }
                }
            }
            Err(error) => {
                self.cursor.parse_errors += 1;
                (
                    EventKind::Notice {
                        level: Level::Warn,
                        text: format!("Codex JSON parse error at line {ordinal}: {error}"),
                    },
                    Fidelity::Partial,
                    None,
                    format!("malformed-line:{ordinal}"),
                )
            }
        };
        let record = RecordId::native(Provider::Codex, &self.session.source.native_id, &key);
        let id = EventId::tagged(
            Provider::Codex,
            &self.session.source.native_id,
            &key,
            projection_tag(&kind),
        );
        let parent = referenced_parent
            .map(|key| RecordId::native(Provider::Codex, &self.session.source.native_id, &key))
            .or(self.cursor.previous);
        if matches!(kind, EventKind::TurnStart { .. })
            || derived
                .iter()
                .any(|(_, k)| matches!(k, EventKind::TurnStart { .. }))
        {
            self.cursor.turn = Some(TurnId(row_turn_id(&line).unwrap_or_else(|| key.clone())));
        }
        let raw = self.retain_raw.then_some(Raw {
            provider: Provider::Codex,
            bytes: line,
        });
        let mut events = vec![];
        for (id, kind) in derived
            .iter()
            .filter(|(_, kind)| matches!(kind, EventKind::TurnStart { .. }))
        {
            events.push(self.event(
                *id,
                (record, parent),
                at.clone(),
                Fidelity::Partial,
                None,
                kind.clone(),
            ));
        }
        events.push(self.event(
            id,
            (record, parent),
            at.clone(),
            if self.retain_raw {
                fidelity
            } else {
                Fidelity::Partial
            },
            raw,
            kind,
        ));
        for (id, kind) in derived
            .into_iter()
            .filter(|(_, kind)| !matches!(kind, EventKind::TurnStart { .. }))
        {
            events.push(self.event(
                id,
                (record, parent),
                at.clone(),
                Fidelity::Partial,
                None,
                kind,
            ));
        }
        if events
            .iter()
            .any(|e| matches!(e.kind, EventKind::TurnEnd { .. }))
        {
            self.cursor.turn = None;
        }
        self.cursor.previous = Some(record);
        self.advance(count)?;
        Ok(Some(events))
    }

    fn advance(&mut self, bytes: u64) -> io::Result<()> {
        self.cursor.offset = self
            .cursor
            .offset
            .checked_add(bytes)
            .ok_or_else(|| invalid("offset overflow"))?;
        self.cursor.line_ordinal += 1;
        Ok(())
    }

    fn event(
        &mut self,
        id: EventId,
        links: (RecordId, Option<RecordId>),
        at: Option<Timestamp>,
        fidelity: Fidelity,
        raw: Option<Raw>,
        kind: EventKind,
    ) -> Event {
        let event = Event {
            seq: self.cursor.next_seq,
            id,
            record: links.0,
            parent: links.1,
            turn: self.cursor.turn.clone(),
            at,
            fidelity,
            raw,
            kind,
        };
        self.cursor.next_seq += 1;
        event
    }

    fn map(
        &mut self,
        row: &Value,
        key: &str,
        derived: &mut Vec<(EventId, EventKind)>,
    ) -> Option<(EventKind, Fidelity)> {
        let payload = row.get("payload")?;
        let kind = match row.get("type")?.as_str()? {
            "session_meta" if self.cursor.line_ordinal == 0 => {
                self.session.source.native_id = payload
                    .get("id")
                    .or_else(|| payload.get("session_id"))?
                    .as_str()?
                    .to_owned();
                self.session.source.cli_version = text(payload, "cli_version");
                self.session.cwd = text(payload, "cwd").map(Into::into);
                self.session.created_at =
                    text(payload, "timestamp").or_else(|| text(row, "timestamp"));
                self.session.git = payload.get("git").map(|g| Git {
                    branch: text(g, "branch"),
                    commit: text(g, "commit_hash"),
                    repo_url: text(g, "repository_url"),
                });
                let instructions = payload
                    .get("base_instructions")
                    .and_then(|v| v.as_str().or_else(|| v.get("text")?.as_str()));
                match instructions {
                    Some(s) => EventKind::Message {
                        role: Role::System,
                        blocks: vec![Block::Text { text: s.to_owned() }],
                    },
                    None => EventKind::Native {
                        provider: Provider::Codex,
                        record_type: "session_meta".into(),
                    },
                }
            }
            "turn_context" => {
                let changes = payload.as_object()?.clone().into_iter().collect();
                derived.push((
                    EventId::tagged(
                        Provider::Codex,
                        &self.session.source.native_id,
                        key,
                        "context",
                    ),
                    EventKind::Context { changes },
                ));
                return Some((
                    EventKind::Native {
                        provider: Provider::Codex,
                        record_type: "turn_context".into(),
                    },
                    Fidelity::Opaque,
                ));
            }
            "response_item" => {
                return response(payload).map(|kind| {
                    let fidelity = if matches!(
                        kind,
                        EventKind::Reasoning {
                            sealed: Some(_),
                            ..
                        } | EventKind::Compaction { .. }
                    ) {
                        Fidelity::Opaque
                    } else {
                        Fidelity::Partial
                    };
                    (kind, fidelity)
                });
            }
            "event_msg" => {
                let typ = payload.get("type")?.as_str()?;
                if let Some((tag, kind)) = turn_event(payload) {
                    derived.push((
                        EventId::tagged(Provider::Codex, &self.session.source.native_id, key, tag),
                        kind,
                    ));
                }
                if typ == "token_count"
                    && let Some(kind) = usage(payload)
                {
                    derived.push((
                        EventId::tagged(
                            Provider::Codex,
                            &self.session.source.native_id,
                            key,
                            "usage",
                        ),
                        kind,
                    ));
                }
                return Some((
                    EventKind::Native {
                        provider: Provider::Codex,
                        record_type: format!("event_msg/{typ}"),
                    },
                    Fidelity::Opaque,
                ));
            }
            "compacted" => {
                let history = payload.get("replacement_history").and_then(Value::as_array);
                let mut replacement = vec![];
                if let Some(history) = history {
                    for (index, item) in history.iter().enumerate() {
                        if item.get("type").and_then(Value::as_str) == Some("message") {
                            let id = EventId::tagged(
                                Provider::Codex,
                                &self.session.source.native_id,
                                key,
                                &format!("replacement-{index}"),
                            );
                            derived.push((id, response(item).unwrap_or(EventKind::Unknown)));
                            replacement.push(id);
                        }
                    }
                }
                let sealed = payload
                    .get("compaction")
                    .and_then(|v| seal(v, SealedForm::EncryptedCompaction))
                    .or_else(|| seal(payload, SealedForm::EncryptedCompaction))
                    .or_else(|| {
                        history?.iter().find_map(|v| {
                            if v.get("type")?.as_str()? == "compaction" {
                                seal(v, SealedForm::EncryptedCompaction)
                            } else {
                                None
                            }
                        })
                    });
                return Some((
                    EventKind::Compaction {
                        summary: text(payload, "message").map(|s| vec![Block::Text { text: s }]),
                        replaced_through: None,
                        replacement,
                        sealed,
                    },
                    Fidelity::Opaque,
                ));
            }
            "token_usage_record" | "world_state" | "inter_agent_communication_metadata" => {
                return Some((
                    EventKind::Native {
                        provider: Provider::Codex,
                        record_type: row["type"].as_str()?.into(),
                    },
                    Fidelity::Opaque,
                ));
            }
            _ => return None,
        };
        Some((kind, Fidelity::Partial))
    }
}

fn row_turn_id(line: &[u8]) -> Option<String> {
    let row: Value = serde_json::from_slice(line).ok()?;
    text(row.get("payload")?, "turn_id")
}

// Only an item's own identity is used. An output's call_id is a foreign key to
// its call, not its own id; using it again would collide with the call event.
pub(crate) fn native_key(row: &Value, ordinal: u64) -> String {
    let item = &row["payload"];
    if row["type"] == "response_item" {
        let id = match item["type"].as_str() {
            Some("function_call" | "custom_tool_call") => item.get("call_id"),
            _ => item.get("id"),
        };
        if let Some(id) = id.and_then(Value::as_str) {
            return id.to_owned();
        }
    }
    row.get("ordinal")
        .and_then(|n| {
            n.as_str()
                .map(str::to_owned)
                .or_else(|| n.as_u64().map(|n| n.to_string()))
        })
        .unwrap_or_else(|| ordinal.to_string())
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::to_owned)
}

fn seal(value: &Value, form: SealedForm) -> Option<Sealed> {
    // lc-ir Provider::Codex denotes OpenAI-native reasoning; OpenAI is not a
    // separate IR provider. The opaque wire string is retained without decoding.
    Some(Sealed {
        provider: Provider::Codex,
        form,
        bytes: value
            .get("encrypted_content")?
            .as_str()?
            .as_bytes()
            .to_vec(),
    })
}

fn response(item: &Value) -> Option<EventKind> {
    Some(match item.get("type")?.as_str()? {
        "message" => EventKind::Message {
            role: match item.get("role")?.as_str()? {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "developer" => Role::Developer,
                "system" => Role::System,
                _ => Role::Unknown,
            },
            blocks: blocks(item.get("content")?)?,
        },
        "reasoning" => EventKind::Reasoning {
            text: item.get("content").and_then(Value::as_array).map(|a| {
                a.iter()
                    .filter_map(|v| v.get("text")?.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
            summary: item.get("summary").and_then(Value::as_array).map(|a| {
                a.iter()
                    .filter_map(|v| v.get("text")?.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            }),
            sealed: seal(item, SealedForm::EncryptedReasoning),
        },
        "compaction" => EventKind::Compaction {
            summary: None,
            replaced_through: None,
            replacement: vec![],
            sealed: seal(item, SealedForm::EncryptedCompaction),
        },
        "function_call" | "custom_tool_call" => {
            let name = text(item, "name")?;
            let input = if item["type"] == "function_call" {
                let arguments = item.get("arguments")?.as_str()?;
                serde_json::from_str(arguments)
                    .map(ToolInput::Json)
                    .unwrap_or_else(|_| ToolInput::Text(arguments.to_owned()))
            } else {
                ToolInput::Text(text(item, "input")?)
            };
            EventKind::ToolCall {
                call_id: text(item, "call_id")?,
                tool_kind: tool_kind(&name),
                name,
                namespace: text(item, "namespace"),
                input,
            }
        }
        "function_call_output" | "custom_tool_call_output" => EventKind::ToolResult {
            call_id: text(item, "call_id")?,
            is_error: item
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            blocks: blocks(item.get("output")?)?,
        },
        _ => return None,
    })
}

fn blocks(value: &Value) -> Option<Vec<Block>> {
    if let Some(text) = value.as_str() {
        return Some(vec![Block::Text {
            text: text.to_owned(),
        }]);
    }
    value
        .as_array()?
        .iter()
        .map(|v| {
            Some(match v.get("type")?.as_str()? {
                "input_text" | "output_text" | "text" => Block::Text {
                    text: text(v, "text")?,
                },
                "input_image" => {
                    // Only inline bodies have a content hash here. Remote images are
                    // not fetched; raw retains their URI and Block::Unknown marks loss.
                    let image = v.get("image_url").and_then(Value::as_str).unwrap_or("");
                    if let Some((mime, body)) = image
                        .strip_prefix("data:")
                        .and_then(|s| s.split_once(";base64,"))
                    {
                        let body = STANDARD.decode(body).ok()?;
                        Block::Image {
                            mime: mime.into(),
                            data: BlobRef {
                                blake3: *blake3::hash(&body).as_bytes(),
                                len: body.len() as u64,
                            },
                        }
                    } else {
                        Block::Unknown
                    }
                }
                _ => Block::Unknown,
            })
        })
        .collect()
}

fn turn_event(payload: &Value) -> Option<(&'static str, EventKind)> {
    Some(match payload.get("type")?.as_str()? {
        "task_started" | "turn_started" => (
            "turn-start",
            EventKind::TurnStart {
                origin: TurnOrigin::User,
            },
        ),
        "task_complete" | "task_completed" | "turn_complete" | "turn_completed" => (
            "turn-end",
            EventKind::TurnEnd {
                outcome: TurnOutcome::Completed,
            },
        ),
        "turn_aborted" => (
            "turn-end",
            EventKind::TurnEnd {
                outcome: TurnOutcome::Interrupted,
            },
        ),
        _ => return None,
    })
}

// Bounded allocation, including drain of an over-limit line. On an I/O error
// the caller restores the reader to the unchanged checkpoint before returning.
fn read_line<R: BufRead>(reader: &mut R) -> io::Result<(Vec<u8>, u64, bool, bool)> {
    let mut line = Vec::new();
    let mut count = reader
        .by_ref()
        .take((MAX_LINE_BYTES + 2) as u64)
        .read_until(b'\n', &mut line)? as u64;
    let mut terminated = line.last() == Some(&b'\n');
    let oversized = line.len() - usize::from(terminated) > MAX_LINE_BYTES;
    if oversized {
        line.clear();
        while !terminated {
            let available = reader.fill_buf()?;
            if available.is_empty() {
                break;
            }
            let used = available
                .iter()
                .position(|b| *b == b'\n')
                .map_or(available.len(), |i| {
                    terminated = true;
                    i + 1
                });
            count = count
                .checked_add(used as u64)
                .ok_or_else(|| invalid("line size overflow"))?;
            reader.consume(used);
        }
    }
    Ok((line, count, oversized, terminated))
}

fn projection_tag(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Message { .. }
        | EventKind::Reasoning { .. }
        | EventKind::ToolCall { .. }
        | EventKind::ToolResult { .. } => "block-0",
        EventKind::Usage { .. } => "usage",
        EventKind::Context { .. } => "context",
        EventKind::TurnStart { .. } => "turn-start",
        EventKind::TurnEnd { .. } => "turn-end",
        _ => "",
    }
}

fn usage(payload: &Value) -> Option<EventKind> {
    let info = payload.get("info")?;
    let counters = info
        .get("last_token_usage")
        .filter(|v| v.is_object())
        .or_else(|| info.get("total_token_usage"))?;
    if !counters.is_object() {
        return None;
    }
    let number = |keys: &[&str]| -> Option<u64> {
        for key in keys {
            if let Some(n) = counters.get(key) {
                return n.as_u64();
            }
        }
        Some(0)
    };
    Some(EventKind::Usage {
        input: number(&["input_tokens"])?,
        output: number(&["output_tokens"])?,
        cache_read: number(&["cached_input_tokens", "cache_read_input_tokens"])?,
        cache_write: number(&["cache_write_input_tokens", "cache_creation_input_tokens"])?,
        reasoning: number(&["reasoning_output_tokens"])?,
    })
}

fn tool_kind(name: &str) -> ToolKind {
    match name {
        "exec" | "exec_command" | "shell" => ToolKind::Execute,
        "apply_patch" => ToolKind::Edit,
        _ => ToolKind::Other,
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
