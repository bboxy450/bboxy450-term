//! Markdown parsing and retained document composition. Files, navigation,
//! signature verification and renderer execution belong to the host.
pub mod links;
pub mod markdown;
pub mod mermaid;
pub use markdown::{Block, BlockKind, Frontmatter, Inline, Markdown, TableAlign, parse};
