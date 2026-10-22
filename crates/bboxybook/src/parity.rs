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
        let rgba = if case.fixture.as_ref().is_some_and(|f| f["layer"] == "panes") {
            crate::panes::render_case(&mut renderer, case)?
        } else if case
            .fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "swarms")
        {
            crate::swarms::render_case(&mut renderer, case)?
        } else if case.fixture.as_ref().is_some_and(|f| f["layer"] == "tabs") {
            crate::tabs::render_case(&mut renderer, case)?
        } else if case
            .fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "seat-row")
        {
            crate::seat_rows::render_case(&mut renderer, case)?
        } else if case
            .fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "components")
        {
            crate::components::render_case(&mut renderer, case)?
        } else if case
            .fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "floating-surfaces")
        {
            crate::overlays::render_surface_case(&mut renderer, case)?
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
    let surfaces = crate::overlays::surface_snapshots(cases)?;
    if !surfaces.is_empty() {
        fs::write(
            out.join("surface-layouts.json"),
            serde_json::to_vec_pretty(&surfaces)?,
        )?;
    }
    let mut snapshots = crate::components::snapshots(cases)?;
    snapshots.extend(crate::styles::snapshots(cases)?);
    snapshots.extend(crate::atoms::snapshots(cases)?);
    snapshots.extend(crate::seat_rows::snapshots(cases)?);
    snapshots.extend(crate::tabs::snapshots(cases)?);
    snapshots.extend(crate::swarms::snapshots(cases)?);
    snapshots.extend(crate::panes::snapshots(cases)?);
    if !snapshots.is_empty() {
        fs::write(
            out.join("layouts.json"),
            serde_json::to_vec_pretty(&snapshots)?,
        )?;
        println!(
            "native/web layout snapshots: {}",
            out.join("layouts.json").display()
        );
    }
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
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct CasePolicy {
    pub historical: bool,
    pub text: bool,
    pub channel_tolerance: u8,
    pub changed_pixel_budget: f64,
    pub geometry_tolerance_px: Option<f64>,
    pub scope: &'static str,
}
impl CasePolicy {
    pub fn for_case(case: &Case) -> Result<Self> {
        fn contains_text(v: &serde_json::Value) -> bool {
            v["kind"] == "text"
                || v["children"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(contains_text))
        }
        let f = case.fixture.as_ref();
        let surface_only = f.is_some_and(|f| f["layer"] == "floating-surfaces");
        let text = if case.component == "materials" || surface_only {
            false
        } else if case.component == "style"
            || f.is_some_and(|f| {
                matches!(
                    f["layer"].as_str(),
                    Some("seat-row" | "tabs" | "swarms" | "panes")
                )
            })
        {
            true
        } else if f.is_some_and(|f| f["layer"] == "components") {
            contains_text(&f.unwrap()["component"]["tree"])
        } else if f.is_some_and(|f| f["layer"] == "atoms") {
            f.unwrap()["atoms"]
                .as_array()
                .is_some_and(|a| a.iter().any(|a| a["kind"] == "glyph"))
        } else {
            gallery::draw(case)?
                .primitives
                .iter()
                .any(|p| matches!(p, bbt_ui_model::draw::Prim::Text { .. }))
        };
        Ok(Self {
            historical: f.is_some_and(|f| f["reference"] == "baseline"),
            text,
            channel_tolerance: diff::CHANNEL_TOLERANCE,
            changed_pixel_budget: if text { 0.02 } else { diff::CHANGED_BUDGET },
            geometry_tolerance_px: text.then_some(0.5),
            scope: if f.is_some_and(|f| f["layer"] == "panes") {
                "Pane/Strip; supplied terminal spans, notes and adjacent SeatRows retained as shared context"
            } else if f.is_some_and(|f| f["layer"] == "swarms") {
                "SwarmCard/Sidebar; pane previews retained as shared context"
            } else if f.is_some_and(|f| f["layer"] == "tabs") {
                "TabRow and MiniGrid compositions, including expanded SeatRows"
            } else if f.is_some_and(|f| f["layer"] == "seat-row") {
                "SeatRow/subagent/task/artifact content; row-menu overlays excluded"
            } else if f.is_some_and(|f| f["layer"] == "atoms") {
                "L5 Glyph/Icon in caller-owned slots; child text geometry independently measured"
            } else if surface_only {
                "container surfaces only; module children hidden"
            } else {
                "native component content"
            },
        })
    }
    fn label(self) -> String {
        let scope = if self.scope.starts_with("container") {
            "surfaces only · "
        } else {
            ""
        };
        let label = if self.text {
            "text: 2% + rect/baseline ≤0.5px".into()
        } else {
            format!("{scope}pixels: 0.1%")
        };
        if self.historical {
            format!("historical v2 · {label}")
        } else {
            label
        }
    }
    fn status(self, score: f64, geometry: Option<&Geometry>) -> &'static str {
        if let Some(limit) = self.geometry_tolerance_px {
            let Some(g) = geometry.filter(|g| g.complete && g.rects > 0 && g.baselines > 0) else {
                return "unproven geometry";
            };
            if g.max_rect_error > limit || g.max_baseline_error > limit {
                return "review geometry";
            }
        }
        if score > self.changed_pixel_budget {
            "review pixels"
        } else {
            "within policy"
        }
    }
}
#[derive(Debug, Default, Clone, serde::Serialize)]
struct Geometry {
    complete: bool,
    rects: usize,
    baselines: usize,
    max_rect_error: f64,
    max_baseline_error: f64,
}
impl Geometry {
    fn from_tree(tree: &serde_json::Value) -> Self {
        fn visit(v: &serde_json::Value, g: &mut Geometry) {
            let pair = v["native"].as_array().zip(v["web"].as_array());
            if let Some((n, w)) = pair.filter(|(n, w)| n.len() == 4 && w.len() == 4) {
                g.rects += 1;
                for (n, w) in n.iter().zip(w) {
                    if let Some((n, w)) = n
                        .as_f64()
                        .zip(w.as_f64())
                        .filter(|(n, w)| n.is_finite() && w.is_finite())
                    {
                        g.max_rect_error = g.max_rect_error.max((n - w).abs());
                    } else {
                        g.complete = false;
                    }
                }
            } else {
                g.complete = false;
            }
            if v["kind"] == "text" {
                if let Some((n, w)) = v["baseline"]["native"]
                    .as_f64()
                    .zip(v["baseline"]["web"].as_f64())
                    .filter(|(n, w)| n.is_finite() && w.is_finite())
                {
                    g.baselines += 1;
                    g.max_baseline_error = g.max_baseline_error.max((n - w).abs());
                } else {
                    g.complete = false;
                }
            }
            if let Some(children) = v["children"].as_array() {
                for child in children {
                    visit(child, g);
                }
            }
        }
        let mut g = Self {
            complete: true,
            ..Default::default()
        };
        visit(tree, &mut g);
        g
    }
    fn label(&self) -> String {
        if self.complete {
            format!(
                "rect {:.3}px / baseline {:.3}px ({} / {})",
                self.max_rect_error, self.max_baseline_error, self.rects, self.baselines
            )
        } else {
            "incomplete".into()
        }
    }
}
pub fn compare_paths(
    root: &Path,
    cases: &[Case],
    web_prefix: &str,
    native_prefix: &str,
) -> Result<()> {
    fs::create_dir_all(root)?;
    let surface_path = root.join(native_prefix).join("surface-layouts.json");
    if surface_path.exists() {
        fs::write(root.join("surface-layouts.json"), fs::read(&surface_path)?)?;
    }
    let layout_path = root.join(native_prefix).join("layouts.json");
    let layout_path = if layout_path.exists() {
        Some(layout_path)
    } else {
        let p = root.join("layouts.json");
        p.exists().then_some(p)
    };
    let layouts: Vec<serde_json::Value> = if let Some(p) = &layout_path {
        serde_json::from_slice(&fs::read(p)?)?
    } else {
        vec![]
    };
    if let Some(p) = layout_path.filter(|p| *p != root.join("layouts.json")) {
        fs::copy(p, root.join("layouts.json"))?;
    }
    struct Row {
        case: Case,
        policy: CasePolicy,
        geometry: Option<Geometry>,
        score: f64,
        error: f64,
        status: String,
        diff: Option<String>,
    }
    let mut rows = vec![];
    for case in cases {
        let rel = case.relative();
        let web = root.join(web_prefix).join(&rel);
        let native = root.join(native_prefix).join(&rel);
        let geometry = layouts
            .iter()
            .find(|v| v["key"] == rel)
            .map(|v| Geometry::from_tree(&v["tree"]));
        let mut row = Row {
            case: case.clone(),
            policy: CasePolicy::for_case(case)?,
            geometry,
            score: 1.,
            error: 1.,
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
                row.status = row.policy.status(row.score, row.geometry.as_ref()).into();
                let dest = root.join("diff").join(&rel);
                fs::create_dir_all(dest.parent().unwrap())?;
                bbt_render::write_png(&dest, &result.rgba, w, h)?;
                row.diff = Some(format!("diff/{rel}"));
            }
        }
        rows.push(row);
    }
    rows.sort_by(|a, b| {
        a.policy
            .historical
            .cmp(&b.policy.historical)
            .then((b.status != "within policy").cmp(&(a.status != "within policy")))
            .then(b.score.total_cmp(&a.score))
            .then(b.error.total_cmp(&a.error))
            .then(a.case.relative().cmp(&b.case.relative()))
    });
    let review = rows
        .iter()
        .filter(|r| !r.policy.historical && r.status != "within policy")
        .count();
    let historical = rows.iter().filter(|r| r.policy.historical).count();
    let component_layer = cases.iter().all(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "components")
    });
    let sources = if cases.iter().any(|c| {
        c.fixture
            .as_ref()
            .is_some_and(|f| f["layer"] == "floating-surfaces")
    }) {
        "L6 floating components plus explicitly marked surface-only launcher/document shells. Module children are hidden; the unrendered content is an external fixed-height slot. Full web context is retained in context/. Native computes shell position, width, fill, blur, shadow and border. surface-layouts.json records independent rectangle diagnostics; module contents are not claimed as implemented."
    } else if cases
        .iter()
        .all(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "seat-row"))
    {
        "L7 SeatRow and nested subagent/task/artifact compositions. Current lab states use committed HEAD; the 9-state legacy matrix uses ux-prototype-v2. Constructor data and external parent flow origin/width/gap are shared, never child geometry. Notes remain common context; separate row-menu overlays are hidden on both sides. layouts.json independently checks every exported part and text baseline."
    } else if cases
        .iter()
        .all(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "tabs"))
    {
        "L7 TabRow and MiniGrid retained compositions. Current lab states use committed HEAD; legacy states use ux-prototype-v2. Public constructor data, external parent bounds and grid viewport sizes are shared, never child geometry. Every part and text baseline is independently checked in layouts.json."
    } else if cases
        .iter()
        .all(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "panes"))
    {
        "L7 Pane/Strip compositions. Seat/options and semantic terminal span content are shared fixture inputs; caller origin/width and min-height define the viewport. Native solves frame height, strip, text, clipping and reconnect placement with CoreText metrics. Child DOM bounds and independent text-range baselines are diagnostics only. Notes and adjacent SeatRows stay shared context. CSS/canvas pose is fixed at 0ms; full web content remains visible."
    } else if cases
        .iter()
        .all(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "swarms"))
    {
        "L7 SwarmCard and Sidebar compositions. Public constructor data and external flow/viewport bounds are shared. Child DOM rectangles and fonts are diagnostic only. Current space-header/sidebar pages use committed HEAD; band/sidebar-v2 retain ux-prototype-v2. Pane previews remain shared web context and are not claimed as native module content. layouts.json checks every exported part and baseline."
    } else if cases
        .iter()
        .all(|c| c.fixture.as_ref().is_some_and(|f| f["layer"] == "atoms"))
    {
        "L5 Glyph/Icon atoms in caller-owned slots. Child text and waiting-kind bounds/baselines come from the same emitted native primitives and CoreText metrics as paint; browser rectangles and inline-baseline probes are independent diagnostics."
    } else if component_layer {
        "L6 retained components. Public constructor inputs and external host origin/width are shared. Child DOM rectangles and first-line baselines are diagnostics only; native layout uses the shared solver and CoreText metrics."
    } else if cases.iter().all(|c| c.component == "materials") {
        "L2 effect-only materials. CSS backdrop and diagnostic labels are common fixtures; native paints material surfaces. No native text is compared."
    } else if cases.iter().all(|c| c.component == "style") {
        "L3 Style paints material, fill, border, shadow and text. Native box/text rects and baselines in layouts.json use the same paint-frame helpers and CoreText advances as the render pass. CSS metrics remain independent diagnostics; no native child bounds are copied from them."
    } else {
        "Pinned base ux-prototype-v2; supplemental committed lab revision and capture policy in capture.json. Legacy text cases without geometry are unproven, regardless of their pixel score."
    };
    let policy = "Per-case policy: channel tolerance 8; text-bearing cases require every rect and Text baseline within 0.5px plus ≤2% changed pixels; pixel-only cases keep ≤0.1%. Missing geometry never passes. The policy gates evidence, not human acceptance. Frozen v2 cases are labelled historical diagnostics; raw results remain visible and do not gate the current design.";
    let mut md = format!(
        "# bboxybook parity\n\n{} cases; {} require review. {}\n\nSources: {}\n\n| Case | Policy | Geometry | Changed | Mean error | Status | Web | Native | Diff |\n|---|---|---|---:|---:|---|---|---|---|\n",
        rows.len(),
        review,
        policy,
        sources
    );
    let mut html = format!(
        "<!doctype html><meta charset=utf-8><title>bboxybook parity</title><style>body{{background:#10121c;color:#eee;font:14px system-ui;margin:24px}}a{{color:#9bc3ff}}table{{width:100%;border-collapse:collapse}}td,th{{padding:8px;border-bottom:1px solid #383c55;text-align:left}}img{{max-width:100%;height:auto}}article{{margin:24px 0;padding:12px;background:#1a1d2b}}section{{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}}h3{{font-size:14px}}figure{{margin:0}}figcaption{{margin:6px 0;color:#aab}}</style><h1>bboxybook parity</h1><p>{}</p><p>{}</p><p>{} cases · {} require review · review cases first, then worst pixels. <a href=report.md>Report</a> · <a href=report.json>Per-case policy/results JSON</a></p>",
        escape(policy),
        escape(sources),
        rows.len(),
        review
    );
    if historical > 0 {
        write!(
            html,
            "<p>{historical} historical v2 cases are diagnostic only.</p>"
        )?;
    }
    if component_layer || cases.iter().all(|c| c.component == "style") {
        html.push_str("<p><a href=layouts.json>Rects and baselines</a></p>");
        if root.join("gallery/index.html").exists() {
            html.push_str("<p><a href=gallery/index.html>Native gallery</a></p>");
        }
    }
    if cases.iter().all(|c| c.component == "style") && root.join("styles.json").exists() {
        html.push_str("<p><a href=styles.json>Style records</a></p>");
    }
    html.push_str("<table><tr><th>Case</th><th>Policy</th><th>Geometry</th><th>Changed</th><th>Mean error</th><th>Status</th></tr>");
    for (i, row) in rows.iter().enumerate() {
        let key = row.case.relative();
        let geometry = row
            .geometry
            .as_ref()
            .map(Geometry::label)
            .unwrap_or_else(|| {
                if row.policy.text {
                    "missing".into()
                } else {
                    "not required".into()
                }
            });
        write!(
            html,
            "<tr><td><a href='#case-{i}'>{}</a></td><td>{}</td><td>{}</td><td>{:.2}%</td><td>{:.4}</td><td>{}</td></tr>",
            escape(&key),
            escape(&row.policy.label()),
            escape(&geometry),
            row.score * 100.,
            row.error,
            escape(&row.status)
        )?;
        writeln!(
            md,
            "| {} | {} | {} | {:.2}% | {:.4} | {} | [web]({}/{}) | [native]({}/{}) | {} |",
            key,
            row.policy.label(),
            geometry,
            row.score * 100.,
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
    html.push_str("</table>");
    if historical > 0 {
        writeln!(
            md,
            "\n{historical} historical v2 cases are diagnostic only.\n"
        )?;
    }
    for (i, row) in rows.iter().enumerate() {
        let key = row.case.relative();
        write!(
            html,
            "<article id='case-{i}'><h3>{} · {:.2}% · {} · {}</h3><section>",
            escape(&key),
            row.score * 100.,
            escape(&row.policy.label()),
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
        html.push_str("</section>");
        if row.policy.scope.starts_with("container") {
            let context = Path::new(web_prefix)
                .parent()
                .unwrap_or(Path::new(""))
                .join("context")
                .join(&key);
            if root.join(&context).exists() {
                write!(
                    html,
                    "<p><a href='{}'>Full web module context (reference only)</a></p>",
                    escape(&context.to_string_lossy())
                )?;
            }
        }
        html.push_str("</article>");
    }
    let records:Vec<_>=rows.iter().map(|r|serde_json::json!({"key":r.case.relative(),"policy":r.policy,"geometry":r.geometry,"changed_fraction":r.score,"mean_error":r.error,"status":r.status})).collect();
    fs::write(
        root.join("report.json"),
        serde_json::to_vec_pretty(&records)?,
    )?;
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
        let mut case = crate::materials::cases().remove(0);
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

    #[test]
    fn historical_diffs_stay_visible_without_gating_the_current_design() -> Result<()> {
        let temp = std::env::temp_dir().join(format!(
            "bboxybook-history-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let mut case = crate::materials::cases().remove(0);
        case.width = 1;
        case.height = 1;
        for (side, pixel) in [("web", [0, 0, 0, 255]), ("native", [255, 255, 255, 255])] {
            let path = temp.join(side).join(case.relative());
            fs::create_dir_all(path.parent().unwrap())?;
            bbt_render::write_png(&path, &pixel, 1, 1)?;
        }
        case.fixture = Some(serde_json::json!({"reference":"baseline"}));
        compare_paths(
            &temp.join("history"),
            &[case.clone()],
            "../web",
            "../native",
        )?;
        let rows: serde_json::Value =
            serde_json::from_slice(&fs::read(temp.join("history/report.json"))?)?;
        assert_eq!(rows[0]["policy"]["historical"], true);
        assert_eq!(rows[0]["status"], "review pixels");
        assert_eq!(rows[0]["changed_fraction"], 1.);
        assert!(
            fs::read_to_string(temp.join("history/report.md"))?
                .contains("1 cases; 0 require review")
        );
        case.fixture = None;
        compare_paths(&temp.join("current"), &[case], "../web", "../native")?;
        assert!(
            fs::read_to_string(temp.join("current/report.md"))?
                .contains("1 cases; 1 require review")
        );
        fs::remove_dir_all(temp)?;
        Ok(())
    }

    #[test]
    fn text_policy_requires_rects_and_baselines_even_when_pixels_pass() {
        let p = CasePolicy {
            historical: false,
            text: true,
            channel_tolerance: 8,
            changed_pixel_budget: 0.02,
            geometry_tolerance_px: Some(0.5),
            scope: "native component content",
        };
        let tree = serde_json::json!({"kind":"text","native":[0.,0.,100.,20.],"web":[0.,0.,100.5,20.],"baseline":{"native":15.,"web":15.5},"children":[]});
        let g = Geometry::from_tree(&tree);
        assert_eq!(p.status(0.02, Some(&g)), "within policy");
        assert_eq!(p.status(0.02001, Some(&g)), "review pixels");
        assert_eq!(p.status(0., None), "unproven geometry");
        let mut bad = tree.clone();
        bad["baseline"]["web"] = serde_json::json!(15.501);
        assert_eq!(
            p.status(0.01, Some(&Geometry::from_tree(&bad))),
            "review geometry"
        );
        bad["baseline"] = serde_json::Value::Null;
        assert_eq!(
            p.status(0., Some(&Geometry::from_tree(&bad))),
            "unproven geometry"
        );
    }

    #[test]
    fn one_percent_pixel_error_passes_only_the_text_case_with_measured_geometry() -> Result<()> {
        let temp = std::env::temp_dir().join(format!(
            "bboxybook-policy-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let mut text = crate::components::cases().remove(0);
        text.fixture =
            Some(serde_json::json!({"layer":"components","component":{"tree":{"kind":"text"}}}));
        let pixel = crate::materials::cases().remove(0);
        for case in [&text, &pixel] {
            for (folder, changed) in [("web", false), ("native", true)] {
                let path = temp.join(folder).join(case.relative());
                fs::create_dir_all(path.parent().unwrap())?;
                let mut bytes = [0, 0, 0, 255].repeat(100);
                if changed {
                    bytes[0..4].copy_from_slice(&[255; 4]);
                }
                bbt_render::write_png(&path, &bytes, 100, 1)?;
            }
        }
        fs::write(
            temp.join("native/layouts.json"),
            serde_json::to_vec(
                &serde_json::json!([{"key":text.relative(),"tree":{"kind":"text","native":[0,0,100,1],"web":[0,0,100,1],"baseline":{"native":1,"web":1},"children":[]}}]),
            )?,
        )?;
        compare_paths(&temp, &[text, pixel], "web", "native")?;
        let rows: serde_json::Value = serde_json::from_slice(&fs::read(temp.join("report.json"))?)?;
        let rows = rows.as_array().unwrap();
        assert!(rows.iter().any(|r| r["policy"]["text"] == true
            && r["policy"]["changed_pixel_budget"] == 0.02
            && r["status"] == "within policy"));
        assert!(rows.iter().any(|r| r["policy"]["text"] == false
            && r["policy"]["changed_pixel_budget"] == 0.001
            && r["status"] == "review pixels"));
        assert!(fs::read_to_string(temp.join("report.md"))?.contains("2% + rect/baseline"));
        fs::remove_dir_all(temp)?;
        Ok(())
    }
}
