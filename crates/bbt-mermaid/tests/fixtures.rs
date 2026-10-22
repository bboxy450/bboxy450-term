//! Every fixture parses and lays out cleanly: no overlapping nodes, edges end
//! on borders, clusters hold their members, everything inside the size, and
//! the layout is deterministic.

use bbt_mermaid::{Geometry, Kind, MonoMeasure, Point, Rect, Shape, Style, layout, parse, render};
use std::path::Path;

const EPS: f32 = 0.5;

fn fixtures(sub: &str) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(sub);
    let mut v: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "mmd"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    v.sort();
    v
}

fn all() -> Vec<(String, String)> {
    let mut v = fixtures("bboxy");
    v.extend(fixtures("docs"));
    v.extend(fixtures("state"));
    v
}

/// Mermaid fences in every Markdown file under `dir`, recursively.
fn mermaid_blocks(dir: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            mermaid_blocks(&p, out);
        } else if p.extension().is_some_and(|e| e == "md") {
            let text = std::fs::read_to_string(&p).unwrap();
            let mut cur: Option<String> = None;
            for line in text.lines() {
                match &mut cur {
                    None if line.trim_start().starts_with("```mermaid") => {
                        cur = Some(String::new())
                    }
                    Some(b) if line.trim_start().starts_with("```") => {
                        out.push((p.display().to_string(), std::mem::take(b)));
                        cur = None;
                    }
                    Some(b) => {
                        b.push_str(line);
                        b.push('\n');
                    }
                    None => {}
                }
            }
        }
    }
}

fn normalized(s: &str) -> String {
    s.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn geo(src: &str) -> Geometry {
    render(src, &MonoMeasure::default(), &Style::default()).unwrap()
}

fn on_rect(r: &Rect, p: Point) -> bool {
    let inside_x = p.x >= r.x - EPS && p.x <= r.right() + EPS;
    let inside_y = p.y >= r.y - EPS && p.y <= r.bottom() + EPS;
    let on_x = (p.x - r.x).abs() < EPS || (p.x - r.right()).abs() < EPS;
    let on_y = (p.y - r.y).abs() < EPS || (p.y - r.bottom()).abs() < EPS;
    inside_x && inside_y && (on_x || on_y)
}

fn on_shape(shape: Shape, r: &Rect, p: Point) -> bool {
    let c = r.center();
    let (hw, hh) = (r.w / 2.0, r.h / 2.0);
    match shape {
        Shape::Diamond | Shape::Choice => {
            ((p.x - c.x).abs() / hw + (p.y - c.y).abs() / hh - 1.0).abs() * hw.min(hh) < EPS
        }
        Shape::Circle | Shape::DoubleCircle | Shape::Start | Shape::End => {
            (((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt() - hw).abs() < EPS
        }
        _ => on_rect(r, p),
    }
}

#[test]
fn fixture_counts() {
    let bboxy = fixtures("bboxy");
    let seq = bboxy
        .iter()
        .filter(|(_, s)| s.trim_start().starts_with("sequenceDiagram"))
        .count();
    assert_eq!(
        (bboxy.len() - seq, seq),
        (8, 5),
        "8 flowcharts and 5 sequence diagrams from .bboxy"
    );
    assert_eq!(fixtures("docs").len(), 20);
    assert_eq!(fixtures("state").len(), 20);
}

#[test]
fn every_bboxy_mermaid_block_has_a_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.bboxy");
    let mut blocks = Vec::new();
    mermaid_blocks(&root, &mut blocks);
    assert!(
        !blocks.is_empty(),
        "no mermaid blocks found under {}",
        root.display()
    );
    let have: Vec<String> = fixtures("bboxy")
        .iter()
        .map(|(_, s)| normalized(s))
        .collect();
    for (file, block) in blocks {
        assert!(
            have.contains(&normalized(&block)),
            "{file}: a mermaid block has no fixture in tests/fixtures/bboxy; add it as <spec>-<n>.mmd:\n{block}"
        );
    }
}

#[test]
fn subgraph_direction_is_honoured_unless_linked_outside() {
    let g = geo("flowchart TB\n subgraph S\n direction LR\n a --> b\n end\n x --> S");
    let pos = |id: &str| g.nodes.iter().find(|n| n.id == id).unwrap().rect.center();
    let (a, b) = (pos("a"), pos("b"));
    assert!(
        b.x > a.x + 1.0 && (b.y - a.y).abs() < 1.0,
        "LR inside TB: {a:?} {b:?}"
    );
    // A link into a member makes the subgraph inherit TB.
    let g = geo("flowchart TB\n subgraph S\n direction LR\n a --> b\n end\n x --> a");
    let pos = |id: &str| g.nodes.iter().find(|n| n.id == id).unwrap().rect.center();
    assert!(pos("b").y > pos("a").y + 1.0);
    // A state diagram composite with its own direction.
    let g = geo("stateDiagram-v2\n state B {\n direction LR\n a --> b\n }\n A --> B");
    let pos = |id: &str| g.nodes.iter().find(|n| n.id == id).unwrap().rect.center();
    assert!(pos("b").x > pos("a").x + 1.0);
}

#[test]
fn multi_edges_take_distinct_paths() {
    for src in [
        "flowchart TD\n A --> B\n A --> B\n A -- x --> B",
        "flowchart LR\n A --> B\n B --> A",
        "flowchart TD\n A --> A\n A --> A",
        "stateDiagram-v2\n A --> B : one\n A --> B : two",
    ] {
        let g = geo(src);
        for (i, e) in g.edges.iter().enumerate() {
            for f in &g.edges[i + 1..] {
                assert_ne!(e.points, f.points, "{src:?}: two edges share a path");
            }
        }
    }
}

#[test]
fn every_fixture_lays_out() {
    for (name, src) in all() {
        let g = render(&src, &MonoMeasure::default(), &Style::default());
        assert!(g.is_ok(), "{name}: {:?}", g.err());
        let g = g.unwrap();
        assert!(!g.nodes.is_empty(), "{name}: no nodes");
        assert!(g.size.w > 0.0 && g.size.h > 0.0, "{name}: empty size");
    }
}

#[test]
fn no_overlapping_nodes() {
    for (name, src) in all() {
        let g = geo(&src);
        let solid: Vec<_> = g
            .nodes
            .iter()
            .filter(|n| n.shape != Shape::Activation)
            .collect();
        for (i, a) in solid.iter().enumerate() {
            for b in &solid[i + 1..] {
                assert!(
                    !a.rect.overlaps(&b.rect),
                    "{name}: {} {:?} overlaps {} {:?}",
                    a.id,
                    a.rect,
                    b.id,
                    b.rect
                );
            }
        }
    }
}

#[test]
fn clusters_hold_members_and_exclude_others() {
    for (name, src) in all() {
        let d = parse(&src).unwrap();
        let (bbt_mermaid::Diagram::Flowchart(f) | bbt_mermaid::Diagram::State(f)) = &d else {
            continue;
        };
        let g = layout(&d, &MonoMeasure::default(), &Style::default()).unwrap();
        let rect_of = |id: &str| g.clusters.iter().find(|c| c.id == id).unwrap().rect;
        for (i, n) in f.nodes.iter().enumerate() {
            let nr = g.nodes[i].rect;
            // Ancestors contain the node; every other cluster is disjoint from it.
            let mut anc = Vec::new();
            let mut p = n.parent;
            while let Some(c) = p {
                anc.push(c);
                p = f.subgraphs[c].parent;
            }
            for (c, sg) in f.subgraphs.iter().enumerate() {
                let cr = rect_of(&sg.id);
                if anc.contains(&c) {
                    assert!(
                        cr.contains_rect(&nr),
                        "{name}: {} outside its cluster {}",
                        n.id,
                        sg.id
                    );
                } else {
                    assert!(
                        !cr.overlaps(&nr),
                        "{name}: {} overlaps foreign cluster {}",
                        n.id,
                        sg.id
                    );
                }
            }
        }
        for (a, sa) in f.subgraphs.iter().enumerate() {
            for (b, sb) in f.subgraphs.iter().enumerate().skip(a + 1) {
                let nested = |x: usize, y: usize| {
                    let mut p = f.subgraphs[x].parent;
                    while let Some(c) = p {
                        if c == y {
                            return true;
                        }
                        p = f.subgraphs[c].parent;
                    }
                    false
                };
                let (ra, rb) = (rect_of(&sa.id), rect_of(&sb.id));
                if nested(a, b) {
                    assert!(
                        rb.contains_rect(&ra),
                        "{name}: {} not inside {}",
                        sa.id,
                        sb.id
                    );
                } else if nested(b, a) {
                    assert!(
                        ra.contains_rect(&rb),
                        "{name}: {} not inside {}",
                        sb.id,
                        sa.id
                    );
                } else {
                    assert!(
                        !ra.overlaps(&rb),
                        "{name}: clusters {} and {} overlap",
                        sa.id,
                        sb.id
                    );
                }
            }
        }
    }
}

#[test]
fn edges_start_and_end_on_borders() {
    for (name, src) in all() {
        let g = geo(&src);
        for e in &g.edges {
            assert!(
                e.points.len() >= 2,
                "{name}: edge {}->{} has no path",
                e.from,
                e.to
            );
            let (p0, p1) = (e.points[0], *e.points.last().unwrap());
            match g.kind {
                Kind::Flowchart | Kind::State => {
                    let ends = |id: &str, p: Point| {
                        g.nodes
                            .iter()
                            .filter(|n| n.id == id)
                            .any(|n| on_shape(n.shape, &n.rect, p))
                            || g.clusters
                                .iter()
                                .filter(|c| c.id == id)
                                .any(|c| on_rect(&c.rect, p))
                    };
                    assert!(
                        ends(&e.from, p0),
                        "{name}: {}->{} starts off border at {p0:?}",
                        e.from,
                        e.to
                    );
                    assert!(
                        ends(&e.to, p1),
                        "{name}: {}->{} ends off border at {p1:?}",
                        e.from,
                        e.to
                    );
                }
                Kind::Sequence => {
                    // On the lifeline, or on the side of an activation box of that participant.
                    let ends = |id: &str, p: Point| {
                        let life = g.nodes.iter().find(|n| n.id == id).unwrap().rect.center().x;
                        (p.x - life).abs() < EPS
                            || g.nodes.iter().any(|n| {
                                n.shape == Shape::Activation
                                    && n.id.starts_with(&format!("act-{id}-"))
                                    && ((p.x - n.rect.x).abs() < EPS
                                        || (p.x - n.rect.right()).abs() < EPS)
                            })
                    };
                    assert!(
                        ends(&e.from, p0),
                        "{name}: message {}->{} starts at {p0:?}",
                        e.from,
                        e.to
                    );
                    assert!(
                        ends(&e.to, p1),
                        "{name}: message {}->{} ends at {p1:?}",
                        e.from,
                        e.to
                    );
                }
            }
        }
    }
}

#[test]
fn everything_inside_the_size() {
    for (name, src) in all() {
        let g = geo(&src);
        let frame = Rect {
            x: -EPS,
            y: -EPS,
            w: g.size.w + 2.0 * EPS,
            h: g.size.h + 2.0 * EPS,
        };
        for n in &g.nodes {
            assert!(
                frame.contains_rect(&n.rect),
                "{name}: node {} outside {:?}",
                n.id,
                g.size
            );
        }
        for c in &g.clusters {
            assert!(
                frame.contains_rect(&c.rect),
                "{name}: cluster {} outside",
                c.id
            );
        }
        for e in &g.edges {
            for p in &e.points {
                assert!(
                    frame.contains_rect(&Rect {
                        x: p.x,
                        y: p.y,
                        w: 0.0,
                        h: 0.0
                    }),
                    "{name}: edge point outside"
                );
            }
            if let Some(l) = &e.label {
                assert!(frame.contains_rect(&l.rect), "{name}: edge label outside");
            }
        }
    }
}

#[test]
fn layout_is_deterministic() {
    for (name, src) in all() {
        let a = geo(&src);
        for _ in 0..3 {
            assert_eq!(a, geo(&src), "{name}: layout differs between runs");
        }
    }
}

#[test]
fn unsupported_syntax_is_an_error() {
    for src in [
        "gantt\n title A\n section S\n a :a1, 2014-01-01, 30d",
        "pie\n \"a\": 1",
        "flowchart TD\n A@{ shape: rect, label: \"x\" }",
        "sequenceDiagram\n box Aqua group\n participant A\n end",
        "",
    ] {
        let e = render(src, &MonoMeasure::default(), &Style::default());
        assert!(e.is_err(), "accepted {src:?}");
    }
}
