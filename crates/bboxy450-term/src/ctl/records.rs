// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Record reads use the same bounded provider DTOs as the window's document loader.
use super::*;
use super::{Client, USAGE_PREFIX, Value};
use bbt_mux::knowledge::data_source as d;
use serde::Serialize;

fn read(client: &mut Client, method: &'static str, args: impl Serialize) -> Result<Value, String> {
    client
        .knowledge(&crate::knowledge::ReadCall {
            method,
            args: serde_json::to_value(args).map_err(|e| e.to_string())?,
        })
        .map_err(|e| {
            if e.code == "transport" {
                e.message
            } else {
                e.to_string()
            }
        })
}

pub(super) fn fetch(client: &mut Client, locator: String) -> Result<Value, String> {
    let reference = read(client, "resolve", d::ResolveRequest { locator })?;
    let reference =
        serde_json::from_value(reference).map_err(|e| format!("record reference: {e}"))?;
    read(client, "fetch", d::FetchRequest { reference })
}

pub(super) fn search_query(query: &str, record_type: Option<&str>) -> Result<String, String> {
    let mut query = query.to_owned();
    if let Some(record_type) = record_type {
        if record_type.is_empty() || record_type.chars().any(char::is_whitespace) {
            return Err(format!(
                "{USAGE_PREFIX}--type requires one nonempty record type"
            ));
        }
        query.push_str(" type:");
        query.push_str(record_type);
    }
    Ok(query)
}

pub(super) fn search(
    client: &mut Client,
    query: String,
    cursor: Option<String>,
) -> Result<Value, String> {
    read(
        client,
        "search",
        d::SearchRequest {
            query,
            cursor,
            limit: None,
        },
    )
}

pub(super) fn changes(client: &mut Client, cursor: String) -> Result<Value, String> {
    read(
        client,
        "changes_since",
        d::ChangesRequest {
            cursor,
            limit: None,
        },
    )
}

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("record", "write", "RecordWrite"),
    ("record", "commit", "KnowledgeCommit"),
    ("record", "fetch", "Knowledge"),
    ("record", "search", "Knowledge"),
    ("record", "changes", "Knowledge"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! record_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "record": [
            /// `record write <file|->`: the knowledge_write JSON from a file or stdin.
            RecordWrite {
                source: String,
            },
            RecordCommit { locators: Vec<String>, message: String, swarm: Option<Ulid> },
            /// Fetch a record through the window's portable knowledge read path.
            RecordFetch {
                locator: String,
            },
            RecordSearch {
                query: String,
                cursor: Option<String>,
            },
            RecordChanges {
                since: String,
            },
        ]] $($head)*);
    };
}
pub(super) use record_commands;

/// `ctl record …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    Ok(Some(match *words {
        ["record", "commit", ref locators @ ..] if !locators.is_empty() && locators.len() <= 32 => {
            Command::RecordCommit {
                locators: locators.iter().map(|s| s.to_string()).collect(),
                swarm: cx.swarm,
                message: flag("message")
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| {
                        format!("{USAGE_PREFIX}record commit requires --message TEXT")
                    })?,
            }
        }
        ["record", "write", source] => Command::RecordWrite {
            source: source.into(),
        },
        ["record", "fetch", locator] => Command::RecordFetch {
            locator: locator.into(),
        },
        ["record", "search", query] => Command::RecordSearch {
            query: records::search_query(query, flag("type").as_deref())?,
            cursor: flag("cursor"),
        },
        ["record", "changes"] => Command::RecordChanges {
            since: flag("since")
                .filter(|s| !s.is_empty())
                .ok_or_else(|| format!("{USAGE_PREFIX}record changes requires --since CURSOR"))?,
        },
        _ => return Ok(None),
    }))
}

/// The flat (pre-grammar) form of this noun's commands; the others are noun-verb only.
pub(super) fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    Ok(match command {
        Command::RecordWrite { source } => {
            let mut text = String::new();
            if source == "-" {
                std::io::stdin().read_to_string(&mut text)
            } else {
                std::fs::File::open(&source).and_then(|mut f| f.read_to_string(&mut text))
            }
            .map_err(|e| format!("record write {source}: {e}"))?;
            let args = serde_json::from_str(&text)
                .map_err(|e| format!("record write {source}: not JSON: {e}"))?;
            let mut row = client.record_write(args)?;
            row["kind"] = json!("record");
            vec![row]
        }
        _ => {
            return Err(no_flat_form());
        }
    })
}

/// This noun's noun-verb commands, run on the admitted connection; prints the one reply.
pub(super) fn run_verb(cx: super::VerbCx<'_>, command: Command) -> Result<(), String> {
    let super::VerbCx {
        mut client,
        out,
        noun,
        verb,
        id,
        human,
        ..
    } = cx;
    let data = match command {
        Command::RecordCommit {
            locators,
            message,
            swarm,
        } => client.knowledge_commit(locators, message, swarm)?,
        Command::RecordFetch { locator } => records::fetch(&mut client, locator)?,
        Command::RecordSearch { query, cursor } => records::search(&mut client, query, cursor)?,
        Command::RecordChanges { since } => records::changes(&mut client, since)?,
        command => super::flat_reply(&mut client, command, verb)?,
    };
    super::print_reply(out, noun, verb, id, data, human)
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "record write -",
    "record commit one --message example",
    "record fetch #1",
    "record search example",
    "record changes --since 0",
];
