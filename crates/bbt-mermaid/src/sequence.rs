//! `sequenceDiagram`: parser and column/row layout.

use crate::text::{clean, label, measure_lines, statements};
use crate::{
    Arrow, Cluster, ClusterKind, Divider, Edge, Error, Geometry, Kind, Line, Measure, Node, Point,
    Rect, Shape, Size, Stroke, Style, TextRole,
};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: String,
    pub lines: Vec<String>,
    /// `Shape::Participant` or `Shape::Actor`.
    pub shape: Shape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotePlace {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FragmentKind {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
    Rect,
}

impl FragmentKind {
    fn word(self) -> &'static str {
        match self {
            FragmentKind::Loop => "loop",
            FragmentKind::Alt => "alt",
            FragmentKind::Opt => "opt",
            FragmentKind::Par => "par",
            FragmentKind::Critical => "critical",
            FragmentKind::Break => "break",
            FragmentKind::Rect => "rect",
        }
    }
    fn cluster(self) -> ClusterKind {
        match self {
            FragmentKind::Loop => ClusterKind::Loop,
            FragmentKind::Alt => ClusterKind::Alt,
            FragmentKind::Opt => ClusterKind::Opt,
            FragmentKind::Par => ClusterKind::Par,
            FragmentKind::Critical => ClusterKind::Critical,
            FragmentKind::Break => ClusterKind::Break,
            FragmentKind::Rect => ClusterKind::Rect,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Message {
        from: usize,
        to: usize,
        stroke: Stroke,
        start: Arrow,
        end: Arrow,
        text: Vec<String>,
        /// `+` before the target: activate it.
        activate: bool,
        /// `-` before the target: deactivate it.
        deactivate: bool,
    },
    Activate(usize),
    Deactivate(usize),
    Note {
        place: NotePlace,
        a: usize,
        b: usize,
        text: Vec<String>,
    },
    Start {
        kind: FragmentKind,
        text: Vec<String>,
    },
    /// `else`, `and`, `option`.
    Divider {
        text: Vec<String>,
    },
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sequence {
    pub actors: Vec<Actor>,
    pub events: Vec<Event>,
    pub autonumber: bool,
}

const ARROWS: &[(&str, Stroke, Arrow, Arrow)] = &[
    ("<<-->>", Stroke::Dotted, Arrow::Filled, Arrow::Filled),
    ("<<->>", Stroke::Solid, Arrow::Filled, Arrow::Filled),
    ("-->>", Stroke::Dotted, Arrow::None, Arrow::Filled),
    ("->>", Stroke::Solid, Arrow::None, Arrow::Filled),
    ("--x", Stroke::Dotted, Arrow::None, Arrow::Cross),
    ("-x", Stroke::Solid, Arrow::None, Arrow::Cross),
    ("--)", Stroke::Dotted, Arrow::None, Arrow::Open),
    ("-)", Stroke::Solid, Arrow::None, Arrow::Open),
    ("-->", Stroke::Dotted, Arrow::None, Arrow::None),
    ("->", Stroke::Solid, Arrow::None, Arrow::None),
];

struct P {
    actors: Vec<Actor>,
    index: HashMap<String, usize>,
}

impl P {
    fn actor(&mut self, id: &str) -> usize {
        if let Some(&i) = self.index.get(id) {
            return i;
        }
        self.index.insert(id.into(), self.actors.len());
        self.actors.push(Actor {
            id: id.into(),
            lines: vec![id.into()],
            shape: Shape::Participant,
        });
        self.actors.len() - 1
    }
}

pub(crate) fn parse(lines: &[(usize, String)]) -> Result<Sequence, Error> {
    let st = statements(lines);
    let mut p = P {
        actors: Vec::new(),
        index: HashMap::new(),
    };
    let mut events = Vec::new();
    let mut autonumber = false;
    let mut depth = 0usize;
    for (n, s) in &st[1..] {
        let n = *n;
        let (kw, rest) = match s.split_once(char::is_whitespace) {
            Some((k, r)) => (k, r.trim()),
            None => (s.as_str(), ""),
        };
        let frag = match kw {
            "loop" => Some(FragmentKind::Loop),
            "alt" => Some(FragmentKind::Alt),
            "opt" => Some(FragmentKind::Opt),
            "par" => Some(FragmentKind::Par),
            "critical" => Some(FragmentKind::Critical),
            "break" => Some(FragmentKind::Break),
            "rect" => Some(FragmentKind::Rect),
            _ => None,
        };
        if let Some(kind) = frag {
            let text = if kind == FragmentKind::Rect {
                Vec::new()
            } else {
                clean(rest)
            };
            events.push(Event::Start { kind, text });
            depth += 1;
            continue;
        }
        match kw {
            "participant" | "actor" => {
                let (id, alias) = match rest.split_once(" as ") {
                    Some((i, a)) => (i.trim(), Some(a.trim())),
                    None => (rest, None),
                };
                if id.is_empty() {
                    return Err(Error::new(n, "participant needs a name"));
                }
                let i = p.actor(id);
                if let Some(a) = alias {
                    p.actors[i].lines = clean(a);
                }
                if kw == "actor" {
                    p.actors[i].shape = Shape::Actor;
                }
            }
            "autonumber" => autonumber = true,
            "title" | "accTitle" | "accDescr" | "link" | "links" => {}
            "activate" | "deactivate" => {
                let a = p.actor(rest);
                events.push(if kw == "activate" {
                    Event::Activate(a)
                } else {
                    Event::Deactivate(a)
                });
            }
            "else" | "and" | "option" => {
                if depth == 0 {
                    return Err(Error::new(n, format!("'{kw}' outside a block")));
                }
                events.push(Event::Divider { text: clean(rest) });
            }
            "end" if rest.is_empty() => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| Error::new(n, "'end' without a block"))?;
                events.push(Event::End);
            }
            "Note" | "note" => events.push(note(&mut p, n, rest)?),
            _ => events.push(message(&mut p, n, s)?),
        }
    }
    if depth > 0 {
        return Err(Error::new(0, "block without 'end'"));
    }
    Ok(Sequence {
        actors: p.actors,
        events,
        autonumber,
    })
}

fn note(p: &mut P, n: usize, rest: &str) -> Result<Event, Error> {
    let (head, text) = rest
        .split_once(':')
        .ok_or_else(|| Error::new(n, "note needs ': text'"))?;
    let head = head.trim();
    let (place, who) = if let Some(w) = head.strip_prefix("left of ") {
        (NotePlace::LeftOf, w)
    } else if let Some(w) = head.strip_prefix("right of ") {
        (NotePlace::RightOf, w)
    } else if let Some(w) = head.strip_prefix("over ") {
        (NotePlace::Over, w)
    } else {
        return Err(Error::new(
            n,
            format!("unsupported note placement '{head}'"),
        ));
    };
    let mut ids = who.split(',').map(str::trim);
    let a = p.actor(ids.next().unwrap_or(""));
    let b = match ids.next() {
        Some(id) => p.actor(id),
        None => a,
    };
    Ok(Event::Note {
        place,
        a: a.min(b),
        b: a.max(b),
        text: clean(text),
    })
}

fn message(p: &mut P, n: usize, s: &str) -> Result<Event, Error> {
    let (lhs, text) = s
        .split_once(':')
        .ok_or_else(|| Error::new(n, format!("unsupported statement '{s}'")))?;
    let mut found = None;
    'scan: for (i, _) in lhs.char_indices() {
        for &(tok, stroke, start, end) in ARROWS {
            if lhs[i..].starts_with(tok) {
                found = Some((i, tok.len(), stroke, start, end));
                break 'scan;
            }
        }
    }
    let (i, len, stroke, start, end) =
        found.ok_or_else(|| Error::new(n, format!("unsupported statement '{s}'")))?;
    let from = lhs[..i].trim();
    let mut to = lhs[i + len..].trim();
    let (mut activate, mut deactivate) = (false, false);
    if let Some(t) = to.strip_prefix('+') {
        activate = true;
        to = t.trim();
    } else if let Some(t) = to.strip_prefix('-') {
        deactivate = true;
        to = t.trim();
    }
    if from.is_empty()
        || to.is_empty()
        || from.contains(char::is_whitespace)
        || to.contains(char::is_whitespace)
    {
        return Err(Error::new(n, format!("unsupported message '{s}'")));
    }
    let from = p.actor(from);
    let to = p.actor(to);
    Ok(Event::Message {
        from,
        to,
        stroke,
        start,
        end,
        text: clean(text),
        activate,
        deactivate,
    })
}

struct Frame {
    kind: FragmentKind,
    text: Vec<String>,
    top: f32,
    x0: f32,
    x1: f32,
    dividers: Vec<(f32, Vec<String>)>,
    depth: usize,
}

pub(crate) fn layout(q: &Sequence, m: &dyn Measure, s: &Style) -> Result<Geometry, Error> {
    let n = q.actors.len();
    if n == 0 {
        return Err(Error::new(0, "sequence diagram has no participants"));
    }
    let act_w = 10.0;
    let act_step = 5.0;
    let self_w = 34.0;
    let self_h = 20.0;
    let note_pad = 8.0;
    // Boxes.
    let boxes: Vec<Size> = q
        .actors
        .iter()
        .map(|a| {
            let t = measure_lines(&a.lines, TextRole::Actor, m);
            match a.shape {
                Shape::Actor => Size {
                    w: (t.w + 2.0 * s.node_pad_x).max(48.0),
                    h: t.h + 40.0,
                },
                _ => Size {
                    w: (t.w + 2.0 * s.node_pad_x).max(72.0),
                    h: t.h + 2.0 * s.node_pad_y,
                },
            }
        })
        .collect();
    let box_h = boxes.iter().map(|b| b.h).fold(0.0, f32::max);
    let numbered = |k: usize, text: &[String]| -> Vec<String> {
        let mut t = text.to_vec();
        if q.autonumber
            && let Some(first) = t.first_mut()
        {
            *first = format!("{k}. {first}");
        }
        t
    };

    // Columns: start from box widths, then widen gaps for labels and notes.
    let mut c = vec![0f32; n];
    c[0] = boxes[0].w / 2.0;
    for i in 1..n {
        c[i] = c[i - 1] + (boxes[i - 1].w + boxes[i].w) / 2.0 + s.actor_gap;
    }
    let mut cons: Vec<(usize, usize, f32)> = Vec::new();
    let mut k = 0;
    for e in &q.events {
        match e {
            Event::Message { from, to, text, .. } => {
                k += 1;
                let w = measure_lines(&numbered(k, text), TextRole::Message, m).w;
                if from == to {
                    if from + 1 < n {
                        cons.push((
                            *from,
                            from + 1,
                            (w + 12.0).max(self_w) + 16.0 + boxes[from + 1].w / 4.0,
                        ));
                    }
                } else {
                    cons.push(((*from).min(*to), (*from).max(*to), w + 32.0));
                }
            }
            Event::Note { place, a, b, text } => {
                let w = measure_lines(text, TextRole::Note, m).w + 2.0 * note_pad;
                match place {
                    NotePlace::RightOf if a + 1 < n => cons.push((*a, a + 1, w + 24.0)),
                    NotePlace::LeftOf if *a > 0 => cons.push((a - 1, *a, w + 24.0)),
                    NotePlace::Over if a == b => {
                        if a + 1 < n {
                            cons.push((*a, a + 1, w / 2.0 + boxes[a + 1].w / 2.0 + 8.0));
                        }
                        if *a > 0 {
                            cons.push((a - 1, *a, w / 2.0 + boxes[a - 1].w / 2.0 + 8.0));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    cons.sort_by_key(|&(a, b, _)| b - a);
    for (a, b, d) in cons {
        let have = c[b] - c[a];
        if have < d {
            let delta = d - have;
            let span = (b - a) as f32;
            for (j, cj) in c.iter_mut().enumerate().skip(a + 1) {
                *cj += if j <= b {
                    delta * (j - a) as f32 / span
                } else {
                    delta
                };
            }
        }
    }

    // Rows.
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut active: Vec<Vec<f32>> = vec![Vec::new(); n];
    let mut acts: Vec<(usize, usize, f32, f32)> = Vec::new(); // actor, level, top, bottom
    let mut frames: Vec<Frame> = Vec::new();
    let mut y = box_h + s.row_gap * 2.0;
    let mut k = 0;
    let mut notes = 0;
    let touch = |frames: &mut Vec<Frame>, a: f32, b: f32| {
        if let Some(f) = frames.last_mut() {
            f.x0 = f.x0.min(a);
            f.x1 = f.x1.max(b);
        }
    };
    let close =
        |active: &mut Vec<Vec<f32>>, acts: &mut Vec<(usize, usize, f32, f32)>, a: usize, y: f32| {
            if let Some(top) = active[a].pop() {
                acts.push((a, active[a].len(), top, y));
            }
        };
    // Activation box x range for actor `a` at its current depth.
    let act_x = |active: &Vec<Vec<f32>>, a: usize| -> Option<(f32, f32)> {
        let d = active[a].len();
        (d > 0).then(|| {
            let cx = c[a] + (d - 1) as f32 * act_step;
            (cx - act_w / 2.0, cx + act_w / 2.0)
        })
    };
    for e in &q.events {
        match e {
            Event::Message {
                from,
                to,
                stroke,
                start,
                end,
                text,
                activate,
                deactivate,
            } => {
                k += 1;
                let lines = numbered(k, text);
                let ts = measure_lines(&lines, TextRole::Message, m);
                y += ts.h + 6.0;
                let ay = y;
                let (f, t) = (*from, *to);
                let pts = if f == t {
                    let x = act_x(&active, f).map_or(c[f], |r| r.1);
                    y += self_h;
                    vec![
                        Point { x, y: ay },
                        Point {
                            x: x + self_w,
                            y: ay,
                        },
                        Point {
                            x: x + self_w,
                            y: ay + self_h,
                        },
                        Point { x, y: ay + self_h },
                    ]
                } else {
                    let right = t > f;
                    let fx = act_x(&active, f).map_or(c[f], |r| if right { r.1 } else { r.0 });
                    let tx = act_x(&active, t).map_or(c[t], |r| if right { r.0 } else { r.1 });
                    vec![Point { x: fx, y: ay }, Point { x: tx, y: ay }]
                };
                let lrect = if f == t {
                    Rect {
                        x: pts[0].x + 6.0,
                        y: ay - ts.h - 3.0,
                        w: ts.w,
                        h: ts.h,
                    }
                } else {
                    let mid = (pts[0].x + pts[1].x) / 2.0;
                    Rect {
                        x: mid - ts.w / 2.0,
                        y: ay - ts.h - 3.0,
                        w: ts.w,
                        h: ts.h,
                    }
                };
                let x0 = pts.iter().map(|p| p.x).fold(lrect.x, f32::min);
                let x1 = pts.iter().map(|p| p.x).fold(lrect.right(), f32::max);
                touch(&mut frames, x0, x1);
                edges.push(Edge {
                    from: q.actors[f].id.clone(),
                    to: q.actors[t].id.clone(),
                    points: pts,
                    stroke: *stroke,
                    start: *start,
                    end: *end,
                    label: Some(label(lines, TextRole::Message, lrect)),
                });
                if *activate {
                    active[t].push(y);
                }
                if *deactivate {
                    close(&mut active, &mut acts, t, y);
                }
                y += s.row_gap + 4.0;
            }
            Event::Activate(a) => active[*a].push(y - s.row_gap),
            Event::Deactivate(a) => close(&mut active, &mut acts, *a, y - s.row_gap),
            Event::Note { place, a, b, text } => {
                let ts = measure_lines(text, TextRole::Note, m);
                let (w, h) = (ts.w + 2.0 * note_pad, ts.h + 2.0 * note_pad);
                let x = match place {
                    NotePlace::LeftOf => c[*a] - act_w - w,
                    NotePlace::RightOf => c[*a] + act_w,
                    NotePlace::Over => {
                        let span = c[*b] - c[*a] + 2.0 * 24.0;
                        let ww = w.max(span);
                        (c[*a] + c[*b]) / 2.0 - ww / 2.0
                    }
                };
                let w = if *place == NotePlace::Over {
                    w.max(c[*b] - c[*a] + 48.0)
                } else {
                    w
                };
                let rect = Rect { x, y, w, h };
                notes += 1;
                touch(&mut frames, rect.x, rect.right());
                nodes.push(Node {
                    id: format!("note-{notes}"),
                    shape: Shape::Note,
                    rect,
                    label: Some(label(
                        text.clone(),
                        TextRole::Note,
                        Rect::centered(rect.center(), ts),
                    )),
                    link: None,
                    classes: Vec::new(),
                    style: None,
                });
                y += h + s.row_gap;
            }
            Event::Start { kind, text } => {
                let ts = measure_lines(text, TextRole::FrameLabel, m);
                let depth = frames.len();
                frames.push(Frame {
                    kind: *kind,
                    text: text.clone(),
                    top: y,
                    x0: f32::MAX,
                    x1: f32::MIN,
                    dividers: Vec::new(),
                    depth,
                });
                y += ts.h.max(m.measure("loop", TextRole::FrameLabel).h) + 8.0 + s.row_gap / 2.0;
            }
            Event::Divider { text } => {
                let ts = measure_lines(text, TextRole::FrameLabel, m);
                y += 4.0;
                if let Some(f) = frames.last_mut() {
                    f.dividers.push((y, text.clone()));
                }
                y += ts.h + 8.0 + s.row_gap / 2.0;
            }
            Event::End => {
                let f = frames.pop().expect("parser balances blocks");
                let pad = 12.0;
                let word = m.measure(f.kind.word(), TextRole::FrameLabel);
                let ts = measure_lines(&f.text, TextRole::FrameLabel, m);
                let tab_w = word.w + 2.0 * 6.0;
                let (mut x0, mut x1) = if f.x0 <= f.x1 {
                    (f.x0 - pad, f.x1 + pad)
                } else {
                    (c[0] - 40.0, c[0] + 40.0)
                };
                let min_w = tab_w + ts.w + 24.0;
                if x1 - x0 < min_w {
                    let grow = (min_w - (x1 - x0)) / 2.0;
                    x0 -= grow;
                    x1 += grow;
                }
                let rect = Rect {
                    x: x0,
                    y: f.top,
                    w: x1 - x0,
                    h: y + 2.0 - f.top,
                };
                let title = (!f.text.is_empty()).then(|| {
                    label(
                        f.text.iter().map(|l| format!("[{l}]")).collect(),
                        TextRole::FrameLabel,
                        Rect {
                            x: x0 + tab_w + 8.0,
                            y: f.top + 4.0,
                            w: ts.w + 2.0 * m.measure("[]", TextRole::FrameLabel).w / 2.0,
                            h: ts.h,
                        },
                    )
                });
                let dividers = f
                    .dividers
                    .iter()
                    .map(|(dy, text)| {
                        let ts = measure_lines(text, TextRole::FrameLabel, m);
                        Divider {
                            y: *dy,
                            label: (!text.is_empty()).then(|| {
                                label(
                                    text.iter().map(|l| format!("[{l}]")).collect(),
                                    TextRole::FrameLabel,
                                    Rect::centered(
                                        Point {
                                            x: (x0 + x1) / 2.0,
                                            y: dy + 4.0 + ts.h / 2.0,
                                        },
                                        ts,
                                    ),
                                )
                            }),
                        }
                    })
                    .collect();
                clusters.push(Cluster {
                    id: f.kind.word().into(),
                    kind: f.kind.cluster(),
                    rect,
                    title,
                    dividers,
                    depth: f.depth,
                });
                touch(&mut frames, x0, x1);
                y += 2.0 + s.row_gap;
            }
        }
    }
    y += s.row_gap;
    for a in 0..n {
        while !active[a].is_empty() {
            close(&mut active, &mut acts, a, y);
        }
    }
    let bottom_y = y + s.row_gap;
    for (a, level, top, bot) in acts.iter().copied() {
        let cx = c[a] + level as f32 * act_step;
        nodes.push(Node {
            id: format!("act-{}-{level}", q.actors[a].id),
            shape: Shape::Activation,
            rect: Rect {
                x: cx - act_w / 2.0,
                y: top,
                w: act_w,
                h: (bot - top).max(4.0),
            },
            label: None,
            link: None,
            classes: Vec::new(),
            style: None,
        });
    }
    let mut lines = Vec::new();
    for (i, a) in q.actors.iter().enumerate() {
        let ts = measure_lines(&a.lines, TextRole::Actor, m);
        for top in [0.0, bottom_y] {
            let rect = Rect {
                x: c[i] - boxes[i].w / 2.0,
                y: top,
                w: boxes[i].w,
                h: box_h,
            };
            let lc = if a.shape == Shape::Actor {
                Point {
                    x: c[i],
                    y: top + box_h - ts.h / 2.0 - 2.0,
                }
            } else {
                rect.center()
            };
            nodes.push(Node {
                id: a.id.clone(),
                shape: a.shape,
                rect,
                label: Some(label(
                    a.lines.clone(),
                    TextRole::Actor,
                    Rect::centered(lc, ts),
                )),
                link: None,
                classes: Vec::new(),
                style: None,
            });
        }
        lines.push(Line {
            a: Point { x: c[i], y: box_h },
            b: Point {
                x: c[i],
                y: bottom_y,
            },
            stroke: Stroke::Dotted,
        });
    }
    // Paint order: actor boxes, activations, notes. Outer frames first.
    nodes.sort_by_key(|n| match n.shape {
        Shape::Participant | Shape::Actor => 0,
        Shape::Activation => 1,
        _ => 2,
    });
    clusters.sort_by_key(|c| c.depth);

    // Shift so the leftmost thing sits at the margin.
    let mut x0 = f32::MAX;
    let mut x1 = f32::MIN;
    for r in nodes
        .iter()
        .map(|n| n.rect)
        .chain(clusters.iter().map(|c| c.rect))
    {
        x0 = x0.min(r.x);
        x1 = x1.max(r.right());
    }
    for e in &edges {
        for p in &e.points {
            x0 = x0.min(p.x);
            x1 = x1.max(p.x);
        }
        if let Some(l) = &e.label {
            x0 = x0.min(l.rect.x);
            x1 = x1.max(l.rect.right());
        }
    }
    let dx = s.margin - x0;
    let dy = s.margin;
    let mv = |r: &mut Rect| {
        r.x += dx;
        r.y += dy;
    };
    for nd in &mut nodes {
        mv(&mut nd.rect);
        if let Some(l) = &mut nd.label {
            mv(&mut l.rect);
        }
    }
    for cl in &mut clusters {
        mv(&mut cl.rect);
        if let Some(l) = &mut cl.title {
            mv(&mut l.rect);
        }
        for d in &mut cl.dividers {
            d.y += dy;
            if let Some(l) = &mut d.label {
                mv(&mut l.rect);
            }
        }
    }
    for e in &mut edges {
        for p in &mut e.points {
            p.x += dx;
            p.y += dy;
        }
        if let Some(l) = &mut e.label {
            mv(&mut l.rect);
        }
    }
    for l in &mut lines {
        l.a.x += dx;
        l.b.x += dx;
        l.a.y += dy;
        l.b.y += dy;
    }
    Ok(Geometry {
        kind: Kind::Sequence,
        size: Size {
            w: x1 - x0 + 2.0 * s.margin,
            h: bottom_y + box_h + 2.0 * s.margin,
        },
        clusters,
        lines,
        nodes,
        edges,
        class_defs: Vec::new(),
        corner_radius: s.corner_radius,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::strip_frontmatter;

    #[test]
    fn parses_messages_notes_and_blocks() {
        let q = parse(&strip_frontmatter(
            "sequenceDiagram\n participant H as harness PTY\n actor U\n H->>+U: hi\n U-->>-H: back\n U-)H: async\n H-xU: lost\n Note over H,U: both\n loop every 20 ms\n H->>H: self\n end\n alt ok\n H->U: a\n else no\n H-->U: b\n end",
        ))
        .unwrap();
        assert_eq!(q.actors.len(), 2);
        assert_eq!(q.actors[0].lines, vec!["harness PTY"]);
        assert_eq!(q.actors[1].shape, Shape::Actor);
        match &q.events[0] {
            Event::Message {
                activate,
                end,
                stroke,
                ..
            } => {
                assert!(activate);
                assert_eq!((*end, *stroke), (Arrow::Filled, Stroke::Solid));
            }
            e => panic!("{e:?}"),
        }
        assert!(matches!(
            q.events[1],
            Event::Message {
                deactivate: true,
                stroke: Stroke::Dotted,
                ..
            }
        ));
        assert!(matches!(
            q.events[2],
            Event::Message {
                end: Arrow::Open,
                ..
            }
        ));
        assert!(matches!(
            q.events[3],
            Event::Message {
                end: Arrow::Cross,
                ..
            }
        ));
        assert!(matches!(
            q.events[4],
            Event::Note {
                place: NotePlace::Over,
                a: 0,
                b: 1,
                ..
            }
        ));
        assert!(matches!(
            q.events.iter().filter(|e| matches!(e, Event::End)).count(),
            2
        ));
    }

    #[test]
    fn rejects_unsupported() {
        let bad = |s: &str| parse(&strip_frontmatter(s)).is_err();
        assert!(bad(
            "sequenceDiagram\n box Aqua group\n participant A\n end"
        ));
        assert!(bad("sequenceDiagram\n loop x\n A->>B: y"));
        assert!(bad("sequenceDiagram\n A=>B: y"));
    }
}
