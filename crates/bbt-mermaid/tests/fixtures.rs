// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Every fixture parses and lays out cleanly: no overlapping nodes, edges end
//! on borders, clusters hold their members, everything inside the size, and
//! the layout is deterministic.

#![allow(
    clippy::disallowed_methods,
    reason = "dev tool and test fixture IO, not the UI thread"
)]

use bbt_mermaid::{Geometry, Kind, MonoMeasure, Point, Rect, Shape, Style, parse, render};
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

// Immutable fixture outputs are shared across the independent geometry checks.
// The determinism check below still renders each source repeatedly itself.
fn rendered() -> &'static [(String, String, Geometry)] {
    static RENDERED: std::sync::OnceLock<Vec<(String, String, Geometry)>> =
        std::sync::OnceLock::new();
    RENDERED.get_or_init(|| {
        all()
            .into_iter()
            .map(|(name, source)| {
                let geometry = render(&source, &MonoMeasure::default(), &Style::default())
                    .unwrap_or_else(|error| panic!("{name}: {error}"));
                (name, source, geometry)
            })
            .collect()
    })
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

/// Open-rectangle segment clipping: endpoint/border contact is allowed, but
/// any positive-length interval in a node's interior is an intersection.
fn path_intersects_node(points: &[Point], r: Rect) -> bool {
    points.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        let (mut enter, mut leave) = (0.0_f32, 1.0_f32);
        for (start, delta, lo, hi) in [
            (a.x, b.x - a.x, r.x, r.right()),
            (a.y, b.y - a.y, r.y, r.bottom()),
        ] {
            if delta == 0.0 {
                if start <= lo || start >= hi {
                    return false;
                }
            } else {
                let (t0, t1) = ((lo - start) / delta, (hi - start) / delta);
                enter = enter.max(t0.min(t1));
                leave = leave.min(t0.max(t1));
            }
        }
        enter < leave
    })
}

// keeps: harness-spec §Refinement pass 2 — 30px message pitch and 4px label gap
#[test]
fn sequence_message_pitch_labels_and_participant_heads_match_refinement() {
    let g = geo(include_str!("fixtures/bboxy/c1-live-seats-2.mmd"));
    assert_eq!(g.edges.len(), 9, "exercise regular and self messages");
    for pair in g.edges.windows(2) {
        assert_eq!(pair[1].points[0].y - pair[0].points[0].y, 30.0);
    }
    for edge in &g.edges {
        assert_eq!(
            edge.points[0].y - edge.label.as_ref().unwrap().rect.bottom(),
            4.0
        );
    }
    for node in &g.nodes {
        assert_eq!(node.rect.h, 30.0);
    }
}

// keeps: harness-spec §Refinement pass 2 — measurement never expands the message pitch
#[test]
fn sequence_multiline_labels_keep_the_fixed_message_pitch() {
    let g = geo("sequenceDiagram\n A->>B: first\n B->>A: two<br/>lines\n A->>B: last");
    assert_eq!(g.edges[1].label.as_ref().unwrap().rect.h, 32.0);
    for pair in g.edges.windows(2) {
        assert_eq!(pair[1].points[0].y - pair[0].points[0].y, 30.0);
    }
    for edge in &g.edges {
        assert_eq!(
            edge.points[0].y - edge.label.as_ref().unwrap().rect.bottom(),
            4.0
        );
    }
}

// keeps: harness-spec §Refinement pass 2 — labels remain 4px above every message
#[test]
fn sequence_fixture_labels_stay_above_arrows_in_frames_notes_and_activations() {
    let mut count = 0;
    for (name, _, g) in rendered()
        .iter()
        .filter(|(_, _, g)| g.kind == Kind::Sequence)
    {
        for edge in &g.edges {
            let gap = edge.points[0].y - edge.label.as_ref().unwrap().rect.bottom();
            assert!((gap - 4.0).abs() < 0.0001, "{name}: label gap {gap}");
            count += 1;
        }
    }
    assert!(count >= 30, "retain the real sequence corpus");
}

// keeps: harness-spec §Refinement pass 2 — back edges avoid every node rectangle
#[test]
fn flowchart_back_edges_use_an_exterior_lane_in_every_direction() {
    for dir in ["TD", "BT", "LR", "RL"] {
        for scope in [false, true] {
            let body = "A[begin] --> B[wide intervening node]\n B --> C[middle]\n C --> D[end]\n D -->|retry| A\n D -.->|again| B";
            let source = if scope {
                format!("flowchart {dir}\n subgraph S[cycle]\n {body}\n end\n X --> A")
            } else {
                format!("flowchart {dir}\n {body}")
            };
            let g = geo(&source);
            for edge in g.edges.iter().filter(|e| e.from == "D") {
                for node in &g.nodes {
                    assert!(
                        !path_intersects_node(&edge.points, node.rect),
                        "{dir}, scope={scope}: {}->{} crosses {}: {:?}",
                        edge.from,
                        edge.to,
                        node.id,
                        edge.points
                    );
                }
                let lane = edge
                    .points
                    .windows(2)
                    .find(|pair| {
                        pair[0] != pair[1]
                            && pair.iter().all(|p| {
                                g.nodes.iter().all(|n| match dir {
                                    "LR" | "RL" => p.y > n.rect.bottom(),
                                    _ => p.x > n.rect.right(),
                                })
                            })
                    })
                    .expect("back edge has a lane outside the node envelope");
                let label = edge.label.as_ref().unwrap().rect.center();
                assert_eq!(
                    label,
                    Point {
                        x: (lane[0].x + lane[1].x) / 2.0,
                        y: (lane[0].y + lane[1].y) / 2.0,
                    }
                );
            }
        }
    }
}

// keeps: harness-spec §Refinement pass 2 — straight-through broken control is rejected
#[test]
fn back_edge_intersection_check_rejects_a_straight_through_control() {
    let g = geo(
        "flowchart TD\n A --> B[an intentionally wide intervening node]\n B --> C\n C --> D\n D --> A",
    );
    let edge = g.edges.iter().find(|e| e.from == "D").unwrap();
    let from = g.nodes.iter().find(|n| n.id == "D").unwrap().rect;
    let to = g.nodes.iter().find(|n| n.id == "A").unwrap().rect;
    let straight = [
        Point {
            x: from.center().x,
            y: from.y,
        },
        Point {
            x: to.center().x,
            y: to.bottom(),
        },
    ];
    let obstacle = g.nodes.iter().find(|n| n.id == "B").unwrap().rect;
    assert!(
        path_intersects_node(&straight, obstacle),
        "broken routing must fail the same oracle"
    );
    assert!(!path_intersects_node(&edge.points, obstacle));
    assert!(
        !path_intersects_node(
            &[
                Point {
                    x: obstacle.x,
                    y: obstacle.y
                },
                Point {
                    x: obstacle.right(),
                    y: obstacle.y
                },
            ],
            obstacle
        ),
        "border contact is allowed"
    );
}

// keeps: harness-spec §Refinement pass 2 — real flowchart back edges avoid node boxes
#[test]
fn flowchart_fixture_back_edges_do_not_intersect_node_rectangles() {
    use bbt_mermaid::{Diagram, Dir};
    let mut count = 0;
    for (name, source, g) in rendered() {
        let Diagram::Flowchart(flow) = parse(source).unwrap() else {
            continue;
        };
        let center = |id: &str| {
            g.nodes
                .iter()
                .find(|n| n.id == id)
                .map(|n| n.rect.center())
                .or_else(|| {
                    g.clusters
                        .iter()
                        .find(|c| c.id == id)
                        .map(|c| c.rect.center())
                })
                .unwrap()
        };
        for edge in &g.edges {
            let (from, to) = (center(&edge.from), center(&edge.to));
            let forward = match flow.dir {
                Dir::TB => to.y - from.y,
                Dir::BT => from.y - to.y,
                Dir::LR => to.x - from.x,
                Dir::RL => from.x - to.x,
            };
            if forward >= 0.0 {
                continue;
            }
            for node in &g.nodes {
                assert!(
                    !path_intersects_node(&edge.points, node.rect),
                    "{name}: {}->{} crosses {} {:?}: {:?}",
                    edge.from,
                    edge.to,
                    node.id,
                    node.rect,
                    edge.points
                );
            }
            count += 1;
        }
    }
    assert!(count > 0, "retain actual back edges in the fixture corpus");
    println!("{count} real flowchart back edges avoid every node rectangle");
}

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
#[test]
fn fixture_counts() {
    assert_eq!(fixtures("docs").len(), 20);
    assert_eq!(fixtures("state").len(), 20);
}

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
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
    let bboxy = fixtures("bboxy");
    let have: Vec<String> = bboxy.iter().map(|(_, s)| normalized(s)).collect();
    for (file, block) in &blocks {
        assert!(
            have.contains(&normalized(block)),
            "{file}: a mermaid block has no fixture in tests/fixtures/bboxy; add it as <spec>-<n>.mmd:\n{block}"
        );
    }
    let want: Vec<String> = blocks.iter().map(|(_, s)| normalized(s)).collect();
    for (name, fixture) in bboxy {
        assert!(
            want.contains(&normalized(&fixture)),
            "tests/fixtures/bboxy/{name}: fixture has no mermaid block under .bboxy; remove it or restore its source block:\n{fixture}"
        );
    }
}

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
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

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
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

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
#[test]
fn every_fixture_lays_out() {
    let fixtures = rendered();
    assert!(!fixtures.is_empty(), "the real fixture corpus is required");
    for (name, _, g) in fixtures {
        assert!(!g.nodes.is_empty(), "{name}: no nodes");
        assert!(g.size.w > 0.0 && g.size.h > 0.0, "{name}: empty size");
    }
}

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
#[test]
fn no_overlapping_nodes() {
    for (name, _, g) in rendered() {
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

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
#[test]
fn clusters_hold_members_and_exclude_others() {
    for (name, src, g) in rendered() {
        let d = parse(src).unwrap();
        let (bbt_mermaid::Diagram::Flowchart(f) | bbt_mermaid::Diagram::State(f)) = &d else {
            continue;
        };
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

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
#[test]
fn edges_start_and_end_on_borders() {
    for (name, _, g) in rendered() {
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

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
#[test]
fn everything_inside_the_size() {
    for (name, _, g) in rendered() {
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

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
#[test]
fn layout_is_deterministic() {
    for (name, src) in all() {
        let a = geo(&src);
        for _ in 0..3 {
            assert_eq!(a, geo(&src), "{name}: layout differs between runs");
        }
    }
}

// keeps: spec document-viewer §Build order — real corpus fixtures, no overlap, border endpoints and deterministic layout
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
