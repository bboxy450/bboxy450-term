//! Source splitting and label text cleanup shared by the parsers.

use crate::{Label, Measure, Rect, Size, TextRole};

/// Lines of the block with a leading `---` frontmatter block removed, numbered from 1.
pub(crate) fn strip_frontmatter(src: &str) -> Vec<(usize, String)> {
    let lines: Vec<(usize, String)> = src
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect();
    let first = lines.iter().position(|(_, l)| !l.trim().is_empty());
    if let Some(f) = first
        && lines[f].1.trim() == "---"
        && let Some(end) = lines[f + 1..].iter().position(|(_, l)| l.trim() == "---")
    {
        return lines[f + end + 2..].to_vec();
    }
    lines
}

/// Split into statements on newlines and (when `semicolons`) on `;` outside
/// brackets, never inside double quotes, so a quoted label may span lines.
/// Sequence diagrams keep `;`: message text is prose. Drops `%%` comment lines and blank statements.
pub(crate) fn statements(lines: &[(usize, String)], semicolons: bool) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut start = 0;
    let mut quoted = false;
    let mut depth = 0i32;
    for (n, line) in lines {
        if !quoted && line.trim_start().starts_with("%%") {
            continue;
        }
        if cur.is_empty() {
            start = *n;
        }
        for c in line.chars() {
            match c {
                '"' => {
                    quoted = !quoted;
                    cur.push(c);
                }
                '[' | '(' | '{' if !quoted => {
                    depth += 1;
                    cur.push(c);
                }
                ']' | ')' | '}' if !quoted => {
                    depth = (depth - 1).max(0);
                    cur.push(c);
                }
                ';' if semicolons && !quoted && depth == 0 => {
                    push(&mut out, start, &mut cur);
                    start = *n;
                }
                _ => cur.push(c),
            }
        }
        if quoted {
            cur.push('\n');
        } else {
            depth = 0;
            push(&mut out, start, &mut cur);
        }
    }
    push(&mut out, start, &mut cur);
    out
}

fn push(out: &mut Vec<(usize, String)>, n: usize, cur: &mut String) {
    let t = cur.trim();
    if !t.is_empty() {
        out.push((n, t.to_string()));
    }
    cur.clear();
}

/// Label text to display lines: unquote, strip Markdown-string backticks and
/// `**`/`_` emphasis, `<br>` to line breaks, `#name;` / `#123;` entities.
pub(crate) fn clean(raw: &str) -> Vec<String> {
    let mut s = raw.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s = &s[1..s.len() - 1];
    }
    let mut s = s.trim().to_string();
    if s.len() >= 2 && s.starts_with('`') && s.ends_with('`') {
        s = s[1..s.len() - 1].replace("**", "").replace('_', " ");
    }
    let mut out = String::new();
    let mut rest = s.as_str();
    while let Some(i) = rest.find(['<', '#', '&']) {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let lower = rest.to_ascii_lowercase();
        if lower.starts_with("<br")
            && let Some(e) = rest.find('>')
        {
            out.push('\n');
            rest = &rest[e + 1..];
            continue;
        }
        if let Some((txt, len)) = entity(rest) {
            out.push_str(&txt);
            rest = &rest[len..];
            continue;
        }
        out.push(rest.chars().next().unwrap());
        rest = &rest[1..];
    }
    out.push_str(rest);
    out.split('\n').map(|l| l.trim().to_string()).collect()
}

fn entity(s: &str) -> Option<(String, usize)> {
    let end = s.find(';')?;
    if end > 10 {
        return None;
    }
    let body = &s[1..end];
    let t = match body {
        "quot" => "\"".to_string(),
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "nbsp" => " ".to_string(),
        "35" if s.starts_with('#') => "#".to_string(),
        b if !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()) => {
            char::from_u32(b.parse().ok()?)?.to_string()
        }
        _ => return None,
    };
    Some((t, end + 1))
}

/// Measure a block of lines: widest line, summed heights.
pub(crate) fn measure_lines(lines: &[String], role: TextRole, m: &dyn Measure) -> Size {
    let mut s = Size::default();
    for l in lines {
        let one = m.measure(if l.is_empty() { " " } else { l }, role);
        s.w = s.w.max(one.w);
        s.h += one.h;
    }
    s
}

pub(crate) fn label(lines: Vec<String>, role: TextRole, rect: Rect) -> Label {
    Label { lines, role, rect }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_quotes_breaks_and_entities() {
        assert_eq!(
            clean("\"GUI app<br/>A/main.rs:509\""),
            vec!["GUI app", "A/main.rs:509"]
        );
        assert_eq!(
            clean("\"A double quote:#quot;\""),
            vec!["A double quote:\""]
        );
        assert_eq!(
            clean("bbt-proto: decision #6, born"),
            vec!["bbt-proto: decision #6, born"]
        );
        assert_eq!(
            clean("\"`The **cat**\nin the hat`\""),
            vec!["The cat", "in the hat"]
        );
    }

    #[test]
    fn statements_keep_quoted_newlines() {
        let lines = strip_frontmatter(
            "---\nconfig: x\n---\nflowchart LR\na(\"x\ny\") --> b; b --> c\n%% hi",
        );
        let st = statements(&lines, true);
        assert_eq!(st.len(), 3);
        assert_eq!(st[0], (4, "flowchart LR".to_string()));
        assert_eq!(st[1].1, "a(\"x\ny\") --> b");
        assert_eq!(st[2].1, "b --> c");
    }
}
