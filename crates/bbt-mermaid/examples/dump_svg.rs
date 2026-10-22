//! Debug dump: lay out every `.mmd` under the given directories and write one SVG each.
//!
//!     cargo run -p bbt-mermaid --example dump_svg -- OUT_DIR FIXTURE_DIR...

#![allow(clippy::disallowed_methods)] // a dev tool reading fixture files, not UI code

use bbt_mermaid::{
    Arrow, Geometry, Label, MonoMeasure, Point, Shape, Stroke, Style, TextRole, render,
};
use std::fmt::Write;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (out, dirs) = args
        .split_first()
        .expect("usage: dump_svg OUT_DIR FIXTURE_DIR...");
    std::fs::create_dir_all(out).unwrap();
    let m = MonoMeasure::default();
    let mut failed = 0;
    for dir in dirs {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        for p in files
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "mmd"))
        {
            let src = std::fs::read_to_string(&p).unwrap();
            let name = p.file_stem().unwrap().to_string_lossy().to_string();
            match render(&src, &m, &Style::default()) {
                Ok(g) => {
                    let path = Path::new(out).join(format!("{name}.svg"));
                    std::fs::write(&path, svg(&g)).unwrap();
                    println!("{name}: {} x {}", g.size.w.round(), g.size.h.round());
                }
                Err(e) => {
                    failed += 1;
                    println!("{name}: ERROR {e}");
                }
            }
        }
    }
    std::process::exit(if failed > 0 { 1 } else { 0 });
}

fn text(s: &mut String, l: &Label, color: &str) {
    // Menlo advances 0.6 em: match MonoMeasure (7.0 per char, 0.9 of it for small roles).
    let size = match l.role {
        TextRole::EdgeLabel | TextRole::FrameLabel | TextRole::Note => 10.5,
        _ => 11.6,
    };
    let lh = l.rect.h / l.lines.len().max(1) as f32;
    for (i, line) in l.lines.iter().enumerate() {
        let y = l.rect.y + lh * (i as f32 + 0.75);
        let esc = line
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let _ = writeln!(
            s,
            r#"<text x="{:.1}" y="{y:.1}" text-anchor="middle" font-family="Menlo, monospace" font-size="{size}" fill="{color}">{esc}</text>"#,
            l.rect.x + l.rect.w / 2.0
        );
    }
}

fn rounded(pts: &[Point], r: f32) -> String {
    let mut d = format!("M{:.1},{:.1}", pts[0].x, pts[0].y);
    for i in 1..pts.len() {
        let p = pts[i];
        if i + 1 < pts.len() {
            let (a, b) = (pts[i - 1], pts[i + 1]);
            let la = ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
            let lb = ((b.x - p.x).powi(2) + (b.y - p.y).powi(2)).sqrt();
            let rr = r.min(la / 2.0).min(lb / 2.0);
            if la > 0.0 && lb > 0.0 {
                let p0 = Point {
                    x: p.x - (p.x - a.x) / la * rr,
                    y: p.y - (p.y - a.y) / la * rr,
                };
                let p1 = Point {
                    x: p.x + (b.x - p.x) / lb * rr,
                    y: p.y + (b.y - p.y) / lb * rr,
                };
                let _ = write!(
                    d,
                    " L{:.1},{:.1} Q{:.1},{:.1} {:.1},{:.1}",
                    p0.x, p0.y, p.x, p.y, p1.x, p1.y
                );
                continue;
            }
        }
        let _ = write!(d, " L{:.1},{:.1}", p.x, p.y);
    }
    d
}

fn head(s: &mut String, tip: Point, from: Point, kind: Arrow) {
    let (dx, dy) = (tip.x - from.x, tip.y - from.y);
    let l = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / l, dy / l);
    let (len, half) = (8.0, 4.0);
    let b = Point {
        x: tip.x - ux * len,
        y: tip.y - uy * len,
    };
    let (l1, l2) = (
        Point {
            x: b.x - uy * half,
            y: b.y + ux * half,
        },
        Point {
            x: b.x + uy * half,
            y: b.y - ux * half,
        },
    );
    let _ = match kind {
        Arrow::None => Ok(()),
        Arrow::Filled => writeln!(
            s,
            r##"<path d="M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} Z" fill="#333"/>"##,
            tip.x, tip.y, l1.x, l1.y, l2.x, l2.y
        ),
        Arrow::Open => writeln!(
            s,
            r##"<path d="M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1}" fill="none" stroke="#333"/>"##,
            l1.x, l1.y, tip.x, tip.y, l2.x, l2.y
        ),
        Arrow::Cross => writeln!(
            s,
            r##"<path d="M{:.1},{:.1} L{:.1},{:.1} M{:.1},{:.1} L{:.1},{:.1}" stroke="#333" stroke-width="1.5"/>"##,
            tip.x - 4.0,
            tip.y - 4.0,
            tip.x + 4.0,
            tip.y + 4.0,
            tip.x - 4.0,
            tip.y + 4.0,
            tip.x + 4.0,
            tip.y - 4.0
        ),
        Arrow::Circle => writeln!(
            s,
            r##"<circle cx="{:.1}" cy="{:.1}" r="4" fill="#fff" stroke="#333"/>"##,
            tip.x - ux * 4.0,
            tip.y - uy * 4.0
        ),
    };
}

fn svg(g: &Geometry) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" viewBox="0 0 {w:.0} {h:.0}"><rect width="100%" height="100%" fill="#fff"/>"##,
        w = g.size.w,
        h = g.size.h
    );
    for c in &g.clusters {
        let r = c.rect;
        let _ = writeln!(
            s,
            r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="6" fill="#f3f1fa" fill-opacity="0.6" stroke="#8a80c0"/>"##,
            r.x, r.y, r.w, r.h
        );
        if let Some(t) = &c.title {
            text(&mut s, t, "#5a4fa0");
        }
        if g.kind == bbt_mermaid::Kind::Sequence {
            let _ = writeln!(
                s,
                r##"<text x="{:.1}" y="{:.1}" font-family="Menlo, monospace" font-size="10.4" font-weight="bold" fill="#5a4fa0">{}</text>"##,
                r.x + 6.0,
                r.y + 13.0,
                c.id
            );
        }
        for d in &c.dividers {
            let _ = writeln!(
                s,
                r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#8a80c0" stroke-dasharray="4 3"/>"##,
                r.x,
                d.y,
                r.right(),
                d.y
            );
            if let Some(l) = &d.label {
                text(&mut s, l, "#5a4fa0");
            }
        }
    }
    for l in &g.lines {
        let _ = writeln!(
            s,
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#999" stroke-dasharray="3 3"/>"##,
            l.a.x, l.a.y, l.b.x, l.b.y
        );
    }
    for n in &g.nodes {
        let r = n.rect;
        let fill = if n.shape == Shape::Note {
            "#fff8c4"
        } else if n.link.is_some() {
            "#e4f0ff"
        } else {
            "#eef0f6"
        };
        let dash = if n.classes.iter().any(|c| {
            g.class_defs
                .iter()
                .any(|d| &d.name == c && d.style.contains("dasharray"))
        }) {
            r#" stroke-dasharray="5 5""#
        } else {
            ""
        };
        let attrs = format!(r##"fill="{fill}" stroke="#556"{dash}"##);
        let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        let _ = match n.shape {
            Shape::Diamond => writeln!(
                s,
                r#"<path d="M{cx:.1},{:.1} L{:.1},{cy:.1} L{cx:.1},{:.1} L{:.1},{cy:.1} Z" {attrs}/>"#,
                r.y,
                r.right(),
                r.bottom(),
                r.x
            ),
            Shape::Circle | Shape::DoubleCircle => writeln!(
                s,
                r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" {attrs}/>"#,
                r.w / 2.0
            ),
            Shape::Stadium => writeln!(
                s,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" {attrs}/>"#,
                r.x,
                r.y,
                r.w,
                r.h,
                r.h / 2.0
            ),
            Shape::Cylinder => {
                let (rx, ry) = (r.w / 2.0, 6.0);
                writeln!(
                    s,
                    r##"<path d="M{:.1},{:.1} A{rx:.1},{ry} 0 0,1 {:.1},{:.1} V{:.1} A{rx:.1},{ry} 0 0,1 {:.1},{:.1} Z" {attrs}/><path d="M{:.1},{:.1} A{rx:.1},{ry} 0 0,0 {:.1},{:.1}" fill="none" stroke="#556"/>"##,
                    r.x,
                    r.y + ry,
                    r.right(),
                    r.y + ry,
                    r.bottom() - ry,
                    r.x,
                    r.bottom() - ry,
                    r.x,
                    r.y + ry,
                    r.right(),
                    r.y + ry
                )
            }
            Shape::Hexagon => {
                let k = r.h / 2.0;
                writeln!(
                    s,
                    r#"<path d="M{:.1},{cy:.1} L{:.1},{:.1} L{:.1},{:.1} L{:.1},{cy:.1} L{:.1},{:.1} L{:.1},{:.1} Z" {attrs}/>"#,
                    r.x,
                    r.x + k,
                    r.y,
                    r.right() - k,
                    r.y,
                    r.right(),
                    r.right() - k,
                    r.bottom(),
                    r.x + k,
                    r.bottom()
                )
            }
            Shape::Start => writeln!(
                s,
                r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" fill="#333"/>"##,
                r.w / 2.0
            ),
            Shape::End => writeln!(
                s,
                r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" fill="#fff" stroke="#333" stroke-width="1.5"/><circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" fill="#333"/>"##,
                r.w / 2.0,
                r.w / 2.0 - 4.0
            ),
            Shape::Choice => writeln!(
                s,
                r#"<path d="M{cx:.1},{:.1} L{:.1},{cy:.1} L{cx:.1},{:.1} L{:.1},{cy:.1} Z" {attrs}/>"#,
                r.y,
                r.right(),
                r.bottom(),
                r.x
            ),
            Shape::Bar => writeln!(
                s,
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="#333"/>"##,
                r.x, r.y, r.w, r.h
            ),
            Shape::Activation => writeln!(
                s,
                r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="#dde" stroke="#556"/>"##,
                r.x, r.y, r.w, r.h
            ),
            Shape::Actor => writeln!(
                s,
                r##"<circle cx="{cx:.1}" cy="{:.1}" r="7" fill="none" stroke="#556"/><path d="M{cx:.1},{:.1} v14 M{:.1},{:.1} h20 M{cx:.1},{:.1} l-8,10 M{cx:.1},{:.1} l8,10" stroke="#556" fill="none"/>"##,
                r.y + 8.0,
                r.y + 15.0,
                cx - 10.0,
                r.y + 20.0,
                r.y + 29.0,
                r.y + 29.0
            ),
            Shape::Rounded => writeln!(
                s,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="8" {attrs}/>"#,
                r.x, r.y, r.w, r.h
            ),
            _ => writeln!(
                s,
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" {attrs}/>"#,
                r.x, r.y, r.w, r.h
            ),
        };
        if n.shape == Shape::Subroutine {
            let _ = writeln!(
                s,
                r##"<path d="M{:.1},{:.1} v{:.1} M{:.1},{:.1} v{:.1}" stroke="#556"/>"##,
                r.x + 8.0,
                r.y,
                r.h,
                r.right() - 8.0,
                r.y,
                r.h
            );
        }
        if let Some(l) = &n.label {
            text(&mut s, l, if n.link.is_some() { "#1060d0" } else { "#222" });
        }
    }
    for e in &g.edges {
        if e.stroke == Stroke::Invisible || e.points.len() < 2 {
            continue;
        }
        let (w, dash) = match e.stroke {
            Stroke::Thick => ("2.6", ""),
            Stroke::Dotted => ("1.2", r#" stroke-dasharray="4 3""#),
            _ => ("1.2", ""),
        };
        let _ = writeln!(
            s,
            r##"<path d="{}" fill="none" stroke="#333" stroke-width="{w}"{dash}/>"##,
            rounded(&e.points, g.corner_radius)
        );
        let n = e.points.len();
        head(&mut s, e.points[n - 1], e.points[n - 2], e.end);
        head(&mut s, e.points[0], e.points[1], e.start);
        if let Some(l) = &e.label {
            if g.kind != bbt_mermaid::Kind::Sequence {
                let r = l.rect;
                let _ = writeln!(
                    s,
                    r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="4" fill="#fff" stroke="#ccc"/>"##,
                    r.x - 3.0,
                    r.y - 2.0,
                    r.w + 6.0,
                    r.h + 4.0
                );
            }
            text(&mut s, l, "#333");
        }
    }
    s.push_str("</svg>\n");
    s
}
