// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Phase A reference tool only. No bboxy SVG interpreter or native painter.
use resvg::{tiny_skia, usvg};
use std::{error::Error, fs, io::BufReader, path::Path};
// Consume the canonical comparator directly, without a renderer/GPU dependency.
#[path = "../bbt-render/src/chrome/diff.rs"]
#[allow(dead_code)]
mod diff;
#[path = "src/viewport.rs"]
mod viewport;

fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    let mut decoder = png::Decoder::new(BufReader::new(fs::File::open(path)?));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    let (w, h) = (info.width, info.height);
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return Err("capture dimensions exceeded".into());
    }
    let size = reader.output_buffer_size().ok_or("missing PNG size")?;
    if size > 4096 * 4096 * 4 {
        return Err("capture byte budget exceeded".into());
    }
    let mut data = vec![0; size];
    let frame = reader.next_frame(&mut data)?;
    let data = &data[..frame.buffer_size()];
    let rgba = match frame.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        png::ColorType::GrayscaleAlpha => data
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        _ => return Err("unsupported reference PNG".into()),
    };
    Ok((w, h, rgba))
}

fn write_png(path: &Path, w: u32, h: u32, data: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut encoder = png::Encoder::new(fs::File::create(path)?, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(data)?;
    Ok(())
}

fn geometry(
    group: &usvg::Group,
    inherited_id: &str,
    scale: f32,
    x: f32,
    y: f32,
    rows: &mut Vec<serde_json::Value>,
) {
    let inherited_id = if group.id().is_empty() {
        inherited_id
    } else {
        group.id()
    };
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => geometry(g, inherited_id, scale, x, y, rows),
            usvg::Node::Path(p) => {
                let b = p.abs_bounding_box();
                let mut start = None;
                let mut end = None;
                let mut subpath = None;
                for segment in p.data().segments() {
                    use tiny_skia::PathSegment::*;
                    let point = match segment {
                        MoveTo(q) => {
                            subpath = Some(q);
                            q
                        }
                        LineTo(q) | QuadTo(_, q) | CubicTo(_, _, q) => q,
                        Close => subpath.unwrap(),
                    };
                    start.get_or_insert(point);
                    end = Some(point);
                }
                let device = |mut q: tiny_skia::Point| {
                    p.abs_transform().map_point(&mut q);
                    [(q.x - x) * scale, (q.y - y) * scale]
                };
                rows.push(serde_json::json!({"id":if p.id().is_empty() { inherited_id } else { p.id() }, "bounds":[(b.x()-x)*scale,(b.y()-y)*scale,b.width()*scale,b.height()*scale],
                    "endpoints":start.zip(end).map(|(a,b)|[device(a),device(b)])}));
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("viewport") if args.len() == 3 || args.len() == 5 => {
            let camera = args.get(3).map(|v| serde_json::from_str(v)).transpose()?;
            let scale = args.get(4).map(|v| v.parse()).transpose()?.unwrap_or(1.);
            let m = viewport::capture_viewport(serde_json::from_str(&args[2])?, camera, scale)?;
            println!(
                "{}",
                serde_json::json!({"viewport":m.viewport,"camera":m.camera,"scale":m.scale,"size":m.size,"transform":m.transform,"root_clip":m.root_clip})
            );
        }
        Some("identity") => println!(
            "{}",
            serde_json::json!({"resvg":"0.48.1","features":[],"text":false,"raster_images":false,"channel_tolerance":diff::CHANNEL_TOLERANCE})
        ),
        Some("compare") if args.len() == 5 => {
            let (w, h, a) = read_png(Path::new(&args[2]))?;
            let (bw, bh, b) = read_png(Path::new(&args[3]))?;
            if (w, h) != (bw, bh) {
                return Err("comparison sizes differ".into());
            }
            let d = diff::compare(&a, &b)?;
            write_png(Path::new(&args[4]), w, h, &d.rgba)?;
            println!(
                "{}",
                serde_json::json!({"changed_fraction":d.changed_fraction,"mean_error":d.mean_error,"width":w,"height":h})
            );
        }
        Some("render") if args.len() == 11 => {
            let n = |i: usize| args[i].parse::<f64>();
            let m = viewport::capture_viewport(
                serde_json::from_str(&args[10])?,
                Some([n(5)?, n(6)?, n(7)?, n(8)?]),
                n(4)?,
            )?;
            let [pw, ph] = m.size;
            let [x, y, w, h] = m.camera.map(|v| v as f32);
            let scale = m.scale as f32;
            let options = usvg::Options {
                image_href_resolver: usvg::ImageHrefResolver {
                    resolve_data: Box::new(|_, _, _| None),
                    resolve_string: Box::new(|_, _| None),
                },
                ..usvg::Options::default()
            };
            let bytes = fs::read(&args[2])?;
            let tree = usvg::Tree::from_data(&bytes, &options)?;
            let mut pixmap =
                tiny_skia::Pixmap::new(pw, ph).ok_or("reference pixmap allocation failed")?;
            pixmap.fill(tiny_skia::Color::WHITE);
            resvg::render(
                &tree,
                tiny_skia::Transform::from_row(
                    m.transform[0],
                    m.transform[1],
                    m.transform[2],
                    m.transform[3],
                    m.transform[4],
                    m.transform[5],
                ),
                &mut pixmap.as_mut(),
            );
            pixmap.save_png(&args[3])?;
            let mut paths = Vec::new();
            geometry(tree.root(), "", scale, x, y, &mut paths);
            fs::write(
                &args[9],
                serde_json::to_vec_pretty(
                    &serde_json::json!({"width":pw,"height":ph,"scale":scale,"camera":[x,y,w,h],"tree_size":[tree.size().width(),tree.size().height()],"paths":paths}),
                )?,
            )?;
        }
        _ => {
            return Err(
                "expected identity, compare A B DIFF, or render SVG PNG SCALE X Y W H JSON".into(),
            );
        }
    }
    Ok(())
}
