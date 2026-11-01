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
}

// Equivalent to a multiline ^\s{0,3}\[[^\]]+\]: check, without a regex
// dependency or rescanning the stable prefix. Inline brackets are not global.
fn definition_line(line: &str) -> bool {
    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
    if indent > 3 {
        return false;
    }
    let Some(label) = line.trim_start().strip_prefix('[') else {
        return false;
    };
    label
        .find(']')
        .is_some_and(|end| end > 0 && label[end + 1..].starts_with(':'))
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
        self.definitions |= self.document.source[self.definition_scan..]
            .lines()
            .any(definition_line);
        self.definition_scan = self.document.source.rfind('\n').map_or(0, |n| n + 1);
        let global = self.definitions;
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
