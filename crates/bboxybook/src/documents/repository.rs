// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Offline corpus gallery: actual tracked Markdown, never a browser screenshot.
use crate::{Result, components, gallery, settings::Metrics};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{
    draw::{DrawCtx, DrawList},
    layout::Rect,
};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn inputs(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--", ".bboxy"])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "cannot enumerate tracked .bboxy: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let mut paths = vec![];
    for value in output.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let relative = PathBuf::from(std::str::from_utf8(value)?);
        if relative.extension().is_some_and(|s| s == "md") {
            if relative.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            }) {
                return Err("document path escapes the worktree".into());
            }
            paths.push(relative);
        }
    }
    paths.sort();
    Ok(paths)
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn check(root: &Path) -> Result<Vec<serde_json::Value>> {
    let metrics = Metrics::default();
    check_sources(
        inputs(root)?.into_iter().map(|path| {
            let source = fs::read_to_string(root.join(&path))?;
            Ok((path, source))
        }),
        &metrics,
    )
}

fn check_sources(
    sources: impl IntoIterator<Item = Result<(PathBuf, String)>>,
    metrics: &Metrics,
) -> Result<Vec<serde_json::Value>> {
    let mut results = vec![];
    for source in sources {
        let (path, source) = source?;
        let doc = bbt_doc::parse(&source);
        if let Some(error) = &doc.frontmatter.error {
            return Err(format!(
                "{}:{}: invalid YAML frontmatter: {error}",
                path.display(),
                doc.frontmatter.error_line.unwrap_or(1)
            )
            .into());
        }
        for theme in [ui::Theme::Dark, ui::Theme::Light] {
            let ctx = ui::Context::new(theme, ui::Density::Cozy);
            let e = doc.modal_element(&path.to_string_lossy(), false, [1246., 900.], ctx, metrics);
            let l = e.layout(Rect::new(0., 0., 1246., 0.), ctx, metrics)?;
            let failures = body_overflows(&l);
            if !failures.is_empty() {
                return Err(format!("{}: {}", path.display(), failures.join("; ")).into());
            }
            results.push(json!({"path":path,"theme":gallery::theme_name(theme),"blocks":doc.blocks.len(),"height":l.rect.h,"overflows":failures.len()}));
        }
    }
    Ok(results)
}

/// Horizontal overflow is checked against the reading column. Descendants of
/// an explicit scrolling viewport are allowed to extend behind its clip.
pub fn body_overflows(layout: &c::Layout) -> Vec<String> {
    fn body(layout: &c::Layout) -> Option<&c::Layout> {
        if matches!(&layout.kind,c::LayoutKind::Named(k) if k=="body") {
            Some(layout)
        } else {
            layout.children.iter().find_map(body)
        }
    }
    fn walk(
        l: &c::Layout,
        left: f32,
        right: f32,
        clipped: bool,
        path: &str,
        out: &mut Vec<String>,
    ) {
        let key = if let c::LayoutKind::Named(k) = &l.kind {
            k.as_str()
        } else {
            path
        };
        if ![l.rect.x, l.rect.y, l.rect.w, l.rect.h, l.baseline]
            .iter()
            .all(|n| n.is_finite())
        {
            out.push(format!("{key}: non-finite geometry"));
            return;
        }
        if !clipped && (l.rect.x < left - 0.5 || l.rect.x + l.rect.w > right + 0.5) {
            out.push(format!(
                "{key}: {}..{} outside {left}..{right}",
                l.rect.x,
                l.rect.x + l.rect.w
            ));
        }
        let clipped = clipped || matches!(&l.kind, c::LayoutKind::ScrollView { .. });
        for child in &l.children {
            walk(child, left, right, clipped, key, out)
        }
    }
    let mut out = vec![];
    if let Some(b) = body(layout) {
        walk(b, b.rect.x, b.rect.x + b.rect.w, false, "body", &mut out)
    }
    out
}

// Real Markdown pages use flat chrome. Keep its retained order, clips and
// groups in one draw list instead of round-tripping the full image for each
// box. The existing executor still handles any future material-bearing page.
pub(super) fn page_pixels(
    renderer: &mut bbt_render::chrome::ChromeRenderer,
    ops: Vec<c::Paint>,
    pixels: Vec<u8>,
    size: [u32; 2],
    ctx: ui::Context,
) -> Result<Vec<u8>> {
    let mut flat = DrawList::default();
    if components::append_draw(ops.clone(), &mut flat).is_err() {
        return components::execute(renderer, ops, pixels, size, ctx);
    }
    use bbt_render::chrome::materials::{self, Backdrop, Scene};
    Ok(materials::render_composed_to_rgba(
        renderer,
        Scene {
            backdrop: Backdrop::Rgba(&pixels),
            items: &[],
            width: size[0],
            height: size[1],
            scale: 1.,
            time_ms: 450.,
            theme: ctx.theme,
        },
        &flat,
    )?)
}

pub fn render(root: &Path, out: &Path) -> Result<()> {
    render_mode(root, out, false)
}

pub fn render_single_renderer(root: &Path, out: &Path) -> Result<()> {
    render_mode(root, out, true)
}

fn cache_snapshot(
    renderer: &bbt_render::chrome::ChromeRenderer,
    history: &mut Vec<serde_json::Value>,
    context: serde_json::Value,
) {
    let status = renderer.text_cache_stats();
    let s = status.coverage;
    history.push(json!({
        "context": context, "cache_mode": status.mode, "whole_line_masks": status.whole_line_masks,
        "atlas_size": s.atlas_size, "atlas_allocated_bytes": u64::from(s.atlas_size).pow(2),
        "atlas_mask_pixels": s.atlas_mask_pixels, "cached_mask_bytes": s.cached_mask_bytes,
        "glyph_entries": s.glyph_entries, "overlap_entries": s.overlap_entries,
        "atlas_entries": s.atlas_entries, "hits": s.hits, "misses": s.misses,
        "evictions": s.evictions, "rebuilds": s.rebuilds,
        "component_requests": s.component_requests, "overlap_requests": s.overlap_requests,
        "max_component_glyphs": s.max_component_glyphs,
    }));
}
fn write_cache_history(out: &Path, single: bool, history: &[serde_json::Value]) -> Result<()> {
    fs::write(
        out.join("cache-stats.json"),
        serde_json::to_vec_pretty(&json!({
            "renderer_mode": if single { "single" } else { "per-document" }, "samples": history,
        }))?,
    )?;
    Ok(())
}

fn render_mode(root: &Path, out: &Path, single: bool) -> Result<()> {
    let paths = inputs(root)?;
    let total = paths.len();
    if paths.is_empty() {
        return Err("no tracked .bboxy Markdown found".into());
    }
    fs::create_dir_all(out)?;
    let metrics = Metrics::default();
    let mut shared = if single {
        Some(bbt_render::chrome::ChromeRenderer::new(crate::images()?)?)
    } else {
        None
    };
    let mut cache_history = Vec::new();
    let [width, height] = [1246u32, 900u32];
    let mut report = vec![];
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>bbt-doc · repository</title><style>body{font:14px system-ui;background:#10131c;color:#dde4fa;margin:24px}section{margin:24px 0}img{width:100%;max-width:1246px;border:1px solid #65718c}a{color:#8ebaff}summary{cursor:pointer}</style><h1>Native repository documents</h1><p>Tracked .bboxy Markdown · top, middle and end viewports · native Mermaid at true size.</p>",
    );
    for relative in paths {
        let mut isolated;
        let renderer = if let Some(renderer) = shared.as_mut() {
            renderer
        } else {
            isolated = bbt_render::chrome::ChromeRenderer::new(crate::images()?)?;
            &mut isolated
        };
        let source = fs::read_to_string(root.join(&relative))?;
        let markdown = bbt_doc::parse(&source);
        fn mermaid_blocks<'a>(blocks: &'a [bbt_doc::Block], out: &mut Vec<&'a str>) {
            for block in blocks {
                match &block.kind {
                    bbt_doc::BlockKind::Code { info, text }
                        if info.split_whitespace().next() == Some("mermaid") =>
                    {
                        out.push(text)
                    }
                    bbt_doc::BlockKind::Quote(blocks)
                    | bbt_doc::BlockKind::Footnote { blocks, .. } => mermaid_blocks(blocks, out),
                    bbt_doc::BlockKind::List { items, .. } => {
                        for item in items {
                            mermaid_blocks(item, out);
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut diagram_sources = vec![];
        mermaid_blocks(&markdown.blocks, &mut diagram_sources);
        let mut variants = vec![];
        for theme in [ui::Theme::Dark, ui::Theme::Light] {
            let ctx = ui::Context::new(theme, ui::Density::Cozy);
            let element = markdown.modal_element(
                &relative.to_string_lossy(),
                false,
                [width as f32, height as f32],
                ctx,
                &metrics,
            );
            let first = element.layout(Rect::new(0., 0., width as f32, 0.), ctx, &metrics)?;
            let overflows = body_overflows(&first);
            if !overflows.is_empty() {
                return Err(format!(
                    "{} {}: {}",
                    relative.display(),
                    gallery::theme_name(theme),
                    overflows.join("; ")
                )
                .into());
            }
            let extent = first.rect.h;
            if !extent.is_finite() {
                return Err(format!("{}: non-finite document height", relative.display()).into());
            }
            let last = (extent - height as f32).max(0.);
            let mut offsets = vec![("top", 0.)];
            if last > height as f32 {
                offsets.push(("middle", last * 0.5));
            }
            if last > 1. {
                offsets.push(("end", last));
            }
            let mut shots = vec![];
            for (name, offset) in offsets {
                let layout = if offset == 0. {
                    first.clone()
                } else {
                    element.layout(Rect::new(0., -offset, width as f32, 0.), ctx, &metrics)?
                };
                let color = bbt_ui_model::draw::Color::hex(theme.palette().bg).0;
                let bg: Vec<u8> = (0..width * height)
                    .flat_map(|_| color.map(|v| (v * 255.).round() as u8))
                    .collect();
                let result = page_pixels(
                    renderer,
                    c::paint_with_draw(&layout, DrawCtx::new(theme, ctx.density)),
                    bg,
                    [width, height],
                    ctx,
                );
                cache_snapshot(
                    renderer,
                    &mut cache_history,
                    json!({"path":relative,"theme":gallery::theme_name(theme),"view":name,"error":result.as_ref().err().map(|e|e.to_string())}),
                );
                if result.is_err() {
                    write_cache_history(out, single, &cache_history)?;
                }
                let pixels = result?;
                let file = relative
                    .strip_prefix(".bboxy")?
                    .with_extension(format!("{}-{name}.png", gallery::theme_name(theme)));
                let path = out.join(&file);
                fs::create_dir_all(path.parent().ok_or("gallery output has no parent")?)?;
                bbt_render::write_png(&path, &pixels, width, height)?;
                shots.push(json!({"view":name,"offset":offset,"png":file}));
            }
            // Full diagram cases complement the document viewports: a figure
            // may lie between top/middle/end crops in a long spec.
            let mut diagrams = vec![];
            for (i, source) in diagram_sources.iter().enumerate() {
                let geometry = bbt_doc::mermaid::geometry(source, ctx, &metrics);
                let w = geometry
                    .as_ref()
                    .map_or(576., |g| (g.size.w + 16.).max(320.))
                    .ceil()
                    .min(4096.);
                let figure =
                    bbt_doc::mermaid::element(source, w, ctx, &metrics, Default::default());
                let layout = figure.layout(Rect::new(2., 2., w, 0.), ctx, &metrics)?;
                let size = [w as u32 + 4, (layout.rect.h.ceil() as u32 + 4).min(4096)];
                let color = bbt_ui_model::draw::Color::hex(theme.palette().bg).0;
                let bg = (0..size[0] * size[1])
                    .flat_map(|_| color.map(|v| (v * 255.).round() as u8))
                    .collect();
                let result = page_pixels(
                    renderer,
                    c::paint_with_draw(&layout, DrawCtx::new(theme, ctx.density)),
                    bg,
                    size,
                    ctx,
                );
                cache_snapshot(
                    renderer,
                    &mut cache_history,
                    json!({"path":relative,"theme":gallery::theme_name(theme),"mermaid":i+1,"error":result.as_ref().err().map(|e|e.to_string())}),
                );
                if result.is_err() {
                    write_cache_history(out, single, &cache_history)?;
                }
                let pixels = result?;
                let file = relative.strip_prefix(".bboxy")?.with_extension(format!(
                    "{}.mermaid{}.png",
                    gallery::theme_name(theme),
                    i + 1
                ));
                let path = out.join(&file);
                fs::create_dir_all(path.parent().ok_or("diagram output has no parent")?)?;
                bbt_render::write_png(&path, &pixels, size[0], size[1])?;
                diagrams.push(json!({"block":i + 1,"png":file,"viewport":size,"geometry_size":geometry.as_ref().ok().map(|g| [g.size.w,g.size.h]),"error":geometry.err().map(|e| e.to_string())}));
            }
            variants.push(json!({"theme":gallery::theme_name(theme),"height":extent,"overflow_count":overflows.len(),"shots":shots,"diagrams":diagrams}));
        }
        html.push_str(&format!(
            "<section><h2>{}</h2>",
            escape(&relative.to_string_lossy())
        ));
        for variant in &variants {
            html.push_str(&format!(
                "<details><summary>{} · {} px</summary>",
                variant["theme"].as_str().unwrap_or(""),
                variant["height"]
            ));
            for shot in variant["shots"].as_array().unwrap() {
                html.push_str(&format!(
                    "<p>{}</p><img loading=lazy src=\"{}\">",
                    shot["view"].as_str().unwrap_or(""),
                    escape(shot["png"].as_str().ok_or("shot path missing")?)
                ));
            }
            for diagram in variant["diagrams"].as_array().unwrap() {
                html.push_str(&format!("<p>Mermaid {} · true-size canvas</p><div style=\"overflow-x:auto\"><img style=\"width:auto;max-width:none\" loading=lazy src=\"{}\"></div>", diagram["block"], escape(diagram["png"].as_str().ok_or("diagram path missing")?)));
            }
            html.push_str("</details>");
        }
        html.push_str("</section>");
        report.push(json!({"path":relative,"bytes":source.len(),"blocks":markdown.blocks.len(),"metadata_fields":markdown.frontmatter.fields.len(),"metadata_error":markdown.frontmatter.error,"variants":variants}));
        println!(
            "document {}/{}: {}",
            report.len(),
            total,
            relative.display()
        );
    }
    fs::write(out.join("report.json"), serde_json::to_vec_pretty(&report)?)?;
    write_cache_history(out, single, &cache_history)?;
    fs::write(out.join("index.html"), html)?;
    println!(
        "{} documents rendered; {}",
        report.len(),
        out.join("index.html").display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // keeps: spec document-viewer.md §Build order 1
    #[test]
    fn representative_sources_lay_out_in_both_themes_without_unclipped_overflow() {
        let sources = [
            (
                ".bboxy/vision.md",
                "# Vision\n\nA paragraph with **bold**, *emphasis*, `code`, and [[events]].\n\n- a list item\n- a second item\n",
            ),
            (
                ".bboxy/tasks/task.md",
                "---\ntype: task\ntitle: Fixture task\nstatus: running\n---\n## Contract\n\n| Key | Value |\n| --- | --- |\n| alpha | wrapped table content |\n\n> A quoted paragraph.\n",
            ),
            (
                ".bboxy/specs/diagram.md",
                "# Diagram\n\n```mermaid\nflowchart LR\n A[alpha] --> B[beta]\n```\n\n```rust\nlet value = 1;\n```\n",
            ),
        ];
        let reports = check_sources(
            sources
                .into_iter()
                .map(|(path, source)| Ok((PathBuf::from(path), source.to_owned()))),
            &Metrics::default(),
        )
        .unwrap();
        assert_eq!(reports.len(), sources.len() * 2);
        for (path, _) in sources {
            let rows: Vec<_> = reports.iter().filter(|row| row["path"] == path).collect();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0]["theme"], "dark");
            assert_eq!(rows[1]["theme"], "light");
            assert!(rows.iter().all(|row| {
                row["blocks"].as_u64().unwrap() > 0
                    && row["overflows"] == 0
                    && row["height"]
                        .as_f64()
                        .is_some_and(|height| height.is_finite() && height > 0.)
            }));
        }
    }

    // keeps: spec document-viewer.md §Markdown
    #[test]
    fn flat_document_composition_preserves_pixels_including_clips_and_alpha() {
        let markdown = bbt_doc::parse(
            "---\ntype: task\ntitle: Native reader\nstatus: running\n---\nA **bold** line with [[events]]{derives:0.7}.\n\n| Key | Value |\n| --- | --- |\n| alpha | wrapped table content |\n",
        );
        let metrics = Metrics::default();
        let mut renderer =
            bbt_render::chrome::ChromeRenderer::new(crate::images().unwrap()).unwrap();
        for theme in [ui::Theme::Dark, ui::Theme::Light] {
            let ctx = ui::Context::new(theme, ui::Density::Cozy);
            let e = markdown.modal_element("task.md", false, [700., 500.], ctx, &metrics);
            let layout = e
                .layout(Rect::new(0., 0., 700., 0.), ctx, &metrics)
                .unwrap();
            let ops = c::paint_with_draw(&layout, DrawCtx::new(theme, ctx.density));
            let color = bbt_ui_model::draw::Color::hex(theme.palette().bg).0;
            let bg: Vec<_> = (0..700 * 500)
                .flat_map(|_| color.map(|v| (v * 255.).round() as u8))
                .collect();
            let before =
                components::execute(&mut renderer, ops.clone(), bg.clone(), [700, 500], ctx)
                    .unwrap();
            let after = page_pixels(&mut renderer, ops, bg, [700, 500], ctx).unwrap();
            assert_eq!(
                before, after,
                "flat corpus composition must be pixel-identical"
            );
        }
    }
}
