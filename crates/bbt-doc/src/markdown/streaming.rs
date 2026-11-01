// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Append-only Markdown with a retained stable prefix and one reparsed tail.
use super::{Markdown, parse};
use std::ops::Range;

#[derive(Debug, Clone, Default)]
pub struct StreamingMarkdown {
    document: Markdown,
    tail: usize,
    stable: usize,
    reparsed: Range<usize>,
    definition_scan: usize,
    definitions: bool,
    definition_candidate: Option<usize>,
}

// A definition can start after nested blockquote/list markers. This is a
// conservative classifier, not a second Markdown parser: ambiguous open labels
// temporarily use the full parser until a closing bracket disambiguates them.
fn definition_label(mut line: &str) -> Option<&str> {
    line = line.strip_prefix('\u{feff}').unwrap_or(line);
    loop {
        line = line.trim_start();
        if let Some(rest) = line.strip_prefix('>') {
            line = rest;
            continue;
        }
        let bytes = line.as_bytes();
        if matches!(bytes.first(), Some(b'-' | b'+' | b'*'))
            && bytes.get(1).is_some_and(u8::is_ascii_whitespace)
        {
            line = &line[1..];
            continue;
        }
        let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
        if (1..=9).contains(&digits)
            && matches!(bytes.get(digits), Some(b'.' | b')'))
            && bytes.get(digits + 1).is_some_and(u8::is_ascii_whitespace)
        {
            line = &line[digits + 1..];
            continue;
        }
        return line.strip_prefix('[');
    }
}
fn definitions(source: &str, from: usize) -> (bool, Option<usize>) {
    fn closing_bracket(label: &str) -> Option<usize> {
        let mut bytes = label.bytes().enumerate();
        while let Some((at, byte)) = bytes.next() {
            if byte == b'\\' {
                // Skip an escaped bracket or backslash; even runs of slashes
                // leave the following bracket available to close the label.
                bytes.next();
            } else if byte == b']' {
                return Some(at);
            }
        }
        None
    }
    let mut start = from;
    let mut possible = None;
    for line in source[from..].split_inclusive('\n') {
        if let Some(label) = definition_label(line) {
            let label_start = start + line.len() - label.len();
            let rest = &source[label_start..];
            match closing_bracket(rest) {
                Some(end) if end > 0 && rest[end + 1..].starts_with(':') => return (true, None),
                None => {
                    possible.get_or_insert(start);
                }
                // The colon may be the next append, including after a label
                // spanning several lines. Keep its original start until then.
                Some(end) if end > 0 && end + 1 == rest.len() => {
                    possible.get_or_insert(start);
                }
                _ => {}
            }
        }
        start += line.len();
    }
    (false, possible)
}
impl StreamingMarkdown {
    pub fn document(&self) -> &Markdown {
        &self.document
    }
    /// Source range actually passed to pulldown-cmark by the latest append.
    pub fn reparsed_range(&self) -> Range<usize> {
        self.reparsed.clone()
    }
    pub fn stable_blocks(&self) -> usize {
        self.stable
    }
    pub fn append(&mut self, delta: &str) -> &Markdown {
        if delta.is_empty() {
            self.reparsed = self.document.source.len()..self.document.source.len();
            return &self.document;
        }
        self.document.source.push_str(delta);
        // Definitions have document-wide scope, including later uses of an old
        // definition. Such documents use the full parser's reference context.
        // Only the previous unfinished line and new bytes need classification.
        // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        let (found, possible) = definitions(
            &self.document.source,
            self.definition_candidate.unwrap_or(self.definition_scan),
        );
        self.definitions |= found;
        self.definition_candidate = possible;
        self.definition_scan = self.document.source.rfind('\n').map_or(0, |n| n + 1);
        let global = self.definitions || self.definition_candidate.is_some();
        let unsettled_header = !self.document.source.contains('\n')
            || self
                .document
                .frontmatter
                .error
                .as_deref()
                .is_some_and(|s| s.starts_with("unterminated YAML"));
        if self.tail == 0 || global || unsettled_header {
            self.reparsed = 0..self.document.source.len();
            self.document = parse(&self.document.source);
        } else {
            self.reparsed = self.tail..self.document.source.len();
            self.document.blocks.truncate(self.stable);
            self.document
                .blocks
                .extend(super::parse::body(&self.document.source, self.tail));
        }
        self.stable = self.document.blocks.len().saturating_sub(1);
        // An incomplete first line can still become a continuation of the prior
        // block (e.g. a lone `|` after a table). Keep that predecessor in the
        // open tail until the next block's first line is newline-terminated.
        if self
            .document
            .blocks
            .last()
            .is_some_and(|b| !self.document.source[b.source.start..].contains('\n'))
        {
            self.stable = self.stable.saturating_sub(1);
        }
        self.tail = self.document.blocks.get(self.stable).map_or(0, |b| {
            // Preserve indentation: parser offsets can start after leading spaces.
            self.document.source[..b.source.start]
                .rfind('\n')
                .map_or(0, |n| n + 1)
        });
        if global || unsettled_header {
            self.tail = 0;
            self.stable = 0;
        }
        &self.document
    }
}
