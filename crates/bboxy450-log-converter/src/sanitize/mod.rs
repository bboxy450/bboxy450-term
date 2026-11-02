// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

//! Adapted from bboxy450-harness src/redact.rs (scrub_import span merging):
//! https://github.com/bboxy450/bboxy450-harness/blob/04edebdc/src/redact.rs
//! Origin policy: https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
//! Deterministic d4947/lc-ir §6 pass, shared by every provider. Apply after
//! parsing, before persisting or exporting demo content. Raw JSON is rewritten
//! too; invalid raw is removed and fidelity becomes Partial. Opaque sealed
//! reasoning is the explicit lc-ir §5 exception: it stays byte-identical.
//!
//! Pass configured secret values with `Sanitizer::new`; `with_capability_file`
//! additionally validates the seat's per-launch file without exposing its path
//! or contents in diagnostics. No commands, profile lookup or network I/O.
use crate::ir::*;
use regex::Regex;
use serde_json::Value;
use std::{
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    sync::OnceLock,
};

pub const TOOL_OUTPUT_LIMIT: usize = 4096;
const CUT: &str = "\n[tool output truncated]";
#[derive(Default)]
pub struct Sanitizer {
    secrets: Vec<String>,
    native_id: Option<String>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum CapabilityRefusal {
    Unavailable,
    Symlink,
    NotRegular,
    Owner,
    Mode,
    Malformed,
}
impl std::fmt::Display for CapabilityRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "capability unavailable",
            Self::Symlink => "capability symlink refused",
            Self::NotRegular => "capability is not regular",
            Self::Owner => "capability owner refused",
            Self::Mode => "capability mode refused",
            Self::Malformed => "capability content refused",
        })
    }
}
impl std::error::Error for CapabilityRefusal {}
impl Sanitizer {
    pub fn new(secrets: impl IntoIterator<Item = String>) -> Self {
        Self {
            secrets: secrets.into_iter().filter(|s| !s.is_empty()).collect(),
            native_id: None,
        }
    }
    // Synchronous converter setup, in a child/preparation worker, never UI.
    #[allow(clippy::disallowed_methods)]
    pub fn with_capability_file(mut self, path: &Path) -> Result<Self, CapabilityRefusal> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|e| {
                if e.raw_os_error() == Some(libc::ELOOP) {
                    CapabilityRefusal::Symlink
                } else {
                    CapabilityRefusal::Unavailable
                }
            })?;
        let meta = file
            .metadata()
            .map_err(|_| CapabilityRefusal::Unavailable)?;
        if !meta.is_file() {
            return Err(CapabilityRefusal::NotRegular);
        }
        if meta.uid() != owner_uid()? {
            return Err(CapabilityRefusal::Owner);
        }
        if meta.mode() & 0o7777 != 0o600 {
            return Err(CapabilityRefusal::Mode);
        }
        let mut bytes = vec![];
        file.take(66)
            .read_to_end(&mut bytes)
            .map_err(|_| CapabilityRefusal::Unavailable)?;
        if bytes.len() != 65
            || bytes[64] != b'\n'
            || !bytes[..64]
                .iter()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
        {
            return Err(CapabilityRefusal::Malformed);
        }
        self.secrets.push(
            String::from_utf8(bytes[..64].to_vec()).map_err(|_| CapabilityRefusal::Malformed)?,
        );
        Ok(self)
    }
    pub fn session(&mut self, session: &mut Session) {
        self.native_id = Some(session.source.native_id.clone());
        session.id = fake_session(session.id);
        session.source.native_id = self.text(&session.source.native_id);
        for value in [&mut session.source.cli_version, &mut session.created_at]
            .into_iter()
            .flatten()
        {
            *value = self.text(value);
        }
        if let Some(cwd) = &session.cwd {
            session.cwd = Some(self.text(&cwd.to_string_lossy()).into());
        }
        if let Some(git) = &mut session.git {
            for text in [&mut git.branch, &mut git.commit, &mut git.repo_url]
                .into_iter()
                .flatten()
            {
                *text = self.text(text);
            }
        }
        for relation in &mut session.lineage {
            relation.session = fake_session(relation.session);
            relation.at = relation.at.map(fake_event);
            if let Some((_, native)) = &mut relation.native {
                *native = self.text(native);
            }
        }
    }
    pub fn event(&self, event: &mut Event) {
        // The named control removes this single post-conversion pass.
        self.sanitize_event(event);
    }
    fn sanitize_event(&self, event: &mut Event) {
        event.id = fake_event(event.id);
        event.record = fake_record(event.record);
        event.parent = event.parent.map(fake_record);
        if let Some(at) = &mut event.at {
            *at = self.text(at);
        }
        if let Some(turn) = &mut event.turn {
            turn.0 = self.text(&turn.0);
        }
        match &mut event.kind {
            EventKind::Message { blocks, .. } => self.blocks(blocks, false),
            EventKind::ToolCall {
                call_id,
                name,
                namespace,
                input,
                ..
            } => {
                *call_id = self.text(call_id);
                *name = self.text(name);
                if let Some(n) = namespace {
                    *n = self.text(n);
                }
                match input {
                    ToolInput::Json(v) => self.value(v),
                    ToolInput::Text(s) => *s = self.text(s),
                    _ => {}
                }
            }
            EventKind::ToolResult {
                call_id, blocks, ..
            } => {
                *call_id = self.text(call_id);
                self.blocks(blocks, true);
            }
            EventKind::Reasoning { text, summary, .. } => {
                for s in [text, summary].into_iter().flatten() {
                    *s = self.text(s);
                }
            }
            EventKind::Compaction {
                summary,
                replaced_through,
                replacement,
                ..
            } => {
                if let Some(blocks) = summary {
                    self.blocks(blocks, false);
                }
                *replaced_through = replaced_through.map(fake_record);
                for id in replacement {
                    *id = fake_event(*id);
                }
            }
            EventKind::Context { changes } => {
                let mut v = serde_json::to_value(&*changes).expect("JSON map");
                self.value(&mut v);
                *changes = serde_json::from_value(v).expect("JSON map");
            }
            EventKind::Notice { text, .. } => *text = self.text(text),
            EventKind::Subagent { session, .. } => *session = fake_session(*session),
            EventKind::Native { record_type, .. } => *record_type = self.text(record_type),
            _ => {}
        }
        if let Some(raw) = &mut event.raw {
            match serde_json::from_slice::<Value>(&raw.bytes) {
                Ok(mut value) => {
                    self.value(&mut value);
                    value.sort_all_objects();
                    raw.bytes = serde_json::to_vec(&value).expect("JSON value");
                }
                Err(_) => {
                    event.raw = None;
                    event.fidelity = Fidelity::Partial;
                }
            }
        }
    }
    fn blocks(&self, blocks: &mut [Block], output: bool) {
        let mut remaining = TOOL_OUTPUT_LIMIT;
        for block in blocks.iter_mut() {
            match block {
                Block::Text { text } => {
                    *text = self.text(text);
                    if output {
                        *text = limited(text, remaining);
                        remaining = remaining.saturating_sub(text.len());
                    }
                }
                Block::Image { mime, .. } | Block::Data { mime, .. } => *mime = self.text(mime),
                _ => {}
            }
        }
    }
    fn value(&self, value: &mut Value) {
        match value {
            Value::String(s) => *s = self.text(s),
            Value::Array(a) => {
                for v in a {
                    self.value(v)
                }
            }
            Value::Object(o) => {
                // Sealed provider bytes remain opaque even in retained raw.
                let opaque = matches!(
                    o.get("type").and_then(Value::as_str),
                    Some("thinking" | "redacted_thinking" | "reasoning" | "compaction")
                );
                let image = matches!(
                    o.get("type").and_then(Value::as_str),
                    Some("image" | "input_image")
                );
                if image {
                    let source = o.get("source");
                    let mime = source
                        .and_then(|s| s.get("media_type"))
                        .cloned()
                        .unwrap_or(Value::String("application/octet-stream".into()));
                    let bytes = source
                        .and_then(|s| s.get("data"))
                        .and_then(Value::as_str)
                        .map(str::len)
                        .unwrap_or(0);
                    o.remove("source");
                    o.remove("image_url");
                    o.remove("data");
                    o.insert("mime".into(), mime);
                    o.insert("encoded_bytes".into(), bytes.into());
                }
                let tool_output = o.get("type").and_then(Value::as_str).is_some_and(|s| {
                    matches!(
                        s,
                        "tool_result" | "function_call_output" | "custom_tool_call_output"
                    )
                });
                let old = std::mem::take(o);
                for (key, mut v) in old {
                    if !(opaque
                        && matches!(key.as_str(), "signature" | "encrypted_content" | "data"))
                    {
                        if key.eq_ignore_ascii_case("authorization")
                            || key.eq_ignore_ascii_case("proxy-authorization")
                        {
                            v = Value::String("<redacted>".into());
                        } else if matches!(
                            key.to_ascii_lowercase().as_str(),
                            "host" | "hostname" | "ssh_host"
                        ) && v.is_string()
                        {
                            v = Value::String("<host>".into());
                        } else {
                            self.value(&mut v);
                        }
                    }
                    if tool_output && matches!(key.as_str(), "content" | "output") {
                        let mut remaining = TOOL_OUTPUT_LIMIT;
                        truncate_value(&mut v, &mut remaining);
                    }
                    if matches!(key.as_str(), "stdout" | "stderr" | "toolUseResult") {
                        let mut remaining = TOOL_OUTPUT_LIMIT;
                        truncate_value(&mut v, &mut remaining);
                    }
                    o.insert(self.text(&key), v);
                }
            }
            _ => {}
        }
    }
    pub fn text(&self, text: &str) -> String {
        let mut spans = vec![];
        for secret in &self.secrets {
            spans.extend(
                text.match_indices(secret)
                    .map(|(a, _)| (a, a + secret.len())),
            );
        }
        for pattern in patterns().iter().take(3) {
            spans.extend(pattern.find_iter(text).map(|m| (m.start(), m.end())));
        }
        spans.sort_unstable();
        let mut merged: Vec<(usize, usize)> = vec![];
        for (start, end) in spans {
            if let Some(last) = merged.last_mut()
                && start <= last.1
            {
                last.1 = last.1.max(end)
            } else {
                merged.push((start, end));
            }
        }
        let mut out = String::new();
        let mut offset = 0;
        for (start, end) in merged {
            out.push_str(&text[offset..start]);
            out.push_str("<redacted>");
            offset = end;
        }
        out.push_str(&text[offset..]);
        if let Some(native) = &self.native_id
            && !native.is_empty()
        {
            out = out.replace(native, &fake_token(native));
        }
        out = patterns()[3].replace_all(&out, "~").into_owned();
        out = patterns()[4].replace_all(&out, "<email>").into_owned();
        out = patterns()[5]
            .replace_all(&out, |c: &regex::Captures<'_>| fake_token(&c[0]))
            .into_owned();
        out = patterns()[6]
            .replace_all(&out, |c: &regex::Captures<'_>| fake_token(&c[0]))
            .into_owned();
        out = patterns()[11].replace_all(&out, "$1<host>").into_owned();
        out = patterns()[10]
            .replace_all(&out, |c: &regex::Captures<'_>| -> String {
                let authority = c[0][c[1].len()..].to_ascii_lowercase();
                let name = authority
                    .rsplit('@')
                    .next()
                    .unwrap_or("")
                    .split(':')
                    .next()
                    .unwrap_or("");
                let name = name.trim_end_matches(|ch: char| {
                    !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '-')
                });
                if !authority.contains('@') && is_public_host(name) {
                    c[0].into()
                } else {
                    format!("{}<host>", &c[1])
                }
            })
            .into_owned();
        out = patterns()[7].replace_all(&out, "<ip>").into_owned();
        // A dotted name ending in a source-file extension is a file, not a host.
        out = patterns()[8]
            .replace_all(&out, |c: &regex::Captures<'_>| -> String {
                let name = c[0].split(':').next().unwrap_or("").to_ascii_lowercase();
                let last = name.rsplit('.').next().unwrap_or("");
                let public = is_public_host(&name);
                let reverse_dns = ["com.", "org.", "io.", "dev."]
                    .iter()
                    .any(|p| name.starts_with(p));
                if FILE_EXTENSIONS.contains(&last) || public || reverse_dns {
                    c[0].into()
                } else {
                    "<host>".into()
                }
            })
            .into_owned();
        // A colon run glued to an identifier (`Op::UserTurn`, `InProc::send`) is a path, not an address.
        let hay = &out;
        let ipv6 = patterns()[9]
            .replace_all(hay, |c: &regex::Captures<'_>| -> String {
                let m = c.get(0).expect("whole match");
                let before = hay[..m.start()].chars().next_back();
                let after = hay[m.end()..].chars().next();
                let glued = before.is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
                    || after.is_some_and(|ch| ch.is_alphabetic() || ch == '_');
                let candidate = c[0].trim_matches(['[', ']']);
                if !glued && candidate.parse::<std::net::Ipv6Addr>().is_ok() {
                    "<ip>".into()
                } else {
                    c[0].into()
                }
            })
            .into_owned();
        ipv6
    }
}
const FILE_EXTENSIONS: &[&str] = &[
    "rs", "md", "js", "ts", "tsx", "jsx", "py", "toml", "json", "jsonl", "jsonc", "yaml", "yml",
    "sh", "txt", "lock", "css", "html", "log", "tla", "cfg", "c", "h", "go", "swift", "png", "svg",
    "zip", "plist", "asc", "sha256", "crate", "app", "dmg", "gz", "tar", "pdf", "jpg", "icns",
    "xml",
];
// Public services named in our own sessions. d4947 hides private hosts; these are not private.
const PUBLIC_HOSTS: &[&str] = &[
    "github.com",
    "githubusercontent.com",
    "crates.io",
    "docs.rs",
    "bboxy450.lol",
    "openrouter.ai",
    "anthropic.com",
    "claude.ai",
    "openai.com",
    "cloudflare.com",
    "apple.com",
    "rust-lang.org",
];
fn is_public_host(name: &str) -> bool {
    PUBLIC_HOSTS
        .iter()
        .any(|h| name == *h || name.ends_with(&format!(".{h}")))
}
fn patterns() -> &'static Vec<Regex> {
    static P: OnceLock<Vec<Regex>> = OnceLock::new();
    P.get_or_init(||[
        r"(?:sk-[A-Za-z0-9_-]{8,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|AKIA[A-Z0-9]{16}|xox[baprs]-[A-Za-z0-9-]{10,}|AIza[A-Za-z0-9_-]{30,})",
        r"(?is)-----BEGIN [^-\r\n]+-----.*?(?:-----END [^-\r\n]+-----|$)",
        r"(?im)(?:\b(?:proxy-)?authorization\s*[:=]\s*[^\r\n]+|\bbearer\s+[A-Za-z0-9._~+/-]+=*)",
        r"/Users/[^/\r\n\x22'<>]+",
        r"[\p{L}0-9.!#$%&'*+/=?^_`{|}~-]+@[\p{L}0-9.-]+",
        r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b",
        r"\b[0-7][0-9A-HJKMNP-TV-Z]{25}\b",
        r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b",
        r"(?i)\b(?:localhost|(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z][a-z0-9-]*)(?::[0-9]+)?\b",
        r"\[?[0-9A-Fa-f]*:[0-9A-Fa-f:]+\]?",
        r#"(?i)([a-z][a-z0-9+.-]*://)[^/\s"<>]+"#,
            r"(?im)(\b(?:hostname|host|ssh_host)\s*[:=]\s*)[^\s]+",
    ].iter().map(|p|Regex::new(p).expect("fixed sanitizer regex")).collect())
}
fn fake_token(s: &str) -> String {
    let h = blake3::hash(s.as_bytes());
    let b = &h.as_bytes()[..16];
    format!(
        "{:08x}-{:04x}-4{:03x}-8{:03x}-{:012x}",
        u32::from_be_bytes(b[..4].try_into().unwrap()),
        u16::from_be_bytes(b[4..6].try_into().unwrap()),
        u16::from_be_bytes(b[6..8].try_into().unwrap()) & 0xfff,
        u16::from_be_bytes(b[8..10].try_into().unwrap()) & 0xfff,
        u64::from_be_bytes([0, 0, b[10], b[11], b[12], b[13], b[14], b[15]])
    )
}
fn fake_event(id: EventId) -> EventId {
    let h = blake3::hash(&id.0);
    EventId(h.as_bytes()[..16].try_into().unwrap())
}
fn fake_session(id: SessionId) -> SessionId {
    SessionId(ulid::Ulid::from_bytes(
        fake_event(EventId(id.0.to_bytes())).0,
    ))
}
fn limited(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.into();
    }
    if limit < CUT.len() {
        return String::new();
    }
    let mut end = limit - CUT.len();
    while !text.is_char_boundary(end) {
        end -= 1
    }
    format!("{}{CUT}", &text[..end])
}
fn truncate_value(value: &mut Value, remaining: &mut usize) {
    match value {
        Value::String(s) => {
            *s = limited(s, *remaining);
            *remaining = remaining.saturating_sub(s.len());
        }
        Value::Array(a) => {
            for v in a {
                truncate_value(v, remaining)
            }
        }
        Value::Object(o) => {
            for (key, v) in o {
                if matches!(
                    key.as_str(),
                    "text" | "content" | "output" | "stdout" | "stderr"
                ) {
                    truncate_value(v, remaining)
                }
            }
        }
        _ => {}
    }
}
fn owner_uid() -> Result<u32, CapabilityRefusal> {
    Ok(rustix::process::geteuid().as_raw())
}

fn fake_record(id: RecordId) -> RecordId {
    RecordId(fake_event(EventId(id.0)).0)
}
