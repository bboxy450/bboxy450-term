//! Resolve record links against a host-supplied catalogue. File IO, navigation
//! and authorization remain with the host; ambiguous handles never pick a file.
use crate::Markdown;

#[derive(Clone, Copy, Debug)]
pub struct Record<'a> {
    pub path: &'a str,
    pub markdown: &'a Markdown,
}

#[derive(Debug)]
pub struct Resolved<'a> {
    pub record: Record<'a>,
    /// A slug link may retain a heading anchor. `#47` alone is a record handle.
    pub anchor: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveError {
    Missing,
    Ambiguous,
}
impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Missing => "record link does not resolve",
            Self::Ambiguous => "record link resolves to multiple records",
        })
    }
}
impl std::error::Error for ResolveError {}

fn path_key(value: &str) -> &str {
    let value = value.strip_prefix(".bboxy/").unwrap_or(value);
    value.strip_suffix(".md").unwrap_or(value)
}

/// Slug, relative path, ULID or `#N`, optionally wrapped as a wikilink. A display
/// alias does not affect identity. A duplicate number/ID/slug is an error even
/// when the first matching record could otherwise be opened.
pub fn resolve<'a>(
    records: impl IntoIterator<Item = Record<'a>>,
    target: &str,
) -> Result<Resolved<'a>, ResolveError> {
    let target = target.trim();
    let target = target
        .strip_prefix("[[")
        .and_then(|s| s.strip_suffix("]]"))
        .unwrap_or(target);
    // GFM tables escape the same alias delimiter as \|. The escape is
    // formatting, not part of the record's identity.
    let target = target
        .split_once('|')
        .map_or(target, |(key, _)| key.strip_suffix('\\').unwrap_or(key))
        .trim();
    if target.is_empty() {
        return Err(ResolveError::Missing);
    }
    let number = if let Some(digits) = target.strip_prefix('#') {
        Some(
            digits
                .parse::<u64>()
                .ok()
                .filter(|n| *n > 0 && n.to_string() == digits)
                .ok_or(ResolveError::Missing)?,
        )
    } else {
        None
    };
    let (key, anchor) = if number.is_some() {
        (target, None)
    } else {
        target
            .split_once('#')
            .map_or((target, None), |(key, anchor)| (key, Some(anchor)))
    };
    let qualified = key.contains('/');
    let key = path_key(key);
    let mut found = None;
    for record in records {
        let matches = if let Some(number) = number {
            record.markdown.frontmatter.number() == Some(number)
        } else {
            let path = path_key(record.path);
            key == path
                || (!qualified && key == path.rsplit('/').next().unwrap_or(path))
                || record
                    .markdown
                    .frontmatter
                    .fields
                    .get("id")
                    .and_then(serde_yaml::Value::as_str)
                    == Some(key)
        };
        if matches {
            if found.is_some() {
                return Err(ResolveError::Ambiguous);
            }
            found = Some(record);
        }
    }
    found
        .map(|record| Resolved {
            record,
            anchor: anchor.map(str::to_owned),
        })
        .ok_or(ResolveError::Missing)
}
