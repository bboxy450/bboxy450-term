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
    pub fn parse(source: &str) -> Result<Self, &'static str> {
        let mut out = Self::default();
        let mut pending_headers = vec![];
        let (mut old, mut new) = (0_u64, 0_u64);
        let (mut old_left, mut new_left) = (0_u64, 0_u64);
        for line in source.lines() {
            if line.starts_with("@@ ") {
                if old_left != 0 || new_left != 0 {
                    return Err("truncated diff hunk");
                }
                let mut ranges = line.split_whitespace();
                ranges.next();
                fn range(s: Option<&str>, prefix: char) -> Result<(u64, u64), &'static str> {
                    let s = s
                        .and_then(|v| v.strip_prefix(prefix))
                        .ok_or("invalid hunk range")?;
                    let (start, count) = s.split_once(',').unwrap_or((s, "1"));
                    Ok((
                        start.parse().map_err(|_| "invalid hunk start")?,
                        count.parse().map_err(|_| "invalid hunk count")?,
                    ))
                }
                (old, old_left) = range(ranges.next(), '-')?;
                (new, new_left) = range(ranges.next(), '+')?;
                if ranges.next() != Some("@@") {
                    return Err("invalid hunk header");
                }
                out.hunks.push(Hunk {
                    file_headers: std::mem::take(&mut pending_headers),
                    header: line.into(),
                    lines: vec![],
                });
            } else if line.starts_with("\\ No newline at end of file") {
                out.hunks
                    .last_mut()
                    .ok_or("orphan diff note")?
                    .lines
                    .push(Line {
                        old: None,
                        new: None,
                        kind: Kind::Note,
                        text: line.into(),
                    });
            } else if old_left > 0 || new_left > 0 {
                let h = out.hunks.last_mut().ok_or("missing diff hunk")?;
                let prefix = line.chars().next().ok_or("unprefixed hunk line")?;
                let kind = match prefix {
                    ' ' => Kind::Context,
                    '+' => Kind::Added,
                    '-' => Kind::Removed,
                    _ => return Err("invalid diff line"),
                };
                let consumes_old = kind != Kind::Added;
                let consumes_new = kind != Kind::Removed;
                if consumes_old && old_left == 0 || consumes_new && new_left == 0 {
                    return Err("hunk count exceeded");
                }
                h.lines.push(Line {
                    old: consumes_old.then_some(old),
                    new: consumes_new.then_some(new),
                    kind,
                    text: line[1..].into(),
                });
                if consumes_old {
                    old_left -= 1;
                    old = old.checked_add(1).ok_or("line number overflow")?;
                }
                if consumes_new {
                    new_left -= 1;
                    new = new.checked_add(1).ok_or("line number overflow")?;
                }
            } else {
                if out.hunks.is_empty() {
                    out.headers.push(line.into());
                } else {
                    pending_headers.push(line.into());
                }
            }
        }
        if old_left != 0 || new_left != 0 {
            return Err("truncated diff hunk");
        }
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
        let [w, h] = body.natural(f32::INFINITY, ctx, m).expect("diff styles").0;
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
