use crate::{
    Result,
    gallery::{self, Case},
};
use bbt_render::chrome::{ChromeRenderer, diff};
use std::{fmt::Write, fs, path::Path};
pub fn render(out: &Path, cases: &[Case]) -> Result<usize> {
    let mut renderer = ChromeRenderer::new(crate::images()?)?;
    println!("adapter: {:?}", renderer.context.adapter_info);
    for (i, case) in cases.iter().enumerate() {
        let rgba = if case
            .fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "components")
        {
            crate::components::render_case(&mut renderer, case)?
        } else if case.fixture.as_ref().is_some_and(|f| f["layer"] == "atoms") {
            crate::atoms::render_case(&mut renderer, case)?
        } else if case.component == "materials" {
            crate::materials::render_case(&mut renderer, case)?
        } else if case.component == "style" {
            crate::styles::render_case(&mut renderer, case)?
        } else {
            let list = gallery::draw(case)?;
            renderer.render_to_rgba(&list, case.width, case.height, 1.0)?
        };
        let path = out.join(case.relative());
        fs::create_dir_all(path.parent().unwrap())?;
        bbt_render::write_png(&path, &rgba, case.width, case.height)?;
        if (i + 1) % 100 == 0 {
            println!("rendered {}/{}", i + 1, cases.len());
        }
    }
    fs::create_dir_all(out)?;
    fs::write(out.join("manifest.json"), serde_json::to_vec_pretty(cases)?)?;
    println!("rendered {} PNGs to {}", cases.len(), out.display());
    Ok(cases.len())
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn compare(root: &Path, cases: &[Case]) -> Result<()> {
    compare_paths(root, cases, "web", "native")
}
pub fn compare_paths(
    root: &Path,
    cases: &[Case],
    web_prefix: &str,
    native_prefix: &str,
) -> Result<()> {
    // A relative reference containing `..` still needs this directory to exist
    // when the OS traverses it, including on the first gallery run.
    fs::create_dir_all(root)?;
    struct Row {
        case: Case,
        score: f64,
        error: f64,
        status: String,
        diff: Option<String>,
    }
    let mut rows = Vec::new();
    for case in cases {
        let rel = case.relative();
        let web = root.join(web_prefix).join(&rel);
        let native = root.join(native_prefix).join(&rel);
        let mut row = Row {
            case: case.clone(),
            score: 1.0,
            error: 1.0,
            status: String::new(),
            diff: None,
        };
        if !web.exists() {
            row.status = "missing web reference".into();
        } else if !native.exists() {
            row.status = "missing native render".into();
        } else {
            let (w, h, a) = crate::read_png(&web)?;
            let (nw, nh, b) = crate::read_png(&native)?;
            if (w, h) != (nw, nh) {
                row.status = format!("size mismatch: web {w}×{h}, native {nw}×{nh}");
            } else {
                let result = diff::compare(&a, &b)?;
                row.score = result.changed_fraction;
                row.error = result.mean_error;
                row.status = if row.score > diff::CHANGED_BUDGET {
                    "review"
                } else {
                    "within tolerance"
                }
                .into();
                let dest = root.join("diff").join(&rel);
                fs::create_dir_all(dest.parent().unwrap())?;
                bbt_render::write_png(&dest, &result.rgba, w, h)?;
                row.diff = Some(format!("diff/{rel}"));
            }
        }
        rows.push(row);
    }
    rows.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(b.error.total_cmp(&a.error))
            .then(a.case.relative().cmp(&b.case.relative()))
    });
    fs::create_dir_all(root)?;
    let review = rows
        .iter()
        .filter(|r| r.score > diff::CHANGED_BUDGET)
        .count();
    let material_layer = cases.iter().all(|c| c.component == "materials");
    let style_layer = cases.iter().all(|c| c.component == "style");
    let atom_layer = cases
        .iter()
        .all(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "atoms"));
    let component_layer = cases.iter().all(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "components")
    });
    let sources = if component_layer {
        "L6 retained Box/Text/Stack from bbt-ui. Captured public UI constructor inputs are shared; child DOM rectangles are diagnostics only (layouts.json), never layout inputs. The captured CSS backdrop, diagnostic labels and external root origin/width are common fixtures. Native components compute child geometry, baselines and height through the shared solver, resolve Style, then emit an ordered draw/material plan. This isolates L6 composition from legacy modules and input controllers. Canonical key: components/<state>-<density>-<theme>."
    } else if atom_layer {
        "L5 stateless atom painters from bbt-ui. Current committed lab glyph/harness-icon pages are the reference (revision in capture.json). CSS outer placement, checker backdrop and diagnostic labels are common fixtures; glyphs/icons are hidden in that backdrop and painted natively. Native atoms own state colours, grade rings, waiting-kind marks and icon tint. CSS pulse is fixed at 0ms. This isolates L5 painting, not L6 layout or module composition. Canonical key: <page>/<state>-<density>-<theme>."
    } else if material_layer {
        "L2 named material shaders. The captured CSS backdrop, diagnostic text and outer square rectangles are common fixtures; only the four material surfaces are painted natively. This isolates L2 and does not assess native text/layout. Canonical key: materials/<state>-<density>-<theme>. L2/L3 captures use a seeded counter-hash random stream and fixed 450ms animation clock; the native emitter consumes the same seed and phase. Remaining canvas/SDF edge rasterisation differences are ranked below. See capture.json for the exact policy."
    } else if style_layer {
        "L3 Style, resolved from compile-time tokens plus serialisable overrides (styles.json). The CSS backdrop/diagnostic labels and external box origin/width are common fixtures; native code resolves height/padding and paints material, fill, border, shadow and text. This is a low-level review fixture, not an L6 component/layout or legacy module rebase. Chrome/fx composite in sRGB; text uses corrected linear coverage. Glow's random stream and 450ms clock are shared with the web capture. Remaining text-mask/border/particle AA differences are ranked below. Canonical key: style/<state>-<density>-<theme>."
    } else {
        "Pinned design baseline `ux-prototype-v2`; current lab/design revision recorded in capture.json. Post-v2 components are explicit fixture previews. Native terminal bodies are blank; glow, blur, cursor animation and input controllers are later briefs."
    };
    let mut md = format!(
        "# bboxybook parity\n\n{} cases; {} require review. Policy: channel tolerance {}, changed-pixel budget {:.1}%. Scores rank image review, not acceptance.\n\nSources: {}\n\n| Component / state / density / theme | Changed | Mean error | Status | Web | Native | Diff |\n|---|---:|---:|---:|---|---|---|\n",
        rows.len(),
        review,
        diff::CHANNEL_TOLERANCE,
        diff::CHANGED_BUDGET * 100.0,
        sources,
    );
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>bboxybook parity</title><style>body{background:#10121c;color:#eee;font:14px system-ui;margin:24px}a{color:#9bc3ff}table{width:100%;border-collapse:collapse}td,th{padding:8px;border-bottom:1px solid #383c55;text-align:left}img{max-width:100%;height:auto}article{margin:24px 0;padding:12px;background:#1a1d2b}section{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}h3{font-size:14px}figure{margin:0}figcaption{margin:6px 0;color:#aab}</style><h1>bboxybook parity</h1>",
    );
    if material_layer || style_layer || atom_layer || component_layer {
        write!(html, "<p>{}</p>", escape(sources))?;
    }
    if component_layer {
        write!(
            html,
            "<p><a href=layouts.json>Child layout snapshots</a> · <a href=gallery/index.html>Box materials/radii, Text and ScrollView gallery</a></p>"
        )?;
    }
    if style_layer {
        write!(
            html,
            "<p><a href=styles.json>Serialised Style records and resolved values</a></p>"
        )?;
    }
    write!(
        html,
        "<p>{} cases · {} require review · worst first. <a href=report.md>Iteration report</a>. Missing references are unproven. Tolerance {} / changed budget {:.1}%.</p><table><tr><th>Case</th><th>Changed</th><th>Mean error</th><th>Status</th></tr>",
        rows.len(),
        review,
        diff::CHANNEL_TOLERANCE,
        diff::CHANGED_BUDGET * 100.0
    )?;
    for (i, row) in rows.iter().enumerate() {
        let key = row.case.relative();
        write!(
            html,
            "<tr><td><a href='#case-{i}'>{}</a></td><td>{:.2}%</td><td>{:.4}</td><td>{}</td></tr>",
            escape(&key),
            row.score * 100.0,
            row.error,
            escape(&row.status)
        )?;
        if row.score > diff::CHANGED_BUDGET {
            writeln!(
                md,
                "| {} | {:.2}% | {:.4} | {} | [web]({}/{}) | [native]({}/{}) | {} |",
                key,
                row.score * 100.0,
                row.error,
                row.status,
                web_prefix,
                key,
                native_prefix,
                key,
                row.diff
                    .as_ref()
                    .map(|s| format!("[diff]({s})"))
                    .unwrap_or_else(|| "—".into())
            )?;
        }
    }
    html.push_str("</table>");
    for (i, row) in rows.iter().enumerate() {
        let key = row.case.relative();
        write!(
            html,
            "<article id='case-{i}'><h3>{} · {:.2}% · {}</h3><section>",
            escape(&key),
            row.score * 100.0,
            escape(&row.status)
        )?;
        for (label, path) in [
            ("web", format!("{web_prefix}/{key}")),
            ("native", format!("{native_prefix}/{key}")),
            ("diff", row.diff.clone().unwrap_or_default()),
        ] {
            if path.is_empty() {
                html.push_str("<figure>Unavailable</figure>");
            } else {
                write!(
                    html,
                    "<figure><figcaption>{label}</figcaption><a href='{}'><img loading=lazy src='{}'></a></figure>",
                    escape(&path),
                    escape(&path)
                )?;
            }
        }
        html.push_str("</section></article>");
    }
    fs::write(root.join("index.html"), html)?;
    fs::write(root.join("report.md"), md)?;
    println!(
        "parity: {} cases, {} review; {}",
        rows.len(),
        review,
        root.join("index.html").display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_report_resolves_relative_reference_directories() -> Result<()> {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let temp =
            std::env::temp_dir().join(format!("bboxybook-report-{}-{suffix}", std::process::id()));
        let mut case = gallery::cases().remove(0);
        case.width = 1;
        case.height = 1;
        let image = temp.join("images").join(case.relative());
        fs::create_dir_all(image.parent().unwrap())?;
        bbt_render::write_png(&image, &[17, 83, 201, 255], 1, 1)?;
        let report = temp.join("review");
        assert!(!report.exists());
        compare_paths(&report, &[case], "../images", "../images")?;
        assert!(
            fs::read_to_string(report.join("report.md"))?.contains("1 cases; 0 require review")
        );
        fs::remove_dir_all(temp)?;
        Ok(())
    }
}
