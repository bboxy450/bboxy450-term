// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The items of a source file.
//!
//! Three tiers (decision "Rust hardcore, Den for knowledge, other languages as
//! sources to port from"): Rust in full from its syntax tree; Den through the Den
//! crate's own parser; Python, TypeScript and Go by syntax tree only, so a port
//! can enumerate what there is to rebuild. A Lean proof file is listed by its
//! declarations. Every other file has no items.
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

/// One definition: a function, type, impl, module, constant, macro or enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Rust: fn, struct, enum, variant, union, trait, impl, mod, const, static,
    /// type, macro. Den: spec, enum, field, event, effect, handler, invariant,
    /// theorem, property, assumption, scenario. Python, TypeScript, Go: fn,
    /// method, class, interface, type, enum. Lean: its own keywords.
    pub kind: &'static str,
    /// The defined name; for an impl, `Type` or `Trait for Type`.
    pub name: String,
    /// First and last line, counted from 1.
    pub start: usize,
    pub end: usize,
    /// The item's first line up to its body.
    pub signature: String,
}

impl Item {
    pub fn contains(&self, other: &Item) -> bool {
        self.start <= other.start && other.end <= self.end && self != other
    }
    /// Whether a lookup for `name` means this item. An impl answers to its type.
    pub fn named(&self, name: &str) -> bool {
        self.name == name || (self.kind == "impl" && self.name.rsplit(' ').next() == Some(name))
    }
    /// The innermost item around this one, as `kind name`.
    pub fn parent(&self, all: &[Item]) -> Option<String> {
        all.iter()
            .filter(|other| other.contains(self))
            .min_by_key(|other| other.end - other.start)
            .map(|other| format!("{} {}", other.kind, other.name))
    }
    /// `impl Type :: fn name`, outermost first.
    pub fn label(&self, all: &[Item]) -> String {
        let mut chain: Vec<&Item> = all.iter().filter(|other| other.contains(self)).collect();
        chain.sort_by_key(|other| other.start);
        chain.push(self);
        chain
            .iter()
            .map(|item| format!("{} {}", item.kind, item.name))
            .collect::<Vec<_>>()
            .join(" :: ")
    }
}

/// The label of the innermost item containing `line`.
pub fn enclosing(all: &[Item], line: usize) -> Option<String> {
    all.iter()
        .filter(|item| item.start <= line && line <= item.end)
        .min_by_key(|item| item.end - item.start)
        .map(|item| item.label(all))
}

/// Whether files of this type have items at all.
pub fn supported(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "den" | "lean" | "py" | "ts" | "tsx" | "go")
    )
}

/// Every item of the file at `path` with this `source`, in source order.
pub fn items_of(path: &Path, source: &str) -> Vec<Item> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => items(source),
        Some("den") => den(source),
        Some("lean") => lean(source),
        Some("py") => parsed(source, tree_sitter_python::LANGUAGE.into(), python),
        Some("ts") => parsed(
            source,
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            typescript,
        ),
        Some("tsx") => parsed(
            source,
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            typescript,
        ),
        Some("go") => parsed(source, tree_sitter_go::LANGUAGE.into(), go),
        _ => vec![],
    }
}

/// Every item in Rust `source`, in source order. A file that does not parse
/// yields the items of the parts that do.
pub fn items(source: &str) -> Vec<Item> {
    parsed(source, tree_sitter_rust::LANGUAGE.into(), rust)
}

/// The kind of a syntax node that is an item, and how its name is spelled.
type Kinds = fn(Node, &str) -> Option<(&'static str, String)>;

fn parsed(source: &str, language: Language, kinds: Kinds) -> Vec<Item> {
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return vec![];
    }
    let Some(tree) = parser.parse(source, None) else {
        return vec![];
    };
    let mut out = vec![];
    collect(tree.root_node(), source, kinds, &mut out);
    out
}

fn text(node: Node, source: &str) -> String {
    source.get(node.byte_range()).unwrap_or_default().to_owned()
}

fn named(node: Node, source: &str) -> String {
    node.child_by_field_name("name")
        .map(|name| text(name, source))
        .unwrap_or_default()
}

fn rust(node: Node, source: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "function_item" | "function_signature_item" => "fn",
        "struct_item" => "struct",
        "enum_item" => "enum",
        "enum_variant" => "variant",
        "union_item" => "union",
        "trait_item" => "trait",
        "impl_item" => "impl",
        "mod_item" => "mod",
        "const_item" => "const",
        "static_item" => "static",
        "type_item" => "type",
        "macro_definition" => "macro",
        _ => return None,
    };
    if kind != "impl" {
        return Some((kind, named(node, source)));
    }
    let target = node
        .child_by_field_name("type")
        .map(|n| text(n, source))
        .unwrap_or_default();
    Some((
        kind,
        match node.child_by_field_name("trait").map(|n| text(n, source)) {
            Some(implemented) => format!("{implemented} for {target}"),
            None => target,
        },
    ))
}

fn python(node: Node, source: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "function_definition" => "fn",
        "class_definition" => "class",
        _ => return None,
    };
    Some((kind, named(node, source)))
}

fn typescript(node: Node, source: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "function_declaration" | "function_signature" => "fn",
        "method_definition" | "method_signature" => "method",
        "class_declaration" | "abstract_class_declaration" => "class",
        "interface_declaration" => "interface",
        "type_alias_declaration" => "type",
        "enum_declaration" => "enum",
        _ => return None,
    };
    Some((kind, named(node, source)))
}

fn go(node: Node, source: &str) -> Option<(&'static str, String)> {
    let kind = match node.kind() {
        "function_declaration" => "fn",
        "method_declaration" => "method",
        "type_spec" => "type",
        _ => return None,
    };
    Some((kind, named(node, source)))
}

fn collect(node: Node, source: &str, kinds: Kinds, out: &mut Vec<Item>) {
    if let Some((kind, name)) = kinds(node, source)
        && !name.is_empty()
    {
        let whole = text(node, source);
        let first = whole.lines().next().unwrap_or_default();
        let signature = first.split_once(" {").map_or(first, |(head, _)| head);
        out.push(Item {
            kind,
            name: name.split_whitespace().collect::<Vec<_>>().join(" "),
            start: node.start_position().row + 1,
            end: node.end_position().row + 1,
            signature: signature
                .trim()
                .trim_end_matches(['{', ':'])
                .trim()
                .to_owned(),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, source, kinds, out);
    }
}

/// Items that are known only by the line they start on: each runs to the line
/// before the next one, without trailing blank lines.
fn by_lines(
    source: &str,
    starts: Vec<(&'static str, String, usize)>,
    whole: Option<usize>,
) -> Vec<Item> {
    let lines: Vec<&str> = source.lines().collect();
    let last = (1..=lines.len())
        .rev()
        .find(|n| !lines[n - 1].trim().is_empty())
        .unwrap_or(1);
    let mut out = vec![];
    for (index, (kind, name, start)) in starts.iter().enumerate() {
        let next = starts[index + 1..]
            .iter()
            .map(|(_, _, line)| *line)
            .find(|line| line > start);
        let mut end = match (whole == Some(index), next) {
            (true, _) | (false, None) => last,
            (false, Some(next)) => next - 1,
        };
        // The record's closing brace belongs to the record, not its last member.
        if whole.is_some() && whole != Some(index) && next.is_none() {
            end = last.saturating_sub(1).max(*start);
        }
        while end > *start && lines.get(end - 1).is_some_and(|l| l.trim().is_empty()) {
            end -= 1;
        }
        out.push(Item {
            kind,
            name: name.clone(),
            start: *start,
            end,
            signature: lines.get(start - 1).map_or(String::new(), |l| {
                l.trim().trim_end_matches('{').trim().to_owned()
            }),
        });
    }
    out
}

/// A Den record's declarations, from the Den crate's own parser.
fn den(source: &str) -> Vec<Item> {
    let Ok(declarations) = bboxy450_den::declarations(source) else {
        return vec![];
    };
    let whole = declarations.iter().position(|d| d.kind == "spec");
    by_lines(
        source,
        declarations
            .into_iter()
            .map(|d| (d.kind, d.name, d.line as usize))
            .collect(),
        whole,
    )
}

/// A Lean file's top-level declarations, by keyword at the start of a line.
fn lean(source: &str) -> Vec<Item> {
    const KEYWORDS: [&str; 9] = [
        "theorem",
        "lemma",
        "def",
        "structure",
        "inductive",
        "instance",
        "abbrev",
        "axiom",
        "namespace",
    ];
    let starts = source
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let mut words = line
                .trim_start_matches("private ")
                .trim_start_matches("protected ")
                .split_whitespace();
            let first = words.next()?;
            let kind = KEYWORDS.into_iter().find(|k| *k == first)?;
            (!line.starts_with(char::is_whitespace)).then_some(())?;
            let name = words.next()?.trim_end_matches(':').to_owned();
            Some((kind, name, index + 1))
        })
        .collect();
    by_lines(source, starts, None)
}
