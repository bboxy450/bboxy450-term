//! `flowchart` / `graph` parser.

use crate::text::{clean, statements};
use crate::{Arrow, ClassDef, Error, Shape, Stroke};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    TB,
    BT,
    LR,
    RL,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FNode {
    pub id: String,
    pub lines: Vec<String>,
    pub shape: Shape,
    pub link: Option<String>,
    pub classes: Vec<String>,
    pub style: Option<String>,
    /// Index into `Flowchart::subgraphs`.
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Subgraph {
    pub id: String,
    pub title: Vec<String>,
    pub parent: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endpoint {
    Node(usize),
    Cluster(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FEdge {
    pub from: Endpoint,
    pub to: Endpoint,
    pub stroke: Stroke,
    pub start: Arrow,
    pub end: Arrow,
    pub label: Option<Vec<String>>,
    /// Ranks spanned at least (`--->` is 2).
    pub minlen: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Flowchart {
    pub dir: Dir,
    pub nodes: Vec<FNode>,
    pub subgraphs: Vec<Subgraph>,
    pub edges: Vec<FEdge>,
    pub class_defs: Vec<ClassDef>,
}

struct RawEdge {
    from: String,
    to: String,
    link: Link,
}

#[derive(Clone)]
struct Link {
    stroke: Stroke,
    start: Arrow,
    end: Arrow,
    label: Option<Vec<String>>,
    minlen: usize,
}

struct P {
    nodes: Vec<FNode>,
    index: HashMap<String, usize>,
    /// Ids that got an explicit shape or label (a real node even if a subgraph shares the id).
    defined: Vec<bool>,
    subgraphs: Vec<Subgraph>,
    stack: Vec<usize>,
    edges: Vec<RawEdge>,
    class_defs: Vec<ClassDef>,
}

pub(crate) fn parse(lines: &[(usize, String)]) -> Result<Flowchart, Error> {
    let st = statements(lines);
    let (hn, header) = &st[0];
    let mut words = header.split_whitespace();
    words.next();
    let dir = match words.next() {
        None => Dir::TB,
        Some(d) => {
            parse_dir(d).ok_or_else(|| Error::new(*hn, format!("unknown direction '{d}'")))?
        }
    };
    let mut p = P {
        nodes: Vec::new(),
        index: HashMap::new(),
        defined: Vec::new(),
        subgraphs: Vec::new(),
        stack: Vec::new(),
        edges: Vec::new(),
        class_defs: Vec::new(),
    };
    for (n, s) in &st[1..] {
        p.statement(*n, s)?;
    }
    if !p.stack.is_empty() {
        return Err(Error::new(0, "subgraph without 'end'"));
    }
    p.finish(dir)
}

fn parse_dir(d: &str) -> Option<Dir> {
    Some(match d {
        "TB" | "TD" => Dir::TB,
        "BT" => Dir::BT,
        "LR" => Dir::LR,
        "RL" => Dir::RL,
        _ => return None,
    })
}

impl P {
    fn statement(&mut self, n: usize, s: &str) -> Result<(), Error> {
        let (kw, rest) = match s.split_once(char::is_whitespace) {
            Some((k, r)) => (k, r.trim()),
            None => (s, ""),
        };
        match kw {
            "subgraph" => {
                let (id, title) =
                    subgraph_header(rest).ok_or_else(|| Error::new(n, "subgraph needs a name"))?;
                let idx = self.subgraphs.len();
                self.subgraphs.push(Subgraph {
                    id,
                    title,
                    parent: self.stack.last().copied(),
                });
                self.stack.push(idx);
            }
            "end" if rest.is_empty() => {
                self.stack
                    .pop()
                    .ok_or_else(|| Error::new(n, "'end' without subgraph"))?;
            }
            "direction" => {
                parse_dir(rest)
                    .ok_or_else(|| Error::new(n, format!("unknown direction '{rest}'")))?;
            }
            "classDef" => {
                let (names, style) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
                for name in names.split(',') {
                    self.class_defs.push(ClassDef {
                        name: name.trim().into(),
                        style: style.trim().into(),
                    });
                }
            }
            "class" => {
                let (ids, class) = rest
                    .rsplit_once(char::is_whitespace)
                    .ok_or_else(|| Error::new(n, "class needs ids and a class name"))?;
                for id in ids.split(',') {
                    let i = self.node(id.trim());
                    self.nodes[i].classes.push(class.trim().into());
                }
            }
            "style" => {
                let (id, style) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
                let i = self.node(id);
                self.nodes[i].style = Some(style.trim().into());
            }
            "linkStyle" | "click" | "accTitle" | "accDescr" | "title" => {}
            _ => self.chain(n, s)?,
        }
        Ok(())
    }

    fn node(&mut self, id: &str) -> usize {
        if let Some(&i) = self.index.get(id) {
            if self.nodes[i].parent.is_none() {
                self.nodes[i].parent = self.stack.last().copied();
            }
            return i;
        }
        let i = self.nodes.len();
        self.nodes.push(FNode {
            id: id.into(),
            lines: vec![id.into()],
            shape: Shape::Rect,
            link: None,
            classes: Vec::new(),
            style: None,
            parent: self.stack.last().copied(),
        });
        self.defined.push(false);
        self.index.insert(id.into(), i);
        i
    }

    fn chain(&mut self, n: usize, s: &str) -> Result<(), Error> {
        let mut c = Cur {
            s: s.chars().collect(),
            i: 0,
            line: n,
        };
        let mut prev = self.group(&mut c)?;
        loop {
            c.ws();
            if c.eof() {
                return Ok(());
            }
            let link = c
                .link()?
                .ok_or_else(|| c.err("expected a link such as -->"))?;
            let next = self.group(&mut c)?;
            for a in &prev {
                for b in &next {
                    self.edges.push(RawEdge {
                        from: a.clone(),
                        to: b.clone(),
                        link: link.clone(),
                    });
                }
            }
            prev = next;
        }
    }

    fn group(&mut self, c: &mut Cur) -> Result<Vec<String>, Error> {
        let mut ids = vec![self.one(c)?];
        loop {
            let save = c.i;
            c.ws();
            if c.peek() == Some('&') {
                c.i += 1;
                ids.push(self.one(c)?);
            } else {
                c.i = save;
                return Ok(ids);
            }
        }
    }

    fn one(&mut self, c: &mut Cur) -> Result<String, Error> {
        c.ws();
        let id = c.ident();
        if id.is_empty() {
            return Err(c.err("expected a node id"));
        }
        if c.starts("@{") {
            return Err(c.err("'@{ }' node metadata is not supported"));
        }
        let shape = c.shape()?;
        let i = self.node(&id);
        if let Some((shape, raw)) = shape {
            let lines = clean(&raw);
            let node = &mut self.nodes[i];
            node.link = (shape == Shape::Subroutine).then(|| lines.join(" "));
            node.shape = shape;
            node.lines = lines;
            self.defined[i] = true;
        }
        if c.starts(":::") {
            c.i += 3;
            let class = c.ident();
            self.nodes[i].classes.push(class);
        }
        Ok(id)
    }

    fn finish(self, dir: Dir) -> Result<Flowchart, Error> {
        let sg: HashMap<&str, usize> = self
            .subgraphs
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect();
        // A bare id naming a subgraph is that subgraph, not a node.
        let keep: Vec<bool> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| self.defined[i] || !sg.contains_key(n.id.as_str()))
            .collect();
        let mut remap = vec![usize::MAX; self.nodes.len()];
        let mut nodes = Vec::new();
        for (i, n) in self.nodes.into_iter().enumerate() {
            if keep[i] {
                remap[i] = nodes.len();
                nodes.push(n);
            }
        }
        let ep = |id: &str| -> Endpoint {
            let i = self.index[id];
            if keep[i] {
                Endpoint::Node(remap[i])
            } else {
                Endpoint::Cluster(sg[id])
            }
        };
        let edges = self
            .edges
            .iter()
            .map(|e| FEdge {
                from: ep(&e.from),
                to: ep(&e.to),
                stroke: e.link.stroke,
                start: e.link.start,
                end: e.link.end,
                label: e.link.label.clone(),
                minlen: e.link.minlen,
            })
            .collect();
        Ok(Flowchart {
            dir,
            nodes,
            subgraphs: self.subgraphs,
            edges,
            class_defs: self.class_defs,
        })
    }
}

/// `subgraph id [title]`, `subgraph id["title"]`, `subgraph "title"`, `subgraph some title`.
fn subgraph_header(rest: &str) -> Option<(String, Vec<String>)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    if rest.starts_with('"') {
        let lines = clean(rest);
        return Some((lines.join(" "), lines));
    }
    let id_end = rest
        .find(|c: char| c == '[' || c.is_whitespace())
        .unwrap_or(rest.len());
    let id = &rest[..id_end];
    let tail = rest[id_end..].trim();
    if tail.starts_with('[') && tail.ends_with(']') {
        return Some((id.into(), clean(&tail[1..tail.len() - 1])));
    }
    if tail.is_empty() {
        return Some((id.into(), vec![id.into()]));
    }
    Some((rest.into(), vec![rest.into()]))
}

struct Cur {
    s: Vec<char>,
    i: usize,
    line: usize,
}

const SHAPES: &[(&str, &str, Shape)] = &[
    ("(((", ")))", Shape::DoubleCircle),
    ("((", "))", Shape::Circle),
    ("([", "])", Shape::Stadium),
    ("[[", "]]", Shape::Subroutine),
    ("[(", ")]", Shape::Cylinder),
    ("{{", "}}", Shape::Hexagon),
    ("[/", "/]", Shape::Parallelogram),
    ("[/", "\\]", Shape::Trapezoid),
    ("[\\", "\\]", Shape::ParallelogramAlt),
    ("[\\", "/]", Shape::TrapezoidAlt),
    ("[", "]", Shape::Rect),
    ("(", ")", Shape::Rounded),
    ("{", "}", Shape::Diamond),
    (">", "]", Shape::Asymmetric),
];

impl Cur {
    fn eof(&self) -> bool {
        self.i >= self.s.len()
    }
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }
    fn at(&self, k: usize) -> Option<char> {
        self.s.get(self.i + k).copied()
    }
    fn starts(&self, p: &str) -> bool {
        p.chars().enumerate().all(|(k, ch)| self.at(k) == Some(ch))
    }
    fn ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
    }
    fn err(&self, m: &str) -> Error {
        let near: String = self.s[self.i.min(self.s.len())..].iter().take(16).collect();
        Error::new(self.line, format!("{m} near '{near}'"))
    }

    /// Node id: letters, digits, `_`, and `-` or `.` between word characters.
    fn ident(&mut self) -> String {
        let word = |c: char| c.is_alphanumeric() || c == '_';
        let start = self.i;
        while let Some(c) = self.peek() {
            if word(c) || ((c == '-' || c == '.') && self.i > start && self.at(1).is_some_and(word))
            {
                self.i += 1;
            } else {
                break;
            }
        }
        self.s[start..self.i].iter().collect()
    }

    fn shape(&mut self) -> Result<Option<(Shape, String)>, Error> {
        // `>` right after an id is a shape only when no link starts here.
        for &(open, _, _) in SHAPES {
            if !self.starts(open) {
                continue;
            }
            let body = self.i + open.chars().count();
            // The text runs to the first closer of any shape with this opener.
            let closers: Vec<(&str, Shape)> = SHAPES
                .iter()
                .filter(|(o, _, _)| *o == open)
                .map(|&(_, c, s)| (c, s))
                .collect();
            let mut j = body;
            let mut quoted = false;
            while j < self.s.len() {
                if self.s[j] == '"' {
                    quoted = !quoted;
                } else if !quoted {
                    for &(close, shape) in &closers {
                        let cl: Vec<char> = close.chars().collect();
                        if self.s[j..].starts_with(&cl) {
                            let raw: String = self.s[body..j].iter().collect();
                            self.i = j + cl.len();
                            return Ok(Some((shape, raw)));
                        }
                    }
                }
                j += 1;
            }
            return Err(self.err(&format!("unclosed '{open}'")));
        }
        Ok(None)
    }

    /// A link: `-->`, `---`, `-.->`, `==>`, `~~~`, `<-->`, `o--o`, `--x`,
    /// `-- text -->`, `-. text .->`, `== text ==>`, then an optional `|text|`.
    fn link(&mut self) -> Result<Option<Link>, Error> {
        let save = self.i;
        let Some(mut l) = self.link_token(true) else {
            self.i = save;
            return Ok(None);
        };
        if l.label.is_none() && self.open_ended(&l) {
            // `-- text -->`: the token so far opens a labelled link.
            let close = match l.stroke {
                Stroke::Thick => "==",
                Stroke::Dotted => ".-",
                _ => "--",
            };
            let start = self.i;
            let mut j = start;
            let mut quoted = false;
            let found = loop {
                if j >= self.s.len() {
                    break None;
                }
                if self.s[j] == '"' {
                    quoted = !quoted;
                } else if !quoted && self.s[j..].starts_with(&close.chars().collect::<Vec<_>>()) {
                    break Some(j);
                }
                j += 1;
            };
            let j = found.ok_or_else(|| self.err("unclosed link label"))?;
            let text: String = self.s[start..j].iter().collect();
            self.i = j;
            let closing = self
                .link_token(false)
                .ok_or_else(|| self.err("bad link close"))?;
            l = Link {
                start: l.start,
                label: Some(clean(&text)),
                ..closing
            };
        }
        self.ws();
        if self.peek() == Some('|') {
            self.i += 1;
            let start = self.i;
            while self.peek().is_some_and(|c| c != '|') {
                self.i += 1;
            }
            if self.eof() {
                return Err(self.err("unclosed |label|"));
            }
            l.label = Some(clean(&self.s[start..self.i].iter().collect::<String>()));
            self.i += 1;
        }
        Ok(Some(l))
    }

    /// `--`, `-.` and `==` with no head open a labelled link.
    fn open_ended(&self, l: &Link) -> bool {
        l.end == Arrow::None && l.minlen == 0
    }

    fn link_token(&mut self, opening: bool) -> Option<Link> {
        self.ws();
        let mut start = Arrow::None;
        match (self.peek(), self.at(1)) {
            (Some('<'), Some('-' | '=')) => {
                start = Arrow::Filled;
                self.i += 1;
            }
            (Some('o'), Some('-' | '=')) if opening => {
                start = Arrow::Circle;
                self.i += 1;
            }
            (Some('x'), Some('-' | '=')) if opening => {
                start = Arrow::Cross;
                self.i += 1;
            }
            _ => {}
        }
        let b = self.i;
        while self
            .peek()
            .is_some_and(|c| matches!(c, '-' | '=' | '.' | '~'))
        {
            self.i += 1;
        }
        let body: String = self.s[b..self.i].iter().collect();
        if body.len() < 2 {
            return None;
        }
        let stroke = if body.contains('~') {
            Stroke::Invisible
        } else if body.contains('=') {
            Stroke::Thick
        } else if body.contains('.') {
            Stroke::Dotted
        } else {
            Stroke::Solid
        };
        let edge_end = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
        let end = match self.peek() {
            Some('>') => {
                self.i += 1;
                Arrow::Filled
            }
            Some('o') if edge_end(self.at(1)) => {
                self.i += 1;
                Arrow::Circle
            }
            Some('x') if edge_end(self.at(1)) => {
                self.i += 1;
                Arrow::Cross
            }
            _ => Arrow::None,
        };
        let main = body.chars().filter(|&c| c != '.').count();
        let dots = body.chars().filter(|&c| c == '.').count();
        let minlen = match stroke {
            Stroke::Dotted => dots.max(1),
            _ if end != Arrow::None => main.saturating_sub(1).max(1),
            _ if main >= 3 => main - 2,
            _ => 0,
        };
        // `--`, `==` and `-.` with nothing after are openers (minlen 0).
        let minlen =
            if opening && end == Arrow::None && (body == "--" || body == "==" || body == "-.") {
                0
            } else {
                minlen.max(1)
            };
        Some(Link {
            stroke,
            start,
            end,
            label: None,
            minlen,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::strip_frontmatter;

    fn fc(src: &str) -> Flowchart {
        parse(&strip_frontmatter(src)).unwrap()
    }

    #[test]
    fn links() {
        let f = fc(
            "flowchart LR\nA-->B\nA --- B\nA-- This is the text! ---B\nA---|t|B\nA-. text .-> B\nA == text ==> B\nA ~~~ B\nA --o B\nA o--o B\nB <--> C\nB ---->|No| E\nA <-- \"x\" --> B",
        );
        let e = &f.edges;
        assert_eq!((e[0].stroke, e[0].end), (Stroke::Solid, Arrow::Filled));
        assert_eq!((e[1].end, e[1].label.clone()), (Arrow::None, None));
        assert_eq!(e[2].label, Some(vec!["This is the text!".to_string()]));
        assert_eq!(e[2].end, Arrow::None);
        assert_eq!(e[3].label, Some(vec!["t".to_string()]));
        assert_eq!((e[4].stroke, e[4].end), (Stroke::Dotted, Arrow::Filled));
        assert_eq!(e[4].label, Some(vec!["text".to_string()]));
        assert_eq!((e[5].stroke, e[5].end), (Stroke::Thick, Arrow::Filled));
        assert_eq!(e[6].stroke, Stroke::Invisible);
        assert_eq!(e[7].end, Arrow::Circle);
        assert_eq!((e[8].start, e[8].end), (Arrow::Circle, Arrow::Circle));
        assert_eq!((e[9].start, e[9].end), (Arrow::Filled, Arrow::Filled));
        assert_eq!(e[10].minlen, 3);
        assert_eq!((e[11].start, e[11].end), (Arrow::Filled, Arrow::Filled));
        assert_eq!(e[11].label, Some(vec!["x".to_string()]));
        assert_eq!(f.nodes.len(), 4);
    }

    #[test]
    fn shapes_groups_classes_and_subgraph_endpoints() {
        let f = fc(
            "flowchart TB\nsubgraph S[\"title\"]\n  a([op]) --> b[(db)] & c[[vision]]:::next\nend\nX --> S\nclass a,X hot\nclassDef next stroke-dasharray: 5 5",
        );
        let by = |id: &str| f.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(by("a").shape, Shape::Stadium);
        assert_eq!(by("b").shape, Shape::Cylinder);
        assert_eq!(by("c").link.as_deref(), Some("vision"));
        assert_eq!(by("c").classes, vec!["next"]);
        assert_eq!(by("a").parent, Some(0));
        assert_eq!(by("X").classes, vec!["hot"]);
        assert_eq!(f.subgraphs[0].title, vec!["title"]);
        assert_eq!(f.edges.len(), 3);
        assert_eq!(f.edges[2].to, Endpoint::Cluster(0));
        assert_eq!(f.class_defs[0].style, "stroke-dasharray: 5 5");
    }

    #[test]
    fn rejects_unsupported() {
        assert!(parse(&strip_frontmatter("flowchart TD\nA@{ shape: rect }")).is_err());
        assert!(parse(&strip_frontmatter("flowchart XX\nA")).is_err());
        assert!(parse(&strip_frontmatter("flowchart TD\nA[open")).is_err());
        assert!(parse(&strip_frontmatter("flowchart TD\nsubgraph S\nA")).is_err());
    }
}
