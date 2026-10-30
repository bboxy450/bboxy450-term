// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure mappings between GitKB and the portable contract: identifiers, slugs,
//! numbers, task status, digests and small text helpers. Translated from
//! bboxy450-term `plugins/gitkb/adapter.py` (portable, native, slug_path,
//! number, NATIVE_TO_PORTABLE, PORTABLE_TO_NATIVE).
use std::io::Read;
use std::path::Path;

use bbt_plugin_sdk::{Refusal, encoded};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// GitKB task status → portable status.
pub fn native_to_portable(native: &str) -> Option<&'static str> {
    Some(match native {
        "draft" | "backlog" => "open",
        "active" => "running",
        "completed" => "done",
        _ => return None,
    })
}

/// Portable status → GitKB task status.
pub fn portable_to_native(portable: &str) -> Option<&'static str> {
    Some(match portable {
        "open" => "draft",
        "claimed" | "running" | "reported" | "rejected" => "active",
        "accepted" | "done" => "completed",
        _ => return None,
    })
}

/// Every native status a portable filter value can select.
pub fn native_statuses(status: &str) -> Vec<String> {
    let mut all = vec![status.to_owned()];
    for native in ["draft", "backlog", "active", "completed"] {
        if native_to_portable(native) == Some(status) {
            all.push(native.into());
        }
    }
    all.push(portable_to_native(status).unwrap_or(status).into());
    all.sort();
    all.dedup();
    all
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// SHA-256 of a UTF-8 string, as the contract's digests use.
pub fn digest(text: &str) -> String {
    sha256_hex(text.as_bytes())
}

/// The source-aware content digest: `[root, documentSource, raw]`.
pub fn content_digest(root: &str, source: &Value, raw: &str) -> String {
    digest(&encoded(&Value::Array(vec![
        root.into(),
        source.clone(),
        raw.into(),
    ])))
}

fn parse_uuid(text: &str) -> Option<u128> {
    let hexits: String = match text.len() {
        36 => {
            let b = text.as_bytes();
            if [8, 13, 18, 23].iter().any(|&i| b[i] != b'-') {
                return None;
            }
            text.chars().filter(|&c| c != '-').collect()
        }
        32 => text.into(),
        _ => return None,
    };
    if hexits.len() != 32 || !hexits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u128::from_str_radix(&hexits, 16).ok()
}

fn format_uuid(n: u128) -> String {
    let h = format!("{n:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// Native UUID → portable ULID text (same 128 bits).
pub fn portable(native: &str) -> Result<String, Refusal> {
    let n = parse_uuid(native)
        .ok_or_else(|| Refusal::new("invalid_native_id", "GitKB must return a canonical UUID"))?;
    Ok((0..26)
        .rev()
        .map(|i| ALPHABET[((n >> (5 * i)) & 31) as usize] as char)
        .collect())
}

/// Portable ULID or canonical native UUID → native UUID.
pub fn native(identifier: &str) -> Result<String, Refusal> {
    let refuse = || {
        Refusal::new(
            "invalid_id",
            "expected a portable ULID or a native canonical UUID",
        )
    };
    let bytes = identifier.as_bytes();
    if identifier.chars().count() == 26 && identifier.chars().next().is_some_and(|c| c <= '7') {
        let mut n: u128 = 0;
        for b in bytes {
            let digit = ALPHABET.iter().position(|a| a == b).ok_or_else(refuse)?;
            n = (n << 5) | digit as u128;
        }
        return Ok(format_uuid(n));
    }
    match parse_uuid(identifier) {
        Some(n) if n != 0 && format_uuid(n) == identifier => Ok(identifier.into()),
        _ => Err(refuse()),
    }
}

/// A bounded GitKB slug from a slug or relative `slug.md` path.
pub fn slug_path(value: &Value) -> Result<String, Refusal> {
    let invalid = || {
        Refusal::new(
            "invalid_path",
            "expected a bounded relative Markdown path or slug",
        )
    };
    let value = value.as_str().ok_or_else(invalid)?;
    if value.chars().count() > 4096 {
        return Err(invalid());
    }
    let value = value.strip_suffix(".md").unwrap_or(value);
    if value.is_empty()
        || value
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.starts_with('.'))
    {
        return Err(Refusal::new(
            "path_outside_root",
            "empty, hidden, absolute and parent paths are refused",
        ));
    }
    let part_ok = |p: &str| {
        let mut chars = p.bytes();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            && chars.all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'_' | b'.' | b'-')
            })
    };
    if !value.split('/').all(part_ok) {
        return Err(Refusal::new(
            "invalid_path",
            "GitKB slugs use lowercase ASCII letters, digits, dots, hyphens and underscores",
        ));
    }
    Ok(value.into())
}

/// The native frontmatter number, or a numbered family suffix (`-7`).
pub fn number(frontmatter: Option<&Map<String, Value>>, slug: &str) -> Result<Value, Refusal> {
    if let Some(value) = frontmatter.and_then(|fm| fm.get("number"))
        && !value.is_null()
    {
        return match value.as_u64() {
            Some(n) if n > 0 => Ok(n.into()),
            _ => Err(Refusal::new(
                "invalid_number",
                "native number must be a positive u64",
            )),
        };
    }
    Ok(slug
        .rsplit_once('-')
        .map(|(_, tail)| tail)
        .filter(|t| t.starts_with(|c: char| ('1'..='9').contains(&c)))
        .filter(|t| t.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|t| t.parse::<u64>().ok())
        .map_or(Value::Null, Value::from))
}

/// The portable task status projected from native frontmatter.
pub fn task_status(bboxy_status: Option<&Value>, native_status: &Value) -> Value {
    if let Some(status) = bboxy_status {
        return status.clone();
    }
    match native_status.as_str() {
        Some(s) if !s.is_empty() => native_to_portable(s).map_or_else(|| s.into(), Value::from),
        _ if native_status.is_null() || native_status == "" => "open".into(),
        _ => native_status.clone(),
    }
}

/// POSIX-shell word splitting (Python `shlex.split`), for search queries.
pub fn shell_words(text: &str) -> Result<Vec<String>, Refusal> {
    let unbalanced = || Refusal::new("invalid_arguments", "unbalanced quote in query");
    let mut words = Vec::new();
    let mut word = String::new();
    let mut active = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if active {
                    words.push(std::mem::take(&mut word));
                    active = false;
                }
            }
            '\'' => {
                active = true;
                loop {
                    match chars.next().ok_or_else(unbalanced)? {
                        '\'' => break,
                        c => word.push(c),
                    }
                }
            }
            '"' => {
                active = true;
                loop {
                    match chars.next().ok_or_else(unbalanced)? {
                        '"' => break,
                        '\\' => {
                            let next = chars.next().ok_or_else(unbalanced)?;
                            if !matches!(next, '"' | '\\') {
                                word.push('\\');
                            }
                            word.push(next);
                        }
                        c => word.push(c),
                    }
                }
            }
            '\\' => {
                active = true;
                word.push(chars.next().ok_or_else(unbalanced)?);
            }
            c => {
                active = true;
                word.push(c);
            }
        }
    }
    if active {
        words.push(word);
    }
    Ok(words)
}

/// Targets of Obsidian-style image embeds `![[target]]` / `![[target|alt]]`.
pub fn embeds(body: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("![[") {
        let after = &rest[start + 3..];
        let target_len = after.find([']', '|']).unwrap_or(after.len());
        let mut end = target_len;
        let mut matched = false;
        if target_len > 0 && end < after.len() {
            if after[end..].starts_with('|') {
                end += after[end..].find(']').unwrap_or(after.len() - end);
            }
            if after[end..].starts_with("]]") {
                matched = true;
            }
        }
        if matched {
            found.push(after[..target_len].to_owned());
            rest = &after[end + 2..];
        } else {
            rest = &rest[start + 1..];
        }
    }
    found
}

/// Python `json.dumps(value, ensure_ascii=False)`: spaced separators,
/// insertion-ordered keys. Used for YAML flow values in frontmatter.
pub fn spaced_json(value: &Value) -> String {
    match value {
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(spaced_json).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(map) => {
            let inner: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", encoded(&k.as_str().into()), spaced_json(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        scalar => encoded(scalar),
    }
}

/// A frontmatter key that YAML reads plainly; others are JSON-quoted.
pub fn yaml_key(key: &str) -> String {
    let mut bytes = key.bytes();
    let plain = bytes
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_' || c == b'$')
        && bytes.all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'$' | b'-'));
    if plain {
        key.into()
    } else {
        encoded(&key.into())
    }
}

/// The first `n` characters (not bytes).
pub fn prefix(text: &str, n: usize) -> String {
    text.chars().take(n).collect()
}

/// Random lowercase hex from the OS generator.
pub fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buffer))
        .expect("/dev/urandom is readable");
    hex(&buffer)
}

/// A `file://` URI, percent-encoding everything but unreserved bytes and `/`.
pub fn file_uri(path: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut uri = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~') {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

/// MIME type by extension for embedded images.
pub fn mime_type(target: &str) -> &'static str {
    let extension = target
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/vnd.microsoft.icon",
        "tif" | "tiff" => "image/tiff",
        "avif" => "image/avif",
        "heic" => "image/heic",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ulid_and_uuid_round_trip() {
        let ulid = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
        let uuid = "01563e3a-b5d3-d676-4c61-efb99302bd5b";
        assert_eq!(native(ulid).unwrap(), uuid);
        assert_eq!(portable(uuid).unwrap(), ulid);
        assert_eq!(native(uuid).unwrap(), uuid);
        assert_eq!(
            native("00000000000000000000000007").unwrap(),
            "00000000-0000-0000-0000-000000000007"
        );
        assert_eq!(portable("01563E3AB5D3D6764C61EFB99302BD5B").unwrap(), ulid);
        for bad in [
            "01563E3A-B5D3-D676-4C61-EFB99302BD5B",
            "00000000-0000-0000-0000-000000000000",
            "01arz3ndektsv4rrffq69g5fav",
            "81ARZ3NDEKTSV4RRFFQ69G5FAV",
            "01ARZ3NDEKTSV4RRFFQ69G5FAU!",
            "notes/shared",
            "",
        ] {
            assert_eq!(native(bad).unwrap_err().code, "invalid_id", "{bad}");
        }
        assert_eq!(portable("x").unwrap_err().code, "invalid_native_id");
    }

    #[test]
    fn slug_rules() {
        let ok = |s: &str| slug_path(&json!(s)).unwrap();
        let code = |v: Value| slug_path(&v).unwrap_err().code;
        assert_eq!(ok("notes/shared.md"), "notes/shared");
        assert_eq!(ok("tasks/harmony-7"), "tasks/harmony-7");
        assert_eq!(ok("_a/b.c-d"), "_a/b.c-d");
        for outside in ["../outside", "/abs", "a//b", ".hidden", "a/./b", ".md", ""] {
            assert_eq!(code(json!(outside)), "path_outside_root", "{outside}");
        }
        for invalid in ["Notes/x", "a b", "a/-b", "ü"] {
            assert_eq!(code(json!(invalid)), "invalid_path", "{invalid}");
        }
        assert_eq!(code(json!(7)), "invalid_path");
        assert_eq!(code(json!("a".repeat(4097))), "invalid_path");
    }

    #[test]
    fn numbers_prefer_native_frontmatter() {
        let fm = |v: Value| v.as_object().unwrap().clone();
        assert_eq!(
            number(Some(&fm(json!({"number": 77}))), "tasks/x-7").unwrap(),
            77
        );
        assert_eq!(number(None, "tasks/harmony-7").unwrap(), 7);
        assert_eq!(number(None, "tasks/harmony-07").unwrap(), Value::Null);
        assert_eq!(number(None, "notes/shared").unwrap(), Value::Null);
        assert_eq!(
            number(Some(&fm(json!({"number": null}))), "a-3").unwrap(),
            3
        );
        for bad in [json!(0), json!(-1), json!(1.5), json!("7"), json!(true)] {
            assert_eq!(
                number(Some(&fm(json!({ "number": bad }))), "a")
                    .unwrap_err()
                    .code,
                "invalid_number"
            );
        }
    }

    #[test]
    fn status_projection_and_native_selection() {
        assert_eq!(task_status(None, &json!("active")), "running");
        assert_eq!(
            task_status(Some(&json!("claimed")), &json!("active")),
            "claimed"
        );
        assert_eq!(task_status(None, &Value::Null), "open");
        assert_eq!(task_status(None, &json!("blocked")), "blocked");
        assert_eq!(native_statuses("running"), ["active", "running"]);
        assert_eq!(native_statuses("open"), ["backlog", "draft", "open"]);
        assert_eq!(native_statuses("active"), ["active"]);
        assert_eq!(native_statuses("accepted"), ["accepted", "completed"]);
    }

    #[test]
    fn words_embeds_and_json() {
        assert_eq!(
            shell_words(r#"Needle "two words" type:task it\'s"#).unwrap(),
            ["Needle", "two words", "type:task", "it's"]
        );
        assert_eq!(shell_words("'open").unwrap_err().code, "invalid_arguments");
        assert_eq!(
            embeds("a ![[images/p.png]] b ![[x.png|alt]] ![[]] ![[bad] ![[c.gif]]"),
            ["images/p.png", "x.png", "c.gif"]
        );
        assert_eq!(
            spaced_json(&json!({"preserved": [1, true, "x"], "ü": null})),
            r#"{"preserved": [1, true, "x"], "ü": null}"#
        );
        assert_eq!(yaml_key("due_date"), "due_date");
        assert_eq!(yaml_key("a key"), "\"a key\"");
        assert_eq!(prefix("äbc", 2), "äb");
        assert_eq!(file_uri(Path::new("/tmp/a b/x")), "file:///tmp/a%20b/x");
        assert_eq!(mime_type("images/pixel.PNG"), "image/png");
        assert_eq!(random_hex(12).len(), 24);
    }
}
