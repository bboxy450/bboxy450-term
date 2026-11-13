// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The items of a Rust source file, from its syntax tree.
use tree_sitter::{Node, Parser};

/// One definition: a function, type, impl, module, constant, macro or enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// fn, struct, enum, variant, union, trait, impl, mod, const, static, type, macro
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

/// Every item in `source`, in source order. A file that does not parse yields
/// the items of the parts that do.
pub fn items(source: &str) -> Vec<Item> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return vec![];
    }
    let Some(tree) = parser.parse(source, None) else {
        return vec![];
    };
    let mut out = vec![];
    collect(tree.root_node(), source, &mut out);
    out
}

fn collect(node: Node, source: &str, out: &mut Vec<Item>) {
    let kind = match node.kind() {
        "function_item" | "function_signature_item" => Some("fn"),
        "struct_item" => Some("struct"),
        "enum_item" => Some("enum"),
        "enum_variant" => Some("variant"),
        "union_item" => Some("union"),
        "trait_item" => Some("trait"),
        "impl_item" => Some("impl"),
        "mod_item" => Some("mod"),
        "const_item" => Some("const"),
        "static_item" => Some("static"),
        "type_item" => Some("type"),
        "macro_definition" => Some("macro"),
        _ => None,
    };
    if let Some(kind) = kind {
        let text = |n: Node| source.get(n.byte_range()).unwrap_or_default().to_owned();
        let name = if kind == "impl" {
            let target = node
                .child_by_field_name("type")
                .map(text)
                .unwrap_or_default();
            match node.child_by_field_name("trait").map(text) {
                Some(implemented) => format!("{implemented} for {target}"),
                None => target,
            }
        } else {
            node.child_by_field_name("name")
                .map(text)
                .unwrap_or_default()
        };
        if !name.is_empty() {
            let whole = text(node);
            let first = whole.lines().next().unwrap_or_default();
            let signature = first.split_once(" {").map_or(first, |(head, _)| head);
            out.push(Item {
                kind,
                name: name.split_whitespace().collect::<Vec<_>>().join(" "),
                start: node.start_position().row + 1,
                end: node.end_position().row + 1,
                signature: signature.trim().trim_end_matches('{').trim().to_owned(),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, source, out);
    }
}
