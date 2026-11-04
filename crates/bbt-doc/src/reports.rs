// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Report record syntax shared by the native writer and repository records gate.
use serde_json::Value;

/// Reports keep their date/slug path; the existing Index allocates `number`.
pub fn validate(path: &str, metadata: &Value) -> Result<(), String> {
    let name = path
        .strip_prefix(".bboxy/")
        .unwrap_or(path)
        .strip_prefix("reports/")
        .and_then(|s| s.strip_suffix(".md"))
        .ok_or("reports require reports/YYYY-MM-DD-slug.md")?;
    let date = name.as_bytes();
    if date.len() < 12
        || date[4] != b'-'
        || date[7] != b'-'
        || date[10] != b'-'
        || !date[..10]
            .iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7) || c.is_ascii_digit())
        || !name[11..]
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
    {
        return Err("reports require reports/YYYY-MM-DD-slug.md".into());
    }
    let year: u32 = name[..4].parse().map_err(|_| "invalid report date")?;
    let month: usize = name[5..7].parse().map_err(|_| "invalid report date")?;
    let day: u32 = name[8..10].parse().map_err(|_| "invalid report date")?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if year == 0 || !(1..=12).contains(&month) || day == 0 || day > days[month - 1] {
        return Err("invalid report date".into());
    }
    if metadata["type"] != "report" {
        return Err("record must have type: report".into());
    }
    locator(&metadata["task"])?;
    if let Some(previous) = metadata.get("supersedes") {
        locator(previous)?;
    }
    if !metadata["commits"].as_array().is_some_and(|items| {
        items.iter().all(|sha| {
            sha.as_str().is_some_and(|s| {
                matches!(s.len(), 40 | 64) && s.bytes().all(|c| c.is_ascii_hexdigit())
            })
        })
    }) {
        return Err("report commits must be a list of full SHAs (possibly empty)".into());
    }
    if metadata["verify"]["command"]
        .as_str()
        .is_none_or(|s| s.trim().is_empty())
        || metadata["verify"]["exit_code"]
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .is_none()
    {
        return Err("report verify requires command and integer exit_code".into());
    }
    let author = metadata["author"]
        .as_str()
        .unwrap_or_default()
        .strip_prefix("seat:")
        .unwrap_or_default();
    if author.len() != 26
        || !author
            .bytes()
            .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b))
        || author.as_bytes()[0] > b'7'
        || author.bytes().all(|b| b == b'0')
    {
        return Err("report author must name a seat:ULID".into());
    }
    if metadata["created"]
        .as_str()
        .is_none_or(|s| s.trim().is_empty())
    {
        return Err("report created timestamp is required".into());
    }
    Ok(())
}

/// Record task/supersedes fields use wikilinks or an unambiguous numeric handle.
pub fn locator(value: &Value) -> Result<&str, String> {
    let value = value.as_str().ok_or("report link must be a string")?;
    if value
        .strip_prefix("[[")
        .and_then(|v| v.strip_suffix("]]"))
        .is_some_and(|v| !v.trim().is_empty())
        || value
            .strip_prefix('#')
            .is_some_and(|v| v.parse::<u64>().is_ok_and(|n| n > 0 && n.to_string() == v))
    {
        Ok(value)
    } else {
        Err("report link requires [[locator]] or #N".into())
    }
}

/// Extract Report trailers; a present malformed link fails closed.
pub fn trailers(message: &str) -> Result<Vec<&str>, String> {
    message
        .lines()
        .filter_map(|line| line.strip_prefix("Report:"))
        .map(|value| {
            let value = value.trim();
            if value.starts_with("[[") && value.ends_with("]]") && value.len() > 4 {
                Ok(value)
            } else {
                Err("Report: trailer requires [[report-locator]]".into())
            }
        })
        .collect()
}
