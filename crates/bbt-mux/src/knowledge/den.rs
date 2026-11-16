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
//! in the file and have no metadata. A machine `spec` is read-only here: its
//! name is its title and its body is the source in a fenced block.
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

/// The author's part of a header record's source: its fields as metadata and
/// its prose. `Err` is the source's diagnostics, ready to return to a writer.
pub(crate) fn source(text: &str) -> Result<(Value, String), String> {
    let record = bboxy450_den::load(text)
        .map_err(|diagnostics| refusal(&diagnostics))?
        .remove(0);
    Ok((Value::Object(authored(&record)), bboxy450_den::body(&record)))
}

/// Reads a `.den` file as the host's (metadata, body).
pub(crate) fn parse(text: &str) -> Result<(Value, String), String> {
    let record = match bboxy450_den::load(text) {
        Ok(mut records) => records.remove(0),
        Err(diagnostics) => {
            // A `spec` with a state machine is not a header record. Reading it
            // is syntax only; `den_check` is what proves it.
            let outline = bboxy450_den::declarations(text).unwrap_or_default();
            let machine = diagnostics.iter().any(|d| d.code == "X190")
                || outline.iter().any(|d| !matches!(d.kind, "spec" | "field"));
            return match outline.first() {
                Some(spec) if machine && spec.kind == "spec" => Ok((
                    json!({"type":"spec","title":spec.name}),
                    format!("```den\n{}\n```\n", text.trim_end()),
                )),
                _ => Err(refusal(&diagnostics)),
            };
        }
    };
    let mut out = authored(&record);
    let mut body = bboxy450_den::body(&record);
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
    let mut out = format!("module {}\n\n{kind} {name} {{\n", module.join("."));
    for key in ["title", "intent"] {
        if let Some(text) = map.get(key).and_then(Value::as_str) {
            out.push_str(&format!("  {key} {}\n", quote(text)));
        }
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
        && is_name(status)
        && status == status.to_ascii_lowercase()
        && status.starts_with(|c: char| c.is_ascii_alphabetic())
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
        Some((text, (record, layout))) => {
            if record.kind != kind {
                return Err(format!(
                    "this file holds a {}, not a {kind}; a record keeps its type",
                    record.kind
                ));
            }
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
        None => generate(path, kind, map, core)?,
    };
    let (record, layout) = bboxy450_den::layout(&source).map_err(|d| refusal(&d))?;
    let own = authored(&record);
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
        let broken = parse("module a.B\ntask B {\n  title \"x\"\n}\n").unwrap_err();
        assert!(broken.contains("X354 line 2:1"), "{broken}");
    }
}
