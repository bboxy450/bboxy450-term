// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private production transcript expansion, layout and Metal paint. No backend or window.
#![cfg(target_os = "macos")]
#![allow(
    clippy::disallowed_methods,
    reason = "owned unattended offscreen fixture"
)]
use bboxy450_harness::{
    frontends::gui::{
        bridge::FrameState,
        model::Transcript,
        options::Options,
        smoke::capture_frame,
        view::{HarnessView, find, layout_text},
    },
    protocol::Envelope,
};
use bbt_render::chrome::ChromeRenderer;
use bbt_ui::components as c;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};

const ITEM: &str = "00000000000000000000000003";

fn transcript(body: Value) -> Transcript {
    let kind = body["kind"].as_str().unwrap();
    let head = (kind == "tool_call").then(|| json!({"kind":"tool_call","call_id":"private-mcp","name":"mcp.bboxy.knowledge_fetch","tool_kind":"other"}));
    let events = [
        json!({"type":"session_started","meta":{"created_ms":0,"cwd":"/private/fixture","fork":null,"harness":"fixture","profile":"private","protocol":{"major":1,"minor":1},"session":"00000000000000000000000001"}}),
        json!({"type":"turn_started","turn":"00000000000000000000000002"}),
        json!({"type":"item_started","item":ITEM,"kind":kind,"head":head,"parent":null,"turn":"00000000000000000000000002"}),
        json!({"type":"item_completed","item":ITEM,"body":body,"status":{"status":"completed"}}),
    ];
    let mut model = Transcript::default();
    for (i, event) in events.into_iter().enumerate() {
        let line = json!({"v":{"major":1,"minor":1},"session":"00000000000000000000000001","epoch":1,"seq":i+1,"at":i+1,"ts_ms":i+1,"cause":null,"event":event}).to_string();
        model.apply(&Envelope::from_line(&line).unwrap());
    }
    model
}

fn png(path: &Path, width: u32, height: u32, pixels: &[u8]) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}

// keeps: rich-text-safety — real native MCP disclosure cannot terminate the GUI renderer.
#[test]
#[ignore = "KNOWN DEFECT: pre-existing on main; fix: tasks/2026-11-06-mux-preexisting-test-failures"]
fn rich_text_safety_private_mcp_expansion_and_inline_paint() {
    if std::env::var_os("BBOXY_RICH_TEXT_PRIVATE").is_none() {
        let home = tempfile::Builder::new()
            .prefix("rich-text-home-")
            .tempdir_in("/tmp")
            .unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "rich_text_safety::rich_text_safety_private_mcp_expansion_and_inline_paint",
                "--ignored",
                "--nocapture",
            ])
            .env_clear()
            .env("HOME", home.path())
            .env("PATH", "/usr/bin:/bin")
            .env("BBOXY_RICH_TEXT_PRIVATE", "1");
        if let Some(out) = std::env::var_os("BBOXY_RICH_TEXT_OUT") {
            child.env("BBOXY_RICH_TEXT_OUT", out);
        }
        let out = child.output().unwrap();
        assert!(
            out.status.success(),
            "private native child {}:\n{}\n{}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        print!("{}", String::from_utf8_lossy(&out.stdout));
        return;
    }
    let output = std::env::var_os("BBOXY_RICH_TEXT_OUT").map(std::path::PathBuf::from);
    if let Some(dir) = &output {
        std::fs::create_dir_all(dir).unwrap();
    }
    bbt_rt::handle().block_on(bboxy450_harness::runtime::blocking(move || {
        let text = format!("{{\"body\":\"{}TAIL_é_👩🏽‍💻_許\"}}", "x".repeat(6000));
        let model = transcript(json!({"kind":"tool_call","call_id":"private-mcp","name":"mcp.bboxy.knowledge_fetch","tool_kind":"other","input_raw":"{\"locator\":\"private\"}","output":[{"type":"text","text":text}],"locations":[],"spill":null}));
        let mut state = FrameState { transcript: Arc::new(model), ready: true, replay: true, ..Default::default() };
        let mut view = HarnessView::default();
        let mut options = Options::parse(Vec::<String>::new()).unwrap();
        options.capture = Some([640, 420]); options.no_chrome = true;
        let mut renderer = ChromeRenderer::new(bboxy450_harness::frontends::gui::assets::icons()).unwrap();
        let collapsed = capture_frame(&mut view, &mut state, &options).unwrap();
        let closed = find(collapsed.layout.as_ref().unwrap(), &format!("tool/{ITEM}/body")).is_none();
        let before = renderer.render_to_rgba(&collapsed.list, 640, 420, 1.).unwrap();
        options.expand_all = true;
        let frame = capture_frame(&mut view, &mut state, &options).unwrap();
        let layout = frame.layout.as_ref().unwrap();
        assert!(find(layout, &format!("tool/{ITEM}/body")).is_some(), "real tool output disclosure must expand");
        assert!(layout_text(layout).contains(&text), "expanded selectable layout retains exact source and Unicode tail");
        let pixels = renderer.render_to_rgba(&frame.list, 640, 420, 1.).expect("expanded MCP native paint must not exit with chrome text line too large");
        assert_ne!(before, pixels, "expansion must paint content");
        assert!(frame.list.primitives.iter().any(|p| matches!(p, bbt_ui_model::draw::Prim::Text {text: t, max_width: None, ..} if t.contains("TAIL_é_👩🏽‍💻_許"))), "full literal tail reaches the native renderer");
        if let Some(dir) = &output { png(&dir.join("expanded-mcp.png"), 640, 420, &pixels); }
        println!("MCP disclosure: initially_closed={closed}; exact_bytes={}; native_render=ok; whole_line_masks={}", text.len(), renderer.text_cache_stats().whole_line_masks);
        inline_paint(&mut renderer, output.as_deref());
    }));
}

fn inline_paint(renderer: &mut ChromeRenderer, output: Option<&Path>) {
    let source = "Run `make verify-acp-controls`, then `é👩🏽‍💻許`!";
    for width in [240, 640] {
        for zoom in [90, 100, 160] {
            let mut state = FrameState {
                transcript: Arc::new(transcript(
                    json!({"kind":"agent_message","markdown":source}),
                )),
                ready: true,
                replay: true,
                ..Default::default()
            };
            let mut view = HarnessView::default();
            let mut options = Options::parse(Vec::<String>::new()).unwrap();
            options.capture = Some([width, 420]);
            options.no_chrome = true;
            options.zoom = Some(bboxy450_harness::frontends::gui::zoom::Zoom::from_percent(
                zoom,
            ));
            let frame = capture_frame(&mut view, &mut state, &options).unwrap();
            let layout = frame.layout.as_ref().unwrap();
            let mut capsules = vec![];
            capsules_in(layout, &mut capsules);
            assert!(
                !capsules.is_empty(),
                "inline code must have a semantic bounded capsule"
            );
            let mut code = String::new();
            for capsule in capsules {
                assert!(
                    capsule.rect.x >= 0. && capsule.rect.right() <= width as f32 + 0.01,
                    "zoomed capsule escaped viewport: {:?}",
                    capsule.rect
                );
                exact_text(capsule, &mut code);
                check_capsule(capsule);
                let mut isolated = bbt_ui_model::draw::DrawList::default();
                for op in c::paint(capsule, bboxy450_harness::frontends::gui::view::context()) {
                    match op {
                        c::Paint::Draw(list) | c::Paint::Underlay(list) => {
                            isolated.primitives.extend(list.primitives)
                        }
                        _ => panic!("inline code must remain a flat shared-toolkit capsule"),
                    }
                }
                let pixels = renderer.render_to_rgba(&isolated, width, 420, 1.).unwrap();
                let rect = capsule.rect;
                for (index, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
                    let x = (index % width as usize) as f32;
                    let y = (index / width as usize) as f32;
                    if x + 1. < rect.x || x > rect.right() || y + 1. < rect.y || y > rect.bottom() {
                        assert_eq!(
                            pixel[3], 0,
                            "stray inline-code ink/backdrop at ({x},{y}), {rect:?}"
                        );
                    }
                }
            }
            assert!(
                code.contains("make verify-acp-controls")
                    || code.replace('\n', "").contains("make verify-acp-controls")
            );
            assert!(
                !code.contains('\u{2009}'),
                "padding must not become copied source"
            );
            for scale in [1., 2.] {
                let pixels = renderer
                    .render_to_rgba(
                        &frame.list,
                        (width as f32 * scale) as u32,
                        (420. * scale) as u32,
                        scale,
                    )
                    .unwrap();
                if ((zoom == 100 && scale == 1.) || (zoom == 160 && scale == 2. && width == 240))
                    && let Some(dir) = output
                {
                    png(
                        &dir.join(format!("inline-{width}-zoom{zoom}-scale{scale}.png")),
                        (width as f32 * scale) as u32,
                        (420. * scale) as u32,
                        &pixels,
                    );
                }
            }
        }
    }
}

fn capsules_in<'a>(node: &'a c::Layout, out: &mut Vec<&'a c::Layout>) {
    if let c::LayoutKind::Named(name) = &node.kind
        && name.contains("/code")
        && name.ends_with("/capsule")
    {
        out.push(node);
        return;
    }
    for child in &node.children {
        capsules_in(child, out);
    }
}

fn check_capsule(node: &c::Layout) {
    fn runs<'a>(node: &'a c::Layout, out: &mut Vec<&'a c::PositionedRun>) {
        if let c::LayoutKind::Text { runs, .. } = &node.kind {
            out.extend(runs);
        }
        for child in &node.children {
            runs(child, out);
        }
    }
    let mut content = vec![];
    runs(node, &mut content);
    assert!(!content.is_empty());
    assert!(
        (content[0].baseline - (node.rect.y + node.baseline)).abs() < 0.01,
        "capsule and first text line share a baseline"
    );
    for run in content {
        assert!(
            run.rect.x >= node.rect.x && run.rect.right() <= node.rect.right() + 0.01,
            "capsule horizontal padding: {:?} {:?}",
            node.rect,
            run.rect
        );
        assert!(
            run.baseline >= node.rect.y && run.baseline < node.rect.bottom(),
            "wrapped text baseline remains inside capsule"
        );
    }
}

fn exact_text(node: &c::Layout, out: &mut String) {
    if let c::LayoutKind::Text { runs, .. } = &node.kind {
        for run in runs {
            out.push_str(&run.text);
        }
    }
    for child in &node.children {
        exact_text(child, out);
    }
}
