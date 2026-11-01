// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Prepared unified diff records. Parsing belongs to a preparation worker;
//! retained layout only projects hunk fold state and token-resolved colours.
use super::seat_row::named;
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, draw::Color, layout::Axis};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Context,
    Added,
    Removed,
    Note,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub old: Option<u64>,
    pub new: Option<u64>,
    pub kind: Kind,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub file_headers: Vec<String>,
    pub header: String,
    pub lines: Vec<Line>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub headers: Vec<String>,
    pub trailer: Vec<String>,
    pub hunks: Vec<Hunk>,
}
impl Diff {
    /// Malformed input remains visible as local notes. Result is retained for
    /// API compatibility; neither a damaged hunk nor a header discards the file.
    pub fn parse(source: &str) -> Result<Self, &'static str> {
        fn ranges(line: &str) -> Option<(u64, u64, u64, u64)> {
            fn range(s: &str, prefix: char) -> Option<(u64, u64)> {
                let s = s.strip_prefix(prefix)?;
                let (start, count) = s.split_once(',').unwrap_or((s, "1"));
                let (start, count): (u64, u64) = (start.parse().ok()?, count.parse().ok()?);
                if count > 0 {
                    start.checked_add(count - 1)?;
                }
                Some((start, count))
            }
            let mut words = line.split_whitespace();
            if words.next()? != "@@" {
                return None;
            }
            let (old, old_left) = range(words.next()?, '-')?;
            let (new, new_left) = range(words.next()?, '+')?;
            (words.next()? == "@@").then_some((old, old_left, new, new_left))
        }
        fn note(text: impl Into<String>) -> Line {
            Line {
                old: None,
                new: None,
                kind: Kind::Note,
                text: text.into(),
            }
        }
        fn header(line: &str, prefix: &str) -> bool {
            line.strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        }
        fn finish(out: &mut Diff, old_left: &mut u64, new_left: &mut u64) {
            if (*old_left != 0 || *new_left != 0)
                && let Some(h) = out.hunks.last_mut()
            {
                h.lines.push(note(format!(
                    "[incomplete hunk: {old_left} old / {new_left} new lines missing]"
                )));
            }
            *old_left = 0;
            *new_left = 0;
        }
        let mut out = Self::default();
        let mut pending_headers = vec![];
        let (mut old, mut new) = (0_u64, 0_u64);
        let (mut old_left, mut new_left) = (0_u64, 0_u64);
        let mut file_headers = true;
        for line in source.lines() {
            // Boundaries outrank hunk counters, even when the counters lie.
            if header(line, "diff --git") || header(line, "---") || header(line, "+++") {
                finish(&mut out, &mut old_left, &mut new_left);
                file_headers = true;
                if out.hunks.is_empty() {
                    out.headers.push(line.into());
                } else {
                    pending_headers.push(line.into());
                }
            } else if line.starts_with("@@") {
                finish(&mut out, &mut old_left, &mut new_left);
                let valid = ranges(line);
                (old, old_left, new, new_left) = valid.unwrap_or((0, 0, 0, 0));
                out.hunks.push(Hunk {
                    file_headers: std::mem::take(&mut pending_headers),
                    header: line.into(),
                    lines: if valid.is_some() {
                        vec![]
                    } else {
                        vec![note("[invalid hunk header: line numbers unavailable]")]
                    },
                });
                file_headers = false;
            } else if out.hunks.is_empty() {
                out.headers.push(line.into());
            } else if file_headers {
                pending_headers.push(line.into());
            } else if let Some(h) = out.hunks.last_mut() {
                if line.starts_with("\\ No newline") || (old_left == 0 && new_left == 0) {
                    h.lines.push(note(line));
                    continue;
                }
                let (mut kind, mut text) = match line.as_bytes().first() {
                    None => (Kind::Context, ""),
                    Some(b' ') => (Kind::Context, &line[1..]),
                    Some(b'+') => (Kind::Added, &line[1..]),
                    Some(b'-') => (Kind::Removed, &line[1..]),
                    _ => (Kind::Note, line),
                };
                if kind != Kind::Added && old_left == 0 || kind != Kind::Removed && new_left == 0 {
                    kind = Kind::Note;
                    text = line;
                }
                let consumes_old = kind != Kind::Added && old_left > 0;
                let consumes_new = kind != Kind::Removed && new_left > 0;
                h.lines.push(Line {
                    old: consumes_old.then_some(old),
                    new: consumes_new.then_some(new),
                    kind,
                    text: text.into(),
                });
                if consumes_old {
                    old_left -= 1;
                    old = old.saturating_add(1);
                }
                if consumes_new {
                    new_left -= 1;
                    new = new.saturating_add(1);
                }
            }
        }
        finish(&mut out, &mut old_left, &mut new_left);
        out.trailer = pending_headers;
        Ok(out)
    }
}
#[derive(Debug, Clone, Default)]
pub struct Folds {
    closed: BTreeSet<usize>,
}
impl Folds {
    pub fn toggle(&mut self, hunk: usize) {
        if !self.closed.remove(&hunk) {
            self.closed.insert(hunk);
        }
    }
    pub fn closed(&self, hunk: usize) -> bool {
        self.closed.contains(&hunk)
    }
}
pub struct DiffView<'a> {
    pub key: &'a str,
    pub diff: &'a Diff,
    pub folds: &'a Folds,
}
impl DiffView<'_> {
    pub fn element(&self, ctx: ui::Context, width: f32, m: &dyn c::Measure) -> c::Element {
        let font = ui::Text {
            mono: true,
            step: ui::TypeStep::Caption,
            ..Default::default()
        };
        let mut children = vec![];
        for h in &self.diff.headers {
            children.push(c::Element::Text(c::Text::new(h, font)).into());
        }
        for (i, h) in self.diff.hunks.iter().enumerate() {
            for header in &h.file_headers {
                children.push(c::Element::Text(c::Text::new(header, font)).into());
            }
            let title = format!(
                "{} {}",
                if self.folds.closed(i) { "▸" } else { "▾" },
                h.header
            );
            children.push(
                named(
                    format!("{}/hunk/{i}/toggle", self.key),
                    c::Element::Text(c::Text::new(
                        title,
                        ui::Text {
                            weight: 600,
                            paint: ui::Paint::solid(ui::Color::Accent),
                            ..font
                        },
                    )),
                )
                .into(),
            );
            if self.folds.closed(i) {
                continue;
            }
            for (j, l) in h.lines.iter().enumerate() {
                let color = match l.kind {
                    Kind::Added => ui::Color::Rgba(Color::hex(TOKENS.color.ui.ok).0),
                    Kind::Removed => ui::Color::Rgba(Color::hex(TOKENS.color.state.errored).0),
                    Kind::Note => ui::Color::Muted,
                    _ => ui::Color::Foreground,
                };
                let sign = match l.kind {
                    Kind::Added => '+',
                    Kind::Removed => '-',
                    _ => ' ',
                };
                let number = |n: Option<u64>| n.map(|n| n.to_string()).unwrap_or_default();
                let mut text = c::Text::new(
                    format!(
                        "{:>5} {:>5} {sign} {}",
                        number(l.old),
                        number(l.new),
                        l.text
                    ),
                    ui::Text {
                        paint: ui::Paint::solid(color),
                        ..font
                    },
                );
                text.wrap = false;
                text.preserve_whitespace = true;
                children.push(
                    named(
                        format!("{}/hunk/{i}/line/{j}", self.key),
                        c::Element::Text(text),
                    )
                    .into(),
                );
            }
        }
        for header in &self.diff.trailer {
            children.push(c::Element::Text(c::Text::new(header, font)).into());
        }
        let body = c::Element::Stack(c::Stack {
            direction: Axis::Column,
            gap: ui::Length::Px(STYLE.spacing.s1 as f32),
            align: c::Alignment::Stretch,
            children,
        });
        let Ok(([w, h], _)) = body.natural(f32::INFINITY, ctx, m) else {
            return named(self.key, body);
        };
        named(
            self.key,
            c::Element::ScrollView(c::ScrollView {
                viewport: [width, h],
                offset: [0., 0.],
                content_width: Some(w.max(width)),
                content: Box::new(body),
            }),
        )
    }
}
