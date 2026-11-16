// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Header records: every Den record that is not a state machine. One model
//! ([`Record`]), one way in ([`load`]), and the three things the knowledge-base
//! host does with it: read the header ([`metadata`]), read the prose
//! ([`body`]) and write its own block ([`set_managed`]).
use crate::Diagnostic;
use crate::ast::Pos;

/// The record keywords `load` reads. `spec` is here for its prose form; a
/// `spec` with a state machine goes to `check`.
pub(crate) const KINDS: [&str; 11] = [
    "spec",
    "interface",
    "invariant",
    "epic",
    "task",
    "report",
    "e2e",
    "decision",
    "proposal",
    "bug",
    "recipe",
];
/// Words that open a machine declaration: a `spec` using one is a machine.
const MACHINE: [&str; 11] = [
    "state",
    "event",
    "on",
    "enum",
    "effect",
    "invariant",
    "theorem",
    "property",
    "assume",
    "scenario",
    "include",
];
/// Top-level fields a record may repeat (an epic's lines).
const REPEATABLE: [&str; 2] = ["channel", "gate"];
/// Fields whose values are links to other records.
const LINKS: [&str; 8] = [
    "serves",
    "realizes",
    "decides",
    "amends",
    "supersedes",
    "constrains",
    "between",
    "partOf",
];
/// The host's task states, as `crates/bbt-mux/src/mcp/work_items.rs` spells them.
const TASK_STATUS: [&str; 8] = [
    "open", "claimed", "running", "reported", "accepted", "rejected", "blocked", "done",
];
const MAX_DEPTH: usize = 8;

/// One header record: the whole of a `.den` file that is not a machine.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Record {
    /// The record keyword: `task`, `epic`, `report`, `decision`, `e2e`,
    /// `proposal`, `bug`, `recipe`, `invariant`, `interface` or `spec`.
    pub kind: &'static str,
    pub name: String,
    /// Module path segments; a backquoted segment is stored without quotes.
    pub module: Vec<String>,
    pub imports: Vec<Vec<String>>,
    /// The author's fields in source order, unknown ones included.
    pub fields: Vec<Field>,
    /// The fields of the `managed` block, which only the host writes.
    pub managed: Vec<Field>,
    /// One-based source line of the record keyword.
    pub line: u32,
    /// True for a `spec` with a state machine. Its `fields` are only its
    /// `intent` and `title`; the machine itself is read by [`crate::check`].
    pub machine: bool,
}
/// One `name value` line of a record or of a nested block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Field {
    pub name: String,
    /// One-based source line and character column of the name.
    pub line: u32,
    pub column: u32,
    pub value: Value,
}
/// A field's value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Value {
    /// `"text"`.
    Text(String),
    /// `"""text"""`: Markdown, with the common indentation removed.
    Block(String),
    /// `$ command`, to the end of the line, with its `~> exit N` if written.
    Command {
        command: String,
        exit: Option<i64>,
    },
    /// A name, a dotted path or a backquoted name.
    Reference(Vec<String>),
    /// `tag"value"`: `id`, `seat`, `sha`, `date`, `time`, `path`, or any other tag.
    Tagged {
        tag: String,
        value: String,
    },
    Integer(i64),
    /// Two or more of the values above, separated by commas.
    List(Vec<Value>),
    /// `head { ... }`: `head` is the text between the field name and the
    /// brace. A block closed on its opening line keeps each `;`-separated
    /// entry as a [`Value::Raw`] field.
    Nested {
        head: String,
        fields: Vec<Field>,
    },
    /// Any other line, verbatim: `Decided on date"2026-10-29" by Operator`.
    Raw(String),
}
impl Value {
    /// The value as plain text for a header: quotes and tags dropped
    /// (`seat"X"` is `seat:X`, as the host writes seats), lists joined with
    /// `, `. A nested block has no plain text.
    pub fn text(&self) -> String {
        match self {
            Value::Text(text) | Value::Block(text) | Value::Raw(text) => text.clone(),
            Value::Command { command, .. } => command.clone(),
            Value::Reference(path) => path.join("."),
            Value::Tagged { tag, value } if tag == "seat" => format!("seat:{value}"),
            Value::Tagged { value, .. } => value.clone(),
            Value::Integer(value) => value.to_string(),
            Value::List(items) => items.iter().map(Value::text).collect::<Vec<_>>().join(", "),
            Value::Nested { .. } => String::new(),
        }
    }
    /// The single values: a list's items, or the value itself.
    fn items(&self) -> &[Value] {
        match self {
            Value::List(items) => items,
            other => std::slice::from_ref(other),
        }
    }
}

/// True when `check`, `declarations` and `render` read the source as a header
/// record. A `spec` that declares a machine, a `loop` and text with no record
/// keyword stay with the machine parser, which reports them as before.
pub(crate) fn is_record(source: &str) -> bool {
    let first_word = |text: &str| -> String {
        text.trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect()
    };
    let mut in_text = false;
    let mut spec = false;
    for line in source.lines() {
        let inside = in_text;
        if line.matches("\"\"\"").count() % 2 == 1 {
            in_text = !in_text;
        }
        let word = first_word(line);
        if inside || word.is_empty() {
            continue;
        }
        if spec {
            if MACHINE.contains(&word.as_str()) {
                return false;
            }
        } else if word == "spec" {
            spec = true;
            // A machine written on the keyword's own line: `spec T { state { ... } }`.
            let inline = line.split_once('{').map_or("", |(_, rest)| rest);
            if MACHINE.contains(&first_word(inline).as_str()) {
                return false;
            }
        } else if word != "module" && word != "import" {
            return word != "loop";
        }
    }
    spec
}

/// A parsed record and the byte offsets `set_managed` edits at.
pub(crate) struct Parsed {
    pub record: Record,
    /// Line of the `managed` keyword.
    pub managed_line: Option<u32>,
    /// Byte range of each managed field's value, parallel to `record.managed`.
    values: Vec<(usize, usize)>,
    /// Offset of the `managed` keyword.
    managed_open: Option<usize>,
    /// Offset of the `managed` block's closing brace.
    managed_close: Option<usize>,
    /// Byte range of each top-level field's value, parallel to `record.fields`.
    top: Vec<(usize, usize)>,
    /// Offset of the record's closing brace.
    close: usize,
}

struct Scan<'a> {
    src: &'a str,
    i: usize,
    record: String,
    /// End offset of the value `value` last returned.
    end: usize,
    managed: Vec<Field>,
    managed_line: Option<u32>,
    values: Vec<(usize, usize)>,
    managed_open: Option<usize>,
    managed_close: Option<usize>,
    top: Vec<(usize, usize)>,
}
type Scanned<T> = Result<T, Diagnostic>;

impl<'a> Scan<'a> {
    fn pos(&self, at: usize) -> Pos {
        let before = &self.src[..at];
        let line_start = before.rfind('\n').map_or(0, |n| n + 1);
        Pos {
            line: 1 + before.matches('\n').count() as u32,
            column: 1 + before[line_start..].chars().count() as u32,
        }
    }
    fn error(&self, code: &'static str, at: usize, message: String, repair: &str) -> Diagnostic {
        self.pos(at).diagnostic(code, &self.record, message, repair)
    }
    fn syntax(&self, at: usize, message: &str) -> Diagnostic {
        self.error(
            "X101",
            at,
            message.into(),
            "write one `name value` field per line inside `kind Name { ... }`",
        )
    }
    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.i).copied()
    }
    fn starts(&self, text: &str) -> bool {
        self.src[self.i..].starts_with(text)
    }
    fn line_end(&self, from: usize) -> usize {
        self.src[from..]
            .find('\n')
            .map_or(self.src.len(), |n| from + n)
    }
    /// Skips spaces within the line.
    fn spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r')) {
            self.i += 1;
        }
    }
    /// Skips everything between statements: space, newlines, `;` and comments.
    fn blank(&mut self) {
        loop {
            self.spaces();
            if self.starts("//") {
                self.i = self.line_end(self.i);
            } else if matches!(self.peek(), Some(b'\n' | b';')) {
                self.i += 1;
            } else {
                return;
            }
        }
    }
    fn word(&mut self) -> Option<&'a str> {
        let start = self.i;
        if !self
            .peek()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        {
            return None;
        }
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            self.i += 1;
        }
        Some(&self.src[start..self.i])
    }
    fn unterminated(&self, at: usize, what: &str, close: &str) -> Diagnostic {
        self.error(
            "X102",
            at,
            format!("this {what} is not terminated"),
            &format!("close it with {close}"),
        )
    }
    /// `"text"` on one line, with `\"`, `\\`, `\n`, `\r` and `\t`.
    fn string(&mut self) -> Scanned<String> {
        let start = self.i;
        self.i += 1;
        let mut value = String::new();
        let mut chars = self.src[self.i..].char_indices();
        while let Some((offset, c)) = chars.next() {
            match c {
                '"' => {
                    self.i += offset + 1;
                    return Ok(value);
                }
                '\n' => break,
                '\\' => value.push(match chars.next() {
                    Some((_, 'n')) => '\n',
                    Some((_, 'r')) => '\r',
                    Some((_, 't')) => '\t',
                    Some((_, '"')) => '"',
                    Some((_, '\\')) => '\\',
                    _ => return Err(self.syntax(self.i + offset, "unknown string escape")),
                }),
                c => value.push(c),
            }
        }
        Err(self.unterminated(start, "string", "`\"` on the same line"))
    }
    /// `"""text"""`, across lines, with no escapes.
    fn block(&mut self) -> Scanned<String> {
        let start = self.i;
        let Some(length) = self.src[start + 3..].find("\"\"\"") else {
            return Err(self.unterminated(start, "text block", "`\"\"\"`"));
        };
        self.i = start + 3 + length + 3;
        Ok(dedent(&self.src[start + 3..start + 3 + length]))
    }
    /// A name, or a backquoted name such as `` `2026-11-09-den-checker-crate` ``.
    fn segment(&mut self) -> Scanned<String> {
        if self.peek() != Some(b'`') {
            return match self.word() {
                Some(word) => Ok(word.to_owned()),
                None => Err(self.syntax(self.i, "expected a name")),
            };
        }
        let start = self.i;
        let line = &self.src[start + 1..self.line_end(start)];
        let Some(length) = line.find('`') else {
            return Err(self.unterminated(
                start,
                "backquoted name",
                "a backquote on the same line",
            ));
        };
        self.i = start + 1 + length + 1;
        Ok(line[..length].to_owned())
    }
    fn path(&mut self) -> Scanned<Vec<String>> {
        let mut path = vec![self.segment()?];
        while self.peek() == Some(b'.')
            && self
                .src
                .as_bytes()
                .get(self.i + 1)
                .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, b'_' | b'`'))
        {
            self.i += 1;
            path.push(self.segment()?);
        }
        Ok(path)
    }
    /// The value from here to the end of the statement.
    fn value(&mut self, depth: usize) -> Scanned<Value> {
        self.spaces();
        let start = self.i;
        if self.peek() == Some(b'$') {
            self.i = self.line_end(start);
            self.end = self.i;
            let text = self.src[start + 1..self.i].trim();
            let exit = text
                .rsplit_once("~> exit")
                .and_then(|(command, code)| Some((command.trim(), code.trim().parse().ok()?)));
            return Ok(match exit {
                Some((command, code)) => Value::Command {
                    command: command.to_owned(),
                    exit: Some(code),
                },
                None => Value::Command {
                    command: text.to_owned(),
                    exit: None,
                },
            });
        }
        let mut items = Vec::new();
        // False once the line is not `item, item, ...`; it is then kept raw.
        let mut typed = true;
        let mut want_item = true;
        self.end = start;
        loop {
            self.spaces();
            let Some(c) = self.peek() else { break };
            // A list continues on the next line after a comma.
            if c == b'\n' && typed && want_item && !items.is_empty() {
                self.i += 1;
                continue;
            }
            if matches!(c, b'\n' | b';' | b'}') || self.starts("//") {
                break;
            }
            let item = if c == b'{' {
                return self.nested(start, depth);
            } else if self.starts("\"\"\"") {
                Some(Value::Block(self.block()?))
            } else if c == b'"' {
                Some(Value::Text(self.string()?))
            } else if c == b'`' || c.is_ascii_alphabetic() || c == b'_' {
                let mut path = self.path()?;
                if c != b'`' && path.len() == 1 && self.peek() == Some(b'"') {
                    Some(Value::Tagged {
                        tag: path.remove(0),
                        value: self.string()?,
                    })
                } else {
                    Some(Value::Reference(path))
                }
            } else if c.is_ascii_digit() {
                let digits = self.i;
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.i += 1;
                }
                self.src[digits..self.i].parse().ok().map(Value::Integer)
            } else {
                self.i += self.src[self.i..].chars().next().map_or(1, char::len_utf8);
                if c == b',' && !want_item {
                    want_item = true;
                    self.end = self.i;
                    continue;
                }
                None
            };
            self.end = self.i;
            match item {
                Some(item) if want_item => {
                    items.push(item);
                    want_item = false;
                }
                _ => typed = false,
            }
        }
        if self.end == start {
            return Err(self.syntax(start, "this field has no value"));
        }
        Ok(if !typed || want_item {
            Value::Raw(self.src[start..self.end].trim().to_owned())
        } else if items.len() == 1 {
            items.remove(0)
        } else {
            Value::List(items)
        })
    }
    /// At `{`: a block closed on this line keeps its entries raw; a block
    /// that continues on the next lines holds fields.
    fn nested(&mut self, start: usize, depth: usize) -> Scanned<Value> {
        let brace = self.i;
        let head = self.src[start..brace].trim().to_owned();
        if depth >= MAX_DEPTH {
            return Err(self.syntax(brace, "blocks are nested too deeply"));
        }
        let rest = self.src[brace + 1..self.line_end(brace)].trim_end();
        let fields = if let Some(inner) = rest.strip_suffix('}') {
            let pos = self.pos(brace);
            self.i = brace + 1 + rest.len();
            inner
                .split(';')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(|entry| {
                    let (name, value) =
                        entry.split_once(char::is_whitespace).unwrap_or((entry, ""));
                    Field {
                        name: name.to_owned(),
                        line: pos.line,
                        column: pos.column,
                        value: Value::Raw(value.trim().to_owned()),
                    }
                })
                .collect()
        } else {
            self.i = brace + 1;
            self.fields(depth + 1, brace)?.0
        };
        self.end = self.i;
        Ok(Value::Nested { head, fields })
    }
    /// Fields up to the closing brace; returns them and the brace's offset.
    fn fields(&mut self, depth: usize, open: usize) -> Scanned<(Vec<Field>, usize)> {
        let mut fields = Vec::new();
        loop {
            self.blank();
            let at = self.i;
            match self.peek() {
                None => return Err(self.syntax(open, "this block is never closed")),
                Some(b'}') => {
                    self.i += 1;
                    return Ok((fields, at));
                }
                _ => {}
            }
            let Some(name) = self.word() else {
                return Err(self.syntax(at, "expected a field name"));
            };
            if depth == 0 && name == "managed" {
                self.managed_block(at)?;
                continue;
            }
            let pos = self.pos(at);
            self.spaces();
            let start = self.i;
            let value = self.value(depth)?;
            if depth == 0 {
                self.top.push((start, self.end));
            }
            fields.push(Field {
                name: name.to_owned(),
                line: pos.line,
                column: pos.column,
                value,
            });
        }
    }
    /// `managed { name = value ... }`.
    fn managed_block(&mut self, at: usize) -> Scanned<()> {
        if self.managed_line.is_some() {
            return Err(self.error(
                "X356",
                at,
                "a record has one managed block".into(),
                "merge the two managed blocks; only the host writes them",
            ));
        }
        self.managed_line = Some(self.pos(at).line);
        self.managed_open = Some(at);
        self.spaces();
        if self.peek() != Some(b'{') {
            return Err(self.syntax(self.i, "expected { after managed"));
        }
        self.i += 1;
        loop {
            self.blank();
            let field = self.i;
            match self.peek() {
                None => return Err(self.syntax(at, "this block is never closed")),
                Some(b'}') => {
                    self.managed_close = Some(field);
                    self.i += 1;
                    return Ok(());
                }
                _ => {}
            }
            let name = self.word();
            self.spaces();
            let (Some(name), Some(b'=')) = (name, self.peek()) else {
                return Err(self.syntax(field, "a managed field is written `name = value`"));
            };
            self.i += 1;
            self.spaces();
            let start = self.i;
            let value = self.value(MAX_DEPTH)?;
            let pos = self.pos(field);
            self.values.push((start, self.end));
            self.managed.push(Field {
                name: name.to_owned(),
                line: pos.line,
                column: pos.column,
                value,
            });
        }
    }
    fn record(mut self) -> Scanned<Parsed> {
        self.blank();
        if self.word() != Some("module") {
            return Err(self.syntax(self.i, "expected module"));
        }
        self.spaces();
        let module = self.path()?;
        let mut imports = Vec::new();
        let (at, keyword) = loop {
            self.blank();
            let at = self.i;
            match self.word() {
                Some("import") => {
                    self.spaces();
                    imports.push(self.path()?);
                }
                keyword => break (at, keyword),
            }
        };
        let Some(kind) = KINDS.iter().find(|kind| Some(**kind) == keyword) else {
            return Err(self.error(
                "X152",
                at,
                format!(
                    "{} is not a record kind",
                    keyword.unwrap_or("the text after the imports")
                ),
                &format!("start the record with one of: {}", KINDS.join(", ")),
            ));
        };
        self.spaces();
        let Some(name) = self.word() else {
            return Err(self.syntax(self.i, "expected the record name"));
        };
        self.record = name.to_owned();
        self.spaces();
        let open = self.i;
        if self.peek() != Some(b'{') {
            return Err(self.syntax(open, "expected { after the record name"));
        }
        self.i += 1;
        let (fields, close) = self.fields(0, open)?;
        self.blank();
        if self.i < self.src.len() {
            return Err(self.syntax(self.i, "a file holds one record"));
        }
        let line = self.pos(at).line;
        Ok(Parsed {
            record: Record {
                kind,
                name: name.to_owned(),
                module,
                imports,
                fields,
                managed: self.managed,
                line,
                machine: false,
            },
            managed_line: self.managed_line,
            values: self.values,
            managed_open: self.managed_open,
            managed_close: self.managed_close,
            top: self.top,
            close,
        })
    }
}

/// A text block's content: the common indentation of its lines removed, and
/// the blank edges left by putting the quotes on their own lines dropped.
fn dedent(raw: &str) -> String {
    let Some((first, rest)) = raw.split_once('\n') else {
        return raw.trim().to_owned();
    };
    let indent_of = |line: &str| line.len() - line.trim_start_matches([' ', '\t']).len();
    let indent = rest
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(indent_of)
        .min()
        .unwrap_or(0);
    let mut lines: Vec<&str> = Vec::new();
    if !first.trim().is_empty() {
        lines.push(first.trim());
    }
    lines.extend(
        rest.lines()
            .map(|line| line.get(indent..).unwrap_or("").trim_end()),
    );
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// Syntax only: the record as written, sound or not.
pub(crate) fn parse(source: &str) -> Result<Parsed, Vec<Diagnostic>> {
    Scan {
        src: source,
        i: 0,
        record: String::new(),
        end: 0,
        managed: Vec::new(),
        managed_line: None,
        values: Vec::new(),
        managed_open: None,
        managed_close: None,
        top: Vec::new(),
    }
    .record()
    .map_err(|diagnostic| vec![diagnostic])
}

/// A machine spec's host part: its `intent`, `title` and `managed` block,
/// found where the machine parser says they start. The machine is checked for
/// syntax only; a repeated managed field is X356, as on a header record.
fn machine(source: &str) -> Result<Parsed, Vec<Diagnostic>> {
    let spec = crate::parser::parse(source)?;
    let offset = |pos: Pos| {
        let start: usize = source
            .split_inclusive('\n')
            .take(pos.line as usize - 1)
            .map(str::len)
            .sum();
        start
            + source[start..]
                .chars()
                .take(pos.column as usize - 1)
                .map(char::len_utf8)
                .sum::<usize>()
    };
    let mut scan = Scan {
        src: source,
        i: 0,
        record: spec.name.text.clone(),
        end: 0,
        managed: Vec::new(),
        managed_line: None,
        values: Vec::new(),
        managed_open: None,
        managed_close: None,
        top: Vec::new(),
    };
    let mut fields = Vec::new();
    for (word, pos) in &spec.header {
        let at = offset(*pos);
        scan.i = at + word.len();
        if *word == "managed" {
            scan.managed_block(at).map_err(|d| vec![d])?;
            continue;
        }
        scan.spaces();
        let start = scan.i;
        let value = scan.value(MAX_DEPTH).map_err(|d| vec![d])?;
        scan.top.push((start, scan.end));
        fields.push(Field {
            name: (*word).to_owned(),
            line: pos.line,
            column: pos.column,
            value,
        });
    }
    let names = |path: &[crate::ast::Name]| path.iter().map(|n| n.text.clone()).collect();
    let record = Record {
        kind: "spec",
        name: spec.name.text.clone(),
        module: names(&spec.module),
        imports: spec.imports.iter().map(|path| names(path)).collect(),
        fields,
        managed: scan.managed,
        line: spec.name.pos.line,
        machine: true,
    };
    let repeated = record.managed.iter().enumerate().find(|(index, field)| {
        record.managed[..*index]
            .iter()
            .any(|earlier| earlier.name == field.name)
    });
    if let Some((_, field)) = repeated {
        return Err(vec![
            Pos {
                line: field.line,
                column: field.column,
            }
            .diagnostic(
                "X356",
                &record.name,
                format!("{} is written twice", field.name),
                "keep one line for this field",
            ),
        ]);
    }
    Ok(Parsed {
        record,
        managed_line: scan.managed_line,
        values: scan.values,
        managed_open: scan.managed_open,
        managed_close: scan.managed_close,
        top: scan.top,
        close: offset(spec.close),
    })
}
/// [`parse`] for a header record, [`machine`] for a machine spec.
fn parse_any(source: &str) -> Result<Parsed, Vec<Diagnostic>> {
    if is_record(source) {
        parse(source)
    } else {
        machine(source)
    }
}

/// The source without its `managed` block: what the author wrote. A block on
/// lines of its own goes with those lines and one blank line before them, the
/// way the host adds it. A source that does not parse is returned as it is.
pub fn unmanaged(source: &str) -> String {
    let Some((open, close)) = parse_any(source)
        .ok()
        .and_then(|parsed| parsed.managed_open.zip(parsed.managed_close))
    else {
        return source.to_owned();
    };
    let end = close + 1;
    let start = source[..open].rfind('\n').map_or(0, |n| n + 1);
    let stop = source[end..]
        .find('\n')
        .map_or(source.len(), |n| end + n + 1);
    if !source[start..open].trim().is_empty() || !source[end..stop].trim().is_empty() {
        return format!("{}{}", &source[..open], &source[end..]);
    }
    let before = &source[..start];
    let blank = before
        .strip_suffix('\n')
        .and_then(|rest| {
            let line = rest.rfind('\n').map_or(0, |n| n + 1);
            rest[line..].trim().is_empty().then_some(line)
        })
        .filter(|_| start > 0);
    format!("{}{}", &source[..blank.unwrap_or(start)], &source[stop..])
}

/// The fields a kind cannot do without.
fn required(record: &Record) -> &'static [&'static str] {
    match record.kind {
        "task" => &["title", "verify", "brief"],
        "epic" | "spec" => &["intent"],
        "report" => &["task", "author", "verify"],
        "decision" => &["status"],
        "e2e" => &["function", "expected"],
        _ => &[],
    }
}
/// The import a reference names: the whole path, or its last segment.
fn import<'a>(record: &'a Record, path: &[String]) -> Option<&'a [String]> {
    record
        .imports
        .iter()
        .find(|import| {
            import.as_slice() == path || (path.len() == 1 && import.last() == path.first())
        })
        .map(Vec::as_slice)
}
fn is_link(record: &Record, field: &Field) -> bool {
    LINKS.contains(&field.name.as_str()) || (record.kind == "report" && field.name == "task")
}

fn check(record: &Record) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut report = |code, field: Option<&Field>, message: String, repair: &str| {
        let pos = field.map_or(
            Pos {
                line: record.line,
                column: 1,
            },
            |field| Pos {
                line: field.line,
                column: field.column,
            },
        );
        diagnostics.push(pos.diagnostic(code, &record.name, message, repair));
    };
    for (fields, code) in [(&record.fields, "X206"), (&record.managed, "X356")] {
        for (index, field) in fields.iter().enumerate() {
            let repeated = fields[..index]
                .iter()
                .any(|earlier| earlier.name == field.name);
            if repeated && (code == "X356" || !REPEATABLE.contains(&field.name.as_str())) {
                report(
                    code,
                    Some(field),
                    format!("{} is written twice", field.name),
                    "keep one line for this field",
                );
            }
        }
    }
    // A record the host has numbered is history: a field missing there is a
    // fact about the past, not an authoring error. New records must have it.
    let stamped = record.managed.iter().any(|field| field.name == "number");
    for name in required(record).iter().filter(|_| !stamped) {
        let present = record
            .fields
            .iter()
            .any(|field| field.name == *name && !field.value.text().trim().is_empty());
        if !present {
            report(
                "X354",
                None,
                format!("a {} needs a {name}", record.kind),
                &format!("add a `{name}` field to {}", record.name),
            );
        }
    }
    for field in record.fields.iter().filter(|field| is_link(record, field)) {
        for item in field.value.items() {
            let known = match item {
                Value::Reference(path) => {
                    path.as_slice() == [record.name.clone()] || import(record, path).is_some()
                }
                _ => false,
            };
            if !known {
                report(
                    "X205",
                    Some(field),
                    format!(
                        "{} names {}, which is neither imported nor this record",
                        field.name,
                        item.text()
                    ),
                    "add an `import` line for the record, or correct the name",
                );
            }
        }
    }
    if record.kind == "task" {
        for field in record
            .fields
            .iter()
            .chain(&record.managed)
            .filter(|field| field.name == "status")
        {
            let status = field.value.text().to_ascii_lowercase();
            if !TASK_STATUS.contains(&status.as_str()) {
                report(
                    "X357",
                    Some(field),
                    format!("{} is not a task status", field.value.text()),
                    "use Open, Claimed, Running, Reported, Accepted, Rejected, Blocked or Done",
                );
            }
        }
    }
    diagnostics
}

/// Loads the header records of one source: the only way in. A file holds one
/// record today, so the vector has one element; the type leaves room for more
/// without a change. `Err` carries every reason the source is unsound, each
/// with its code, position and repair. A `spec` with a state machine is not a
/// header record; it is refused with X190 and goes to [`crate::check`].
///
/// | Code | Unsound condition |
/// |---|---|
/// | X102 | A string, text block or backquoted name is not terminated |
/// | X152 | The record keyword is not a record kind |
/// | X205 | A link names a record that is neither imported nor this record |
/// | X206 | A field is written twice |
/// | X354 | A field the record's kind requires is missing or empty |
/// | X356 | A `managed` field, or the `managed` block, is written twice |
/// | X357 | A task status is not one the host knows |
///
/// Any other malformed text is X101, as for a machine. Unknown fields are
/// kept: the model is additive.
pub fn load(source: &str) -> Result<Vec<Record>, Vec<Diagnostic>> {
    let record = parse(source)?.record;
    let machine = record
        .fields
        .iter()
        .find(|field| record.kind == "spec" && MACHINE.contains(&field.name.as_str()));
    let diagnostics = match machine {
        Some(field) => vec![
            Pos {
                line: field.line,
                column: field.column,
            }
            .diagnostic(
                "X190",
                &record.name,
                "a spec with a state machine is not a header record",
                "read it with check, model or declarations",
            ),
        ],
        None => check(&record),
    };
    if diagnostics.is_empty() {
        Ok(vec![record])
    } else {
        Err(diagnostics)
    }
}

/// The record's title: its `title`, else the first sentence of its `intent`,
/// else its name.
fn title(record: &Record) -> String {
    let field = |name: &str| {
        record
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.value.text())
    };
    field("title")
        .or_else(|| {
            field("intent").map(|intent| {
                let intent = intent.trim();
                intent
                    .find(". ")
                    .map_or(intent, |end| &intent[..=end])
                    .to_owned()
            })
        })
        .unwrap_or_else(|| record.name.clone())
}
/// True for the fields [`body`] returns.
fn is_prose(field: &Field) -> bool {
    matches!(field.name.as_str(), "brief" | "body" | "steps")
        && matches!(field.value, Value::Text(_) | Value::Block(_))
}
/// The top-level fields [`metadata`] cannot carry: nested blocks and the
/// lines a record repeats. Lowering lists them under `not_lowered:`.
pub(crate) fn not_lowered(record: &Record) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();
    for field in &record.fields {
        let nested = matches!(field.value, Value::Nested { .. });
        if (nested || REPEATABLE.contains(&field.name.as_str()))
            && !names.contains(&field.name.as_str())
        {
            names.push(&field.name);
        }
    }
    names
}

/// The record as the flat header the knowledge-base host keeps today, in a
/// fixed order: `type`, `title`, the author's fields, then the managed ones.
///
/// | Den | Header key |
/// |---|---|
/// | record keyword | `type` |
/// | `title`, else the first sentence of `intent`, else the name | `title` |
/// | `serves`, `realizes`, `decides`, `amends` | `links`: `[[module.path]]`, comma separated |
/// | `paths` | `allowed_paths` |
/// | `verify $ command ~> exit N` | `verify`, and `verify_exit` when written |
/// | `status Word rest` | `status` (lowercase), and `status_note` for the rest |
/// | `managed.status` | `status` (lowercase) |
/// | `managed.delegator` | `delegated_by` |
/// | any other field, managed or not | its own name |
///
/// `brief`, `body` and `steps` are the [`body`]. Nested blocks and repeated
/// lines (`staff`, `work`, `channel`, `gate`, ...) have no flat form and are
/// left out. A key that would occur twice is prefixed `den_` the second time.
pub fn metadata(record: &Record) -> Vec<(String, String)> {
    let mut out = vec![
        ("type".to_owned(), record.kind.to_owned()),
        ("title".to_owned(), title(record)),
    ];
    fn push(out: &mut Vec<(String, String)>, key: &str, value: String) {
        let key = if out.iter().any(|(existing, _)| existing == key) {
            format!("den_{key}")
        } else {
            key.to_owned()
        };
        out.push((key, value));
    }
    let relation = |field: &Field| {
        matches!(
            field.name.as_str(),
            "serves" | "realizes" | "decides" | "amends"
        )
    };
    let mut links = record
        .fields
        .iter()
        .filter(|field| relation(field))
        .flat_map(|field| field.value.items())
        .map(|item| match item {
            Value::Reference(path) => {
                format!("[[{}]]", import(record, path).unwrap_or(path).join("."))
            }
            other => other.text(),
        })
        .collect::<Vec<_>>();
    let skipped = not_lowered(record);
    for field in &record.fields {
        let name = field.name.as_str();
        if name == "title" || is_prose(field) || skipped.contains(&name) {
            continue;
        }
        if relation(field) {
            if !links.is_empty() {
                push(&mut out, "links", std::mem::take(&mut links).join(", "));
            }
            continue;
        }
        let text = field.value.text();
        match (name, &field.value) {
            ("paths", _) => push(&mut out, "allowed_paths", text),
            ("verify", Value::Command { exit, .. }) => {
                push(&mut out, "verify", text);
                if let Some(exit) = exit {
                    push(&mut out, "verify_exit", exit.to_string());
                }
            }
            ("status", _) => {
                let (status, note) = text.split_once(char::is_whitespace).unwrap_or((&text, ""));
                push(&mut out, "status", status.to_ascii_lowercase());
                if !note.trim().is_empty() {
                    push(&mut out, "status_note", note.trim().to_owned());
                }
            }
            _ => push(&mut out, name, text),
        }
    }
    for field in &record.managed {
        let text = field.value.text();
        match field.name.as_str() {
            "status" => push(&mut out, "status", text.to_ascii_lowercase()),
            "delegator" => push(&mut out, "delegated_by", text),
            name => push(&mut out, name, text),
        }
    }
    out
}

/// The record's prose as Markdown: its `brief`, `body` and `steps` text, in
/// source order, separated by a blank line.
pub fn body(record: &Record) -> String {
    record
        .fields
        .iter()
        .filter(|field| is_prose(field))
        .map(|field| field.value.text())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Sets one field of the host-owned `managed` block and returns the new
/// source. `value` is Den text (`Claimed`, `seat"01M..."`). Only the value's
/// bytes change when the field exists; otherwise one line is added, and the
/// `managed { }` block with it when the record has none. Every other byte is
/// kept. `record` must be the name of the record in `source` (X201), and the
/// result must load: an unsound source or value is refused with its diagnostics.
pub fn set_managed(
    source: &str,
    record: &str,
    field: &str,
    value: &str,
) -> Result<String, Vec<Diagnostic>> {
    let parsed = parse_any(source)?;
    let at = Pos {
        line: parsed.record.line,
        column: 1,
    };
    if parsed.record.name != record {
        return Err(vec![at.diagnostic(
            "X201",
            &parsed.record.name,
            format!("this file holds {}, not {record}", parsed.record.name),
            "pass the name of the record the source declares",
        )]);
    }
    let line_start = |offset: usize| source[..offset].rfind('\n').map_or(0, |n| n + 1);
    let indent_at = |offset: usize| {
        let line = &source[line_start(offset)..];
        &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
    };
    // Where the new text goes, and what it replaces.
    let existing = parsed.record.managed.iter().position(|f| f.name == field);
    let (start, end, text) = if let Some(index) = existing {
        let (start, end) = parsed.values[index];
        (start, end, value.to_owned())
    } else if let Some(close) = parsed.managed_close {
        let own_line = source[line_start(close)..close].trim().is_empty();
        match parsed.values.first() {
            Some(&(first, _)) if own_line => {
                let at = line_start(close);
                (at, at, format!("{}{field} = {value}\n", indent_at(first)))
            }
            None if own_line => {
                let at = line_start(close);
                (at, at, format!("{}  {field} = {value}\n", indent_at(close)))
            }
            _ => match parsed.values.last() {
                Some(&(_, last)) => (last, last, format!("; {field} = {value}")),
                None => (close, close, format!("{field} = {value} ")),
            },
        }
    } else if source[line_start(parsed.close)..parsed.close]
        .trim()
        .is_empty()
    {
        let at = line_start(parsed.close);
        let indent = indent_at(parsed.close);
        let block = format!("{indent}  managed {{\n{indent}    {field} = {value}\n{indent}  }}\n");
        (at, at, block)
    } else {
        let at = parsed.close;
        (at, at, format!(" managed {{ {field} = {value} }} "))
    };
    let out = format!("{}{text}{}", &source[..start], &source[end..]);
    // The edit must be exactly one managed value: a `value` that closes the
    // block or opens a field would change the author's part of the record.
    let written = if parsed.record.machine {
        machine(&out)?.record
    } else {
        load(&out)?.remove(0)
    };
    let before = &parsed.record;
    let expected = before.managed.len() + usize::from(existing.is_none());
    if value.contains('\n') || written.fields != before.fields || written.managed.len() != expected
    {
        return Err(vec![at.diagnostic(
            "X101",
            record,
            format!("{value:?} is not one Den value"),
            "pass a single value: a name, a string, an integer or a tagged literal",
        )]);
    }
    Ok(out)
}

/// Where the host edits a record's source: byte ranges into the text
/// [`layout`] was given.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Layout {
    /// The value of each top-level field, parallel to [`Record::fields`].
    pub values: Vec<(usize, usize)>,
    /// The `managed` block, from its keyword through its closing brace.
    pub managed: Option<(usize, usize)>,
    /// The record's closing brace.
    pub close: usize,
}
/// Loads the record like [`load`] and says where its parts are, so the host
/// can replace a prose value or its whole `managed` block and keep every
/// other byte. A machine spec is read for its syntax only, as a record whose
/// fields are its `intent` and `title` ([`Record::machine`]). The host loads
/// its result again: an edit is not checked here.
pub fn layout(source: &str) -> Result<(Record, Layout), Vec<Diagnostic>> {
    let (record, parsed) = if is_record(source) {
        (load(source)?.remove(0), parse(source)?)
    } else {
        let parsed = machine(source)?;
        (parsed.record.clone(), parsed)
    };
    let layout = Layout {
        values: parsed.top,
        managed: parsed
            .managed_open
            .zip(parsed.managed_close)
            .map(|(open, close)| (open, close + 1)),
        close: parsed.close,
    };
    Ok((record, layout))
}
