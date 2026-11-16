// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The one place a knowledge record becomes Den source and back: [`parse`]
//! reads a `.den` file as the host's (metadata, body), [`render`] writes that
//! pair as Den. The round trip is exact, because signatures cover the pair.
//!
//! | Den | Host |
//! |---|---|
//! | record keyword | `type` |
//! | `title`, `intent` and any other one-line field | the same key, as text (an integer stays a number, a list is an array) |
//! | `verify $ command ~> exit N` | `verify`, and `verify_exit` when written |
//! | `paths "a", "b"` | `allowed_paths`, always an array |
//! | `status Word rest` | `status` (lowercase) and `status_note` |
//! | `serves`, `realizes`, `decides`, `amends` | `links`: `{rel, to}` with the imported path joined by `/` |
//! | `brief`, `body`, `steps` text | the body, joined by a blank line |
//! | `managed { key = json"..." }` | `key`, decoded from the JSON text; it overrides the author's field of that name |
//! | `managed { status = Open }` | a task's `status` (lowercase) |
//! | `managed { other = json"{...}" }` | every key that is not a Den name |
//! | `managed { body_tail = json"..." }` | the white space that ends the body |
//!
//! Nested blocks and repeated lines (`staff`, `work`, `channel`, `gate`) stay
//! in the file and have no metadata. A machine `spec` is a record too: its
//! `title` (else its name) and `intent` are its metadata with its `managed`
//! block, its body is its source without that block in a fenced block, and
//! the host writes only its `managed` block.
use bboxy450_den::{Diagnostic, Field, Record, Value as Den};
use serde_json::{Map, Value, json};

const RELATIONS: [&str; 4] = ["serves", "realizes", "decides", "amends"];
const TASK_STATUS: [&str; 8] = [
    "open", "claimed", "running", "reported", "accepted", "rejected", "blocked", "done",
];
const KINDS: [&str; 11] = [
    "spec",
    "interface",
    "invariant",
    "epic",
    "task",
    "report",
    "e2e",
    "decision",
    "proposal",
    "bug",
    "recipe",
];
const SPACE: [char; 4] = [' ', '\t', '\n', '\r'];

pub(crate) fn is_path(path: &str) -> bool {
    path.ends_with(".den")
}
/// True when a write's body opens like Den source rather than like prose.
pub(crate) fn opens_like_source(body: &str) -> bool {
    let word = body
        .trim_start()
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .unwrap_or_default();
    word == "module" || KINDS.contains(&word)
}
/// Each diagnostic with its code, line, message and repair, one per line.
fn refusal(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| {
            format!(
                "{} line {}:{}: {}. Repair: {}",
                d.code,
                d.line,
                d.column,
                d.message.trim_end_matches('.'),
                d.repair
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn is_prose(field: &Field) -> bool {
    matches!(field.name.as_str(), "brief" | "body" | "steps")
        && matches!(field.value, Den::Text(_) | Den::Block(_))
}
fn items(value: &Den) -> &[Den] {
    match value {
        Den::List(items) => items,
        other => std::slice::from_ref(other),
    }
}
fn is_name(key: &str) -> bool {
    key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The metadata the author's own fields state, without the `managed` block.
fn authored(record: &Record) -> Map<String, Value> {
    let mut out = Map::new();
    let mut links = Vec::new();
    for field in &record.fields {
        let name = field.name.as_str();
        if matches!(field.value, Den::Nested { .. })
            || matches!(name, "channel" | "gate")
            || is_prose(field)
        {
            continue;
        }
        let text = field.value.text();
        match (name, &field.value) {
            (relation, value) if RELATIONS.contains(&relation) => {
                for item in items(value) {
                    let to = match item {
                        Den::Reference(path) => record
                            .imports
                            .iter()
                            .find(|import| {
                                *import == path
                                    || (path.len() == 1 && import.last() == path.first())
                            })
                            .unwrap_or(path)
                            .join("/"),
                        other => other.text(),
                    };
                    links.push(json!({"rel":relation,"to":to}));
                }
            }
            ("paths", value) => {
                let paths: Vec<_> = items(value).iter().map(Den::text).collect();
                out.insert("allowed_paths".into(), json!(paths));
            }
            ("verify", Den::Command { exit: Some(exit), .. }) if record.kind == "report" => {
                out.insert("verify".into(), json!({"command":text,"exit_code":exit}));
            }
            ("verify", Den::Command { exit, .. }) => {
                out.insert("verify".into(), json!(text));
                if let Some(exit) = exit {
                    out.insert("verify_exit".into(), json!(exit));
                }
            }
            ("status", _) => {
                let (status, note) = text.split_once(char::is_whitespace).unwrap_or((&text, ""));
                out.insert("status".into(), json!(status.to_ascii_lowercase()));
                if !note.trim().is_empty() {
                    out.insert("status_note".into(), json!(note.trim()));
                }
            }
            (_, Den::Integer(number)) => {
                out.insert(name.into(), json!(number));
            }
            (_, Den::List(list)) => {
                let list: Vec<_> = list.iter().map(Den::text).collect();
                out.insert(name.into(), json!(list));
            }
            _ => {
                out.insert(name.into(), json!(text));
            }
        }
    }
    if !links.is_empty() {
        out.insert("links".into(), json!(links));
    }
    out.insert("type".into(), json!(record.kind));
    out
}

/// [`authored`], and a machine spec's name as its title when it has none.
fn own(record: &Record) -> Map<String, Value> {
    let mut out = authored(record);
    if record.machine {
        out.entry("title").or_insert_with(|| json!(record.name));
    }
    out
}
/// A machine spec's body: its source without the `managed` block, fenced.
fn machine_body(text: &str) -> String {
    format!(
        "```den\n{}\n```\n",
        bboxy450_den::unmanaged(text).trim_end()
    )
}
/// The record a source holds. A header record must be sound; a `spec` with a
/// state machine is read for its syntax only: `den_check` is what proves it.
fn record(text: &str) -> Result<Record, String> {
    match bboxy450_den::load(text) {
        Ok(mut records) => Ok(records.remove(0)),
        Err(diagnostics) => match bboxy450_den::layout(text) {
            Ok((record, _)) if record.machine => Ok(record),
            Err(machine) if diagnostics.iter().any(|d| d.code == "X190") => {
                Err(refusal(&machine))
            }
            _ => Err(refusal(&diagnostics)),
        },
    }
}
/// The author's part of a record's source: its fields as metadata and its
/// prose. `Err` is the source's diagnostics, ready to return to a writer.
pub(crate) fn source(text: &str) -> Result<(Value, String), String> {
    let record = record(text)?;
    let body = if record.machine {
        machine_body(text)
    } else {
        bboxy450_den::body(&record)
    };
    Ok((Value::Object(own(&record)), body))
}

/// A `.den` record lowered to the Markdown a YAML-header reader expects:
/// the host's metadata as the header, then the prose. Read-only; never written back.
pub fn markdown(text: &str) -> Result<String, String> {
    let (metadata, body) = parse(text)?;
    let header = serde_yaml::to_string(&metadata).map_err(|e| e.to_string())?;
    Ok(format!("---\n{header}---\n{body}"))
}

/// Reads a `.den` file as the host's (metadata, body).
pub(crate) fn parse(text: &str) -> Result<(Value, String), String> {
    let record = record(text)?;
    let mut out = own(&record);
    let mut body = if record.machine {
        machine_body(text)
    } else {
        bboxy450_den::body(&record)
    };
    for field in &record.managed {
        let name = field.name.as_str();
        let value = match &field.value {
            Den::Tagged { tag, value } if tag == "json" => serde_json::from_str::<Value>(value)
                .map_err(|error| format!("managed {name} is not JSON: {error}"))?,
            Den::Integer(number) => json!(number),
            value if name == "status" => json!(value.text().to_ascii_lowercase()),
            value => json!(value.text()),
        };
        match (name, value) {
            ("other", Value::Object(other)) => out.extend(other),
            ("body_tail", Value::String(tail)) => body.push_str(&tail),
            ("delegator", value @ Value::String(_))
                if !matches!(field.value, Den::Tagged { ref tag, .. } if tag == "json") =>
            {
                out.insert("delegated_by".into(), value);
            }
            (_, value) => {
                out.insert(name.into(), value);
            }
        }
    }
    Ok((Value::Object(out), body))
}

/// `"text"` on one line, with the escapes the Den loader reads.
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
/// Prose as a `"""` block indented under `pad`, or as one quoted line when a
/// block would not read back as exactly this text.
fn prose(text: &str, pad: &str) -> String {
    let indented = |line: &str| line.starts_with([' ', '\t']);
    let block = !text.is_empty()
        && !text.contains("\"\"\"")
        && !text.contains('\r')
        && text.lines().all(|line| line == line.trim_end())
        && text
            .lines()
            .any(|line| !line.is_empty() && !indented(line));
    if !block {
        return quote(text);
    }
    let mut out = String::from("\"\"\"\n");
    for line in text.lines() {
        if !line.is_empty() {
            out.push_str(pad);
            out.push_str("  ");
            out.push_str(line);
        }
        out.push('\n');
    }
    out.push_str(pad);
    out.push_str("  \"\"\"");
    out
}
fn line_start(source: &str, offset: usize) -> usize {
    source[..offset].rfind('\n').map_or(0, |n| n + 1)
}
/// The white space that indents the line holding `offset`.
fn pad(source: &str, offset: usize) -> &str {
    let line = &source[line_start(source, offset)..];
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}
/// Where a new top-level line goes: before the `managed` block, else before
/// the closing brace; and whether that point starts its own line.
fn insertion(source: &str, layout: &bboxy450_den::Layout) -> (usize, bool) {
    let target = layout.managed.map_or(layout.close, |(start, _)| start);
    let start = line_start(source, target);
    if source[start..target].trim().is_empty() {
        (start, true)
    } else {
        (target, false)
    }
}

/// A record locator (`[[tasks/x]]`, `.bboxy/tasks/x.md`, `#12`) as a Den path.
fn den_path(locator: &str) -> Option<String> {
    let inner = locator.trim();
    let inner = inner
        .strip_prefix("[[")
        .and_then(|rest| rest.strip_suffix("]]"))
        .unwrap_or(inner);
    let inner = inner.split('|').next().unwrap_or_default();
    let inner = inner.strip_prefix(".bboxy/").unwrap_or(inner);
    let inner = inner
        .strip_suffix(".md")
        .or_else(|| inner.strip_suffix(".den"))
        .unwrap_or(inner);
    let segments: Vec<_> = inner.split('/').collect();
    if inner.contains(['`', '\n', '\r']) || segments.iter().any(|s| s.is_empty()) {
        return None;
    }
    let segments: Vec<_> = segments
        .iter()
        .map(|s| if is_name(s) { (*s).to_owned() } else { format!("`{s}`") })
        .collect();
    Some(segments.join("."))
}
/// A status the record can state as one word: `open` is `status Open`.
fn plain_status(status: &str) -> bool {
    is_name(status)
        && status == status.to_ascii_lowercase()
        && status.starts_with(|c: char| c.is_ascii_alphabetic())
}
/// A one-line shell command a `$` field can hold exactly.
fn command(text: &str) -> Option<&str> {
    (!text.is_empty()
        && text == text.trim()
        && !text.contains(['\n', '\r'])
        && !text.contains("~> exit"))
    .then_some(text)
}
/// The fields of a kind the host knows as typed Den, beyond `title`, `intent`,
/// `verify`, `paths`, `status` and the prose: what a new record's checker
/// requires, and what reads well. The exact value of any key whose typed form
/// differs from the host's stays in `managed`, which overrides it on reading.
/// Returns the imports the links need and the field lines.
fn typed(kind: &str, map: &Map<String, Value>) -> (Vec<String>, Vec<String>) {
    let text = |key: &str| map.get(key).and_then(Value::as_str);
    let mut imports: Vec<String> = Vec::new();
    let mut lines = Vec::new();
    let mut link = |field: &str, targets: Vec<&str>, lines: &mut Vec<String>| {
        let paths: Vec<_> = targets.into_iter().filter_map(den_path).collect();
        if paths.is_empty() {
            return;
        }
        for path in &paths {
            if !imports.contains(path) {
                imports.push(path.clone());
            }
        }
        lines.push(format!("{field} {}", paths.join(", ")));
    };
    let strings = |key: &str| -> Option<Vec<&str>> {
        match map.get(key)? {
            Value::String(one) => Some(vec![one.as_str()]),
            Value::Array(items) if !items.is_empty() => {
                items.iter().map(Value::as_str).collect::<Option<Vec<_>>>()
            }
            _ => None,
        }
    };
    let tagged = |tag: &str, items: Vec<&str>| {
        items
            .iter()
            .map(|item| format!("{tag}{}", quote(item)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let words: &[&str] = match kind {
        "e2e" => &["function", "group", "expected", "coverage", "result", "run_by", "last_run"],
        "invariant" => &["holds", "passes", "cost"],
        _ => &[],
    };
    for key in words {
        if let Some(value) = text(key) {
            lines.push(format!("{key} {}", quote(value)));
        }
    }
    match kind {
        "report" => {
            if let Some(task) = text("task") {
                link("task", vec![task], &mut lines);
            }
            if let Some(author) = text("author") {
                lines.push(match author.strip_prefix("seat:") {
                    Some(seat) => format!("author seat{}", quote(seat)),
                    None => format!("author {}", quote(author)),
                });
            }
            if let Some(commits) = strings("commits") {
                lines.push(format!("commits {}", tagged("sha", commits)));
            }
            let verify = &map.get("verify").cloned().unwrap_or_default();
            if let (Some(run), Some(exit)) = (
                verify["command"].as_str().and_then(command),
                verify["exit_code"].as_i64(),
            ) {
                lines.push(format!("verify $ {run} ~> exit {exit}"));
            }
            if let Some(previous) = text("supersedes") {
                link("supersedes", vec![previous], &mut lines);
            }
        }
        "e2e" => {
            if let Some(evidence) = strings("evidence") {
                lines.push(format!("evidence {}", tagged("path", evidence)));
            }
        }
        "decision" => {
            if let Some(date) = text("date") {
                lines.push(format!("date date{}", quote(date)));
            }
            // A decision's status is required: its first word, when that is a name.
            if let Some(status) = text("status")
                && !plain_status(status)
                && let Some(word) = status.split(|c: char| !c.is_ascii_alphanumeric()).next()
                && is_name(word)
            {
                let mut word = word.to_ascii_lowercase();
                word[..1].make_ascii_uppercase();
                lines.push(format!("status {word}"));
            }
            let links = map.get("links").and_then(Value::as_array);
            for relation in ["decides", "amends"] {
                let targets = links
                    .into_iter()
                    .flatten()
                    .filter(|link| link["rel"] == relation)
                    .filter_map(|link| link["to"].as_str())
                    .collect();
                link(relation, targets, &mut lines);
            }
        }
        "invariant" => {
            if let Some(check) = text("check").and_then(command) {
                lines.push(format!("check $ {check}"));
            }
        }
        _ => {}
    }
    (imports, lines)
}

/// A new record's source, before its `managed` block: the well-known text
/// fields typed, everything else left to `managed`.
fn generate(
    path: &str,
    kind: &str,
    map: &Map<String, Value>,
    prose_text: &str,
) -> Result<String, String> {
    if !KINDS.contains(&kind) {
        return Err(format!(
            "a {kind} record is not written in Den; use a .md path, or one of: {}",
            KINDS.join(", ")
        ));
    }
    let stem = path.strip_suffix(".den").unwrap_or(path);
    let mut module = Vec::new();
    for segment in stem.split('/') {
        if segment.contains(['`', '\n']) {
            return Err("a Den path cannot hold a backquote".into());
        }
        module.push(if is_name(segment) {
            segment.to_owned()
        } else {
            format!("`{segment}`")
        });
    }
    // `2026-11-15-den-kb` is DenKb: words from the first that starts with a letter.
    let name: String = stem
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .skip_while(|word| !word.starts_with(|c: char| c.is_ascii_alphabetic()))
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut word = word.to_owned();
            word[..1].make_ascii_uppercase();
            word
        })
        .collect();
    let name = if name.is_empty() { "Record" } else { &name };
    let (imports, typed) = typed(kind, map);
    let mut out = format!("module {}\n\n", module.join("."));
    for import in &imports {
        out.push_str(&format!("import {import}\n"));
    }
    if !imports.is_empty() {
        out.push('\n');
    }
    out.push_str(&format!("{kind} {name} {{\n"));
    for key in ["title", "intent"] {
        if let Some(text) = map.get(key).and_then(Value::as_str) {
            out.push_str(&format!("  {key} {}\n", quote(text)));
        }
    }
    for line in typed {
        out.push_str(&format!("  {line}\n"));
    }
    if let Some(command) = map.get("verify").and_then(Value::as_str)
        && !command.is_empty()
        && command == command.trim()
        && !command.contains(['\n', '\r'])
        && !command.contains("~> exit")
    {
        out.push_str(&format!("  verify $ {command}\n"));
    }
    if let Some(paths) = map.get("allowed_paths").and_then(Value::as_array)
        && !paths.is_empty()
        && paths.iter().all(Value::is_string)
    {
        let paths: Vec<_> = paths
            .iter()
            .map(|path| quote(path.as_str().unwrap_or_default()))
            .collect();
        out.push_str(&format!("  paths {}\n", paths.join(", ")));
    }
    if kind != "task"
        && let Some(status) = map.get("status").and_then(Value::as_str)
        && plain_status(status)
    {
        let mut word = status.to_owned();
        word[..1].make_ascii_uppercase();
        out.push_str(&format!("  status {word}\n"));
    }
    if !prose_text.is_empty() {
        let field = if kind == "task" { "brief" } else { "body" };
        out.push_str(&format!("  {field} {}\n", prose(prose_text, "  ")));
    }
    out.push_str("}\n");
    Ok(out)
}

/// Writes (metadata, body) as Den source for `path`. With a `previous` source
/// that loads, the author's text is kept byte for byte: only the prose value
/// (when the body changed) and the `managed` block are rewritten. Without
/// one, the whole file is generated. A pair that would not [`parse`] back
/// exactly is refused, never written lossily.
pub(crate) fn render(
    path: &str,
    metadata: &Value,
    body: &str,
    previous: Option<&str>,
) -> Result<String, String> {
    let map = metadata
        .as_object()
        .ok_or("frontmatter must be a mapping")?;
    let kind = map
        .get("type")
        .and_then(Value::as_str)
        .ok_or("a record needs a type")?;
    let core = body.trim_end_matches(SPACE);
    let tail = &body[core.len()..];
    let mut source = match previous.and_then(|text| Some((text, bboxy450_den::layout(text).ok()?)))
    {
        Some((_, (record, _))) if record.kind != kind => {
            return Err(format!(
                "this file holds a {}, not a {kind}; a record keeps its type",
                record.kind
            ));
        }
        // The host writes a machine's `managed` block and nothing else.
        Some((text, (record, _))) if record.machine => {
            if machine_body(text) != body {
                return Err(format!(
                    "{path} is a machine spec: its body is its Den source, which frontmatter cannot change; write the Den source as the body instead"
                ));
            }
            text.to_owned()
        }
        Some((text, (record, layout))) => {
            let fields: Vec<_> = (0..record.fields.len())
                .filter(|index| is_prose(&record.fields[*index]))
                .collect();
            let mut text = text.to_owned();
            if bboxy450_den::body(&record) != core {
                let field_pad = |index: usize| {
                    let (start, _) = layout.values[index];
                    pad(&text, start).to_owned()
                };
                match fields.as_slice() {
                    [] => {
                        let (at, own_line) = insertion(&text, &layout);
                        let field_pad = if own_line {
                            let inner = layout.managed.is_some();
                            let outer = pad(&text, at).to_owned();
                            if inner { outer } else { format!("{outer}  ") }
                        } else {
                            "  ".to_owned()
                        };
                        let line = format!("{field_pad}body {}\n", prose(core, &field_pad));
                        let line = if own_line { line } else { format!("\n{line}") };
                        text.insert_str(at, &line);
                    }
                    [.., last] => {
                        // Earlier prose fields stay; the rest of the body is the last one's.
                        let kept: String = fields[..fields.len() - 1]
                            .iter()
                            .map(|index| format!("{}\n\n", record.fields[*index].value.text()))
                            .collect();
                        let Some(rest) = core.strip_prefix(kept.as_str()) else {
                            return Err("this record has several prose fields and the new body changes an earlier one; write the Den source instead".into());
                        };
                        let (start, end) = layout.values[*last];
                        let value = prose(rest, &field_pad(*last));
                        text.replace_range(start..end, &value);
                    }
                }
            }
            text
        }
        None => {
            let machine = |text: &str| text.trim_start().starts_with("```den");
            // A previous machine that does not parse is never replaced by a header.
            let broken_machine = |text: &str| {
                bboxy450_den::load(text).is_err_and(|d| d.iter().any(|d| d.code == "X190"))
            };
            if previous.is_some_and(broken_machine)
                || (kind == "spec" && machine(core))
            {
                return Err(format!(
                    "{path}: a machine spec is written as its Den source, not from frontmatter; send the source as the body"
                ));
            }
            let generated = generate(path, kind, map, core)?;
            // A host-numbered record is history: stamp its number first, so the
            // check below sees the stamp that exempts it from new-record fields.
            // The full managed block replaces this one.
            match map.get("number") {
                Some(number) => {
                    let close = generated.rfind('}').ok_or("generated record has no close")?;
                    let mut stamped = generated.clone();
                    stamped.insert_str(
                        close,
                        &format!("\n  managed {{\n    number = json{}\n  }}\n", quote(&number.to_string())),
                    );
                    stamped
                }
                None => generated,
            }
        }
    };
    let (record, layout) = bboxy450_den::layout(&source).map_err(|d| refusal(&d))?;
    let own = own(&record);
    let tail = if record.machine { "" } else { tail };
    let mut lines = Vec::new();
    let mut other = Map::new();
    for (key, value) in map {
        if own.get(key) == Some(value) {
            continue;
        }
        let status = value
            .as_str()
            .filter(|status| key == "status" && kind == "task" && TASK_STATUS.contains(status));
        if let Some(status) = status {
            let mut word = status.to_owned();
            word[..1].make_ascii_uppercase();
            lines.push(format!("status = {word}"));
        } else if is_name(key) && !matches!(key.as_str(), "other" | "body_tail" | "delegator") {
            lines.push(format!("{key} = json{}", quote(&value.to_string())));
        } else {
            other.insert(key.clone(), value.clone());
        }
    }
    if !tail.is_empty() {
        lines.push(format!("body_tail = json{}", quote(&json!(tail).to_string())));
    }
    if !other.is_empty() {
        lines.push(format!(
            "other = json{}",
            quote(&Value::Object(other).to_string())
        ));
    }
    let block = |pad: &str| {
        let mut block = String::from("managed {\n");
        for line in &lines {
            block.push_str(&format!("{pad}  {line}\n"));
        }
        block.push_str(pad);
        block.push('}');
        block
    };
    match layout.managed {
        Some((start, end)) => {
            let block = block(pad(&source, start));
            source.replace_range(start..end, &block);
        }
        None => {
            let (at, own_line) = insertion(&source, &layout);
            let inner = format!("{}  ", pad(&source, at));
            let text = if own_line {
                format!("\n{inner}{}\n", block(&inner))
            } else {
                format!("\n  {}\n", block("  "))
            };
            source.insert_str(at, &text);
        }
    }
    match parse(&source) {
        Ok((read, read_body)) if read == *metadata && read_body == body => Ok(source),
        Ok((read, _)) => {
            let empty = Map::new();
            let read = read.as_object().unwrap_or(&empty);
            let differing: Vec<_> = read
                .keys()
                .chain(map.keys())
                .filter(|key| read.get(*key) != map.get(*key))
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            Err(format!(
                "{path} cannot hold this record exactly ({}); a field the Den source states cannot be removed through frontmatter: write the Den source instead",
                if differing.is_empty() {
                    "body".to_owned()
                } else {
                    differing.join(", ")
                }
            ))
        }
        Err(diagnostics) => Err(diagnostics),
    }
}

/// The Den source of a Markdown record with the same (metadata, body), so
/// every signature over that pair stays valid. `path` is the Markdown path,
/// root-relative; the Den path is the same with `.den`. Refused, naming the
/// first differing key, when Den cannot hold the record exactly.
pub fn convert(markdown: &str, path: &str) -> Result<String, String> {
    let document = crate::index::Document::parse(path, markdown);
    if let Some(error) = document.error {
        return Err(error);
    }
    let den = format!("{}.den", path.strip_suffix(".md").unwrap_or(path));
    let source = render(&den, &document.metadata, &document.body, None)?;
    let (metadata, body) = parse(&source)?;
    if metadata != document.metadata {
        let empty = Map::new();
        let (old, new) = (
            document.metadata.as_object().unwrap_or(&empty),
            metadata.as_object().unwrap_or(&empty),
        );
        let key = old
            .keys()
            .chain(new.keys())
            .filter(|key| old.get(*key) != new.get(*key))
            .min()
            .cloned()
            .unwrap_or_default();
        return Err(format!("the Den record differs at {key}"));
    }
    if body != document.body {
        return Err("the Den record differs in its body".into());
    }
    Ok(source)
}

/// The record directories `kb-convert` reads: the kinds Den holds.
const CONVERTIBLE: [&str; 10] = [
    "tasks",
    "reports",
    "decisions",
    "specs",
    "invariants",
    "interfaces",
    "bugs",
    "e2e",
    "epics",
    "proposals",
];
/// What `kb-convert` did with one directory.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Converted {
    pub converted: usize,
    /// Of `converted`, records whose signature already failed as Markdown.
    pub unsigned: usize,
    pub skipped: usize,
    pub refused: usize,
}
fn markdown_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<Result<_, _>>()?;
    entries.sort();
    for path in entries {
        let hidden = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'));
        if hidden || path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            markdown_files(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "md") {
            out.push(path);
        }
    }
    Ok(())
}
/// One file: `Ok(Some(den))` converts, `Ok(None)` is skipped with the reason
/// in `why`, `Err` is refused.
fn convert_file(text: &str, relative: &str) -> Result<Result<String, String>, String> {
    let document = crate::index::Document::parse(relative, text);
    if let Some(error) = &document.error {
        return Err(error.clone());
    }
    let kind = document.metadata["type"].as_str().unwrap_or_default();
    if !KINDS.contains(&kind) || kind == "recipe" {
        return Ok(Err(format!("a {kind} record stays Markdown")));
    }
    if relative.ends_with("/SKILL.md") {
        return Ok(Err("a skill stays Markdown".into()));
    }
    let signature = |doc: &crate::index::Document| {
        crate::provenance::queries::document_signature(doc)["status"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };
    // Conversion never makes provenance better or worse: a signature that
    // already fails converts and still fails; the host re-signs it on its next
    // write. The caller reports it.
    let before = signature(&document);
    let source = convert(text, relative)?;
    let den = format!("{}.den", relative.strip_suffix(".md").unwrap_or(relative));
    let after = signature(&crate::index::Document::parse(&den, &source));
    if after != before {
        return Err(format!("its signature is {before} as Markdown and {after} as Den"));
    }
    Ok(Ok(source))
}

/// `kb-convert --root DIR --kind KIND [--dry-run]`: each `ROOT/KIND/**/*.md`
/// record becomes the `.den` file beside it with the same (metadata, body) and
/// signature, and the `.md` goes. One line per file and a summary on `out`.
/// A dry run writes nothing.
pub fn convert_kind(
    root: &std::path::Path,
    kind: &str,
    dry_run: bool,
    out: &mut dyn std::io::Write,
) -> Result<Converted, String> {
    if !CONVERTIBLE.contains(&kind) {
        return Err(format!(
            "{kind} is not converted; Den kinds: {}",
            CONVERTIBLE.join(", ")
        ));
    }
    let mut files = Vec::new();
    markdown_files(&root.join(kind), &mut files).map_err(|e| format!("{kind}: {e}"))?;
    let mut done = Converted::default();
    let io = |e: std::io::Error| e.to_string();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .ok_or("a record path is not UTF-8")?
            .to_owned();
        let den = path.with_extension("den");
        let outcome = if den.exists() {
            Ok(Err("its .den twin already exists".to_owned()))
        } else {
            match std::fs::read_to_string(&path) {
                Ok(text) => convert_file(&text, &relative),
                Err(error) => Err(error.to_string()),
            }
        };
        let line = match outcome {
            Ok(Ok(source)) => {
                if !dry_run {
                    let temp = den.with_extension("den.tmp");
                    std::fs::write(&temp, &source).map_err(io)?;
                    std::fs::rename(&temp, &den).map_err(io)?;
                    std::fs::remove_file(&path).map_err(io)?;
                }
                done.converted += 1;
                let parsed = crate::index::Document::parse(
                    &format!("{}.den", relative.strip_suffix(".md").unwrap_or(&relative)),
                    &source,
                );
                if crate::provenance::queries::document_signature(&parsed)["status"] == "invalid" {
                    done.unsigned += 1;
                    "converted (its signature already failed as Markdown; re-signed on its next write)".to_owned()
                } else {
                    "converted".to_owned()
                }
            }
            Ok(Err(why)) => {
                done.skipped += 1;
                format!("skipped: {why}")
            }
            Err(why) => {
                done.refused += 1;
                format!("refused: {}", why.replace('\n', " "))
            }
        };
        writeln!(out, "{relative}: {line}").map_err(io)?;
    }
    writeln!(
        out,
        "kb-convert {kind}: {} converted ({} with a signature that already failed), {} skipped, {} refused{}",
        done.converted,
        done.unsigned,
        done.skipped,
        done.refused,
        if dry_run { " (dry run: nothing written)" } else { "" }
    )
    .map_err(io)?;
    Ok(done)
}

/// The `kb-convert` command line; the exit status: 0 clean, 1 when a record
/// was refused, 2 for a usage error.
pub fn run_cli(args: &[String]) -> i32 {
    let usage = "usage: bboxy450-term kb-convert --kind DIR [--root DIR] [--dry-run]";
    let (mut root, mut kind, mut dry_run) = (std::path::PathBuf::from(".bboxy"), None, false);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => match args.next() {
                Some(dir) => root = dir.into(),
                None => return usage_error(usage),
            },
            "--kind" => kind = args.next().cloned(),
            "--dry-run" => dry_run = true,
            _ => return usage_error(usage),
        }
    }
    let Some(kind) = kind else {
        return usage_error(usage);
    };
    match convert_kind(&root, &kind, dry_run, &mut std::io::stdout().lock()) {
        Ok(done) => i32::from(done.refused > 0),
        Err(error) => {
            eprintln!("kb-convert: {error}");
            2
        }
    }
}
fn usage_error(usage: &str) -> i32 {
    eprintln!("{usage}");
    2
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "Reads one checked-in Den fixture."
)]
mod tests {
    use super::*;

    fn round(path: &str, metadata: Value, body: &str, previous: Option<&str>) -> String {
        let source = render(path, &metadata, body, previous).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(parse(&source).unwrap(), (metadata, body.to_owned()), "{source}");
        source
    }

    #[test]
    fn every_metadata_value_round_trips_exactly() {
        let values = [
            json!("plain"),
            json!(""),
            json!("quote \" backslash \\ newline \n tab \t cr \r done"),
            json!("unicode é 漢字 🦊 \u{2028} \u{0} \u{7f}"),
            json!("\"\"\" } managed { x = 1 }"),
            json!(null),
            json!(true),
            json!(false),
            json!(0),
            json!(-7),
            json!(u64::MAX),
            json!(i64::MIN),
            json!(1.5),
            json!(1e300),
            json!([]),
            json!({}),
            json!([1, "two", null, [3.25, {"k": "v"}]]),
            json!({"nested": {"deep": [true, null, "x\ny"]}, "with-dash": 1, "": "empty key"}),
            json!({"principal":"seat:01M71GNVXV7ZHXMGM50M5SB2RA","public_key":"AAAA+/=="}),
        ];
        let bodies = [
            "One line.",
            "Brief.\n\n- a\n  - nested\n\n```rust\nfn main() {}\n```\n",
            "ends with spaces  \n\n\n",
            "  every line\n  is indented",
            "has \"\"\" quotes",
            "trailing space inside \nnext",
            "\n\nleading blank lines",
            "crlf\r\nline",
            "## Report from seat:X\n\n```json\n{\n  \"a\": 1\n}\n```\n",
        ];
        for (index, value) in values.iter().enumerate() {
            for body in bodies {
                let metadata = json!({
                    "type": "task", "title": "T \"quoted\"", "verify": "true", "status": "open",
                    "id": "01M71JZAEJPC72B5QT15N0E09Q", "number": 7,
                    "allowed_paths": ["crates/a/", "b \"c\""],
                    "sample": value, "with-dash": value, "other": value, "body_tail": value,
                    "delegator": value, "status_note": value, "type_": value,
                });
                let source = round("tasks/2026-11-15-round-trip.den", metadata, body, None);
                assert!(source.starts_with("module tasks.`2026-11-15-round-trip`\n\ntask RoundTrip {\n"), "{index}: {source}");
            }
        }
        // Well-known keys holding other types go to `managed` and still round trip.
        for value in values {
            for kind in ["epic", "spec", "decision", "bug", "recipe"] {
                let mut metadata = json!({"type": kind, "intent": "Why.", "status": "accepted",
                    "title": value, "verify": value, "allowed_paths": value, "links": value});
                if value.is_null() {
                    metadata["intent"] = json!("x\ny");
                }
                round(&format!("{kind}s/a_b.den"), metadata, "", None);
            }
        }
    }

    /// Every kind the host recommends in Den is generated from frontmatter and
    /// a body, its typed fields filled, and reads back exactly.
    #[test]
    fn every_den_kind_is_generated_from_frontmatter() {
        let stamp = json!({"id":"01M71JZAEJPC72B5QT15N0E09Q","number":44,"created":"2026-11-16T10:00:00Z",
            "provenance":{"digest":"sha256:ab","sig":"AAAA"},"links":[{"rel":"ref","to":"[[specs/a]]"}]});
        let cases = [
            ("reports/2026-11-16-landed.den", json!({"type":"report","title":"Landed",
                "task":"[[tasks/2026-11-15-den-task]]","author":"seat:01M71H0S7MDHJ0QTCQFPQYXJHW",
                "commits":["cd1f193b9cd1f193b9cd1f193b9cd1f193b9abcd","ae541605fae541605fae541605fae541605fabcd"],
                "verify":{"command":"cargo nextest run -p bboxy450-den","exit_code":0},
                "supersedes":"reports/2026-11-15-landed.md"}), "Changes.\n\nGaps: none.\n",
                &["import tasks.`2026-11-15-den-task`", "task tasks.`2026-11-15-den-task`", "author seat\"01M71H0S7MDHJ0QTCQFPQYXJHW\"", "commits sha\"cd1f", "verify $ cargo nextest run -p bboxy450-den ~> exit 0", "supersedes reports.`2026-11-15-landed`"][..]),
            ("reports/2026-11-16-empty.den", json!({"type":"report","title":"No commits","task":"#12",
                "author":"seat:01M71H0S7MDHJ0QTCQFPQYXJHW","commits":[],"verify":{"command":"true","exit_code":1}}), "", &["task `#12`"][..]),
            ("e2e/workspace.den", json!({"type":"e2e","title":"Install","function":"Managed install",
                "group":"deployment","status":"passed","coverage":"full for this installation",
                "last_run":"2026-11-06T05:11:11.148567+00:00","expected":"Both apps \"ready\"",
                "result":"Passed source checks","run_by":"operator","evidence":["/a/b.json","/c d.json"]}), "1. Install.\n",
                &["function \"Managed install\"", "status Passed", "evidence path\"/a/b.json\", path\"/c d.json\""][..]),
            ("e2e/one.den", json!({"type":"e2e","function":"F","expected":"E","evidence":"/only.json","steps":["a"]}), "", &["evidence path\"/only.json\""][..]),
            ("decisions/d7-den-default.den", json!({"type":"decision","title":"Den by default","status":"Accepted (operator)",
                "date":"2026-11-16","links":[{"rel":"decides","to":"[[specs/spec-language]]"},{"rel":"amends","to":"decisions/d6-x.md"},{"rel":"ref","to":"x"}]}),
                "Alternatives. Choice. Reason.\n", &["import specs.`spec-language`", "status Accepted", "decides specs.`spec-language`", "amends decisions.`d6-x`", "date date\"2026-11-16\""][..]),
            ("decisions/d8-plain.den", json!({"type":"decision","status":"accepted"}), "Body.", &["status Accepted"][..]),
            ("invariants/signed-main.den", json!({"type":"invariant","title":"every commit is signed","status":"active",
                "check":"python3 -I scripts/verify-signatures.py \"${SIGNED_RANGE:-HEAD}\"","cost":"the chain breaks"}), "",
                &["check $ python3 -I scripts/verify-signatures.py \"${SIGNED_RANGE:-HEAD}\"", "status Active", "cost \"the chain breaks\""][..]),
            ("interfaces/app-harness.den", json!({"type":"interface","title":"ICD: app ↔ harness","status":"baselined"}), "## Participants\n", &["status Baselined"][..]),
            ("proposals/research.den", json!({"type":"proposal","title":"Bounded research"}), "Motivation.\n", &[][..]),
            ("bugs/stale.den", json!({"type":"bug","title":"Stale catalogue","status":"reported","verify":"make land",
                "owner":"seat:01M6MADHFD19P4TKM7P7X6QESA","artifacts":["[[01M6MJ4RX3HH93AAS66QZY6KKP]]"]}), "Seen.\n", &["verify $ make land"][..]),
        ];
        for (path, mut metadata, body, expected) in cases {
            for (key, value) in stamp.as_object().unwrap() {
                metadata.as_object_mut().unwrap().entry(key.clone()).or_insert(value.clone());
            }
            let source = round(path, metadata.clone(), body, None);
            println!("--- {path} ---\n{source}");
            for line in expected {
                assert!(source.contains(line), "{path}: {line}\n{source}");
            }
            // Unsound output is refused, never written: each is sound.
            assert!(bboxy450_den::load(&source).is_ok(), "{path}\n{source}");
            // The host's later writes keep it exact.
            metadata["updated"] = json!("2026-11-16T11:00:00Z");
            round(path, metadata, body, Some(&source));
        }
        let missing = render("e2e/x.den", &json!({"type":"e2e","function":"F"}), "", None).unwrap_err();
        assert!(missing.contains("needs a expected"), "{missing}");
    }

    #[test]
    fn a_previous_source_keeps_the_authors_text_and_takes_the_new_managed_block() {
        let previous = "module tasks.`2026-11-09-den-checker-crate`\n\nimport specs.SpecLanguage\n\ntask DenCheckerCrate {\n  title  \"Den v1\"   // kept\n  serves SpecLanguage\n  verify $ cargo nextest run -p bboxy450-den\n  brief  \"\"\"\n    Build it.\n  \"\"\"\n  priority 2\n\n  managed {\n    id        = id\"01M71JZAEJPC72B5QT15N0E09Q\"\n    status    = Open\n    delegator = seat\"01M71GNVXV7ZHXMGM50M5SB2RA\"\n  }\n}\n";
        let (mut metadata, body) = parse(previous).unwrap();
        assert_eq!(
            metadata,
            json!({"type":"task","title":"Den v1","verify":"cargo nextest run -p bboxy450-den",
                "links":[{"rel":"serves","to":"specs/SpecLanguage"}],"priority":2,
                "id":"01M71JZAEJPC72B5QT15N0E09Q","status":"open",
                "delegated_by":"seat:01M71GNVXV7ZHXMGM50M5SB2RA"})
        );
        assert_eq!(body, "Build it.");
        metadata["status"] = json!("claimed");
        metadata["assignee"] = json!("seat:01M71GNVXV7ZHXMGM50M5SB2RB");
        metadata.as_object_mut().unwrap().remove("delegated_by");
        let claimed = round("tasks/x.den", metadata.clone(), &body, Some(previous));
        let author = previous.split("  managed {").next().unwrap();
        assert!(claimed.starts_with(author), "{claimed}");
        assert!(claimed.contains("    status = Claimed\n"), "{claimed}");
        assert!(!claimed.contains("delegator"), "{claimed}");
        // A report appends to the brief; an override of an author's field is managed.
        metadata["verify"] = json!("make test");
        let reported = format!("{body}\n\n## Report\n\ndone\n");
        let source = round("tasks/x.den", metadata.clone(), &reported, Some(&claimed));
        assert!(source.contains("  title  \"Den v1\"   // kept\n  serves SpecLanguage\n  verify $ cargo nextest run -p bboxy450-den\n  brief  \"\"\"\n    Build it.\n\n    ## Report\n\n    done\n    \"\"\"\n  priority 2\n"), "{source}");
        assert!(source.contains("verify = json\"\\\"make test\\\"\""), "{source}");
        // A field the source states cannot be dropped through metadata.
        metadata.as_object_mut().unwrap().remove("priority");
        let refused = render("tasks/x.den", &metadata, &reported, Some(&source)).unwrap_err();
        assert!(refused.contains("(priority)"), "{refused}");
        // A record keeps its kind, and an unsound result is refused with its code.
        metadata["type"] = json!("epic");
        assert!(render("tasks/x.den", &metadata, "", Some(&source)).unwrap_err().contains("keeps its type"));
        let unsound = render("tasks/y.den", &json!({"type":"task","title":"No verify"}), "Brief.", None).unwrap_err();
        assert!(unsound.starts_with("X354 line 3:1: a task needs a verify. Repair: "), "{unsound}");
        assert!(render("notes/y.den", &json!({"type":"note"}), "", None).unwrap_err().contains("not written in Den"));
    }

    #[test]
    fn a_body_is_added_to_a_record_without_prose_and_one_line_records_are_edited() {
        for previous in [
            "module epics.E\nepic E {\n  intent \"Why.\"\n}\n",
            "module epics.E\nepic E { intent \"Why.\" }\n",
            "module epics.E\nepic E { intent \"Why.\"; managed { id = id\"X\" } }\n",
            "module epics.E\nepic E {\n    intent \"Why.\"\n    body \"old\"\n    steps \"\"\"\n      1. one\n    \"\"\"\n    managed {}\n}\n",
        ] {
            let (mut metadata, body) = parse(previous).unwrap();
            metadata["number"] = json!(3);
            let source = round("epics/E.den", metadata.clone(), &format!("{body}\n\nMore.\n"), Some(previous));
            assert!(source.contains("intent \"Why.\""), "{source}");
            if !previous.contains("steps") {
                round("epics/E.den", metadata, "", Some(&source));
            }
        }
    }

    /// `kb-convert` over copies of real records (`tests/fixtures/kb-convert`):
    /// each is converted or skipped with a reason, the Index sees the same
    /// records, ids and numbers, every signature verifies, and a tampered
    /// record is refused and left alone.
    #[test]
    fn kb_convert_keeps_records_ids_numbers_and_signatures() {
        use crate::index::Index;
        use crate::store::sqlite::SqliteStore;
        use std::path::Path;
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/kb-convert");
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".bboxy");
        let kinds = ["tasks", "reports", "decisions", "specs", "interfaces"];
        for kind in kinds {
            std::fs::create_dir_all(root.join(kind)).unwrap();
            for entry in std::fs::read_dir(fixtures.join(kind)).unwrap() {
                let path = entry.unwrap().path();
                std::fs::copy(&path, root.join(kind).join(path.file_name().unwrap())).unwrap();
            }
        }
        // One signed record whose body was edited after signing.
        let honest = root.join("reports/2026-11-01-seat-launch-environment.md");
        let tampered = root.join("reports/2026-11-02-tampered.md");
        let text = std::fs::read_to_string(&honest).unwrap();
        std::fs::write(&tampered, text.replacen("\n---\n", "\n---\nEdited after signing.\n", 1)).unwrap();
        let tampered_text = std::fs::read_to_string(&tampered).unwrap();
        let seen = |index: &Index| -> std::collections::BTreeMap<String, (String, Value)> {
            index.documents.iter().map(|(path, doc)| {
                let stem = path.trim_end_matches(".md").trim_end_matches(".den").to_owned();
                (stem, (doc.id.clone(), doc.metadata["number"].clone()))
            }).collect()
        };
        let mut index = Index::open(&root, SqliteStore::open(&temp.path().join("db")).unwrap()).unwrap();
        index.scan().unwrap();
        let before = seen(&index);
        let signed: Vec<_> = index.documents.iter()
            .filter(|(_, doc)| crate::provenance::queries::document_signature(doc)["status"] == "valid")
            .map(|(path, _)| path.trim_end_matches(".md").to_owned()).collect();
        let status = |doc: &crate::index::Document| {
            crate::provenance::queries::document_signature(doc)["status"].as_str().unwrap_or_default().to_owned()
        };
        let statuses: std::collections::BTreeMap<String, String> = index.documents.iter()
            .map(|(path, doc)| (path.clone(), status(doc))).collect();
        assert!(signed.len() >= 4, "{statuses:?}");
        // A dry run writes nothing.
        let mut out = Vec::new();
        convert_kind(&root, "reports", true, &mut out).unwrap();
        assert!(!root.join("reports/2026-11-01-seat-launch-environment.den").exists());
        let mut totals = Converted::default();
        let mut lines = String::new();
        for kind in kinds {
            let mut out = Vec::new();
            let done = convert_kind(&root, kind, false, &mut out).unwrap();
            lines.push_str(&String::from_utf8(out).unwrap());
            totals.converted += done.converted;
            totals.skipped += done.skipped;
            totals.refused += done.refused;
            totals.unsigned += done.unsigned;
        }
        println!("{statuses:?}\n{lines}");
        assert!(lines.contains("reports/2026-11-02-tampered.md: converted (its signature already failed"), "{lines}");
        assert!(lines.contains("decisions/2026-10-22-architecture-round-1.md: skipped: a note record stays Markdown"), "{lines}");
        // Every file has one outcome. A signature that already failed (records
        // edited after signing, and the tampered one) converts, flagged, and still fails.
        let outcomes: Vec<_> = lines.lines().filter(|line| !line.starts_with("kb-convert ")).collect();
        assert_eq!(outcomes.len(), statuses.len(), "{lines}");
        for line in outcomes {
            let (path, outcome) = line.split_once(": ").unwrap();
            let flagged = outcome.starts_with("converted (its signature already failed");
            assert_eq!(flagged, statuses[path] == "invalid", "{line}");
            assert!(!outcome.starts_with("refused: "), "{line}");
            assert!(flagged || outcome == "converted" || outcome.len() > "skipped: ".len(), "{line}");
        }
        assert_eq!(totals.refused, 0);
        assert_eq!(totals.unsigned, statuses.values().filter(|s| *s == "invalid").count());
        assert_eq!(totals.converted + totals.skipped + totals.refused, statuses.len());
        assert!(totals.converted >= 7, "{lines}");
        // Never better: the tampered record's Den still fails its signature.
        let den_text = std::fs::read_to_string(root.join("reports/2026-11-02-tampered.den")).unwrap();
        let den_doc = crate::index::Document::parse("reports/2026-11-02-tampered.den", &den_text);
        assert_eq!(crate::provenance::queries::document_signature(&den_doc)["status"], "invalid");
        assert_eq!(crate::index::Document::parse("r.md", &tampered_text).body, den_doc.body);
        // The Index reads the same records, by id and number, all still signed.
        let mut index = Index::open(&root, SqliteStore::open(&temp.path().join("db2")).unwrap()).unwrap();
        index.scan().unwrap();
        assert_eq!(seen(&index), before);
        let den = index.documents.keys().filter(|path| path.ends_with(".den")).count();
        assert_eq!(den, totals.converted);
        for stem in &signed {
            let doc = index.documents.get(&format!("{stem}.den"))
                .or_else(|| index.documents.get(&format!("{stem}.md"))).unwrap();
            assert_eq!(crate::provenance::queries::document_signature(doc)["status"], "valid", "{stem}");
        }
        // Converting again finds nothing left to convert.
        let mut out = Vec::new();
        assert_eq!(convert_kind(&root, "tasks", false, &mut out).unwrap().converted, 0);
        assert!(convert_kind(&root, "notes", false, &mut Vec::new()).unwrap_err().contains("not converted"));
        // A numbered record is history: a field it never had does not block it.
        assert!(convert("---\nnumber: 1\ntype: task\ntitle: T\n---\nB\n", "tasks/t.md").is_ok());
    }

    #[test]
    fn a_machine_spec_reads_as_its_source() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../bboxy450-den/tests/sound/turnstile.den"
        ))
        .unwrap();
        let (metadata, body) = parse(&text).unwrap();
        assert_eq!(metadata["type"], "spec");
        assert!(metadata["title"].as_str().is_some_and(|title| !title.is_empty()));
        assert!(body.starts_with("```den\nmodule ") && body.ends_with("\n```\n"));
        // The host stamps it: only the managed block is new, and the body stays.
        let mut stamped = metadata.clone();
        stamped["id"] = json!("01M71JZAEJPC72B5QT15N0E09Q");
        stamped["number"] = json!(12);
        stamped["provenance"] = json!({"sig":"AAAA","digest":"sha256:x"});
        let written = round("specs/turnstile.den", stamped.clone(), &body, Some(&text));
        let close = text.trim_end().len() - 1;
        assert!(written.starts_with(&text[..close]), "{written}");
        assert!(written.contains("\n  managed {\n    id = json\"\\\"01M71JZAEJPC72B5QT15N0E09Q\\\"\"\n"), "{written}");
        assert_eq!(bboxy450_den::unmanaged(&written), text);
        let again = round("specs/turnstile.den", stamped.clone(), &body, Some(&written));
        assert_eq!(again, written);
        // A title the source states is the title; a new one is managed.
        stamped["title"] = json!("Turnstile, retitled");
        round("specs/turnstile.den", stamped.clone(), &body, Some(&written));
        // Frontmatter cannot change the machine, nor write one from nothing.
        let changed = render("specs/turnstile.den", &stamped, "```den\nmodule x\n```\n", Some(&written)).unwrap_err();
        assert!(changed.contains("is a machine spec"), "{changed}");
        let fresh = render("specs/new.den", &stamped, &body, None).unwrap_err();
        assert!(fresh.contains("written as its Den source"), "{fresh}");
        let broken_machine = text.replacen("state {", "state {{", 1);
        assert!(render("specs/turnstile.den", &stamped, &body, Some(&broken_machine)).unwrap_err().contains("written as its Den source"));
        // Its source is what a writer sends: metadata and body as parse reads them.
        assert_eq!(source(&written).unwrap(), source(&text).unwrap());
        assert_eq!(source(&text).unwrap(), (metadata, body));
        let broken = parse("module a.B\ntask B {\n  title \"x\"\n}\n").unwrap_err();
        assert!(broken.contains("X354 line 2:1"), "{broken}");
    }
}
