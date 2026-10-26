//! `stateDiagram` / `stateDiagram-v2` parser. States become flowchart nodes and
//! composite states become subgraphs, so the layered engine lays them out.

use crate::flowchart::{Dir, Endpoint, FEdge, FNode, Flowchart, Subgraph, parse_dir};
use crate::text::{clean, statements};
use crate::{Arrow, ClassDef, Error, Shape, Stroke};
use std::collections::HashMap;

struct P {
    nodes: Vec<FNode>,
    index: HashMap<String, usize>,
    subgraphs: Vec<Subgraph>,
    composite: HashMap<String, usize>,
    stack: Vec<usize>,
    edges: Vec<FEdge>,
    class_defs: Vec<ClassDef>,
    /// `id : text` lines, applied once every state and composite exists.
    descs: Vec<(String, Vec<String>)>,
    dir: Dir,
    notes: usize,
}

pub(crate) fn parse(lines: &[(usize, String)]) -> Result<Flowchart, Error> {
    let st = statements(lines, true);
    // Composite states first, so transitions naming one attach to its box.
    let mut composite = HashMap::new();
    for (_, s) in &st[1..] {
        if let Some(rest) = s.strip_prefix("state ")
            && let Some(head) = rest.trim().strip_suffix('{')
        {
            let (id, _) = state_head(head.trim());
            let n = composite.len();
            composite.entry(id).or_insert(n);
        }
    }
    let mut p = P {
        nodes: Vec::new(),
        index: HashMap::new(),
        subgraphs: Vec::new(),
        composite,
        stack: Vec::new(),
        edges: Vec::new(),
        class_defs: Vec::new(),
        descs: Vec::new(),
        dir: Dir::TB,
        notes: 0,
    };
    let mut i = 1;
    while i < st.len() {
        let (n, s) = (st[i].0, st[i].1.as_str());
        i += 1;
        if let Some(head) = s.strip_prefix("note ").or_else(|| s.strip_prefix("Note ")) {
            let (place, text) = match head.split_once(':') {
                Some((pl, t)) => (pl.trim(), clean(t)),
                None => {
                    // Block note: lines up to `end note`.
                    let mut body = Vec::new();
                    loop {
                        let Some((_, l)) = st.get(i) else {
                            return Err(Error::new(n, "note without 'end note'"));
                        };
                        i += 1;
                        if l.trim() == "end note" {
                            break;
                        }
                        body.extend(clean(l));
                    }
                    (head.trim(), body)
                }
            };
            p.note(n, place, text)?;
            continue;
        }
        p.statement(n, s)?;
    }
    if !p.stack.is_empty() {
        return Err(Error::new(0, "composite state without '}'"));
    }
    for (id, d) in std::mem::take(&mut p.descs) {
        if let Some(&c) = p.composite.get(&id) {
            p.subgraphs[c].title.extend(d);
        } else {
            let k = p.state(&id);
            p.nodes[k].lines.extend(d);
        }
    }
    Ok(Flowchart {
        dir: p.dir,
        nodes: p.nodes,
        subgraphs: p.subgraphs,
        edges: p.edges,
        class_defs: p.class_defs,
    })
}

/// `"Long name" as id` or `id`, with an optional `<<kind>>`; returns (id, display name).
fn state_head(h: &str) -> (String, Option<Vec<String>>) {
    let h = h.split("<<").next().unwrap_or(h).trim();
    if h.starts_with('"')
        && let Some((name, id)) = h.rsplit_once(" as ")
    {
        return (id.trim().to_string(), Some(clean(name)));
    }
    (h.to_string(), None)
}

impl P {
    fn statement(&mut self, n: usize, s: &str) -> Result<(), Error> {
        let (kw, rest) = match s.split_once(char::is_whitespace) {
            Some((k, r)) => (k, r.trim()),
            None => (s, ""),
        };
        match kw {
            "}" => {
                self.stack
                    .pop()
                    .ok_or_else(|| Error::new(n, "'}' without a composite state"))?;
            }
            "--" => return Err(Error::new(n, "concurrent regions ('--') are not supported")),
            "direction" => {
                let d = parse_dir(rest)
                    .ok_or_else(|| Error::new(n, format!("unknown direction '{rest}'")))?;
                match self.stack.last() {
                    Some(&g) => self.subgraphs[g].dir = Some(d),
                    None => self.dir = d,
                }
            }
            "classDef" => {
                let (names, style) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
                for name in names.split(',') {
                    let style = style.trim().trim_end_matches(';');
                    self.class_defs.push(ClassDef {
                        name: name.trim().into(),
                        style: style.into(),
                    });
                }
            }
            "class" => {
                let (ids, class) = rest
                    .rsplit_once(char::is_whitespace)
                    .ok_or_else(|| Error::new(n, "class needs ids and a name"))?;
                for id in ids.split(',').map(str::trim) {
                    // Classes on ids that are not states are ignored, as Mermaid does.
                    if let Some(&k) = self.index.get(id) {
                        self.nodes[k].classes.push(class.trim().into());
                    }
                }
            }
            "style" => {
                let (id, style) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
                let k = self.state(id);
                self.nodes[k].style = Some(style.trim().into());
            }
            "hide" | "scale" | "accTitle" | "accTitle:" | "accDescr" | "accDescr:" | "title" => {}
            "state" => self.state_decl(n, rest)?,
            _ => self.transition_or_desc(n, s)?,
        }
        Ok(())
    }

    fn state_decl(&mut self, n: usize, rest: &str) -> Result<(), Error> {
        if let Some(head) = rest.strip_suffix('{') {
            let (id, name) = state_head(head.trim());
            if id.is_empty() {
                return Err(Error::new(n, "composite state needs a name"));
            }
            let idx = self.subgraphs.len();
            if self.composite.get(&id) != Some(&idx) {
                return Err(Error::new(
                    n,
                    format!("composite state '{id}' declared twice"),
                ));
            }
            self.subgraphs.push(Subgraph {
                id: id.clone(),
                title: name.unwrap_or_else(|| vec![id]),
                parent: self.stack.last().copied(),
                dir: None,
            });
            self.stack.push(idx);
            return Ok(());
        }
        if rest.starts_with('"') && !rest.contains(" as ") {
            return Err(Error::new(n, "floating state text is not supported"));
        }
        let (id, name) = state_head(rest);
        if id.is_empty() || id.contains(char::is_whitespace) {
            return Err(Error::new(n, format!("bad state declaration '{rest}'")));
        }
        let k = self.state(&id);
        if let Some(name) = name {
            self.nodes[k].lines = name;
        }
        if let Some(kind) = rest.split("<<").nth(1).and_then(|t| t.split(">>").next()) {
            self.nodes[k].shape = match kind.trim() {
                "choice" => Shape::Choice,
                "fork" | "join" => Shape::Bar,
                other => {
                    return Err(Error::new(
                        n,
                        format!("unsupported state kind '<<{other}>>'"),
                    ));
                }
            };
            self.nodes[k].lines = Vec::new();
        }
        Ok(())
    }

    fn transition_or_desc(&mut self, n: usize, s: &str) -> Result<(), Error> {
        if let Some((a, rest)) = s.split_once("-->") {
            let (b, label) = match label_colon(rest) {
                Some(i) => (rest[..i].trim(), Some(clean(&rest[i + 1..]))),
                None => (rest.trim(), None),
            };
            let from = self.endpoint(n, a.trim(), true)?;
            let to = self.endpoint(n, b, false)?;
            self.edges.push(FEdge {
                from,
                to,
                stroke: Stroke::Solid,
                start: Arrow::None,
                end: Arrow::Filled,
                label,
                minlen: 1,
            });
            return Ok(());
        }
        let (id, desc) = match label_colon(s) {
            Some(i) => (s[..i].trim(), Some(clean(&s[i + 1..]))),
            None => (s.trim(), None),
        };
        let (id, class) = split_class(id);
        if id.is_empty() || id.contains(|c: char| c.is_whitespace() || c == '"') {
            return Err(Error::new(n, format!("unsupported statement '{s}'")));
        }
        if !self.composite.contains_key(id) {
            let k = self.state(id);
            if let Some(c) = class {
                self.nodes[k].classes.push(c.into());
            }
        }
        if let Some(d) = desc {
            self.descs.push((id.into(), d));
        }
        Ok(())
    }

    fn endpoint(&mut self, n: usize, raw: &str, source: bool) -> Result<Endpoint, Error> {
        let (id, class) = split_class(raw);
        if id.is_empty() || id.contains(char::is_whitespace) {
            return Err(Error::new(n, format!("bad state '{raw}'")));
        }
        if id == "[*]" {
            let scope = self
                .stack
                .last()
                .map_or("root".to_string(), |&g| self.subgraphs[g].id.clone());
            let key = format!("[*]{}/{scope}", if source { "start" } else { "end" });
            let k = self.state(&key);
            self.nodes[k].shape = if source { Shape::Start } else { Shape::End };
            self.nodes[k].lines = Vec::new();
            return Ok(Endpoint::Node(k));
        }
        if let Some(&c) = self.composite.get(id) {
            return Ok(Endpoint::Cluster(c));
        }
        let k = self.state(id);
        if let Some(c) = class {
            self.nodes[k].classes.push(c.into());
        }
        Ok(Endpoint::Node(k))
    }

    fn note(&mut self, n: usize, place: &str, text: Vec<String>) -> Result<(), Error> {
        let (right, id) = if let Some(id) = place.strip_prefix("right of ") {
            (true, id.trim())
        } else if let Some(id) = place.strip_prefix("left of ") {
            (false, id.trim())
        } else {
            return Err(Error::new(
                n,
                format!("unsupported note placement '{place}'"),
            ));
        };
        let target = self.endpoint(n, id, true)?;
        self.notes += 1;
        let k = self.state(&format!("note-{}", self.notes));
        self.nodes[k].shape = Shape::Note;
        self.nodes[k].lines = text;
        // A same-rank edge (minlen 0) keeps the note beside its state.
        let (from, to) = if right {
            (target, Endpoint::Node(k))
        } else {
            (Endpoint::Node(k), target)
        };
        self.edges.push(FEdge {
            from,
            to,
            stroke: Stroke::Dotted,
            start: Arrow::None,
            end: Arrow::None,
            label: None,
            minlen: 0,
        });
        Ok(())
    }

    fn state(&mut self, id: &str) -> usize {
        if let Some(&k) = self.index.get(id) {
            if self.nodes[k].parent.is_none() {
                self.nodes[k].parent = self.stack.last().copied();
            }
            return k;
        }
        self.index.insert(id.into(), self.nodes.len());
        self.nodes.push(FNode {
            id: id.into(),
            lines: vec![id.into()],
            shape: Shape::Rounded,
            link: None,
            classes: Vec::new(),
            style: None,
            parent: self.stack.last().copied(),
        });
        self.nodes.len() - 1
    }
}

/// The `:` that starts a label or description, skipping `:::class`.
fn label_colon(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b':' {
            if b[i..].starts_with(b":::") {
                i += 3;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn split_class(s: &str) -> (&str, Option<&str>) {
    match s.split_once(":::") {
        Some((id, c)) => (id.trim(), Some(c.trim())),
        None => (s.trim(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::strip_frontmatter;

    fn sd(src: &str) -> Flowchart {
        parse(&strip_frontmatter(src)).unwrap()
    }

    // keeps: spec document-viewer §Mermaid — supported syntax, deterministic true-size geometry and honest source fallback
    #[test]
    fn states_transitions_and_composites() {
        let f = sd(
            "stateDiagram-v2\n[*] --> Draft\nDraft --> Review : submit\nstate Review {\n [*] --> Screening\n}\nReview --> [*]\nstate \"Long name\" as L\nL : more\nstate c <<choice>>\nstate j <<join>>",
        );
        let by = |id: &str| f.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(by("[*]start/root").shape, Shape::Start);
        assert_eq!(by("[*]end/root").shape, Shape::End);
        assert_eq!(by("[*]start/Review").parent, Some(0));
        assert_eq!(by("Screening").parent, Some(0));
        assert_eq!(f.edges[1].to, Endpoint::Cluster(0));
        assert_eq!(f.edges[1].label, Some(vec!["submit".to_string()]));
        assert_eq!(by("L").lines, vec!["Long name", "more"]);
        assert_eq!(by("c").shape, Shape::Choice);
        assert_eq!(by("j").shape, Shape::Bar);
    }

    // keeps: spec document-viewer §Mermaid — supported syntax, deterministic true-size geometry and honest source fallback
    #[test]
    fn notes_sit_beside_their_state() {
        let c = sd("stateDiagram\n[*] --> Still:::calm\nStill --> Moving:::fast : go");
        assert_eq!(c.nodes[1].classes, vec!["calm"]);
        assert_eq!(c.edges[1].label, Some(vec!["go".to_string()]));
        assert_eq!(c.nodes[2].classes, vec!["fast"]);
        let f = sd(
            "stateDiagram-v2\nA --> B\nnote right of A\n  line one\n  line two\nend note\nnote left of B : hi",
        );
        let note = f.nodes.iter().find(|n| n.id == "note-1").unwrap();
        assert_eq!(note.lines, vec!["line one", "line two"]);
        assert_eq!(f.edges[1].minlen, 0);
        assert_eq!(f.edges[2].to, Endpoint::Node(1));
    }

    // keeps: spec document-viewer §Mermaid — supported syntax, deterministic true-size geometry and honest source fallback
    #[test]
    fn rejects_unsupported() {
        let bad = |s: &str| parse(&strip_frontmatter(s)).is_err();
        assert!(bad(
            "stateDiagram-v2\nstate A {\n a --> b\n --\n c --> d\n}"
        ));
        assert!(bad("stateDiagram-v2\nstate A {\n a --> b"));
        assert!(bad("stateDiagram-v2\nstate x <<history>>"));
    }
}
