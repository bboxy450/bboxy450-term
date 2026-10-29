// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-term signed MA-10 implementation 0885ac378f162c6dfddadb9a7117957d68751bbd.
//! Sending-tool name resolution. Delivery consumes signed IDs and never parses text.
use std::collections::{BTreeMap, BTreeSet};

use bbt_proto::messaging::{MAX_MENTIONS, MessageRefusal};
use ulid::Ulid;

/// The caller supplies only the sending swarm's open seats. This grants no route.
pub fn resolve<'a>(
    text: &str,
    seats: impl IntoIterator<Item = (Ulid, &'a str)>,
) -> Result<Vec<Ulid>, MessageRefusal> {
    if !text.contains('@') {
        return Ok(Vec::new());
    }
    let mut names: BTreeMap<&str, BTreeSet<Ulid>> = BTreeMap::new();
    let mut full_names: BTreeMap<String, BTreeSet<Ulid>> = BTreeMap::new();
    let mut tokens: BTreeMap<String, BTreeSet<Ulid>> = BTreeMap::new();
    for (id, name) in seats {
        names.entry(name).or_default().insert(id);
        full_names
            .entry(name.to_lowercase())
            .or_default()
            .insert(id);
        tokens.entry(name.to_lowercase()).or_default().insert(id);
        if let Some(first) = name.split_whitespace().next() {
            tokens.entry(first.to_lowercase()).or_default().insert(id);
        }
    }
    let mut ids = BTreeSet::new();
    let mut chars = text.char_indices().peekable();
    let mut previous = None;
    while let Some((offset, c)) = chars.next() {
        let boundary = !previous.is_some_and(|p: char| p.is_alphanumeric() || "_-.+@".contains(p));
        previous = Some(c);
        if c != '@' || !boundary {
            continue;
        }
        let start = offset + 1;
        let braced = text[start..].starts_with('{');
        let name = if braced {
            let Some(end) = text[start + 1..].find('}') else {
                return Err(refusal(
                    "invalid_mention",
                    "A full-name mention needs a closing }.",
                    &names,
                ));
            };
            let end = start + 1 + end;
            while chars.peek().is_some_and(|(i, _)| *i <= end) {
                previous = chars.next().map(|(_, c)| c);
            }
            &text[start + 1..end]
        } else {
            let mut end = start;
            while let Some(&(i, c)) = chars.peek() {
                if !c.is_alphanumeric() && c != '_' && c != '-' {
                    break;
                }
                chars.next();
                previous = Some(c);
                end = i + c.len_utf8();
            }
            if end == start {
                continue; // A lone @ is prose, not a name.
            }
            &text[start..end]
        };
        let lookup = if braced { &full_names } else { &tokens };
        let matches = lookup.get(&name.to_lowercase());
        let id = match matches {
            Some(matches) if matches.len() == 1 => *matches.first().unwrap(),
            Some(_) => {
                return Err(refusal(
                    "ambiguous_mention",
                    &format!("More than one open seat is named {name:?}."),
                    &names,
                ));
            }
            None => {
                return Err(refusal(
                    "unknown_mention",
                    &format!("No open seat is named {name:?} in this swarm."),
                    &names,
                ));
            }
        };
        ids.insert(id);
        if ids.len() > MAX_MENTIONS {
            return Err(refusal(
                "mention_limit_32",
                "A message can name at most 32 distinct seats.",
                &names,
            ));
        }
    }
    Ok(ids.into_iter().collect())
}

fn refusal(kind: &str, message: &str, names: &BTreeMap<&str, BTreeSet<Ulid>>) -> MessageRefusal {
    let mut hint = "Use @name, or @{full display name}. Valid names in this swarm: ".to_owned();
    for (count, name) in names.keys().enumerate() {
        let label = format!("@{{{name}}}");
        if count == 16 || hint.len() + label.len() > 1024 {
            hint.push_str(" …; list the swarm's seats for more.");
            break;
        }
        if count != 0 {
            hint.push_str(", ");
        }
        hint.push_str(&label);
    }
    if names.is_empty() {
        hint.push_str("none.");
    }
    MessageRefusal {
        kind: kind.into(),
        message: message.into(),
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: ICD messaging-seats mention grammar, aliases, uniqueness and canonical IDs.
    #[test]
    fn token_aliases_and_full_names_are_case_insensitive_unique_and_sorted() {
        let names = [
            (Ulid::from(8), "Vesper Greywind"),
            (Ulid::from(2), "Nyx Copperwing"),
        ];
        assert_eq!(
            resolve("(@VEsPeR), @{NYX COPPERWING}; @vesper", names).unwrap(),
            [Ulid::from(2), Ulid::from(8)]
        );
        assert!(
            resolve("mail user@example.org; path/a@host; lone @", names)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            resolve("@NoSuchSeat", names).unwrap_err().kind,
            "unknown_mention"
        );
        assert_eq!(
            resolve("@{Vesper Greywind", names).unwrap_err().kind,
            "invalid_mention"
        );
        let names = [
            (Ulid::from(1), "Vesper"),
            (Ulid::from(2), "Vesper Greywind"),
        ];
        assert_eq!(
            resolve("@VESPER", names).unwrap_err().kind,
            "ambiguous_mention"
        );
        assert_eq!(
            resolve("@{Vesper Greywind}", names).unwrap(),
            [Ulid::from(2)]
        );
    }

    // keeps: ICD messaging-seats cap at 32 distinct IDs and bounded recovery hints.
    #[test]
    fn cap_counts_distinct_ids_and_refusals_keep_bounded_valid_names() {
        let names: Vec<_> = (1..=33)
            .map(|n| (Ulid::from(n as u128), format!("Seat{n} Display")))
            .collect();
        let roster = || names.iter().map(|(id, name)| (*id, name.as_str()));
        let text = (1..=32)
            .map(|n| format!("@Seat{n}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            resolve(&format!("{text} @Seat1"), roster()).unwrap().len(),
            32
        );
        assert_eq!(
            resolve(&format!("{text} @Seat33"), roster())
                .unwrap_err()
                .kind,
            "mention_limit_32"
        );
        let error = resolve("@unknown", roster()).unwrap_err();
        assert!(error.hint.contains("Seat1 Display"));
        assert!(error.hint.len() <= 1100);
    }
}
