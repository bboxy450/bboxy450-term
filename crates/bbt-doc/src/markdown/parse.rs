// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use pulldown_cmark::{Alignment, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

pub fn parse(source: &str) -> Markdown {
    let (frontmatter, offset) = frontmatter(source);
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_WIKILINKS;
    let events = Parser::new_ext(&source[offset..], options)
        .into_offset_iter()
        .map(|(event, span)| (event, span.start + offset..span.end + offset))
        .collect();
    let blocks = Reader { events, at: 0 }.blocks(None);
    Markdown {
        source: source.into(),
        body_start: offset,
        frontmatter,
        blocks,
    }
}

fn frontmatter(source: &str) -> (Frontmatter, usize) {
    let bom = usize::from(source.starts_with('\u{feff}')) * '\u{feff}'.len_utf8();
    let mut lines = source[bom..].split_inclusive('\n');
    let Some(first) = lines
        .next()
        .filter(|s| s.trim_end_matches(['\r', '\n']) == "---")
    else {
        return (Frontmatter::default(), bom);
    };
    let start = bom + first.len();
    let mut cursor = start;
    for line in lines {
        if matches!(line.trim_end_matches(['\r', '\n']), "---" | "...") {
            let raw = &source[start..cursor];
            let value = if raw.trim().is_empty() {
                Ok(BTreeMap::new())
            } else {
                serde_yaml::from_str::<BTreeMap<String, serde_yaml::Value>>(raw)
            };
            let (fields, error, error_line) = match value {
                Ok(fields) => {
                    let invalid_number = fields
                        .get("number")
                        .is_some_and(|value| value.as_u64().is_none_or(|number| number == 0));
                    let error = invalid_number
                        .then(|| "frontmatter number must be a positive integer".into());
                    let error_line = invalid_number
                        .then(|| raw.lines().position(|line| line.starts_with("number:")))
                        .flatten()
                        .map(|line| line + 2);
                    (fields, error, error_line)
                }
                Err(error) => {
                    // YAML starts on Markdown line two; BOM and CRLF do not
                    // add lines. Preserve the location before stringifying it.
                    let line = error.location().map_or(2, |loc| loc.line() + 1);
                    (BTreeMap::new(), Some(error.to_string()), Some(line))
                }
            };
            return (
                Frontmatter {
                    fields,
                    source: raw.into(),
                    error,
                    error_line,
                },
                cursor + line.len(),
            );
        }
        cursor += line.len();
    }
    (
        Frontmatter {
            error: Some("unterminated YAML frontmatter · shown as source".into()),
            error_line: Some(1),
            ..Default::default()
        },
        bom,
    )
}

struct Reader<'a> {
    events: Vec<(Event<'a>, Range<usize>)>,
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self) -> Option<(Event<'_>, Range<usize>)> {
        let v = self.events.get(self.at)?.clone();
        self.at += 1;
        Some(v)
    }
    fn last_end(&self, start: usize) -> usize {
        self.events
            .get(self.at.saturating_sub(1))
            .map_or(start, |v| v.1.end)
    }
    fn blocks(&mut self, end: Option<TagEnd>) -> Vec<Block> {
        let mut blocks = Vec::new();
        while self.at < self.events.len() {
            if matches!(&self.events[self.at].0, Event::End(e) if Some(*e) == end) {
                self.at += 1;
                break;
            }
            let start = self.events[self.at].1.start;
            let before = self.at;
            let event = self.events[self.at].0.clone();
            let kind = match event {
                Event::Start(Tag::Paragraph) => {
                    self.at += 1;
                    Some(BlockKind::Paragraph(self.inlines(Some(TagEnd::Paragraph))))
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    self.at += 1;
                    Some(BlockKind::Heading {
                        level: level as u8,
                        content: self.inlines(Some(TagEnd::Heading(level))),
                    })
                }
                Event::Start(Tag::BlockQuote(kind)) => {
                    self.at += 1;
                    Some(BlockKind::Quote(
                        self.blocks(Some(TagEnd::BlockQuote(kind))),
                    ))
                }
                Event::Start(Tag::FootnoteDefinition(label)) => {
                    self.at += 1;
                    Some(BlockKind::Footnote {
                        label: label.into_string(),
                        blocks: self.blocks(Some(TagEnd::FootnoteDefinition)),
                    })
                }
                Event::Start(Tag::List(start)) => {
                    self.at += 1;
                    let mut items = vec![];
                    while self.at < self.events.len() {
                        match self.events[self.at].0.clone() {
                            Event::Start(Tag::Item) => {
                                self.at += 1;
                                items.push(self.blocks(Some(TagEnd::Item)));
                            }
                            Event::End(TagEnd::List(_)) => {
                                self.at += 1;
                                break;
                            }
                            _ => self.at += 1,
                        }
                    }
                    Some(BlockKind::List { start, items })
                }
                Event::Start(Tag::CodeBlock(info)) => {
                    self.at += 1;
                    let info = match info {
                        CodeBlockKind::Fenced(s) => s.into_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    let mut text = String::new();
                    while let Some((event, _)) = self.take() {
                        match event {
                            Event::End(TagEnd::CodeBlock) => break,
                            Event::Text(s) => text.push_str(&s),
                            _ => {}
                        }
                    }
                    Some(BlockKind::Code { info, text })
                }
                Event::Start(Tag::HtmlBlock) => {
                    self.at += 1;
                    let mut text = String::new();
                    while let Some((event, _)) = self.take() {
                        match event {
                            Event::End(TagEnd::HtmlBlock) => break,
                            Event::Html(s) | Event::Text(s) => text.push_str(&s),
                            _ => {}
                        }
                    }
                    Some(BlockKind::Html(text))
                }
                Event::Start(Tag::Table(align)) => {
                    self.at += 1;
                    let align = align
                        .into_iter()
                        .map(|a| match a {
                            Alignment::Center => TableAlign::Center,
                            Alignment::Right => TableAlign::End,
                            _ => TableAlign::Start,
                        })
                        .collect();
                    let mut header = vec![];
                    let mut rows = vec![];
                    while self.at < self.events.len() {
                        let head = match self.events[self.at].0 {
                            Event::Start(Tag::TableHead) => true,
                            Event::Start(Tag::TableRow) => false,
                            Event::End(TagEnd::Table) => {
                                self.at += 1;
                                break;
                            }
                            _ => {
                                self.at += 1;
                                continue;
                            }
                        };
                        self.at += 1;
                        let mut cells = vec![];
                        while self.at < self.events.len() {
                            match self.events[self.at].0 {
                                Event::Start(Tag::TableCell) => {
                                    self.at += 1;
                                    cells.push(self.inlines(Some(TagEnd::TableCell)));
                                }
                                Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                                    self.at += 1;
                                    break;
                                }
                                _ => self.at += 1,
                            }
                        }
                        if head {
                            header = cells;
                        } else {
                            rows.push(cells);
                        }
                    }
                    Some(BlockKind::Table {
                        align,
                        header,
                        rows,
                    })
                }
                Event::Rule => {
                    self.at += 1;
                    Some(BlockKind::Rule)
                }
                _ if inline_event(&event) => Some(BlockKind::Paragraph(self.inlines(None))),
                _ => {
                    self.at += 1;
                    None
                }
            };
            if let Some(kind) = kind {
                blocks.push(Block {
                    source: start..self.last_end(start),
                    kind,
                });
            }
            debug_assert!(self.at > before, "the event reader must advance");
        }
        blocks
    }
    fn inlines(&mut self, end: Option<TagEnd>) -> Vec<Inline> {
        let mut values = vec![];
        while self.at < self.events.len() {
            let event = self.events[self.at].0.clone();
            if matches!(&event, Event::End(e) if Some(*e) == end) {
                self.at += 1;
                break;
            }
            if !inline_event(&event) {
                break;
            }
            self.at += 1;
            let value = match event {
                Event::Text(s) | Event::InlineHtml(s) => {
                    let mut s = s.into_string();
                    if let Some(Inline::WikiLink {
                        relation, weight, ..
                    }) = values.last_mut()
                        && let Some((rel, w, length)) = annotation(&s)
                    {
                        *relation = Some(rel);
                        *weight = w;
                        s.drain(..length);
                    }
                    if s.is_empty() {
                        continue;
                    }
                    if let Some(Inline::Text(previous)) = values.last_mut() {
                        previous.push_str(&s);
                        continue;
                    }
                    Inline::Text(s)
                }
                Event::Code(s) => Inline::Code(s.into_string()),
                Event::SoftBreak => Inline::SoftBreak,
                Event::HardBreak => Inline::HardBreak,
                Event::TaskListMarker(done) => Inline::Task(done),
                Event::FootnoteReference(label) => Inline::Footnote(label.into_string()),
                Event::Start(Tag::Emphasis) => {
                    Inline::Emphasis(self.inlines(Some(TagEnd::Emphasis)))
                }
                Event::Start(Tag::Strong) => Inline::Strong(self.inlines(Some(TagEnd::Strong))),
                Event::Start(Tag::Strikethrough) => {
                    Inline::Strike(self.inlines(Some(TagEnd::Strikethrough)))
                }
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    ..
                }) => {
                    let content = self.inlines(Some(TagEnd::Link));
                    if let LinkType::WikiLink { has_pothole } = link_type {
                        // pulldown retains the table escape in the destination
                        // immediately before an alias delimiter.
                        let target = if has_pothole {
                            dest_url.strip_suffix('\\').unwrap_or(&dest_url)
                        } else {
                            &dest_url
                        };
                        Inline::WikiLink {
                            target: target.into(),
                            display: Inline::plain_text(&content),
                            relation: None,
                            weight: None,
                        }
                    } else {
                        Inline::Link {
                            target: dest_url.into_string(),
                            title: title.into_string(),
                            content,
                        }
                    }
                }
                Event::Start(Tag::Image { dest_url, link_type, .. }) => Inline::Image {
                    target: if matches!(link_type, LinkType::WikiLink { has_pothole:true }) { dest_url.strip_suffix('\\').unwrap_or(&dest_url).into() } else {dest_url.into_string()},
                    wiki: matches!(link_type,LinkType::WikiLink {..}),
                    alt: Inline::plain_text(&self.inlines(Some(TagEnd::Image))),
                    asset: None,
                },
                _ => continue,
            };
            values.push(value);
        }
        values
    }
}

fn inline_event(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::Text(_)
            | Event::Code(_)
            | Event::InlineHtml(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::Start(
                Tag::Emphasis
                    | Tag::Strong
                    | Tag::Strikethrough
                    | Tag::Link { .. }
                    | Tag::Image { .. }
            )
    )
}
fn annotation(value: &str) -> Option<(String, Option<f32>, usize)> {
    let end = value.strip_prefix('{')?.find('}')? + 1;
    let inside = &value[1..end];
    let (name, weight) = inside
        .split_once(':')
        .map_or((inside, None), |(name, w)| (name, Some(w)));
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    {
        return None;
    }
    let weight = match weight {
        Some(w) => Some(
            w.parse::<f32>()
                .ok()
                .filter(|w| w.is_finite() && *w >= 0.)?,
        ),
        None => None,
    };
    Some((name.into(), weight, end + 1))
}
