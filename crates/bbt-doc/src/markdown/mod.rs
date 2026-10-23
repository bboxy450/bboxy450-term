mod layout;
mod parse;
pub use parse::parse;

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};

/// Lossless metadata values. A displayed signature is not verification.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    pub fields: BTreeMap<String, serde_yaml::Value>,
    pub source: String,
    pub error: Option<String>,
    /// One-based line in the original Markdown, including its opening delimiter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_line: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Markdown {
    pub source: String,
    /// Byte offset after the BOM and any closed frontmatter block.
    #[serde(default)]
    pub body_start: usize,
    pub frontmatter: Frontmatter,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    /// Byte offsets into the original UTF-8 source, including frontmatter.
    pub source: Range<usize>,
    pub kind: BlockKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockKind {
    Paragraph(Vec<Inline>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    List {
        start: Option<u64>,
        items: Vec<Vec<Block>>,
    },
    Quote(Vec<Block>),
    Code {
        info: String,
        text: String,
    },
    Table {
        align: Vec<TableAlign>,
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Footnote {
        label: String,
        blocks: Vec<Block>,
    },
    Rule,
    /// HTML is retained as source, never interpreted as UI or executed.
    Html(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Inline {
    Text(String),
    Code(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Strike(Vec<Inline>),
    Link {
        target: String,
        title: String,
        content: Vec<Inline>,
    },
    WikiLink {
        target: String,
        display: String,
        relation: Option<String>,
        weight: Option<f32>,
    },
    Footnote(String),
    Task(bool),
    SoftBreak,
    HardBreak,
    /// Image decoding/registration is deferred; its alt text stays readable.
    Image {
        target: String,
        alt: String,
    },
}

impl Inline {
    pub fn plain_text(values: &[Self]) -> String {
        let mut out = String::new();
        for value in values {
            match value {
                Self::Text(s) | Self::Code(s) => out.push_str(s),
                Self::Emphasis(s)
                | Self::Strong(s)
                | Self::Strike(s)
                | Self::Link { content: s, .. } => out.push_str(&Self::plain_text(s)),
                Self::WikiLink { display, .. } => out.push_str(display),
                Self::Footnote(s) => {
                    out.push('[');
                    out.push_str(s);
                    out.push(']');
                }
                Self::Task(done) => out.push_str(if *done { "☑ " } else { "☐ " }),
                Self::SoftBreak => out.push(' '),
                Self::HardBreak => out.push('\n'),
                Self::Image { alt, .. } => out.push_str(alt),
            }
        }
        out
    }
}
