//! Layered (Sugiyama-lite) flowchart layout.
//!
//! Every subgraph is laid out on its own, inside out, and becomes one box in
//! its parent's layout, so clusters never overlap each other or foreign nodes.
//! An edge that crosses a cluster border passes through a port: a zero-size
//! node on the cluster's entry side (top in flow order) or exit side (bottom),
//! routed inside the cluster like any other node and pinned to its border.
//!
//! Per level: cycle removal (DFS), ranks (longest path, doubled so every
//! edge label gets its own middle rank, sources pulled toward their targets),
//! waypoints for long edges, barycentre ordering plus adjacent transposes,
//! x by weighted median-like averaging under a separation constraint solved
//! exactly with pool-adjacent-violators, y by rank heights. Everything runs in
//! a top-to-bottom frame and is mapped to the diagram direction at the end.

use crate::flowchart::{Dir, Endpoint, Flowchart};
use crate::text::{label, measure_lines};
use crate::{
    Cluster, ClusterKind, Edge, Error, Geometry, Kind, Measure, Node, Point, Rect, Shape, Size,
    Style, TextRole,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Item {
    Node(usize),
    Cluster(usize),
    /// Entry port of edge `e` in this level's cluster.
    In(usize),
    /// Exit port of edge `e`.
    Out(usize),
}

/// One edge segment inside a level.
struct Seg {
    edge: usize,
    s: Item,
    t: Item,
    minlen: usize,
    /// Real-size label, only on the segment at the edge's common level.
    label: Option<Size>,
}

#[derive(Default)]
struct LevelOut {
    size: Size,
    centers: BTreeMap<Item, Point>,
    /// Port positions on this cluster's border, keyed by (edge, is_entry).
    ports: BTreeMap<(usize, bool), Point>,
    paths: BTreeMap<usize, Vec<Point>>,
    labels: BTreeMap<usize, Point>,
    /// Edges whose segment here runs against the flow.
    reversed: BTreeSet<usize>,
}

struct Ctx<'a> {
    f: &'a Flowchart,
    s: &'a Style,
    swap: bool,
    node_real: Vec<Size>,
    label_real: Vec<Option<Size>>,
    title_real: Vec<Size>,
    outs: Vec<LevelOut>,
    /// Edges whose ports sit on the opposite sides (entry at the bottom, exit at the top)
    /// because the edge runs against the flow at its common level.
    flips: BTreeSet<usize>,
}

/// Scope 0 is the root; scope `c + 1` is subgraph `c`.
fn container(f: &Flowchart, e: Endpoint) -> usize {
    match e {
        Endpoint::Node(n) => f.nodes[n].parent.map_or(0, |p| p + 1),
        Endpoint::Cluster(c) => f.subgraphs[c].parent.map_or(0, |p| p + 1),
    }
}

fn scope_parent(f: &Flowchart, scope: usize) -> Option<usize> {
    if scope == 0 {
        None
    } else {
        Some(f.subgraphs[scope - 1].parent.map_or(0, |p| p + 1))
    }
}

/// Scopes containing `e`, innermost first, ending at the root.
fn chain(f: &Flowchart, e: Endpoint) -> Vec<usize> {
    let mut v = vec![container(f, e)];
    while let Some(p) = scope_parent(f, *v.last().unwrap()) {
        v.push(p);
    }
    v
}

/// The item in `scope` that is or contains `e`.
fn item_in(f: &Flowchart, e: Endpoint, scope: usize) -> Item {
    let c = container(f, e);
    if c == scope {
        return match e {
            Endpoint::Node(n) => Item::Node(n),
            Endpoint::Cluster(k) => Item::Cluster(k),
        };
    }
    let mut s = c;
    loop {
        let p = scope_parent(f, s).expect("scope is an ancestor");
        if p == scope {
            return Item::Cluster(s - 1);
        }
        s = p;
    }
}

fn node_size(shape: Shape, t: Size, s: &Style) -> Size {
    let (px, py) = (s.node_pad_x, s.node_pad_y);
    let (w, h) = (t.w + 2.0 * px, t.h + 2.0 * py);
    match shape {
        Shape::Stadium => Size { w: w + h / 2.0, h },
        Shape::Subroutine => Size { w: w + 16.0, h },
        Shape::Cylinder => Size { w, h: h + 12.0 },
        Shape::Circle => {
            let d = t.w.max(t.h) + 2.0 * px;
            Size { w: d, h: d }
        }
        Shape::DoubleCircle => {
            let d = t.w.max(t.h) + 2.0 * px + 10.0;
            Size { w: d, h: d }
        }
        Shape::Asymmetric => Size { w: w + h / 3.0, h },
        // A w0 x h0 text box fits a 1.5 w0 x 3 h0 diamond: 1/1.5 + 1/3 = 1 puts its corners on the border.
        Shape::Diamond => Size {
            w: 1.5 * (t.w + px),
            h: 3.0 * (t.h + py),
        },
        Shape::Hexagon => Size { w: w + h / 2.0, h },
        Shape::Parallelogram | Shape::ParallelogramAlt | Shape::Trapezoid | Shape::TrapezoidAlt => {
            Size { w: w + h * 0.6, h }
        }
        _ => Size { w, h },
    }
}

pub(crate) fn layout(f: &Flowchart, m: &dyn Measure, s: &Style) -> Result<Geometry, Error> {
    let swap = matches!(f.dir, Dir::LR | Dir::RL);
    let node_real: Vec<Size> = f
        .nodes
        .iter()
        .map(|n| node_size(n.shape, measure_lines(&n.lines, TextRole::Node, m), s))
        .collect();
    let label_real = f
        .edges
        .iter()
        .map(|e| {
            e.label
                .as_ref()
                .map(|l| measure_lines(l, TextRole::EdgeLabel, m))
        })
        .collect();
    let title_real = f
        .subgraphs
        .iter()
        .map(|g| measure_lines(&g.title, TextRole::ClusterTitle, m))
        .collect();
    let nscopes = f.subgraphs.len() + 1;
    let mut cx = Ctx {
        f,
        s,
        swap,
        node_real,
        label_real,
        title_real,
        outs: Vec::new(),
        flips: BTreeSet::new(),
    };
    cx.outs.resize_with(nscopes, LevelOut::default);

    // Split every edge into per-scope segments.
    let mut segs: Vec<Vec<Seg>> = (0..nscopes).map(|_| Vec::new()).collect();
    let mut loops: Vec<Vec<usize>> = vec![Vec::new(); nscopes];
    let mut routes: Vec<Vec<usize>> = Vec::new(); // scopes of each edge, source side first
    for (k, e) in f.edges.iter().enumerate() {
        let a = chain(f, e.from);
        let b = chain(f, e.to);
        let contains = |outer: Endpoint, inner: &[usize]| match outer {
            Endpoint::Cluster(c) => inner.contains(&(c + 1)),
            Endpoint::Node(_) => false,
        };
        // A subgraph linked to its own member: the edge enters (or leaves) through the subgraph's border.
        if contains(e.from, &b) || contains(e.to, &a) {
            let into = contains(e.from, &b);
            let (Endpoint::Cluster(c), inner, chain_in) = (
                if into { e.from } else { e.to },
                if into { e.to } else { e.from },
                if into { &b } else { &a },
            ) else {
                unreachable!()
            };
            let top = chain_in.iter().position(|&x| x == c + 1).unwrap();
            let mut route = Vec::new();
            for &sc in chain_in[..=top].iter() {
                let it = item_in(f, inner, sc);
                let label = if sc == c + 1 { cx.label_real[k] } else { None };
                let minlen = if sc == c + 1 { e.minlen } else { 1 };
                segs[sc].push(if into {
                    Seg {
                        edge: k,
                        s: Item::In(k),
                        t: it,
                        minlen,
                        label,
                    }
                } else {
                    Seg {
                        edge: k,
                        s: it,
                        t: Item::Out(k),
                        minlen,
                        label,
                    }
                });
                route.push(sc);
            }
            if into {
                route.reverse();
            }
            routes.push(route);
            continue;
        }
        let (ia, l) = a
            .iter()
            .enumerate()
            .find(|(_, x)| b.contains(x))
            .map(|(i, &x)| (i, x))
            .unwrap();
        let ib = b.iter().position(|&x| x == l).unwrap();
        if e.from == e.to {
            loops[l].push(k);
            routes.push(vec![l]);
            continue;
        }
        let mut route = Vec::new();
        for &sc in &a[..ia] {
            segs[sc].push(Seg {
                edge: k,
                s: item_in(f, e.from, sc),
                t: Item::Out(k),
                minlen: 1,
                label: None,
            });
            route.push(sc);
        }
        segs[l].push(Seg {
            edge: k,
            s: item_in(f, e.from, l),
            t: item_in(f, e.to, l),
            minlen: e.minlen,
            label: cx.label_real[k],
        });
        route.push(l);
        for &sc in b[..ib].iter().rev() {
            segs[sc].push(Seg {
                edge: k,
                s: Item::In(k),
                t: item_in(f, e.to, sc),
                minlen: 1,
                label: None,
            });
            route.push(sc);
        }
        routes.push(route);
    }

    // Inside out: deeper scopes first.
    let depth = |sc: usize| {
        let mut d = 0;
        let mut x = sc;
        while let Some(p) = scope_parent(f, x) {
            d += 1;
            x = p;
        }
        d
    };
    let mut order: Vec<usize> = (0..nscopes).collect();
    order.sort_by_key(|&sc| std::cmp::Reverse(depth(sc)));
    // Second pass when a cluster-crossing edge turns out to run backwards:
    // its ports move to the facing sides so it never crosses its own clusters.
    for _ in 0..2 {
        for &sc in &order {
            let out = cx.level(sc, &segs[sc], &loops[sc]);
            cx.outs[sc] = out;
        }
        let flips: BTreeSet<usize> = (0..f.edges.len())
            .filter(|&k| {
                routes[k].len() > 1
                    && routes[k]
                        .iter()
                        .any(|&sc| cx.outs[sc].reversed.contains(&k))
            })
            .collect();
        if flips == cx.flips {
            break;
        }
        cx.flips = flips;
    }

    // Absolute offsets of each scope's top-left, in the frame.
    let mut offset = vec![Point::default(); nscopes];
    let mut by_depth: Vec<usize> = (1..nscopes).collect();
    by_depth.sort_by_key(|&sc| depth(sc));
    for sc in by_depth {
        let p = scope_parent(f, sc).unwrap();
        let c = cx.outs[p].centers[&Item::Cluster(sc - 1)];
        let sz = cx.outs[sc].size;
        offset[sc] = Point {
            x: offset[p].x + c.x - sz.w / 2.0,
            y: offset[p].y + c.y - sz.h / 2.0,
        };
    }
    let abs = |sc: usize, p: Point| Point {
        x: p.x + offset[sc].x,
        y: p.y + offset[sc].y,
    };

    let root = cx.outs[0].size;
    let tf = |p: Point| -> Point {
        let q = match f.dir {
            Dir::TB => p,
            Dir::BT => Point {
                x: p.x,
                y: root.h - p.y,
            },
            Dir::LR => Point { x: p.y, y: p.x },
            Dir::RL => Point {
                x: root.h - p.y,
                y: p.x,
            },
        };
        Point {
            x: q.x + s.margin,
            y: q.y + s.margin,
        }
    };
    let real_size = Size {
        w: if swap { root.h } else { root.w } + 2.0 * s.margin,
        h: if swap { root.w } else { root.h } + 2.0 * s.margin,
    };

    let mut nodes = Vec::new();
    for (i, n) in f.nodes.iter().enumerate() {
        let sc = container(f, Endpoint::Node(i));
        let c = tf(abs(sc, cx.outs[sc].centers[&Item::Node(i)]));
        let ts = measure_lines(&n.lines, TextRole::Node, m);
        let mut lc = c;
        if n.shape == Shape::Cylinder {
            lc.y += 4.0;
        }
        nodes.push(Node {
            id: n.id.clone(),
            shape: n.shape,
            rect: Rect::centered(c, cx.node_real[i]),
            label: Some(label(
                n.lines.clone(),
                TextRole::Node,
                Rect::centered(lc, ts),
            )),
            link: n.link.clone(),
            classes: n.classes.clone(),
            style: n.style.clone(),
        });
    }

    let mut clusters = Vec::new();
    let mut cl_order: Vec<usize> = (0..f.subgraphs.len()).collect();
    cl_order.sort_by_key(|&c| depth(c + 1));
    for c in cl_order {
        let sc = c + 1;
        let fs = cx.outs[sc].size;
        let center = tf(Point {
            x: offset[sc].x + fs.w / 2.0,
            y: offset[sc].y + fs.h / 2.0,
        });
        let rs = if swap { Size { w: fs.h, h: fs.w } } else { fs };
        let rect = Rect::centered(center, rs);
        let ts = cx.title_real[c];
        let title = label(
            f.subgraphs[c].title.clone(),
            TextRole::ClusterTitle,
            Rect {
                x: center.x - ts.w / 2.0,
                y: rect.y + s.cluster_pad * 0.5,
                w: ts.w,
                h: ts.h,
            },
        );
        clusters.push(Cluster {
            id: f.subgraphs[c].id.clone(),
            kind: ClusterKind::Subgraph,
            rect,
            title: Some(title),
            dividers: Vec::new(),
            depth: depth(sc) - 1,
        });
    }

    let mut edges = Vec::new();
    for (k, e) in f.edges.iter().enumerate() {
        let mut pts: Vec<Point> = Vec::new();
        for &sc in &routes[k] {
            for &p in &cx.outs[sc].paths[&k] {
                let q = tf(abs(sc, p));
                if pts
                    .last()
                    .is_none_or(|l| (l.x - q.x).abs() > 0.01 || (l.y - q.y).abs() > 0.01)
                {
                    pts.push(q);
                }
            }
        }
        let lab = routes[k]
            .iter()
            .find_map(|&sc| cx.outs[sc].labels.get(&k).map(|&p| tf(abs(sc, p))));
        let name = |ep: Endpoint| match ep {
            Endpoint::Node(n) => f.nodes[n].id.clone(),
            Endpoint::Cluster(c) => f.subgraphs[c].id.clone(),
        };
        edges.push(Edge {
            from: name(e.from),
            to: name(e.to),
            points: pts,
            stroke: e.stroke,
            start: e.start,
            end: e.end,
            label: match (&e.label, lab) {
                (Some(lines), Some(c)) => Some(label(
                    lines.clone(),
                    TextRole::EdgeLabel,
                    Rect::centered(c, cx.label_real[k].unwrap()),
                )),
                _ => None,
            },
        });
    }

    Ok(Geometry {
        kind: Kind::Flowchart,
        size: real_size,
        clusters,
        lines: Vec::new(),
        nodes,
        edges,
        class_defs: f.class_defs.clone(),
        corner_radius: s.corner_radius,
    })
}

/// A vertex of the layered graph: a level item or an edge waypoint.
struct V {
    w: f32,
    h: f32,
    /// Width reserved in its rank (adds room for self loops).
    lw: f32,
    rank: usize,
    shape: Shape,
    real: bool,
}

impl Ctx<'_> {
    fn frame(&self, s: Size) -> Size {
        if self.swap {
            Size { w: s.h, h: s.w }
        } else {
            s
        }
    }

    fn level(&self, scope: usize, segs: &[Seg], loops: &[usize]) -> LevelOut {
        let f = self.f;
        let st = self.s;
        // Items: nodes, clusters, then ports in segment order.
        let mut items: Vec<Item> = Vec::new();
        for i in 0..f.nodes.len() {
            if container(f, Endpoint::Node(i)) == scope {
                items.push(Item::Node(i));
            }
        }
        for c in 0..f.subgraphs.len() {
            if container(f, Endpoint::Cluster(c)) == scope {
                items.push(Item::Cluster(c));
            }
        }
        for sg in segs {
            for it in [sg.s, sg.t] {
                if matches!(it, Item::In(_) | Item::Out(_)) && !items.contains(&it) {
                    items.push(it);
                }
            }
        }
        let idx: BTreeMap<Item, usize> = items.iter().enumerate().map(|(i, &it)| (it, i)).collect();
        let loop_room = 28.0;
        let mut vs: Vec<V> = items
            .iter()
            .map(|&it| {
                let (sz, shape) = match it {
                    Item::Node(n) => (self.frame(self.node_real[n]), f.nodes[n].shape),
                    Item::Cluster(c) => (self.outs[c + 1].size, Shape::Rect),
                    _ => (Size::default(), Shape::Rect),
                };
                let has_loop = matches!(it, Item::Node(n) if loops.iter().any(|&k| f.edges[k].from == Endpoint::Node(n)));
                let lw = sz.w + if has_loop { 2.0 * loop_room } else { 0.0 };
                V { w: sz.w, h: sz.h, lw, rank: 0, shape, real: matches!(it, Item::Node(_) | Item::Cluster(_)) }
            })
            .collect();
        let n_items = vs.len();
        let is_port = |i: usize| matches!(items[i], Item::In(_) | Item::Out(_));

        // Attach offsets (frame, relative to the item centre) where a segment enters a child cluster's port.
        let attach = |it: Item, edge: usize, entry: bool| -> Option<Point> {
            let Item::Cluster(c) = it else { return None };
            let o = &self.outs[c + 1];
            o.ports.get(&(edge, entry)).map(|p| Point {
                x: p.x - o.size.w / 2.0,
                y: p.y - o.size.h / 2.0,
            })
        };

        // 1. Cycle removal.
        let es: Vec<(usize, usize)> = segs.iter().map(|g| (idx[&g.s], idx[&g.t])).collect();
        let mut out_adj: Vec<Vec<usize>> = vec![Vec::new(); n_items];
        for (k, &(a, _)) in es.iter().enumerate() {
            out_adj[a].push(k);
        }
        let mut reversed = vec![false; es.len()];
        let mut state = vec![0u8; n_items];
        fn dfs(
            v: usize,
            es: &[(usize, usize)],
            adj: &[Vec<usize>],
            st: &mut [u8],
            rev: &mut [bool],
        ) {
            st[v] = 1;
            for &k in &adj[v] {
                let t = es[k].1;
                match st[t] {
                    0 => dfs(t, es, adj, st, rev),
                    1 => rev[k] = true,
                    _ => {}
                }
            }
            st[v] = 2;
        }
        for v in 0..n_items {
            if state[v] == 0 {
                dfs(v, &es, &out_adj, &mut state, &mut reversed);
            }
        }
        let flipped =
            |it: Item| matches!(it, Item::In(e) | Item::Out(e) if self.flips.contains(&e));
        for (k, g) in segs.iter().enumerate() {
            if flipped(g.s) || flipped(g.t) {
                reversed[k] = true;
            }
        }
        let dag: Vec<(usize, usize)> = es
            .iter()
            .zip(&reversed)
            .map(|(&(a, b), &r)| if r { (b, a) } else { (a, b) })
            .collect();

        // 2. Ranks on non-port items, doubled.
        let len = |k: usize| 2 * segs[k].minlen.max(1);
        let mut indeg = vec![0usize; n_items];
        for (k, &(a, b)) in dag.iter().enumerate() {
            if !is_port(a) && !is_port(b) {
                indeg[b] += 1;
                let _ = k;
            }
        }
        let mut topo = Vec::new();
        let mut ready: Vec<usize> = (0..n_items)
            .filter(|&v| !is_port(v) && indeg[v] == 0)
            .collect();
        let mut rank = vec![0i64; n_items];
        while let Some(&v) = ready.first() {
            ready.remove(0);
            topo.push(v);
            for (k, &(a, b)) in dag.iter().enumerate() {
                if a == v && !is_port(b) {
                    rank[b] = rank[b].max(rank[a] + len(k) as i64);
                    indeg[b] -= 1;
                    if indeg[b] == 0 {
                        let pos = ready.partition_point(|&x| x < b);
                        ready.insert(pos, b);
                    }
                }
            }
        }
        for &v in topo.iter().rev() {
            let has_in = dag.iter().any(|&(a, b)| b == v && !is_port(a));
            let outs: Vec<i64> = dag
                .iter()
                .enumerate()
                .filter(|&(_, &(a, b))| a == v && !is_port(b))
                .map(|(k, &(_, b))| rank[b] - len(k) as i64)
                .collect();
            if !has_in && let Some(&mn) = outs.iter().min() {
                rank[v] = mn;
            }
        }
        let reals: Vec<usize> = (0..n_items).filter(|&v| !is_port(v)).collect();
        let lo = reals.iter().map(|&v| rank[v]).min().unwrap_or(0);
        // Entry ports sit on the top rank and exit ports below the last, unless flipped.
        let top_port = |it: &Item| {
            matches!(it, Item::In(_)) != flipped(*it) && matches!(it, Item::In(_) | Item::Out(_))
        };
        let has_in_port = items.iter().any(top_port);
        let shift = if has_in_port { 2 } else { 0 };
        for &v in &reals {
            rank[v] += shift - lo;
        }
        let hi = reals.iter().map(|&v| rank[v]).max().unwrap_or(0);
        for v in 0..n_items {
            match items[v] {
                Item::In(_) | Item::Out(_) => {
                    rank[v] = if top_port(&items[v]) { 0 } else { hi + 2 }
                }
                _ => {}
            }
            vs[v].rank = rank[v] as usize;
        }

        // 3. Waypoints.
        let mut lsegs: Vec<S2> = Vec::new();
        let mut chains: Vec<Vec<usize>> = Vec::new();
        let mut label_v: Vec<Option<usize>> = Vec::new();
        for (k, &(a, b)) in dag.iter().enumerate() {
            let g = &segs[k];
            let (oa, ob) = {
                let os = attach(g.s, g.edge, false).map_or(0.0, |p| p.x);
                let ot = attach(g.t, g.edge, true).map_or(0.0, |p| p.x);
                if reversed[k] { (ot, os) } else { (os, ot) }
            };
            let (ra, rb) = (vs[a].rank, vs[b].rank);
            let mid = ra + (rb - ra) / 2;
            let mut ch = vec![a];
            let mut lv = None;
            for r in ra + 1..rb {
                let (w, h) = match g.label {
                    Some(ls) if r == mid => {
                        let fs = self.frame(Size {
                            w: ls.w + 2.0 * st.label_pad,
                            h: ls.h + 2.0 * st.label_pad,
                        });
                        (fs.w, fs.h)
                    }
                    _ => (0.0, 0.0),
                };
                if w > 0.0 {
                    lv = Some(vs.len());
                }
                vs.push(V {
                    w,
                    h,
                    lw: w,
                    rank: r,
                    shape: Shape::Rect,
                    real: false,
                });
                ch.push(vs.len() - 1);
            }
            ch.push(b);
            for i in 0..ch.len() - 1 {
                let (u, v) = (ch[i], ch[i + 1]);
                let dummies = (u >= n_items) as u8 + (v >= n_items) as u8;
                let w = [1.0, 2.0, 8.0][dummies as usize];
                lsegs.push(S2 {
                    u,
                    v,
                    w,
                    off_u: if i == 0 { oa } else { 0.0 },
                    off_v: if i + 2 == ch.len() { ob } else { 0.0 },
                });
            }
            chains.push(ch);
            label_v.push(lv);
        }
        let nv = vs.len();
        let nranks = vs.iter().map(|v| v.rank).max().unwrap_or(0) + 1;

        // 4. Ordering. Initial: DFS in input order.
        let mut succ: Vec<Vec<usize>> = vec![Vec::new(); nv];
        let mut pred: Vec<Vec<usize>> = vec![Vec::new(); nv];
        for (i, sg) in lsegs.iter().enumerate() {
            succ[sg.u].push(i);
            pred[sg.v].push(i);
        }
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); nranks];
        let mut seen = vec![false; nv];
        let mut stack: Vec<usize> = Vec::new();
        for start in (0..n_items).chain(n_items..nv) {
            if seen[start] {
                continue;
            }
            stack.push(start);
            while let Some(v) = stack.pop() {
                if seen[v] {
                    continue;
                }
                seen[v] = true;
                layers[vs[v].rank].push(v);
                for &i in succ[v].iter().rev() {
                    if !seen[lsegs[i].v] {
                        stack.push(lsegs[i].v);
                    }
                }
            }
        }
        let mut pos = vec![0usize; nv];
        let set_pos = |layers: &[Vec<usize>], pos: &mut [usize]| {
            for l in layers {
                for (i, &v) in l.iter().enumerate() {
                    pos[v] = i;
                }
            }
        };
        set_pos(&layers, &mut pos);
        let (succ_r, lsegs_r) = (&succ, &lsegs);
        let crossings = |layers: &[Vec<usize>], pos: &[usize]| -> usize {
            let mut total = 0;
            for l in layers {
                let s: Vec<(usize, usize)> = l
                    .iter()
                    .flat_map(|&u| succ_r[u].iter().map(move |&i| (pos[u], pos[lsegs_r[i].v])))
                    .collect();
                for i in 0..s.len() {
                    for j in i + 1..s.len() {
                        if (s[i].0 < s[j].0 && s[i].1 > s[j].1)
                            || (s[i].0 > s[j].0 && s[i].1 < s[j].1)
                        {
                            total += 1;
                        }
                    }
                }
            }
            total
        };
        let mut best = layers.clone();
        let mut best_c = crossings(&layers, &pos);
        for it in 0..24 {
            let down = it % 2 == 0;
            let rs: Vec<usize> = if down {
                (1..nranks).collect()
            } else {
                (0..nranks.saturating_sub(1)).rev().collect()
            };
            for r in rs {
                let mut keyed: Vec<(f32, usize)> = layers[r]
                    .iter()
                    .map(|&v| {
                        let nb: Vec<usize> = if down {
                            pred[v].iter().map(|&i| lsegs[i].u).collect()
                        } else {
                            succ[v].iter().map(|&i| lsegs[i].v).collect()
                        };
                        let b = if nb.is_empty() {
                            pos[v] as f32
                        } else {
                            nb.iter().map(|&u| pos[u] as f32).sum::<f32>() / nb.len() as f32
                        };
                        (b, v)
                    })
                    .collect();
                keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
                layers[r] = keyed.into_iter().map(|(_, v)| v).collect();
                for (i, &v) in layers[r].iter().enumerate() {
                    pos[v] = i;
                }
            }
            // Transpose adjacent pairs while it helps.
            for _ in 0..4 {
                let mut improved = false;
                for r in 0..nranks {
                    for i in 0..layers[r].len().saturating_sub(1) {
                        let (v, w) = (layers[r][i], layers[r][i + 1]);
                        let pair = |a: usize, b: usize| -> usize {
                            let mut c = 0;
                            for side in [&pred, &succ] {
                                let na: Vec<usize> =
                                    side[a].iter().map(|&k| other(&lsegs[k], a)).collect();
                                let nb: Vec<usize> =
                                    side[b].iter().map(|&k| other(&lsegs[k], b)).collect();
                                for &p in &na {
                                    for &q in &nb {
                                        if pos[p] > pos[q] {
                                            c += 1;
                                        }
                                    }
                                }
                            }
                            c
                        };
                        if pair(w, v) < pair(v, w) {
                            layers[r].swap(i, i + 1);
                            pos[v] = i + 1;
                            pos[w] = i;
                            improved = true;
                        }
                    }
                }
                if !improved {
                    break;
                }
            }
            let c = crossings(&layers, &pos);
            if c < best_c {
                best_c = c;
                best = layers.clone();
            }
        }
        let layers = best;
        set_pos(&layers, &mut pos);

        // 5. x: averaging toward neighbours under separation constraints.
        let gap = |a: usize, b: usize| {
            if vs[a].real && vs[b].real {
                st.node_sep
            } else {
                st.edge_sep
            }
        };
        let mut x = vec![0f32; nv];
        for l in &layers {
            let mut cur = 0.0;
            for (i, &v) in l.iter().enumerate() {
                if i > 0 {
                    let u = l[i - 1];
                    cur += (vs[u].lw + vs[v].lw) / 2.0 + gap(u, v);
                }
                x[v] = cur;
            }
        }
        let sweep = |x: &mut Vec<f32>, r: usize, use_pred: bool, use_succ: bool| {
            let l = &layers[r];
            let mut desired = Vec::with_capacity(l.len());
            let mut wt = Vec::with_capacity(l.len());
            for &v in l {
                let (mut sum, mut ws) = (0.0, 0.0);
                if use_pred {
                    for &i in &pred[v] {
                        let g = &lsegs[i];
                        sum += g.w * (x[g.u] + g.off_u - g.off_v);
                        ws += g.w;
                    }
                }
                if use_succ {
                    for &i in &succ[v] {
                        let g = &lsegs[i];
                        sum += g.w * (x[g.v] + g.off_v - g.off_u);
                        ws += g.w;
                    }
                }
                if ws > 0.0 {
                    desired.push(sum / ws);
                    wt.push(ws);
                } else {
                    desired.push(x[v]);
                    wt.push(0.01);
                }
            }
            let sep: Vec<f32> = (1..l.len())
                .map(|i| (vs[l[i - 1]].lw + vs[l[i]].lw) / 2.0 + gap(l[i - 1], l[i]))
                .collect();
            let placed = pav(&desired, &wt, &sep);
            for (i, &v) in l.iter().enumerate() {
                x[v] = placed[i];
            }
        };
        for _ in 0..12 {
            for r in 1..nranks {
                sweep(&mut x, r, true, false);
            }
            for r in (0..nranks.saturating_sub(1)).rev() {
                sweep(&mut x, r, false, true);
            }
        }
        for _ in 0..4 {
            for r in 0..nranks {
                sweep(&mut x, r, true, true);
            }
        }

        // 6. y by rank heights.
        let mut rank_h = vec![0f32; nranks];
        for v in &vs {
            rank_h[v.rank] = rank_h[v.rank].max(v.h);
        }
        let mut rank_y = vec![0f32; nranks];
        // Rows that only lead to a port sit tight: the cluster padding already separates them.
        let has_out_port = items
            .iter()
            .any(|it| matches!(it, Item::In(_) | Item::Out(_)) && !top_port(it));
        let mut y = 0.0;
        for r in 0..nranks {
            rank_y[r] = y + rank_h[r] / 2.0;
            let port_gap =
                (has_in_port && r <= 1) || (has_out_port && r + 3 >= nranks && r + 1 < nranks);
            y += rank_h[r]
                + if port_gap {
                    st.rank_sep / 6.0
                } else {
                    st.rank_sep / 2.0
                };
        }
        let at = |v: usize| Point {
            x: x[v],
            y: rank_y[vs[v].rank],
        };

        // 7. Paths, source to target.
        let mut out = LevelOut::default();
        for (k, g) in segs.iter().enumerate() {
            let mut mid: Vec<Point> = chains[k][1..chains[k].len() - 1]
                .iter()
                .map(|&v| at(v))
                .collect();
            if reversed[k] {
                mid.reverse();
            }
            let (si, ti) = (idx[&g.s], idx[&g.t]);
            let anchor = |i: usize, it: Item, entry: bool| -> Option<Point> {
                attach(it, g.edge, entry).map(|o| Point {
                    x: at(i).x + o.x,
                    y: at(i).y + o.y,
                })
            };
            let sa = anchor(si, g.s, false);
            let ta = anchor(ti, g.t, true);
            let toward_t = mid.first().copied().or(ta).unwrap_or(at(ti));
            let toward_s = mid.last().copied().or(sa).unwrap_or(at(si));
            let p0 = sa.unwrap_or_else(|| clip(at(si), &vs[si], toward_t));
            let p1 = ta.unwrap_or_else(|| clip(at(ti), &vs[ti], toward_s));
            let mut pts = vec![p0];
            pts.extend(mid);
            pts.push(p1);
            out.paths.insert(g.edge, pts);
            if reversed[k] && !flipped(g.s) && !flipped(g.t) {
                out.reversed.insert(g.edge);
            }
            if let Some(lv) = label_v[k] {
                out.labels.insert(g.edge, at(lv));
            }
        }
        for &k in loops {
            let Endpoint::Node(n) = f.edges[k].from else {
                continue;
            };
            let i = idx[&Item::Node(n)];
            let c = at(i);
            let (hw, hh) = (vs[i].w / 2.0, vs[i].h / 2.0);
            let xr = c.x + hw + loop_room * 0.8;
            let (ya, yb) = (c.y - hh * 0.5, c.y + hh * 0.5);
            out.paths.insert(
                k,
                vec![
                    Point { x: c.x + hw, y: ya },
                    Point { x: xr, y: ya },
                    Point { x: xr, y: yb },
                    Point { x: c.x + hw, y: yb },
                ],
            );
            if self.label_real[k].is_some() {
                let ls = self.frame(self.label_real[k].unwrap());
                out.labels.insert(
                    k,
                    Point {
                        x: xr + ls.w / 2.0 + 4.0,
                        y: c.y,
                    },
                );
            }
        }
        for (i, &it) in items.iter().enumerate() {
            out.centers.insert(it, at(i));
        }

        // 8. Normalise to the content box, then pad clusters and pin ports to the border.
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        let mut grow = |r: Rect| {
            x0 = x0.min(r.x);
            y0 = y0.min(r.y);
            x1 = x1.max(r.right());
            y1 = y1.max(r.bottom());
        };
        for (i, v) in vs.iter().enumerate().take(n_items) {
            grow(Rect::centered(at(i), Size { w: v.w, h: v.h }));
        }
        for (&k, &c) in &out.labels {
            grow(Rect::centered(c, self.frame(self.label_real[k].unwrap())));
        }
        for p in out.paths.values().flatten() {
            grow(Rect {
                x: p.x,
                y: p.y,
                w: 0.0,
                h: 0.0,
            });
        }
        if x0 > x1 {
            (x0, y0, x1, y1) = (0.0, 0.0, 0.0, 0.0);
        }
        let (mut pt, mut pb, mut pl, mut pr) = (0.0, 0.0, 0.0, 0.0);
        if scope > 0 {
            let c = scope - 1;
            let title = self.title_real[c];
            let p = st.cluster_pad;
            let (mut t, b, mut l, mut r) = (title.h + p * 1.5, p, p, p);
            let content_real_w = if self.swap { y1 - y0 } else { x1 - x0 };
            let need = title.w + 2.0 * p - (content_real_w + l + r);
            if need > 0.0 {
                l += need / 2.0;
                r += need / 2.0;
            }
            if x0 > x1 - 1.0 && y0 > y1 - 1.0 {
                t = t.max(p);
            }
            (pt, pb, pl, pr) = match self.f.dir {
                Dir::TB => (t, b, l, r),
                Dir::BT => (b, t, l, r),
                Dir::LR => (l, r, t, b),
                Dir::RL => (r, l, t, b),
            };
        }
        let dx = pl - x0;
        let dy = pt - y0;
        let mv = |p: &mut Point| {
            p.x += dx;
            p.y += dy;
        };
        out.centers.values_mut().for_each(mv);
        out.labels.values_mut().for_each(mv);
        out.paths.values_mut().flatten().for_each(mv);
        out.size = Size {
            w: x1 - x0 + pl + pr,
            h: y1 - y0 + pt + pb,
        };
        for (k, g) in segs.iter().enumerate() {
            let _ = k;
            if let Item::In(e) = g.s {
                let h = if flipped(g.s) { out.size.h } else { 0.0 };
                let p = out.centers.get_mut(&Item::In(e)).unwrap();
                p.y = h;
                let p = *p;
                out.paths.get_mut(&g.edge).unwrap()[0] = p;
                out.ports.insert((e, true), p);
            }
            if let Item::Out(e) = g.t {
                let h = if flipped(g.t) { 0.0 } else { out.size.h };
                let p = out.centers.get_mut(&Item::Out(e)).unwrap();
                p.y = h;
                let p = *p;
                *out.paths.get_mut(&g.edge).unwrap().last_mut().unwrap() = p;
                out.ports.insert((e, false), p);
            }
        }
        out
    }
}

/// A layered-graph segment between adjacent ranks; offsets are port attach x.
struct S2 {
    u: usize,
    v: usize,
    w: f32,
    off_u: f32,
    off_v: f32,
}

fn other(s: &S2, v: usize) -> usize {
    if s.u == v { s.v } else { s.u }
}

/// Point where the ray from `c` toward `p` leaves the shape of `v`.
fn clip(c: Point, v: &V, p: Point) -> Point {
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    let (hw, hh) = (v.w / 2.0, v.h / 2.0);
    if hw <= 0.0 || hh <= 0.0 {
        return c;
    }
    if dx.abs() < 1e-4 && dy.abs() < 1e-4 {
        return Point {
            x: c.x,
            y: c.y + hh,
        };
    }
    let t = match v.shape {
        Shape::Diamond => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        Shape::Circle | Shape::DoubleCircle => hw.min(hh) / (dx * dx + dy * dy).sqrt(),
        _ => {
            let tx = if dx.abs() > 1e-6 {
                hw / dx.abs()
            } else {
                f32::MAX
            };
            let ty = if dy.abs() > 1e-6 {
                hh / dy.abs()
            } else {
                f32::MAX
            };
            tx.min(ty)
        }
    };
    Point {
        x: c.x + dx * t,
        y: c.y + dy * t,
    }
}

/// Weighted isotonic placement: positions as close as possible to `d`
/// (weights `w`) keeping order and gaps `sep[i]` between items i and i+1.
fn pav(d: &[f32], w: &[f32], sep: &[f32]) -> Vec<f32> {
    let n = d.len();
    let mut off = vec![0f32; n];
    for i in 1..n {
        off[i] = off[i - 1] + sep[i - 1];
    }
    // blocks: (weight, mean, count)
    let mut blocks: Vec<(f32, f32, usize)> = Vec::new();
    for i in 0..n {
        blocks.push((w[i], d[i] - off[i], 1));
        while blocks.len() >= 2 {
            let b = blocks[blocks.len() - 1];
            let a = blocks[blocks.len() - 2];
            if a.1 <= b.1 {
                break;
            }
            let wsum = a.0 + b.0;
            blocks.pop();
            *blocks.last_mut().unwrap() = (wsum, (a.0 * a.1 + b.0 * b.1) / wsum, a.2 + b.2);
        }
    }
    let mut out = Vec::with_capacity(n);
    for (_, m, c) in blocks {
        for _ in 0..c {
            let i = out.len();
            out.push(m + off[i]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::pav;

    #[test]
    fn pav_keeps_order_and_gaps() {
        let x = pav(&[10.0, 0.0, 5.0], &[1.0, 1.0, 1.0], &[4.0, 4.0]);
        assert!(x[1] - x[0] >= 4.0 - 1e-4 && x[2] - x[1] >= 4.0 - 1e-4);
        let x = pav(&[0.0, 100.0], &[1.0, 1.0], &[10.0]);
        assert_eq!(x, vec![0.0, 100.0]);
    }
}
