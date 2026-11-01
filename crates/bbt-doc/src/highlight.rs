// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Owned, off-thread code preparation. Layout consumes spans; it never lexes.
use crate::{Block, BlockKind, Markdown};
use std::collections::BTreeMap;
use std::sync::OnceLock;
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::SyntaxSet,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub color: [f32; 4],
    pub bold: bool,
    pub italic: bool,
}
/// Custom highlighters may supply the same owned spans. Text must concatenate
/// byte-for-byte to the code; malformed results fall back to unstyled source.
pub trait Highlighter {
    fn highlight(&self, info: &str, text: &str) -> Vec<Span>;
}
struct Resources {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
}
static RESOURCES: OnceLock<Resources> = OnceLock::new();
pub struct RustHighlighter {
    resources: &'static Resources,
    light: bool,
}
impl RustHighlighter {
    pub fn new(light: bool) -> Self {
        bbt_rt::assert_not_ui_thread();
        Self {
            resources: RESOURCES.get_or_init(|| Resources {
                syntaxes: SyntaxSet::load_defaults_newlines(),
                themes: ThemeSet::load_defaults(),
            }),
            light,
        }
    }
}
impl Highlighter for RustHighlighter {
    fn highlight(&self, info: &str, text: &str) -> Vec<Span> {
        bbt_rt::assert_not_ui_thread();
        let syntax = self
            .resources
            .syntaxes
            .find_syntax_by_token(info.split_whitespace().next().unwrap_or(""))
            .unwrap_or_else(|| self.resources.syntaxes.find_syntax_plain_text());
        let theme = &self.resources.themes.themes[if self.light {
            "InspiredGitHub"
        } else {
            "base16-ocean.dark"
        }];
        let mut h = HighlightLines::new(syntax, theme);
        let mut spans = vec![];
        for line in text.split_inclusive('\n') {
            let Ok(ranges) = h.highlight_line(line, &self.resources.syntaxes) else {
                return vec![];
            };
            for (style, value) in ranges {
                let c = style.foreground;
                spans.push(Span {
                    text: value.into(),
                    color: [c.r, c.g, c.b, c.a].map(|n| n as f32 / 255.),
                    bold: style.font_style.contains(FontStyle::BOLD),
                    italic: style.font_style.contains(FontStyle::ITALIC),
                });
            }
        }
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prepared_jobs_share_syntax_and_theme_resources() {
        let a = RustHighlighter::new(false);
        let b = RustHighlighter::new(true);
        assert!(std::ptr::eq(a.resources, b.resources));
    }
}
#[derive(Debug, Clone)]
struct CodeSpans {
    info: String,
    text: String,
    spans: Vec<Span>,
}
#[derive(Debug, Clone, Default)]
pub struct Highlights(BTreeMap<usize, CodeSpans>);
impl Highlights {
    pub fn prepare(document: &Markdown, highlighter: &impl Highlighter) -> Self {
        bbt_rt::assert_not_ui_thread();
        fn visit(blocks: &[Block], h: &impl Highlighter, out: &mut BTreeMap<usize, CodeSpans>) {
            for block in blocks {
                match &block.kind {
                    BlockKind::Code { info, text } if info != "mermaid" => {
                        let spans = h.highlight(info, text);
                        if spans.iter().map(|s| s.text.as_str()).collect::<String>() == *text {
                            out.insert(
                                block.source.start,
                                CodeSpans {
                                    info: info.clone(),
                                    text: text.clone(),
                                    spans,
                                },
                            );
                        }
                    }
                    BlockKind::Quote(bs) | BlockKind::Footnote { blocks: bs, .. } => {
                        visit(bs, h, out)
                    }
                    BlockKind::List { items, .. } => {
                        for item in items {
                            visit(item, h, out);
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut result = Self::default();
        visit(&document.blocks, highlighter, &mut result.0);
        result
    }
    pub fn spans(&self, block: &Block) -> Option<&[Span]> {
        self.0.get(&block.source.start).filter(|code| {
            matches!(&block.kind, BlockKind::Code { info, text } if code.info == *info && code.text == *text)
        }).map(|code|code.spans.as_slice())
    }
}
/// Move this job to a blocking worker, like documents::Work::prepare. The UI
/// owns only the previous Prepared until the captured generation/revision wins.
pub struct Work {
    pub generation: u64,
    pub revision: u64,
    pub source: String,
    pub light: bool,
}
#[derive(Debug, Clone)]
pub struct Prepared {
    pub generation: u64,
    pub revision: u64,
    pub document: Markdown,
    pub highlights: Highlights,
}
impl Work {
    /// Schedule all parsing and lexing on the shared runtime's blocking workers.
    pub async fn prepare_off_thread(self) -> Result<Prepared, String> {
        bbt_rt::handle()
            .spawn_blocking(move || self.prepare())
            .await
            .map_err(|e| e.to_string())
    }
    pub fn prepare(self) -> Prepared {
        bbt_rt::assert_not_ui_thread();
        let document = crate::parse(&self.source);
        let highlights = Highlights::prepare(&document, &RustHighlighter::new(self.light));
        Prepared {
            generation: self.generation,
            revision: self.revision,
            document,
            highlights,
        }
    }
}

/// Current waiter identity and last prepared frame. A newer request keeps the
/// old frame visible; replacement/drop retires both captured owner and revision.
#[derive(Debug, Default)]
pub struct Preparation {
    generation: u64,
    revision: u64,
    active: bool,
    prepared: Option<Prepared>,
}
impl Preparation {
    pub fn request(&mut self, source: String, light: bool) -> Work {
        self.active = true;
        self.revision = self
            .revision
            .checked_add(1)
            .expect("preparation revision exhausted");
        Work {
            generation: self.generation,
            revision: self.revision,
            source,
            light,
        }
    }
    pub fn retire(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("preparation generation exhausted");
        self.revision = 0;
        self.active = false;
        self.prepared = None;
    }
    pub fn install(&mut self, prepared: Prepared) -> bool {
        if !self.active
            || prepared.generation != self.generation
            || prepared.revision != self.revision
        {
            return false;
        }
        self.prepared = Some(prepared);
        true
    }
    pub fn prepared(&self) -> Option<&Prepared> {
        self.prepared.as_ref()
    }
}
